//! # envfish-daemon
//!
//! The Local Agent boundary. In the target architecture every client (CLI,
//! desktop, MCP) talks to this process, which alone holds the open vault and
//! decides, via the Permission Engine, what an AI client may do.
//!
//! **Phase 1 status:** skeleton only. The CLI and desktop app link `envfish-core`
//! directly. What exists here fixes the shape of the API so later phases can
//! move clients behind it without changing their vocabulary:
//!
//! - [`AgentRequest`] / [`AgentResponse`]: the wire-level message set. Note that
//!   there is no request that returns a secret value, and there never will be.
//! - [`Agent`]: an in-process handler that maps requests onto `envfish-core`.
//!   Later phases wrap it in a Unix Domain Socket / Named Pipe server.

use envfish_core::{EnvFish, Environment, Project, Variable};
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentResponse {
    Pong { version: String },
    Projects(Vec<Project>),
    Environments(Vec<Environment>),
    Variables(Vec<Variable>),
    Error { message: String },
}

/// In-process request handler. Transport (UDS / Named Pipe) is added in Phase 2.
pub struct Agent {
    core: EnvFish,
}

impl Agent {
    pub fn new(core: EnvFish) -> Self {
        Self { core }
    }

    pub async fn handle(&self, request: AgentRequest) -> AgentResponse {
        let result = match request {
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
        };
        result.unwrap_or_else(|err| AgentResponse::Error {
            message: err.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use envfish_core::vault::InMemoryMasterKeyProvider;

    #[tokio::test]
    async fn ping_and_list() {
        let core = EnvFish::open_in_memory(&InMemoryMasterKeyProvider::random())
            .await
            .unwrap();
        core.create_project("my-app", None).await.unwrap();
        let agent = Agent::new(core);

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
}
