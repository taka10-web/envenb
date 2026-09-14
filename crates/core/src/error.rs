pub type Result<T> = std::result::Result<T, CoreError>;

/// Core error type. Messages never include secret values.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("environment not found: {0}")]
    EnvironmentNotFound(String),

    #[error("variable not found: {0}")]
    VariableNotFound(String),

    #[error(
        "a SECRET cannot be turned into a PUBLIC variable; delete it and set the value again if it is not secret"
    )]
    CannotRevealSecret,

    #[error("connection not found: {0}")]
    ConnectionNotFound(String),

    #[error("credential not found: {0}")]
    CredentialNotFound(String),

    #[error("ai client not found: {0}")]
    ClientNotFound(String),

    #[error("permission rule not found: {0}")]
    PermissionNotFound(String),

    #[error("approval not found: {0}")]
    ApprovalNotFound(String),

    #[error("approval is no longer pending: {0}")]
    ApprovalNotPending(String),

    #[error("a {0} with that name already exists")]
    AlreadyExists(&'static str),

    #[error("invalid name: {0}")]
    InvalidName(String),

    #[error("no project selected; run `envfish use <project>` first")]
    NoCurrentProject,

    #[error("no environment selected; run `envfish env <environment>` first")]
    NoCurrentEnvironment,

    #[error("vault error: {0}")]
    Vault(#[from] envfish_vault::VaultError),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("io error at {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("state file is corrupted: {0}")]
    StateParse(#[from] serde_json::Error),

    #[error("could not determine a data directory for EnvFish")]
    NoDataDir,
}
