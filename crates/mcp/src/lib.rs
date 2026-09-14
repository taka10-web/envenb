//! # envfish-mcp
//!
//! A Model Context Protocol server over stdio (JSON-RPC 2.0, one message per line).
//! Claude Code, Codex and other MCP clients spawn `envfish mcp --client <name>`.
//!
//! Tools exposed (all metadata or brokered actions; **no tool returns a secret**):
//!
//! | tool               | returns                                                   |
//! |--------------------|-----------------------------------------------------------|
//! | `list_projects`    | project names / ids                                       |
//! | `list_environments`| environments of a project                                 |
//! | `list_connections` | connections (kind, name, base_url; credential *name* only)|
//! | `list_variables`   | PUBLIC name+value, SECRET name only                       |
//! | `call_service`     | HTTP response from the Broker (credential scrubbed)       |
//! | `supabase_select`  | convenience wrapper: PostgREST `GET /rest/v1/<table>`     |
//!
//! Every brokered call goes through the Permission Engine. `ASK` blocks until a
//! human approves in the desktop app or with `envfish ai approve`, or times out.

use std::sync::Arc;
use std::time::Duration;

use envfish_broker::{Broker, BrokerRequest};
use envfish_core::{
    Action, AiClient, ApprovalStatus, AuditRecord, Decision, EnvFish, PermissionScope, VariableKind,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(180);
const APPROVAL_POLL: Duration = Duration::from_millis(600);

pub struct McpServer {
    core: Arc<EnvFish>,
    broker: Broker,
    client: AiClient,
}

#[derive(Debug, thiserror::Error)]
enum ToolError {
    #[error("{0}")]
    User(String),
    #[error("permission denied: {0}")]
    Denied(String),
    #[error("approval timed out or was denied: {0}")]
    NotApproved(String),
    #[error(transparent)]
    Core(#[from] envfish_core::CoreError),
    #[error(transparent)]
    Broker(#[from] envfish_broker::BrokerError),
}

impl McpServer {
    pub async fn new(core: Arc<EnvFish>, client_name: &str, client_kind: &str) -> envfish_core::Result<Self> {
        let client = core.register_ai_client(client_name, client_kind).await?;
        let broker = Broker::new(core.clone());
        Ok(Self { core, broker, client })
    }

    pub fn client(&self) -> &AiClient {
        &self.client
    }

    /// Serve until stdin closes.
    pub async fn serve_stdio(&self) -> std::io::Result<()> {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();
        let mut lines = stdin.lines();
        while let Some(line) = lines.next_line().await? {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let msg: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    let err = rpc_error(Value::Null, -32700, &format!("parse error: {e}"));
                    write_msg(&mut stdout, &err).await?;
                    continue;
                }
            };
            if let Some(reply) = self.handle(msg).await {
                write_msg(&mut stdout, &reply).await?;
            }
        }
        Ok(())
    }

    /// Handle one JSON-RPC message. Notifications return `None`.
    pub async fn handle(&self, msg: Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        tracing::debug!(method, "mcp request");

        let result: Result<Value, String> = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or(PROTOCOL_VERSION),
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "envfish", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "EnvFish brokers access to project secrets. You can list projects, environments, connections and PUBLIC variables, and call connected services through call_service. Secret values are never available; do not ask for them."
            })),
            "notifications/initialized" | "notifications/cancelled" => return None,
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_definitions() })),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                match self.call_tool(name, &args).await {
                    Ok(v) => Ok(tool_ok(v)),
                    Err(e) => Ok(tool_err(&e.to_string())),
                }
            }
            "resources/list" => Ok(json!({ "resources": [] })),
            "prompts/list" => Ok(json!({ "prompts": [] })),
            _ => {
                let id = id?;
                return Some(rpc_error(id, -32601, &format!("method not found: {method}")));
            }
        };
        let id = id?;
        Some(match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(e) => rpc_error(id, -32000, &e),
        })
    }

    async fn call_tool(&self, name: &str, args: &Value) -> Result<Value, ToolError> {
        match name {
            "list_projects" => {
                let projects = self.core.list_projects().await?;
                Ok(json!({ "projects": projects
                    .iter()
                    .map(|p| json!({"id": p.id, "name": p.name, "local_path": p.local_path}))
                    .collect::<Vec<_>>() }))
            }
            "list_environments" => {
                let project = self.resolve_project_arg(args).await?;
                let envs = self.core.list_environments(&project.id).await?;
                Ok(json!({ "environments": envs
                    .iter()
                    .map(|e| json!({"id": e.id, "name": e.name}))
                    .collect::<Vec<_>>() }))
            }
            "list_connections" => {
                let project = self.resolve_project_arg(args).await?;
                let conns = match args.get("environment").and_then(|v| v.as_str()) {
                    Some(env) => {
                        let e = self.core.resolve_environment(&project.id, env).await?;
                        self.core.list_connections(&e.id).await?
                    }
                    None => self.core.list_project_connections(&project.id).await?,
                };
                let mut out = Vec::new();
                for c in conns {
                    let env = self.core.get_environment(&c.environment_id).await?;
                    out.push(json!({
                        "id": c.id, "name": c.name, "kind": c.kind, "environment": env.name,
                        "base_url": c.base_url, "status": if c.auth_secret.is_some() || c.auth_style == "none" { "connected" } else { "no credential" }
                    }));
                }
                Ok(json!({ "connections": out }))
            }
            "list_variables" => {
                let (project, env) = self.resolve_env(args).await?;
                let vars = self.core.list_variables(&env.id).await?;
                let _ = project;
                Ok(json!({ "variables": vars.iter().map(|v| match v.kind {
                    VariableKind::Public => json!({"name": v.name, "kind": "PUBLIC", "value": v.value}),
                    VariableKind::Secret => json!({"name": v.name, "kind": "SECRET", "value": null, "note": "value withheld; use call_service"}),
                }).collect::<Vec<_>>() }))
            }
            "list_credentials" => {
                let (_project, env) = self.resolve_env(args).await?;
                let creds = self.core.list_credentials(&env.id).await?;
                Ok(json!({ "credentials": creds.iter().map(|c| json!({
                    "name": c.name, "kind": c.kind, "note": c.note,
                    "fields": c.fields.iter().map(|f| json!({"field": f.field, "secret": f.secret, "value": f.value})).collect::<Vec<_>>(),
                    "note_for_ai": "secret fields are withheld; ask the user to use them"
                })).collect::<Vec<_>>() }))
            }
            "call_service" => {
                let req = BrokerRequest {
                    method: str_arg(args, "method").unwrap_or("GET").to_string(),
                    path: str_arg(args, "path")?.to_string(),
                    query: pairs(args.get("query")),
                    headers: pairs(args.get("headers")),
                    body: args.get("body").filter(|b| !b.is_null()).cloned(),
                };
                self.brokered_call(args, req).await
            }
            "supabase_select" => {
                let table = str_arg(args, "table")?;
                if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(ToolError::User("table must be an identifier".into()));
                }
                let mut query = vec![(
                    "select".to_string(),
                    str_arg(args, "select").unwrap_or("*").to_string(),
                )];
                if let Some(limit) = args.get("limit").and_then(|l| l.as_u64()) {
                    query.push(("limit".into(), limit.min(1000).to_string()));
                }
                query.extend(pairs(args.get("filters")));
                let req = BrokerRequest {
                    method: "GET".into(),
                    path: format!("/rest/v1/{table}"),
                    query,
                    headers: vec![],
                    body: None,
                };
                self.brokered_call(args, req).await
            }
            other => Err(ToolError::User(format!("unknown tool: {other}"))),
        }
    }

    /// Resolve the project from `args`, or fall back to the only one that exists.
    /// Agents routinely reach for `list_environments` before they know a name.
    async fn resolve_project_arg(&self, args: &Value) -> Result<envfish_core::Project, ToolError> {
        if let Some(name) = args
            .get("project")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            return Ok(self.core.resolve_project(name).await?);
        }
        let mut projects = self.core.list_projects().await?;
        match projects.len() {
            1 => Ok(projects.remove(0)),
            0 => Err(ToolError::User(
                "no projects exist yet; the user creates one with `envfish project add <name>`".into(),
            )),
            _ => {
                let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
                Err(ToolError::User(format!(
                    "several projects exist; pass `project`: {}",
                    names.join(", ")
                )))
            }
        }
    }

    async fn resolve_env(
        &self,
        args: &Value,
    ) -> Result<(envfish_core::Project, envfish_core::Environment), ToolError> {
        let project = self.resolve_project_arg(args).await?;
        let env = self
            .core
            .resolve_environment(&project.id, str_arg(args, "environment")?)
            .await?;
        Ok((project, env))
    }

    /// Permission → (approval) → broker → audit. The credential never enters this function.
    async fn brokered_call(&self, args: &Value, req: BrokerRequest) -> Result<Value, ToolError> {
        let (project, env) = self.resolve_env(args).await?;
        let connection = self
            .core
            .resolve_connection(&env.id, str_arg(args, "connection")?)
            .await?;
        let action = Action::from_http_method(&req.method);
        let summary = Broker::summarize(&req);
        let scope = PermissionScope {
            client_id: Some(self.client.id.clone()),
            project_id: Some(project.id.clone()),
            environment_id: Some(env.id.clone()),
            connection_id: Some(connection.id.clone()),
        };
        let audit = |decision: &str| AuditRecord {
            client: Some(self.client.clone()),
            client_name: self.client.name.clone(),
            project_id: Some(project.id.clone()),
            environment_id: Some(env.id.clone()),
            connection_id: Some(connection.id.clone()),
            action,
            summary: summary.clone(),
            decision: decision.to_string(),
        };

        match self.core.decide(&scope, action).await? {
            Decision::Deny => {
                self.core.record_audit(audit("DENIED")).await?;
                return Err(ToolError::Denied(format!(
                    "{action} on {} / {} / {} is denied by EnvFish policy",
                    project.name, env.name, connection.name
                )));
            }
            Decision::Ask => {
                self.core.record_audit(audit("ASKED")).await?;
                let approval = self
                    .core
                    .request_approval(
                        &self.client,
                        &scope,
                        action,
                        &summary,
                        APPROVAL_TIMEOUT.as_secs() as i64,
                    )
                    .await?;
                tracing::info!(id = %approval.id, "waiting for human approval");
                let deadline = tokio::time::Instant::now() + APPROVAL_TIMEOUT;
                loop {
                    tokio::time::sleep(APPROVAL_POLL).await;
                    let a = self.core.get_approval(&approval.id).await?;
                    match a.status {
                        ApprovalStatus::Approved => break,
                        ApprovalStatus::Pending if tokio::time::Instant::now() < deadline => continue,
                        ApprovalStatus::Denied => {
                            self.core.record_audit(audit("DENIED")).await?;
                            return Err(ToolError::NotApproved("denied by the user".into()));
                        }
                        _ => {
                            self.core.record_audit(audit("DENIED")).await?;
                            return Err(ToolError::NotApproved("no decision within the time limit".into()));
                        }
                    }
                }
            }
            Decision::Allow => {}
        }

        match self.broker.call(&connection, &req).await {
            Ok(resp) => {
                self.core.record_audit(audit("ALLOWED")).await?;
                Ok(json!({
                    "status": resp.status,
                    "content_type": resp.content_type,
                    "truncated": resp.truncated,
                    "body": resp.body,
                }))
            }
            Err(e) => {
                self.core.record_audit(audit("ERROR")).await?;
                Err(e.into())
            }
        }
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::User(format!("missing argument: {key}")))
}

fn pairs(v: Option<&Value>) -> Vec<(String, String)> {
    match v {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.as_str().map(String::from).unwrap_or_else(|| v.to_string()),
                )
            })
            .collect(),
        _ => vec![],
    }
}

/// MCP requires `structuredContent` to be a JSON *object*, and only when the tool
/// declares an `outputSchema`. Anything else goes out as text only.
fn tool_ok(v: Value) -> Value {
    let text = serde_json::to_string_pretty(&v).unwrap_or_default();
    let mut result = json!({ "content": [{ "type": "text", "text": text }], "isError": false });
    if v.is_object() {
        result["structuredContent"] = v;
    }
    result
}

fn tool_err(msg: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": msg }], "isError": true })
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

async fn write_msg(out: &mut tokio::io::Stdout, v: &Value) -> std::io::Result<()> {
    let mut s = serde_json::to_string(v).unwrap_or_default();
    s.push('\n');
    out.write_all(s.as_bytes()).await?;
    out.flush().await
}

pub fn tool_definitions() -> Vec<Value> {
    let env_props = json!({
        "project": { "type": "string", "description": "Project name or id" },
        "environment": { "type": "string", "description": "Environment name or id (e.g. development)" }
    });
    vec![
        json!({
            "name": "list_projects",
            "description": "List EnvFish projects registered on this machine. Start here: other tools take a project name.",
            "inputSchema": { "type": "object", "properties": {} },
            "outputSchema": {
                "type": "object",
                "properties": { "projects": { "type": "array", "items": { "type": "object", "properties": {
                    "id": { "type": "string" }, "name": { "type": "string" }, "local_path": { "type": ["string", "null"] }
                }, "required": ["id", "name"] } } },
                "required": ["projects"]
            }
        }),
        json!({
            "name": "list_environments",
            "description": "List environments (development, staging, production, ...) of a project. Omit `project` when only one project exists.",
            "inputSchema": { "type": "object", "properties": { "project": env_props["project"] } },
            "outputSchema": {
                "type": "object",
                "properties": { "environments": { "type": "array", "items": { "type": "object", "properties": {
                    "id": { "type": "string" }, "name": { "type": "string" }
                }, "required": ["id", "name"] } } },
                "required": ["environments"]
            }
        }),
        json!({
            "name": "list_connections",
            "description": "List external service connections of a project. Returns names and status only; credentials are never returned. Omit `project` when only one project exists.",
            "inputSchema": { "type": "object", "properties": env_props },
            "outputSchema": {
                "type": "object",
                "properties": { "connections": { "type": "array", "items": { "type": "object", "properties": {
                    "id": { "type": "string" }, "name": { "type": "string" }, "kind": { "type": "string" },
                    "environment": { "type": "string" }, "base_url": { "type": "string" }, "status": { "type": "string" }
                }, "required": ["id", "name", "kind"] } } },
                "required": ["connections"]
            }
        }),
        json!({
            "name": "list_variables",
            "description": "List variables of an environment. PUBLIC variables include values; SECRET variables are names only. Omit `project` when only one project exists.",
            "inputSchema": { "type": "object", "properties": env_props, "required": ["environment"] },
            "outputSchema": {
                "type": "object",
                "properties": { "variables": { "type": "array", "items": { "type": "object", "properties": {
                    "name": { "type": "string" }, "kind": { "type": "string", "enum": ["PUBLIC", "SECRET"] },
                    "value": { "type": ["string", "null"] }, "note": { "type": "string" }
                }, "required": ["name", "kind"] } } },
                "required": ["variables"]
            }
        }),
        json!({
            "name": "list_credentials",
            "description": "List stored credentials (test accounts, SSH targets, databases, files) of an environment: names, kinds and non-secret fields such as host or URL. Secret fields are never returned. Omit `project` when only one project exists.",
            "inputSchema": { "type": "object", "properties": env_props, "required": ["environment"] },
            "outputSchema": {
                "type": "object",
                "properties": { "credentials": { "type": "array", "items": { "type": "object", "properties": {
                    "name": { "type": "string" }, "kind": { "type": "string" }, "note": { "type": ["string", "null"] },
                    "fields": { "type": "array", "items": { "type": "object" } }
                }, "required": ["name", "kind"] } } },
                "required": ["credentials"]
            }
        }),
        json!({
            "name": "call_service",
            "description": "Call an external API through a connection. EnvFish injects the credential and returns the HTTP response. Subject to per-client permissions; WRITE/DELETE may require human approval.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": env_props["project"],
                    "environment": env_props["environment"],
                    "connection": { "type": "string", "description": "Connection name or id" },
                    "method": { "type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"], "default": "GET" },
                    "path": { "type": "string", "description": "Path relative to the connection base URL, e.g. /models" },
                    "query": { "type": "object", "additionalProperties": { "type": "string" } },
                    "headers": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Extra headers (auth headers are managed by EnvFish and rejected)" },
                    "body": { "description": "JSON body for POST/PUT/PATCH" }
                },
                "required": ["project", "environment", "connection", "path"]
            }
        }),
        json!({
            "name": "supabase_select",
            "description": "Read rows from a Supabase table via PostgREST (GET /rest/v1/<table>).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": env_props["project"],
                    "environment": env_props["environment"],
                    "connection": { "type": "string", "description": "Supabase connection name" },
                    "table": { "type": "string" },
                    "select": { "type": "string", "default": "*" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000 },
                    "filters": { "type": "object", "additionalProperties": { "type": "string" }, "description": "PostgREST filters, e.g. {\"id\": \"eq.1\"}" }
                },
                "required": ["project", "environment", "connection", "table"]
            }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use envfish_core::vault::InMemoryMasterKeyProvider;

    async fn server() -> McpServer {
        let core = Arc::new(
            EnvFish::open_in_memory(&InMemoryMasterKeyProvider::random())
                .await
                .unwrap(),
        );
        core.create_project("my-app", None).await.unwrap();
        McpServer::new(core, "test-client", "mcp").await.unwrap()
    }

    #[tokio::test]
    async fn initialize_and_list_tools() {
        let s = server().await;
        let init = s
            .handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}))
            .await
            .unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "envfish");
        assert!(
            s.handle(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
                .await
                .is_none()
        );
        let tools = s
            .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
            .await
            .unwrap();
        let names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"call_service") && names.contains(&"list_projects"));
        for forbidden in [
            "get_secret",
            "read_secret",
            "dump_vault",
            "export_all_secrets",
            "reveal",
        ] {
            assert!(!names.iter().any(|n| n.contains(forbidden)));
        }
    }

    #[tokio::test]
    async fn secrets_never_leave_list_variables() {
        let s = server().await;
        let p = s.core.resolve_project("my-app").await.unwrap();
        let e = s.core.create_environment(&p.id, "development").await.unwrap();
        s.core
            .set_public_variable(&e.id, "APP_URL", "http://x")
            .await
            .unwrap();
        s.core
            .set_secret_variable(
                &e.id,
                "OPENAI_API_KEY",
                envfish_core::SecretValue::new("sk-NEEDLE"),
            )
            .await
            .unwrap();
        let resp = s
            .handle(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_variables","arguments":{"project":"my-app","environment":"development"}}}))
            .await
            .unwrap();
        let text = resp.to_string();
        assert!(text.contains("APP_URL") && text.contains("OPENAI_API_KEY"));
        assert!(!text.contains("sk-NEEDLE"));
        assert_eq!(resp["result"]["isError"], false);
    }

    #[tokio::test]
    async fn denied_action_is_audited_and_never_reaches_the_network() {
        let s = server().await;
        let p = s.core.resolve_project("my-app").await.unwrap();
        let e = s.core.create_environment(&p.id, "production").await.unwrap();
        s.core
            .set_secret_variable(&e.id, "KEY", envfish_core::SecretValue::new("k"))
            .await
            .unwrap();
        s.core
            .create_connection(envfish_core::NewConnection {
                environment_id: e.id.clone(),
                kind: envfish_core::ConnectionKind::GenericHttp,
                name: "api".into(),
                base_url: Some("https://192.0.2.1".into()),
                auth_secret: Some("KEY".into()),
                auth_style: None,
                metadata: None,
            })
            .await
            .unwrap();
        let resp = s
            .handle(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"call_service","arguments":{"project":"my-app","environment":"production","connection":"api","method":"DELETE","path":"/users/1"}}}))
            .await
            .unwrap();
        assert_eq!(resp["result"]["isError"], true);
        assert!(
            resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("denied")
        );
        let audit = s.core.list_audit(10).await.unwrap();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].decision, "DENIED");
        assert_eq!(audit[0].summary, "DELETE /users/1");
    }

    /// MCP requires `structuredContent` to be a JSON object, and requires a tool
    /// that returns one to declare an `outputSchema`. Returning a bare array made
    /// every list tool fail client-side validation.
    #[tokio::test]
    async fn list_tools_return_objects_matching_their_output_schema() {
        let s = server().await;
        let p = s.core.resolve_project("my-app").await.unwrap();
        let e = s.core.create_environment(&p.id, "development").await.unwrap();
        s.core
            .set_public_variable(&e.id, "APP_URL", "http://x")
            .await
            .unwrap();

        for (tool, args, key) in [
            ("list_projects", json!({}), "projects"),
            (
                "list_environments",
                json!({ "project": "my-app" }),
                "environments",
            ),
            ("list_connections", json!({ "project": "my-app" }), "connections"),
            (
                "list_variables",
                json!({ "project": "my-app", "environment": "development" }),
                "variables",
            ),
            (
                "list_credentials",
                json!({ "project": "my-app", "environment": "development" }),
                "credentials",
            ),
        ] {
            let resp = s
                .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                               "params":{"name": tool, "arguments": args}}))
                .await
                .unwrap();
            let result = &resp["result"];
            assert_eq!(result["isError"], false, "{tool} failed: {result}");
            let structured = &result["structuredContent"];
            assert!(
                structured.is_object(),
                "{tool}: structuredContent must be an object"
            );
            assert!(structured[key].is_array(), "{tool}: expected a `{key}` array");

            // The declared schema must match what we actually send.
            let def = tool_definitions()
                .into_iter()
                .find(|d| d["name"] == tool)
                .unwrap_or_else(|| panic!("{tool} is not declared"));
            assert_eq!(def["outputSchema"]["type"], "object", "{tool}: outputSchema");
            assert!(
                def["outputSchema"]["properties"][key].is_object(),
                "{tool}: outputSchema is missing `{key}`"
            );
        }
    }

    /// With a single project, an agent should not have to name it.
    #[tokio::test]
    async fn project_argument_is_optional_when_unambiguous() {
        let s = server().await;
        let p = s.core.resolve_project("my-app").await.unwrap();
        s.core.create_environment(&p.id, "development").await.unwrap();

        let resp = s
            .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                           "params":{"name":"list_environments","arguments":{}}}))
            .await
            .unwrap();
        assert_eq!(resp["result"]["isError"], false, "{resp}");
        assert_eq!(
            resp["result"]["structuredContent"]["environments"][0]["name"],
            "development"
        );

        // With two projects the tool asks for one by name instead of guessing.
        s.core.create_project("other", None).await.unwrap();
        let resp = s
            .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
                           "params":{"name":"list_environments","arguments":{}}}))
            .await
            .unwrap();
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("my-app") && text.contains("other"), "{text}");
    }

    #[tokio::test]
    async fn unknown_method_is_a_jsonrpc_error() {
        let s = server().await;
        let resp = s
            .handle(json!({"jsonrpc":"2.0","id":9,"method":"nope"}))
            .await
            .unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }
}
