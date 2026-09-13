use envfish_core::{
    Action, AiClient, Approval, ApprovalStatus, AuditEntry, Connection, ConnectionKind, Decision,
    Environment, NewConnection, Permission, PermissionScope, Project, SecretValue, Settings, StatusReport,
    Variable,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use zeroize::Zeroize;

use crate::AppState;

/// Errors are flattened to their display string; `CoreError` never embeds secret values.
type CmdResult<T> = Result<T, String>;

fn map_err(err: envfish_core::CoreError) -> String {
    tracing::warn!(error = %err, "command failed");
    err.to_string()
}

#[tauri::command]
pub async fn status(state: State<'_, AppState>) -> CmdResult<StatusReport> {
    state.core.status().await.map_err(map_err)
}

// ---- settings ----

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CmdResult<Settings> {
    state.core.settings().await.map_err(map_err)
}

#[tauri::command]
pub async fn set_language(state: State<'_, AppState>, language: String) -> CmdResult<Settings> {
    state.core.set_language(&language).await.map_err(map_err)
}

#[tauri::command]
pub async fn set_theme(state: State<'_, AppState>, theme: String) -> CmdResult<Settings> {
    state.core.set_theme(&theme).await.map_err(map_err)
}

// ---- projects ----

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> CmdResult<Vec<Project>> {
    state.core.list_projects().await.map_err(map_err)
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    name: String,
    local_path: Option<String>,
) -> CmdResult<Project> {
    state
        .core
        .create_project(&name, local_path.as_deref())
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn delete_project(state: State<'_, AppState>, project_id: String) -> CmdResult<()> {
    state.core.delete_project(&project_id).await.map_err(map_err)
}

// ---- environments ----

#[tauri::command]
pub async fn list_environments(
    state: State<'_, AppState>,
    project_id: String,
) -> CmdResult<Vec<Environment>> {
    state.core.list_environments(&project_id).await.map_err(map_err)
}

#[tauri::command]
pub async fn create_environment(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> CmdResult<Environment> {
    state
        .core
        .create_environment(&project_id, &name)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn delete_environment(state: State<'_, AppState>, environment_id: String) -> CmdResult<()> {
    state
        .core
        .delete_environment(&environment_id)
        .await
        .map_err(map_err)
}

// ---- variables ----

/// Secret entries come back with `value: null`.
#[tauri::command]
pub async fn list_variables(state: State<'_, AppState>, environment_id: String) -> CmdResult<Vec<Variable>> {
    state.core.list_variables(&environment_id).await.map_err(map_err)
}

#[tauri::command]
pub async fn set_public_variable(
    state: State<'_, AppState>,
    environment_id: String,
    name: String,
    value: String,
) -> CmdResult<Variable> {
    state
        .core
        .set_public_variable(&environment_id, &name, &value)
        .await
        .map_err(map_err)
}

/// The only command that receives plaintext. It is moved into `SecretValue`
/// immediately, sealed by the vault, and the incoming buffer is zeroized.
#[tauri::command]
pub async fn set_secret_variable(
    state: State<'_, AppState>,
    environment_id: String,
    name: String,
    mut value: String,
) -> CmdResult<Variable> {
    let secret = SecretValue::new(value.as_str());
    value.zeroize();
    state
        .core
        .set_secret_variable(&environment_id, &name, secret)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn delete_variable(
    state: State<'_, AppState>,
    environment_id: String,
    name: String,
) -> CmdResult<()> {
    state
        .core
        .delete_variable(&environment_id, &name)
        .await
        .map_err(map_err)
}

/// `.env.example` text (secrets blank) for copy / save from the UI.
#[tauri::command]
pub async fn render_env_example(state: State<'_, AppState>, environment_id: String) -> CmdResult<String> {
    state
        .core
        .render_env_example(&environment_id)
        .await
        .map_err(map_err)
}

// ---- .env import ----

#[derive(Serialize)]
pub struct DotenvPreviewEntry {
    pub name: String,
    pub value: String,
    pub suggestion: envfish_core::dotenv::Suggestion,
    pub line: usize,
}

#[derive(Serialize)]
pub struct DotenvPreview {
    pub entries: Vec<DotenvPreviewEntry>,
    pub invalid_lines: Vec<usize>,
}

/// Parse `.env` text pasted or read in the UI and suggest kinds. Nothing is stored yet.
#[tauri::command]
pub async fn preview_dotenv(text: String) -> CmdResult<DotenvPreview> {
    let (entries, invalid_lines) = envfish_core::dotenv::parse(&text);
    Ok(DotenvPreview {
        entries: entries
            .into_iter()
            .map(|e| DotenvPreviewEntry {
                suggestion: envfish_core::dotenv::classify(&e.name, &e.value),
                name: e.name,
                value: e.value,
                line: e.line,
            })
            .collect(),
        invalid_lines,
    })
}

#[derive(Deserialize)]
pub struct ImportEntry {
    pub name: String,
    pub value: String,
    pub kind: envfish_core::VariableKind,
}

#[tauri::command]
pub async fn import_variables(
    state: State<'_, AppState>,
    environment_id: String,
    entries: Vec<ImportEntry>,
) -> CmdResult<envfish_core::ImportReport> {
    let items = entries.into_iter().map(|e| (e.name, e.value, e.kind)).collect();
    state
        .core
        .import_variables(&environment_id, items)
        .await
        .map_err(map_err)
}

// ---- connections ----

#[tauri::command]
pub async fn list_connections(state: State<'_, AppState>, project_id: String) -> CmdResult<Vec<Connection>> {
    state
        .core
        .list_project_connections(&project_id)
        .await
        .map_err(map_err)
}

#[derive(Deserialize)]
pub struct NewConnectionInput {
    pub environment_id: String,
    pub kind: ConnectionKind,
    pub name: String,
    pub base_url: Option<String>,
    pub auth_secret: Option<String>,
    pub auth_style: Option<String>,
    /// Non-secret JSON. For `aws`: `{ region, service, access_key_id_secret }`.
    pub metadata: Option<serde_json::Value>,
}

#[tauri::command]
pub async fn create_connection(
    state: State<'_, AppState>,
    input: NewConnectionInput,
) -> CmdResult<Connection> {
    state
        .core
        .create_connection(NewConnection {
            environment_id: input.environment_id,
            kind: input.kind,
            name: input.name,
            base_url: input.base_url,
            auth_secret: input.auth_secret,
            auth_style: input.auth_style,
            metadata: input.metadata,
        })
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn delete_connection(state: State<'_, AppState>, connection_id: String) -> CmdResult<()> {
    state
        .core
        .delete_connection(&connection_id)
        .await
        .map_err(map_err)
}

// ---- AI clients / permissions / approvals / audit ----

#[tauri::command]
pub async fn list_ai_clients(state: State<'_, AppState>) -> CmdResult<Vec<AiClient>> {
    state.core.list_ai_clients().await.map_err(map_err)
}

#[tauri::command]
pub async fn register_ai_client(
    state: State<'_, AppState>,
    name: String,
    kind: String,
) -> CmdResult<AiClient> {
    state.core.register_ai_client(&name, &kind).await.map_err(map_err)
}

#[tauri::command]
pub async fn delete_ai_client(state: State<'_, AppState>, client_id: String) -> CmdResult<()> {
    state.core.delete_ai_client(&client_id).await.map_err(map_err)
}

#[tauri::command]
pub async fn list_permissions(state: State<'_, AppState>) -> CmdResult<Vec<Permission>> {
    state.core.list_permissions().await.map_err(map_err)
}

#[derive(Deserialize)]
pub struct PermissionInput {
    pub client_id: Option<String>,
    pub project_id: Option<String>,
    pub environment_id: Option<String>,
    pub connection_id: Option<String>,
    pub action: Action,
    pub decision: Decision,
}

#[tauri::command]
pub async fn set_permission(state: State<'_, AppState>, input: PermissionInput) -> CmdResult<Permission> {
    state
        .core
        .set_permission(
            PermissionScope {
                client_id: input.client_id,
                project_id: input.project_id,
                environment_id: input.environment_id,
                connection_id: input.connection_id,
            },
            input.action,
            input.decision,
        )
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn delete_permission(state: State<'_, AppState>, permission_id: String) -> CmdResult<()> {
    state
        .core
        .delete_permission(&permission_id)
        .await
        .map_err(map_err)
}

/// Effective decision for a scope (rules + defaults), for the AI Access matrix.
#[tauri::command]
pub async fn effective_decision(
    state: State<'_, AppState>,
    client_id: Option<String>,
    project_id: Option<String>,
    environment_id: Option<String>,
    connection_id: Option<String>,
    action: Action,
) -> CmdResult<Decision> {
    state
        .core
        .decide(
            &PermissionScope {
                client_id,
                project_id,
                environment_id,
                connection_id,
            },
            action,
        )
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn list_approvals(state: State<'_, AppState>, pending_only: bool) -> CmdResult<Vec<Approval>> {
    let status = if pending_only {
        Some(ApprovalStatus::Pending)
    } else {
        None
    };
    state.core.list_approvals(status, 100).await.map_err(map_err)
}

#[tauri::command]
pub async fn resolve_approval(
    state: State<'_, AppState>,
    approval_id: String,
    approve: bool,
) -> CmdResult<Approval> {
    state
        .core
        .resolve_approval(&approval_id, approve)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn list_audit(state: State<'_, AppState>, limit: i64) -> CmdResult<Vec<AuditEntry>> {
    state.core.list_audit(limit).await.map_err(map_err)
}
