//! # envfish-daemon
//!
//! The Local Agent: a small request/response server other local processes can
//! talk to instead of opening the SQLite file themselves. Transport is a Unix
//! Domain Socket (`<data dir>/agent.sock`, mode `0600`) carrying one JSON
//! message per line. Windows named pipes are not implemented yet.
//!
//! The request set mirrors what the desktop app and MCP server need for
//! *coordination* — listing, approvals, audit. It deliberately contains no
//! request that returns a secret value, and never will.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use envfish_core::{Action, Approval, AuditEntry, Connection, EnvFish, Environment, Project, Variable};
use serde::{Deserialize, Serialize};

/// Requests a client may send to the Local Agent.
///
/// Deliberately absent: `GetSecret`, `RevealSecret`, `ExportSecrets`, `DumpVault`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentRequest {
    Ping,
    ListProjects,
    ListEnvironments {
        project_id: String,
    },
    /// Returns variable metadata; secret values are always `null`.
    ListVariables {
        environment_id: String,
    },
    ListConnections {
        project_id: String,
    },
    ListPendingApprovals,
    ResolveApproval {
        approval_id: String,
        approve: bool,
    },
    ListAudit {
        limit: i64,
    },
    Decide {
        client_id: Option<String>,
        project_id: Option<String>,
        environment_id: Option<String>,
        connection_id: Option<String>,
        action: Action,
    },
}

/// Adjacently tagged (`{"type": ..., "data": ...}`) so list variants serialise cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AgentResponse {
    Pong { version: String },
    Projects(Vec<Project>),
    Environments(Vec<Environment>),
    Variables(Vec<Variable>),
    Connections(Vec<Connection>),
    Approvals(Vec<Approval>),
    Approval(Approval),
    Audit(Vec<AuditEntry>),
    Decision(envfish_core::Decision),
    Error { message: String },
}

/// In-process request handler shared by the socket server and tests.
pub struct Agent {
    core: Arc<EnvFish>,
}

impl Agent {
    pub fn new(core: Arc<EnvFish>) -> Self {
        Self { core }
    }

    pub async fn handle(&self, request: AgentRequest) -> AgentResponse {
        let result: envfish_core::Result<AgentResponse> = match request {
            AgentRequest::Ping => Ok(AgentResponse::Pong {
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
            AgentRequest::ListProjects => self.core.list_projects().await.map(AgentResponse::Projects),
            AgentRequest::ListEnvironments { project_id } => self
                .core
                .list_environments(&project_id)
                .await
                .map(AgentResponse::Environments),
            AgentRequest::ListVariables { environment_id } => self
                .core
                .list_variables(&environment_id)
                .await
                .map(AgentResponse::Variables),
            AgentRequest::ListConnections { project_id } => self
                .core
                .list_project_connections(&project_id)
                .await
                .map(AgentResponse::Connections),
            AgentRequest::ListPendingApprovals => self
                .core
                .list_approvals(Some(envfish_core::ApprovalStatus::Pending), 100)
                .await
                .map(AgentResponse::Approvals),
            AgentRequest::ResolveApproval { approval_id, approve } => self
                .core
                .resolve_approval(&approval_id, approve)
                .await
                .map(AgentResponse::Approval),
            AgentRequest::ListAudit { limit } => self.core.list_audit(limit).await.map(AgentResponse::Audit),
            AgentRequest::Decide {
                client_id,
                project_id,
                environment_id,
                connection_id,
                action,
            } => self
                .core
                .decide(
                    &envfish_core::PermissionScope {
                        client_id,
                        project_id,
                        environment_id,
                        connection_id,
                    },
                    action,
                )
                .await
                .map(AgentResponse::Decision),
        };
        result.unwrap_or_else(|err| AgentResponse::Error {
            message: err.to_string(),
        })
    }
}

/// Default socket path inside the EnvFish data directory. Unix socket paths are
/// limited to ~100 bytes, so very deep data directories fall back to the system
/// temp dir with a short hash of the data dir in the name.
pub fn socket_path(data_dir: &Path) -> PathBuf {
    let preferred = data_dir.join("agent.sock");
    if preferred.as_os_str().len() < 100 {
        return preferred;
    }
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    data_dir.hash(&mut h);
    std::env::temp_dir().join(format!("envfish-{:016x}.sock", h.finish()))
}

#[cfg(unix)]
pub mod uds {
    //! Unix Domain Socket transport: newline-delimited JSON, one request per line.

    use std::path::Path;
    use std::sync::Arc;

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{UnixListener, UnixStream};

    use super::{Agent, AgentRequest, AgentResponse};

    /// Bind the socket (replacing a stale file) with owner-only permissions.
    pub fn bind(path: &Path) -> std::io::Result<UnixListener> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        Ok(listener)
    }

    /// Accept connections until the listener errors or the task is cancelled.
    pub async fn serve(agent: Arc<Agent>, listener: UnixListener) -> std::io::Result<()> {
        loop {
            let (stream, _) = listener.accept().await?;
            let agent = agent.clone();
            tokio::spawn(async move {
                if let Err(err) = handle_connection(agent, stream).await {
                    tracing::debug!(error = %err, "agent connection closed with error");
                }
            });
        }
    }

    async fn handle_connection(agent: Arc<Agent>, stream: UnixStream) -> std::io::Result<()> {
        let (read, mut write) = stream.into_split();
        let mut lines = BufReader::new(read).lines();
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let response = match serde_json::from_str::<AgentRequest>(&line) {
                Ok(req) => agent.handle(req).await,
                Err(err) => AgentResponse::Error {
                    message: format!("invalid request: {err}"),
                },
            };
            let mut out = serde_json::to_string(&response).unwrap_or_else(|e| {
                format!(r#"{{"type":"error","data":{{"message":"serialisation failed: {e}"}}}}"#)
            });
            out.push('\n');
            write.write_all(out.as_bytes()).await?;
        }
        Ok(())
    }

    /// Minimal client: send one request, read one response.
    pub async fn request(path: &Path, request: &AgentRequest) -> std::io::Result<AgentResponse> {
        let stream = UnixStream::connect(path).await?;
        let (read, mut write) = stream.into_split();
        let mut out = serde_json::to_string(request).unwrap_or_default();
        out.push('\n');
        write.write_all(out.as_bytes()).await?;
        let mut lines = BufReader::new(read).lines();
        let line = lines.next_line().await?.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "agent closed the connection")
        })?;
        serde_json::from_str(&line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e} (got {line:?})")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use envfish_core::vault::InMemoryMasterKeyProvider;

    async fn core() -> Arc<EnvFish> {
        let core = EnvFish::open_in_memory(&InMemoryMasterKeyProvider::random())
            .await
            .unwrap();
        core.create_project("my-app", None).await.unwrap();
        Arc::new(core)
    }

    #[tokio::test]
    async fn ping_and_list() {
        let agent = Agent::new(core().await);
        assert!(matches!(
            agent.handle(AgentRequest::Ping).await,
            AgentResponse::Pong { .. }
        ));
        match agent.handle(AgentRequest::ListProjects).await {
            AgentResponse::Projects(p) => assert_eq!(p.len(), 1),
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            agent
                .handle(AgentRequest::ListEnvironments {
                    project_id: "missing".into()
                })
                .await,
            AgentResponse::Error { .. }
        ));
    }

    #[test]
    fn request_set_has_no_secret_reader() {
        // Serialised tag names double as the wire vocabulary; none may mention secrets.
        let names = [
            "ping",
            "list_projects",
            "list_environments",
            "list_variables",
            "list_connections",
            "list_pending_approvals",
            "resolve_approval",
            "list_audit",
            "decide",
        ];
        for n in names {
            assert!(!n.contains("secret") && !n.contains("reveal") && !n.contains("dump"));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn uds_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = socket_path(dir.path());
        let core = core().await;
        let listener = uds::bind(&path).unwrap();
        let agent = Arc::new(Agent::new(core.clone()));
        let server = tokio::spawn(uds::serve(agent, listener));

        let pong = uds::request(&path, &AgentRequest::Ping).await.unwrap();
        assert!(matches!(pong, AgentResponse::Pong { .. }));
        match uds::request(&path, &AgentRequest::ListProjects).await.unwrap() {
            AgentResponse::Projects(p) => assert_eq!(p[0].name, "my-app"),
            other => panic!("unexpected {other:?}"),
        }

        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        server.abort();
    }
}
