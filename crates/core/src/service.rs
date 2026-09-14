//! The application service. Everything above this layer (CLI, desktop, daemon)
//! talks to [`EnvFish`]; nothing above this layer touches the vault or SQL.

use chrono::Utc;
#[cfg(feature = "keychain")]
use envfish_vault::KeychainMasterKeyProvider;
use envfish_vault::{FileMasterKeyProvider, MasterKeyProvider, Vault};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::{CoreError, Result};
use crate::model::{Environment, Project, Variable, VariableKind};
use crate::paths::Paths;
use crate::repo;
use crate::secret::SecretValue;

/// Snapshot for `envfish status` and the desktop overview. Contains counts and
/// identifiers only.
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub data_dir: String,
    pub database_path: String,
    pub master_key_location: String,
    pub project_count: usize,
    pub secret_count: i64,
}

pub struct EnvFish {
    pool: SqlitePool,
    vault: Vault,
    paths: Paths,
}

impl std::fmt::Debug for EnvFish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvFish")
            .field("paths", &self.paths)
            .finish_non_exhaustive()
    }
}

impl EnvFish {
    /// Open EnvFish at the default (or `ENVFISH_HOME`) data directory using the
    /// file-based master key provider.
    pub async fn open_default() -> Result<Self> {
        let paths = Paths::resolve()?;
        Self::open_at(paths).await
    }

    /// Open using the key backend recorded in settings (`file` by default).
    pub async fn open_at(paths: Paths) -> Result<Self> {
        paths.ensure_root()?;
        let pool = crate::db::open_pool(&paths.database()).await?;
        let backend = match crate::repo_ai::get_setting(&pool, crate::service_ai::SETTING_KEY_BACKEND).await?
        {
            Some(b) => b,
            None => {
                let chosen = Self::default_key_backend(&paths);
                crate::repo_ai::set_setting(&pool, crate::service_ai::SETTING_KEY_BACKEND, chosen).await?;
                chosen.to_string()
            }
        };
        let provider = Self::provider_for(&paths, &backend)?;
        let vault = Vault::open(provider.as_ref())?;
        Ok(Self { pool, vault, paths })
    }

    /// Backend for a data directory that has no recorded choice yet.
    ///
    /// - `ENVFISH_KEY_BACKEND` (file | keychain) wins when set (tests, CI, scripts).
    /// - An existing `master.key` file keeps the file backend (upgrade path).
    /// - On macOS with the keychain feature, new vaults default to the OS keychain so
    ///   that copying the data directory is not enough to read secrets.
    /// - Otherwise `file`.
    pub fn default_key_backend(paths: &Paths) -> &'static str {
        if let Ok(v) = std::env::var("ENVFISH_KEY_BACKEND") {
            return if v == "keychain" { "keychain" } else { "file" };
        }
        if paths.master_key().exists() || std::env::var_os("CI").is_some() {
            return "file";
        }
        if cfg!(all(target_os = "macos", feature = "keychain")) {
            return "keychain";
        }
        "file"
    }

    /// Build the master key provider for a backend name.
    pub fn provider_for(paths: &Paths, backend: &str) -> Result<Box<dyn MasterKeyProvider>> {
        match backend {
            "file" => Ok(Box::new(FileMasterKeyProvider::new(paths.master_key()))),
            #[cfg(feature = "keychain")]
            "keychain" => Ok(Box::new(KeychainMasterKeyProvider::new(paths.profile()))),
            other => Err(CoreError::InvalidName(format!(
                "unsupported key backend: {other}"
            ))),
        }
    }

    /// Open with an explicit master key provider (tests, future keychain backends).
    pub async fn open_with(paths: Paths, provider: &dyn MasterKeyProvider) -> Result<Self> {
        paths.ensure_root()?;
        let pool = crate::db::open_pool(&paths.database()).await?;
        let vault = Vault::open(provider)?;
        Ok(Self { pool, vault, paths })
    }

    /// Fully in-memory instance for tests.
    #[doc(hidden)]
    pub async fn open_in_memory(provider: &dyn MasterKeyProvider) -> Result<Self> {
        let pool = crate::db::open_memory_pool().await?;
        let vault = Vault::open(provider)?;
        Ok(Self {
            pool,
            vault,
            paths: Paths::at(":memory:"),
        })
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub(crate) fn vault(&self) -> &Vault {
        &self.vault
    }

    // ---------- status ----------

    pub async fn status(&self) -> Result<StatusReport> {
        let projects = repo::list_projects(&self.pool).await?;
        let secret_count = repo::count_secrets(&self.pool).await?;
        Ok(StatusReport {
            data_dir: self.paths.root().display().to_string(),
            database_path: self.paths.database().display().to_string(),
            master_key_location: self.vault.key_location().to_string(),
            project_count: projects.len(),
            secret_count,
        })
    }

    // ---------- projects ----------

    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        repo::list_projects(&self.pool).await
    }

    pub async fn get_project(&self, id: &str) -> Result<Project> {
        repo::find_project_by_id(&self.pool, id)
            .await?
            .ok_or_else(|| CoreError::ProjectNotFound(id.to_string()))
    }

    /// Resolve a project by id or (case-insensitive) name.
    pub async fn resolve_project(&self, id_or_name: &str) -> Result<Project> {
        if let Some(p) = repo::find_project_by_id(&self.pool, id_or_name).await? {
            return Ok(p);
        }
        repo::find_project_by_name(&self.pool, id_or_name)
            .await?
            .ok_or_else(|| CoreError::ProjectNotFound(id_or_name.to_string()))
    }

    pub async fn create_project(&self, name: &str, local_path: Option<&str>) -> Result<Project> {
        let name = validate_name(name, "project")?;
        if repo::find_project_by_name(&self.pool, &name).await?.is_some() {
            return Err(CoreError::AlreadyExists("project"));
        }
        let now = Utc::now();
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            local_path: local_path.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            created_at: now,
            updated_at: now,
        };
        repo::insert_project(&self.pool, &project).await?;
        tracing::info!(project = %project.name, "project created");
        Ok(project)
    }

    pub async fn delete_project(&self, id: &str) -> Result<()> {
        if !repo::delete_project(&self.pool, id).await? {
            return Err(CoreError::ProjectNotFound(id.to_string()));
        }
        Ok(())
    }

    // ---------- environments ----------

    pub async fn list_environments(&self, project_id: &str) -> Result<Vec<Environment>> {
        self.get_project(project_id).await?;
        repo::list_environments(&self.pool, project_id).await
    }

    pub async fn get_environment(&self, id: &str) -> Result<Environment> {
        repo::find_environment_by_id(&self.pool, id)
            .await?
            .ok_or_else(|| CoreError::EnvironmentNotFound(id.to_string()))
    }

    pub async fn resolve_environment(&self, project_id: &str, id_or_name: &str) -> Result<Environment> {
        if let Some(e) = repo::find_environment_by_id(&self.pool, id_or_name).await?
            && e.project_id == project_id
        {
            return Ok(e);
        }
        repo::find_environment_by_name(&self.pool, project_id, id_or_name)
            .await?
            .ok_or_else(|| CoreError::EnvironmentNotFound(id_or_name.to_string()))
    }

    pub async fn create_environment(&self, project_id: &str, name: &str) -> Result<Environment> {
        self.get_project(project_id).await?;
        let name = validate_name(name, "environment")?;
        if repo::find_environment_by_name(&self.pool, project_id, &name)
            .await?
            .is_some()
        {
            return Err(CoreError::AlreadyExists("environment"));
        }
        let now = Utc::now();
        let env = Environment {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            name,
            created_at: now,
            updated_at: now,
        };
        repo::insert_environment(&self.pool, &env).await?;
        Ok(env)
    }

    pub async fn delete_environment(&self, id: &str) -> Result<()> {
        if !repo::delete_environment(&self.pool, id).await? {
            return Err(CoreError::EnvironmentNotFound(id.to_string()));
        }
        Ok(())
    }

    // ---------- variables ----------

    /// List variables. Secret entries have `value == None`.
    pub async fn list_variables(&self, environment_id: &str) -> Result<Vec<Variable>> {
        self.get_environment(environment_id).await?;
        repo::list_variables(&self.pool, environment_id).await
    }

    /// Create or update a PUBLIC variable.
    pub async fn set_public_variable(
        &self,
        environment_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Variable> {
        self.get_environment(environment_id).await?;
        let name = validate_var_name(name)?;
        self.ensure_kind_slot(environment_id, &name, VariableKind::Public)
            .await?;
        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();
        repo::upsert_public_variable(&self.pool, &id, environment_id, &name, value, now).await?;
        self.find_variable(environment_id, &name).await
    }

    /// Create or update a SECRET variable. The plaintext is sealed before it reaches SQLite.
    pub async fn set_secret_variable(
        &self,
        environment_id: &str,
        name: &str,
        value: SecretValue,
    ) -> Result<Variable> {
        self.get_environment(environment_id).await?;
        let name = validate_var_name(name)?;
        self.ensure_kind_slot(environment_id, &name, VariableKind::Secret)
            .await?;

        // Reuse the existing row id when updating so the AEAD associated data stays stable.
        let id = match repo::find_variable_kind(&self.pool, environment_id, &name).await? {
            Some((existing_id, VariableKind::Secret)) => existing_id,
            _ => uuid::Uuid::new_v4().to_string(),
        };
        let sealed = self.vault.encrypt(value.as_secret_string(), id.as_bytes())?;
        drop(value);

        repo::upsert_secret(&self.pool, &id, environment_id, &name, &sealed, Utc::now()).await?;
        tracing::info!(variable = %name, "secret stored");
        self.find_variable(environment_id, &name).await
    }

    pub async fn delete_variable(&self, environment_id: &str, name: &str) -> Result<()> {
        if !repo::delete_variable_by_name(&self.pool, environment_id, name).await? {
            return Err(CoreError::VariableNotFound(name.to_string()));
        }
        Ok(())
    }

    /// Decrypt a secret for **in-process use only** (future: process runner, broker).
    ///
    /// This is `pub(crate)` on purpose. No public, AI-reachable API exposes it; when a
    /// human-only reveal is added it will be a separate entry point with its own gate.
    pub(crate) async fn open_secret(&self, environment_id: &str, name: &str) -> Result<SecretValue> {
        let (id, sealed) = repo::find_sealed_secret(&self.pool, environment_id, name)
            .await?
            .ok_or_else(|| CoreError::VariableNotFound(name.to_string()))?;
        let plaintext = self.vault.decrypt(&sealed, id.as_bytes())?;
        Ok(SecretValue::from(plaintext))
    }

    async fn find_variable(&self, environment_id: &str, name: &str) -> Result<Variable> {
        repo::list_variables(&self.pool, environment_id)
            .await?
            .into_iter()
            .find(|v| v.name == name)
            .ok_or_else(|| CoreError::VariableNotFound(name.to_string()))
    }

    /// A name may exist as PUBLIC or SECRET but not both. Changing kind requires an explicit delete.
    async fn ensure_kind_slot(&self, environment_id: &str, name: &str, wanted: VariableKind) -> Result<()> {
        match repo::find_variable_kind(&self.pool, environment_id, name).await? {
            Some((_, existing)) if existing != wanted => Err(CoreError::AlreadyExists(match existing {
                VariableKind::Public => "PUBLIC variable",
                VariableKind::Secret => "SECRET variable",
            })),
            _ => Ok(()),
        }
    }
}

fn validate_name(raw: &str, what: &str) -> Result<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(CoreError::InvalidName(format!("{what} name must not be empty")));
    }
    if name.len() > 100 {
        return Err(CoreError::InvalidName(format!("{what} name is too long")));
    }
    Ok(name.to_string())
}

fn validate_var_name(raw: &str) -> Result<String> {
    let name = raw.trim();
    let ok = !name.is_empty()
        && name.len() <= 200
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.chars().next().is_some_and(|c| c.is_ascii_digit());
    if !ok {
        return Err(CoreError::InvalidName(format!(
            "variable name must match [A-Za-z_][A-Za-z0-9_]*, got {name:?}"
        )));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use envfish_vault::InMemoryMasterKeyProvider;
    use sqlx::Row;

    async fn app() -> EnvFish {
        EnvFish::open_in_memory(&InMemoryMasterKeyProvider::random())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn project_crud_and_resolution() {
        let app = app().await;
        let p = app.create_project("my-app", Some("/tmp/my-app")).await.unwrap();
        assert_eq!(app.list_projects().await.unwrap().len(), 1);
        assert_eq!(app.resolve_project("my-app").await.unwrap().id, p.id);
        assert_eq!(app.resolve_project(&p.id).await.unwrap().id, p.id);
        assert!(matches!(
            app.create_project("MY-APP", None).await,
            Err(CoreError::AlreadyExists("project"))
        ));
        assert!(matches!(
            app.create_project("  ", None).await,
            Err(CoreError::InvalidName(_))
        ));
        app.delete_project(&p.id).await.unwrap();
        assert!(app.list_projects().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn environments_belong_to_projects() {
        let app = app().await;
        let a = app.create_project("A", None).await.unwrap();
        let b = app.create_project("B", None).await.unwrap();
        let dev = app.create_environment(&a.id, "development").await.unwrap();
        app.create_environment(&b.id, "development").await.unwrap();
        assert_eq!(app.list_environments(&a.id).await.unwrap().len(), 1);
        assert!(app.resolve_environment(&b.id, &dev.id).await.is_err());
        assert_eq!(
            app.resolve_environment(&a.id, "DEVELOPMENT").await.unwrap().id,
            dev.id
        );
        assert!(matches!(
            app.list_environments("nope").await,
            Err(CoreError::ProjectNotFound(_))
        ));
    }

    #[tokio::test]
    async fn public_and_secret_variables() {
        let app = app().await;
        let p = app.create_project("P", None).await.unwrap();
        let e = app.create_environment(&p.id, "dev").await.unwrap();

        let pubv = app
            .set_public_variable(&e.id, "APP_URL", "http://localhost")
            .await
            .unwrap();
        assert_eq!(pubv.kind, VariableKind::Public);
        assert_eq!(pubv.value.as_deref(), Some("http://localhost"));

        let sec = app
            .set_secret_variable(&e.id, "OPENAI_API_KEY", SecretValue::new("sk-live-999"))
            .await
            .unwrap();
        assert_eq!(sec.kind, VariableKind::Secret);
        assert_eq!(sec.value, None, "listing must never carry secret values");

        let listed = app.list_variables(&e.id).await.unwrap();
        assert_eq!(listed.len(), 2);
        let json = serde_json::to_string(&listed).unwrap();
        assert!(!json.contains("sk-live-999"));

        // Round trip through the internal opener.
        let opened = app.open_secret(&e.id, "OPENAI_API_KEY").await.unwrap();
        assert_eq!(opened.expose(), "sk-live-999");

        // Update keeps the same id and still decrypts.
        let updated = app
            .set_secret_variable(&e.id, "OPENAI_API_KEY", SecretValue::new("sk-live-000"))
            .await
            .unwrap();
        assert_eq!(updated.id, sec.id);
        assert_eq!(
            app.open_secret(&e.id, "OPENAI_API_KEY").await.unwrap().expose(),
            "sk-live-000"
        );

        // Kind conflicts are rejected.
        assert!(matches!(
            app.set_public_variable(&e.id, "OPENAI_API_KEY", "x").await,
            Err(CoreError::AlreadyExists(_))
        ));
        assert!(matches!(
            app.set_secret_variable(&e.id, "APP_URL", SecretValue::new("x"))
                .await,
            Err(CoreError::AlreadyExists(_))
        ));

        app.delete_variable(&e.id, "OPENAI_API_KEY").await.unwrap();
        assert_eq!(app.list_variables(&e.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn secret_plaintext_never_hits_sqlite() {
        let app = app().await;
        let p = app.create_project("P", None).await.unwrap();
        let e = app.create_environment(&p.id, "dev").await.unwrap();
        let needle = "PLAINTEXT-NEEDLE-4242";
        app.set_secret_variable(&e.id, "TOKEN", SecretValue::new(needle))
            .await
            .unwrap();

        let rows = sqlx::query("SELECT ciphertext, nonce FROM secrets")
            .fetch_all(&app.pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        let ct: Vec<u8> = rows[0].get("ciphertext");
        let nonce: Vec<u8> = rows[0].get("nonce");
        assert!(!contains_subslice(&ct, needle.as_bytes()));
        assert!(!contains_subslice(&nonce, needle.as_bytes()));
        assert_eq!(nonce.len(), 24);
    }

    #[tokio::test]
    async fn variable_name_validation() {
        let app = app().await;
        let p = app.create_project("P", None).await.unwrap();
        let e = app.create_environment(&p.id, "dev").await.unwrap();
        for bad in ["", "1ABC", "has space", "has-dash", "ünïcode"] {
            assert!(
                matches!(
                    app.set_public_variable(&e.id, bad, "v").await,
                    Err(CoreError::InvalidName(_))
                ),
                "{bad:?} should be rejected"
            );
        }
        assert!(app.set_public_variable(&e.id, "_OK_123", "v").await.is_ok());
    }

    fn contains_subslice(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len()).any(|w| w == needle)
    }
}
