//! Phase 2 service methods: settings, connections, AI clients, permissions,
//! approvals, audit log, `.env` import/export, and the two *internal-tier*
//! secret consumers (process runner, broker).

use std::process::Command;

use chrono::{Duration, Utc};
use serde::Serialize;

use crate::dotenv;
use crate::error::{CoreError, Result};
use crate::model::{
    Action, AiClient, Approval, ApprovalStatus, AuditEntry, Connection, ConnectionKind, Decision, Permission,
    Settings, Variable, VariableKind,
};
use crate::permission::{self, Scope};
use crate::repo;
use crate::repo_ai as ai;
use crate::secret::SecretValue;
use crate::service::EnvFish;
use envfish_vault::MasterKeyProvider as _;

pub const SETTING_LANGUAGE: &str = "language";
pub const SETTING_KEY_BACKEND: &str = "key_backend";
pub const SETTING_THEME: &str = "theme";

/// Input for creating a connection.
#[derive(Debug, Clone)]
pub struct NewConnection {
    pub environment_id: String,
    pub kind: ConnectionKind,
    pub name: String,
    pub base_url: Option<String>,
    /// Name of the SECRET variable holding the credential.
    pub auth_secret: Option<String>,
    pub auth_style: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Where a permission rule / decision applies.
#[derive(Debug, Clone, Default)]
pub struct PermissionScope {
    pub client_id: Option<String>,
    pub project_id: Option<String>,
    pub environment_id: Option<String>,
    pub connection_id: Option<String>,
}

/// What to record in the audit log.
#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub client: Option<AiClient>,
    pub client_name: String,
    pub project_id: Option<String>,
    pub environment_id: Option<String>,
    pub connection_id: Option<String>,
    pub action: Action,
    pub summary: String,
    /// ALLOWED | DENIED | ASKED | ERROR
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ImportReport {
    pub public_added: usize,
    pub secret_added: usize,
    pub skipped: Vec<String>,
}

/// Decrypted environment for a child process. Holds plaintext; keep it short-lived.
pub struct ProcessEnv {
    vars: Vec<(String, SecretValue)>,
    public_count: usize,
    secret_count: usize,
}

impl ProcessEnv {
    pub fn names(&self) -> Vec<&str> {
        self.vars.iter().map(|(n, _)| n.as_str()).collect()
    }
    pub fn public_count(&self) -> usize {
        self.public_count
    }
    pub fn secret_count(&self) -> usize {
        self.secret_count
    }
    /// Inject into a `Command`. Values never touch this process's own environment.
    pub fn apply_to(&self, cmd: &mut Command) {
        for (name, value) in &self.vars {
            cmd.env(name, value.expose());
        }
    }
}

impl std::fmt::Debug for ProcessEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessEnv")
            .field("names", &self.names())
            .finish()
    }
}

impl EnvFish {
    // ---------- settings ----------

    pub async fn settings(&self) -> Result<Settings> {
        Ok(Settings {
            language: ai::get_setting(self.pool(), SETTING_LANGUAGE)
                .await?
                .unwrap_or_else(|| "system".into()),
            key_backend: ai::get_setting(self.pool(), SETTING_KEY_BACKEND)
                .await?
                .unwrap_or_else(|| "file".into()),
            theme: ai::get_setting(self.pool(), SETTING_THEME)
                .await?
                .unwrap_or_else(|| "system".into()),
        })
    }

    pub async fn set_theme(&self, theme: &str) -> Result<Settings> {
        let t = theme.trim().to_ascii_lowercase();
        if !matches!(t.as_str(), "system" | "light" | "dark") {
            return Err(CoreError::InvalidName(format!(
                "theme must be system, light or dark, got {theme:?}"
            )));
        }
        ai::set_setting(self.pool(), SETTING_THEME, &t).await?;
        self.settings().await
    }

    pub async fn set_language(&self, language: &str) -> Result<Settings> {
        let lang = language.trim().to_ascii_lowercase();
        if !matches!(lang.as_str(), "ja" | "en" | "system") {
            return Err(CoreError::InvalidName(format!(
                "language must be ja, en or system, got {language:?}"
            )));
        }
        ai::set_setting(self.pool(), SETTING_LANGUAGE, &lang).await?;
        self.settings().await
    }

    /// Move the master key between the `file` backend and the OS `keychain`.
    ///
    /// The key is copied to the target, read back to prove it is retrievable, the
    /// setting is flipped, and only then is the source removed. Secrets in SQLite are
    /// untouched: the key bytes are identical, so no re-encryption is needed.
    pub async fn switch_key_backend(&self, backend: &str) -> Result<Settings> {
        let current = self.settings().await?.key_backend;
        if current == backend {
            return self.settings().await;
        }
        let source = EnvFish::provider_for(self.paths(), &current)?;
        let key = source.load_or_create()?;
        match backend {
            "file" => {
                let target = envfish_vault::FileMasterKeyProvider::new(self.paths().master_key());
                if target.exists() {
                    return Err(CoreError::InvalidName(
                        "a master.key file already exists; remove it manually before switching".into(),
                    ));
                }
                target.store(&key)?;
                target.load_or_create()?;
            }
            #[cfg(feature = "keychain")]
            "keychain" => {
                let target = envfish_vault::KeychainMasterKeyProvider::new(self.paths().profile());
                if target.exists()? {
                    return Err(CoreError::InvalidName(
                        "a key already exists in the OS keychain for this data directory".into(),
                    ));
                }
                target.store(&key)?;
                target.load_or_create()?;
            }
            other => {
                return Err(CoreError::InvalidName(format!(
                    "key backend must be file or keychain, got {other:?}"
                )));
            }
        }
        ai::set_setting(self.pool(), SETTING_KEY_BACKEND, backend).await?;
        match current.as_str() {
            "file" => envfish_vault::FileMasterKeyProvider::new(self.paths().master_key()).delete()?,
            #[cfg(feature = "keychain")]
            "keychain" => envfish_vault::KeychainMasterKeyProvider::new(self.paths().profile()).delete()?,
            _ => {}
        }
        tracing::info!(from = %current, to = %backend, "master key backend switched");
        self.settings().await
    }

    // ---------- connections ----------

    pub async fn list_connections(&self, environment_id: &str) -> Result<Vec<Connection>> {
        self.get_environment(environment_id).await?;
        ai::list_connections_for_env(self.pool(), environment_id).await
    }

    pub async fn list_project_connections(&self, project_id: &str) -> Result<Vec<Connection>> {
        self.get_project(project_id).await?;
        ai::list_connections_for_project(self.pool(), project_id).await
    }

    pub async fn get_connection(&self, id: &str) -> Result<Connection> {
        ai::find_connection_by_id(self.pool(), id)
            .await?
            .ok_or_else(|| CoreError::ConnectionNotFound(id.to_string()))
    }

    pub async fn resolve_connection(&self, environment_id: &str, id_or_name: &str) -> Result<Connection> {
        if let Some(c) = ai::find_connection_by_id(self.pool(), id_or_name).await?
            && c.environment_id == environment_id
        {
            return Ok(c);
        }
        ai::find_connection_by_name(self.pool(), environment_id, id_or_name)
            .await?
            .ok_or_else(|| CoreError::ConnectionNotFound(id_or_name.to_string()))
    }

    pub async fn create_connection(&self, input: NewConnection) -> Result<Connection> {
        let env = self.get_environment(&input.environment_id).await?;
        let name = input.name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(CoreError::InvalidName(
                "connection name must be 1-100 characters".into(),
            ));
        }
        if ai::find_connection_by_name(self.pool(), &env.id, name)
            .await?
            .is_some()
        {
            return Err(CoreError::AlreadyExists("connection"));
        }
        let base_url = input
            .base_url
            .map(|u| u.trim().trim_end_matches('/').to_string())
            .filter(|u| !u.is_empty())
            .or_else(|| input.kind.default_base_url().map(String::from))
            .ok_or_else(|| CoreError::InvalidName("base_url is required for this connection kind".into()))?;
        if !(base_url.starts_with("https://")
            || base_url.starts_with("http://localhost")
            || base_url.starts_with("http://127.0.0.1"))
        {
            return Err(CoreError::InvalidName(
                "base_url must use https:// (http:// is allowed for localhost only)".into(),
            ));
        }
        // The credential must exist as a SECRET in the same environment.
        if let Some(secret_name) = input.auth_secret.as_deref() {
            match repo::find_variable_kind(self.pool(), &env.id, secret_name).await? {
                Some((_, VariableKind::Secret)) => {}
                Some((_, VariableKind::Public)) => {
                    return Err(CoreError::InvalidName(format!(
                        "{secret_name} is a PUBLIC variable; a connection credential must be a SECRET"
                    )));
                }
                None => return Err(CoreError::VariableNotFound(secret_name.to_string())),
            }
        }
        let auth_style = input
            .auth_style
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| input.kind.default_auth_style().to_string());
        let metadata = input.metadata.clone().unwrap_or_else(|| serde_json::json!({}));
        if input.kind == ConnectionKind::Aws {
            for key in ["region", "service", "access_key_id_secret"] {
                if metadata
                    .get(key)
                    .and_then(|v| v.as_str())
                    .is_none_or(str::is_empty)
                {
                    return Err(CoreError::InvalidName(format!(
                        "aws connections require metadata.{key}"
                    )));
                }
            }
            let id_secret = metadata["access_key_id_secret"].as_str().unwrap_or_default();
            match repo::find_variable_kind(self.pool(), &env.id, id_secret).await? {
                Some((_, VariableKind::Secret)) => {}
                _ => return Err(CoreError::VariableNotFound(id_secret.to_string())),
            }
        }
        let now = Utc::now();
        let conn = Connection {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: env.project_id.clone(),
            environment_id: env.id.clone(),
            kind: input.kind,
            name: name.to_string(),
            base_url,
            auth_secret: input.auth_secret,
            auth_style,
            metadata,
            created_at: now,
            updated_at: now,
        };
        ai::insert_connection(self.pool(), &conn).await?;
        Ok(conn)
    }

    pub async fn delete_connection(&self, id: &str) -> Result<()> {
        if !ai::delete_connection(self.pool(), id).await? {
            return Err(CoreError::ConnectionNotFound(id.to_string()));
        }
        Ok(())
    }

    // ---------- ai clients ----------

    pub async fn list_ai_clients(&self) -> Result<Vec<AiClient>> {
        ai::list_ai_clients(self.pool()).await
    }

    pub async fn resolve_ai_client(&self, id_or_name: &str) -> Result<AiClient> {
        ai::find_ai_client(self.pool(), id_or_name)
            .await?
            .ok_or_else(|| CoreError::ClientNotFound(id_or_name.to_string()))
    }

    /// Find or register a client by name and mark it seen.
    pub async fn register_ai_client(&self, name: &str, kind: &str) -> Result<AiClient> {
        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(CoreError::InvalidName(
                "client name must be 1-100 characters".into(),
            ));
        }
        if let Some(existing) = ai::find_ai_client(self.pool(), name).await? {
            ai::touch_ai_client(self.pool(), &existing.id).await?;
            return self.resolve_ai_client(&existing.id).await;
        }
        let client = AiClient {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            created_at: Utc::now(),
            last_seen_at: Some(Utc::now()),
        };
        ai::insert_ai_client(self.pool(), &client).await?;
        Ok(client)
    }

    pub async fn delete_ai_client(&self, id: &str) -> Result<()> {
        if !ai::delete_ai_client(self.pool(), id).await? {
            return Err(CoreError::ClientNotFound(id.to_string()));
        }
        Ok(())
    }

    // ---------- permissions ----------

    pub async fn list_permissions(&self) -> Result<Vec<Permission>> {
        ai::list_permissions(self.pool()).await
    }

    /// Create or update the rule for exactly this scope + action.
    pub async fn set_permission(
        &self,
        scope: PermissionScope,
        action: Action,
        decision: Decision,
    ) -> Result<Permission> {
        if let Some(existing) = ai::find_permission_exact(
            self.pool(),
            scope.client_id.as_deref(),
            scope.project_id.as_deref(),
            scope.environment_id.as_deref(),
            scope.connection_id.as_deref(),
            action,
        )
        .await?
        {
            ai::update_permission_decision(self.pool(), &existing.id, decision).await?;
            return Ok(Permission {
                decision,
                updated_at: Utc::now(),
                ..existing
            });
        }
        let now = Utc::now();
        let p = Permission {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: scope.client_id,
            project_id: scope.project_id,
            environment_id: scope.environment_id,
            connection_id: scope.connection_id,
            action,
            decision,
            created_at: now,
            updated_at: now,
        };
        ai::insert_permission(self.pool(), &p).await?;
        Ok(p)
    }

    pub async fn delete_permission(&self, id: &str) -> Result<()> {
        if !ai::delete_permission(self.pool(), id).await? {
            return Err(CoreError::PermissionNotFound(id.to_string()));
        }
        Ok(())
    }

    /// Evaluate the permission engine for one request.
    pub async fn decide(&self, scope: &PermissionScope, action: Action) -> Result<Decision> {
        let env_name = match &scope.environment_id {
            Some(id) => self.get_environment(id).await?.name,
            None => String::new(),
        };
        let rules = ai::list_permissions(self.pool()).await?;
        let s = Scope {
            client_id: scope.client_id.as_deref(),
            project_id: scope.project_id.as_deref(),
            environment_id: scope.environment_id.as_deref(),
            connection_id: scope.connection_id.as_deref(),
        };
        Ok(permission::decide(&rules, &s, action, &env_name))
    }

    // ---------- approvals ----------

    pub async fn request_approval(
        &self,
        client: &AiClient,
        scope: &PermissionScope,
        action: Action,
        summary: &str,
        ttl_seconds: i64,
    ) -> Result<Approval> {
        let now = Utc::now();
        let approval = Approval {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: client.id.clone(),
            client_name: client.name.clone(),
            project_id: scope.project_id.clone(),
            environment_id: scope.environment_id.clone(),
            connection_id: scope.connection_id.clone(),
            action,
            summary: summary.chars().take(500).collect(),
            status: ApprovalStatus::Pending,
            created_at: now,
            resolved_at: None,
            expires_at: now + Duration::seconds(ttl_seconds.max(5)),
        };
        ai::insert_approval(self.pool(), &approval).await?;
        Ok(approval)
    }

    pub async fn get_approval(&self, id: &str) -> Result<Approval> {
        ai::expire_approvals(self.pool()).await?;
        ai::find_approval(self.pool(), id)
            .await?
            .ok_or_else(|| CoreError::ApprovalNotFound(id.to_string()))
    }

    pub async fn list_approvals(&self, status: Option<ApprovalStatus>, limit: i64) -> Result<Vec<Approval>> {
        ai::expire_approvals(self.pool()).await?;
        ai::list_approvals(self.pool(), status, limit).await
    }

    /// Human decision. Accepts an id prefix (>= 6 chars) for CLI convenience.
    pub async fn resolve_approval(&self, id_or_prefix: &str, approve: bool) -> Result<Approval> {
        ai::expire_approvals(self.pool()).await?;
        let id = if id_or_prefix.len() >= 36 {
            id_or_prefix.to_string()
        } else {
            let pending = ai::list_approvals(self.pool(), Some(ApprovalStatus::Pending), 200).await?;
            let matches: Vec<_> = pending
                .into_iter()
                .filter(|a| a.id.starts_with(id_or_prefix))
                .collect();
            match matches.len() {
                1 => matches[0].id.clone(),
                0 => return Err(CoreError::ApprovalNotFound(id_or_prefix.to_string())),
                _ => {
                    return Err(CoreError::InvalidName(format!(
                        "approval id prefix {id_or_prefix} is ambiguous"
                    )));
                }
            }
        };
        let status = if approve {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Denied
        };
        if !ai::resolve_approval(self.pool(), &id, status).await? {
            return Err(CoreError::ApprovalNotPending(id));
        }
        self.get_approval(&id).await
    }

    // ---------- audit ----------

    pub async fn record_audit(&self, rec: AuditRecord) -> Result<AuditEntry> {
        let project_name = match &rec.project_id {
            Some(id) => repo::find_project_by_id(self.pool(), id).await?.map(|p| p.name),
            None => None,
        };
        let environment_name = match &rec.environment_id {
            Some(id) => repo::find_environment_by_id(self.pool(), id)
                .await?
                .map(|e| e.name),
            None => None,
        };
        let connection_name = match &rec.connection_id {
            Some(id) => ai::find_connection_by_id(self.pool(), id).await?.map(|c| c.name),
            None => None,
        };
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: rec.client.as_ref().map(|c| c.id.clone()),
            client_name: rec.client_name,
            project_id: rec.project_id,
            project_name,
            environment_id: rec.environment_id,
            environment_name,
            connection_id: rec.connection_id,
            connection_name,
            action: rec.action,
            summary: rec.summary.chars().take(500).collect(),
            decision: rec.decision,
            created_at: Utc::now(),
        };
        ai::insert_audit(self.pool(), &entry).await?;
        Ok(entry)
    }

    pub async fn list_audit(&self, limit: i64) -> Result<Vec<AuditEntry>> {
        ai::list_audit(self.pool(), limit.clamp(1, 1000)).await
    }

    // ---------- .env import / export ----------

    /// Import already-classified entries. Existing variables of the same kind are
    /// overwritten; kind conflicts are skipped and reported.
    pub async fn import_variables(
        &self,
        environment_id: &str,
        entries: Vec<(String, String, VariableKind)>,
    ) -> Result<ImportReport> {
        self.get_environment(environment_id).await?;
        let mut report = ImportReport::default();
        for (name, value, kind) in entries {
            let outcome = match kind {
                VariableKind::Public => self
                    .set_public_variable(environment_id, &name, &value)
                    .await
                    .map(|_| ()),
                VariableKind::Secret => self
                    .set_secret_variable(environment_id, &name, SecretValue::new(value))
                    .await
                    .map(|_| ()),
            };
            match outcome {
                Ok(()) => match kind {
                    VariableKind::Public => report.public_added += 1,
                    VariableKind::Secret => report.secret_added += 1,
                },
                Err(err) => report.skipped.push(format!("{name}: {err}")),
            }
        }
        Ok(report)
    }

    /// `.env.example` text for an environment (secrets blank).
    pub async fn render_env_example(&self, environment_id: &str) -> Result<String> {
        let vars: Vec<Variable> = self.list_variables(environment_id).await?;
        Ok(dotenv::render_example(&vars))
    }

    // ---------- internal-tier secret consumers ----------
    //
    // These two methods hand plaintext to *code*, never to a wire format. They are
    // the only sanctioned consumers of decrypted secrets: the process runner
    // (`envfish run`) and the Broker. Neither is reachable from an AI-facing API
    // by value: the runner is human-initiated, the Broker returns responses only.

    /// Decrypt every variable of an environment for injection into a child process.
    pub async fn resolve_process_env(&self, environment_id: &str) -> Result<ProcessEnv> {
        let vars = self.list_variables(environment_id).await?;
        let mut out = Vec::with_capacity(vars.len());
        let (mut public_count, mut secret_count) = (0, 0);
        for v in vars {
            match v.kind {
                VariableKind::Public => {
                    public_count += 1;
                    out.push((v.name, SecretValue::new(v.value.unwrap_or_default())));
                }
                VariableKind::Secret => {
                    secret_count += 1;
                    let value = self.open_secret(environment_id, &v.name).await?;
                    out.push((v.name, value));
                }
            }
        }
        Ok(ProcessEnv {
            vars: out,
            public_count,
            secret_count,
        })
    }

    /// Run `f` with the plaintext of one SECRET. The closure shape discourages
    /// holding onto the value; the result must not embed it.
    pub async fn with_secret<R>(
        &self,
        environment_id: &str,
        name: &str,
        f: impl FnOnce(&str) -> R,
    ) -> Result<R> {
        let secret = self.open_secret(environment_id, name).await?;
        Ok(f(secret.expose()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use envfish_vault::InMemoryMasterKeyProvider;

    async fn app() -> EnvFish {
        EnvFish::open_in_memory(&InMemoryMasterKeyProvider::random())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn settings_roundtrip() {
        let app = app().await;
        assert_eq!(app.settings().await.unwrap().language, "system");
        assert_eq!(app.set_language("EN").await.unwrap().language, "en");
        assert!(app.set_language("fr").await.is_err());
    }

    #[tokio::test]
    async fn connection_requires_secret_credential() {
        let app = app().await;
        let p = app.create_project("P", None).await.unwrap();
        let e = app.create_environment(&p.id, "development").await.unwrap();
        let new = |secret: Option<&str>| NewConnection {
            environment_id: e.id.clone(),
            kind: ConnectionKind::Openai,
            name: "openai".into(),
            base_url: None,
            auth_secret: secret.map(String::from),
            auth_style: None,
            metadata: None,
        };
        assert!(matches!(
            app.create_connection(new(Some("OPENAI_API_KEY"))).await,
            Err(CoreError::VariableNotFound(_))
        ));
        app.set_public_variable(&e.id, "PUBKEY", "x").await.unwrap();
        assert!(matches!(
            app.create_connection(new(Some("PUBKEY"))).await,
            Err(CoreError::InvalidName(_))
        ));
        app.set_secret_variable(&e.id, "OPENAI_API_KEY", SecretValue::new("sk-1"))
            .await
            .unwrap();
        let c = app.create_connection(new(Some("OPENAI_API_KEY"))).await.unwrap();
        assert_eq!(c.base_url, "https://api.openai.com/v1");
        assert_eq!(c.auth_style, "bearer");
        assert_eq!(app.resolve_connection(&e.id, "OPENAI").await.unwrap().id, c.id);
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("sk-1"));
        assert!(matches!(
            app.create_connection(new(Some("OPENAI_API_KEY"))).await,
            Err(CoreError::AlreadyExists("connection"))
        ));
    }

    #[tokio::test]
    async fn permissions_and_approvals() {
        let app = app().await;
        let p = app.create_project("P", None).await.unwrap();
        let dev = app.create_environment(&p.id, "development").await.unwrap();
        let prod = app.create_environment(&p.id, "production").await.unwrap();
        let client = app
            .register_ai_client("Claude Code", "claude_code")
            .await
            .unwrap();
        let again = app
            .register_ai_client("claude code", "claude_code")
            .await
            .unwrap();
        assert_eq!(client.id, again.id);

        let scope_dev = PermissionScope {
            client_id: Some(client.id.clone()),
            project_id: Some(p.id.clone()),
            environment_id: Some(dev.id.clone()),
            connection_id: None,
        };
        let scope_prod = PermissionScope {
            environment_id: Some(prod.id.clone()),
            ..scope_dev.clone()
        };
        assert_eq!(
            app.decide(&scope_dev, Action::Read).await.unwrap(),
            Decision::Allow
        );
        assert_eq!(
            app.decide(&scope_dev, Action::Write).await.unwrap(),
            Decision::Ask
        );
        assert_eq!(
            app.decide(&scope_prod, Action::Read).await.unwrap(),
            Decision::Ask
        );
        assert_eq!(
            app.decide(&scope_prod, Action::Write).await.unwrap(),
            Decision::Deny
        );

        let rule = app
            .set_permission(scope_dev.clone(), Action::Write, Decision::Allow)
            .await
            .unwrap();
        assert_eq!(
            app.decide(&scope_dev, Action::Write).await.unwrap(),
            Decision::Allow
        );
        let updated = app
            .set_permission(scope_dev.clone(), Action::Write, Decision::Deny)
            .await
            .unwrap();
        assert_eq!(updated.id, rule.id, "same scope+action updates in place");
        assert_eq!(app.list_permissions().await.unwrap().len(), 1);

        let approval = app
            .request_approval(&client, &scope_prod, Action::Read, "GET /rest/v1/beans", 60)
            .await
            .unwrap();
        assert_eq!(approval.status, ApprovalStatus::Pending);
        let prefix = &approval.id[..8];
        let resolved = app.resolve_approval(prefix, true).await.unwrap();
        assert_eq!(resolved.status, ApprovalStatus::Approved);
        assert!(matches!(
            app.resolve_approval(&approval.id, false).await,
            Err(CoreError::ApprovalNotPending(_))
        ));

        let expired = app
            .request_approval(&client, &scope_prod, Action::Read, "x", 5)
            .await
            .unwrap();
        // Force expiry by rewriting the deadline in the past.
        sqlx::query("UPDATE approvals SET expires_at = '2000-01-01T00:00:00+00:00' WHERE id = ?")
            .bind(&expired.id)
            .execute(app.pool())
            .await
            .unwrap();
        assert_eq!(
            app.get_approval(&expired.id).await.unwrap().status,
            ApprovalStatus::Expired
        );

        let entry = app
            .record_audit(AuditRecord {
                client: Some(client.clone()),
                client_name: client.name.clone(),
                project_id: Some(p.id.clone()),
                environment_id: Some(prod.id.clone()),
                connection_id: None,
                action: Action::Read,
                summary: "GET /rest/v1/beans".into(),
                decision: "ALLOWED".into(),
            })
            .await
            .unwrap();
        assert_eq!(entry.project_name.as_deref(), Some("P"));
        assert_eq!(entry.environment_name.as_deref(), Some("production"));
        assert_eq!(app.list_audit(10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn import_and_process_env() {
        let app = app().await;
        let p = app.create_project("P", None).await.unwrap();
        let e = app.create_environment(&p.id, "development").await.unwrap();
        let report = app
            .import_variables(
                &e.id,
                vec![
                    ("APP_URL".into(), "http://localhost".into(), VariableKind::Public),
                    ("OPENAI_API_KEY".into(), "sk-import".into(), VariableKind::Secret),
                    ("bad name".into(), "x".into(), VariableKind::Public),
                ],
            )
            .await
            .unwrap();
        assert_eq!(
            (report.public_added, report.secret_added, report.skipped.len()),
            (1, 1, 1)
        );

        let env = app.resolve_process_env(&e.id).await.unwrap();
        assert_eq!((env.public_count(), env.secret_count()), (1, 1));
        let mut cmd = std::process::Command::new("true");
        env.apply_to(&mut cmd);
        let injected: Vec<_> = cmd
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(|v| v.to_os_string())))
            .collect();
        assert!(
            injected
                .iter()
                .any(|(k, v)| k == "OPENAI_API_KEY" && v.as_deref().is_some_and(|v| v == "sk-import"))
        );
        assert!(!format!("{env:?}").contains("sk-import"));

        let len = app
            .with_secret(&e.id, "OPENAI_API_KEY", |s| s.len())
            .await
            .unwrap();
        assert_eq!(len, "sk-import".len());

        let example = app.render_env_example(&e.id).await.unwrap();
        assert!(example.contains("APP_URL=http://localhost"));
        assert!(example.contains("OPENAI_API_KEY=\n"));
        assert!(!example.contains("sk-import"));
    }
}
