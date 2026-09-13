use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub local_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum VariableKind {
    Public,
    Secret,
}

impl VariableKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VariableKind::Public => "PUBLIC",
            VariableKind::Secret => "SECRET",
        }
    }
}

impl std::fmt::Display for VariableKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for VariableKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "PUBLIC" => Ok(VariableKind::Public),
            "SECRET" => Ok(VariableKind::Secret),
            other => Err(format!("unknown variable kind: {other}")),
        }
    }
}

/// A variable as seen by *listing* consumers (CLI tables, the desktop UI, and
/// eventually AI-facing tools).
///
/// `value` is always `None` when `kind == Secret`. This type is serializable
/// precisely because it can never carry a secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub kind: VariableKind,
    pub value: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Phase 2 entities
// ---------------------------------------------------------------------------

/// How a connection authenticates. Credentials are referenced by SECRET name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    /// Any HTTP API: base URL + one credential injected per `auth_style`.
    GenericHttp,
    /// OpenAI-compatible API (`Authorization: Bearer <key>`).
    Openai,
    /// Supabase project (`apikey` + `Authorization: Bearer` headers, PostgREST under `/rest/v1`).
    Supabase,
}

impl ConnectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectionKind::GenericHttp => "generic_http",
            ConnectionKind::Openai => "openai",
            ConnectionKind::Supabase => "supabase",
        }
    }

    /// Base URL used when the user omits one.
    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            ConnectionKind::Openai => Some("https://api.openai.com/v1"),
            _ => None,
        }
    }

    pub fn default_auth_style(self) -> &'static str {
        match self {
            ConnectionKind::GenericHttp => "bearer",
            ConnectionKind::Openai => "bearer",
            ConnectionKind::Supabase => "supabase",
        }
    }
}

impl std::str::FromStr for ConnectionKind {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            "generic_http" | "http" | "generic" => Ok(ConnectionKind::GenericHttp),
            "openai" => Ok(ConnectionKind::Openai),
            "supabase" => Ok(ConnectionKind::Supabase),
            other => Err(format!("unknown connection kind: {other}")),
        }
    }
}

impl std::fmt::Display for ConnectionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An external service connection. Safe to serialize: `auth_secret` is the *name*
/// of a SECRET variable, never its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub project_id: String,
    pub environment_id: String,
    pub kind: ConnectionKind,
    pub name: String,
    pub base_url: String,
    pub auth_secret: Option<String>,
    /// `bearer` | `header:<Name>` | `query:<name>` | `none` | `supabase`
    pub auth_style: String,
    /// Non-secret JSON metadata (extra headers, notes).
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiClient {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Action {
    Read,
    Write,
    Delete,
}

impl Action {
    pub const ALL: [Action; 3] = [Action::Read, Action::Write, Action::Delete];

    pub fn as_str(self) -> &'static str {
        match self {
            Action::Read => "READ",
            Action::Write => "WRITE",
            Action::Delete => "DELETE",
        }
    }

    /// Map an HTTP method onto an action class.
    pub fn from_http_method(method: &str) -> Action {
        match method.to_ascii_uppercase().as_str() {
            "GET" | "HEAD" | "OPTIONS" => Action::Read,
            "DELETE" => Action::Delete,
            _ => Action::Write,
        }
    }
}

impl std::str::FromStr for Action {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "READ" => Ok(Action::Read),
            "WRITE" => Ok(Action::Write),
            "DELETE" => Ok(Action::Delete),
            other => Err(format!("unknown action: {other}")),
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Decision {
    Allow,
    Ask,
    Deny,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "ALLOW",
            Decision::Ask => "ASK",
            Decision::Deny => "DENY",
        }
    }
}

impl std::str::FromStr for Decision {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "ALLOW" => Ok(Decision::Allow),
            "ASK" => Ok(Decision::Ask),
            "DENY" => Ok(Decision::Deny),
            other => Err(format!("unknown decision: {other}")),
        }
    }
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A permission rule. `None` scope fields match anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub client_id: Option<String>,
    pub project_id: Option<String>,
    pub environment_id: Option<String>,
    pub connection_id: Option<String>,
    pub action: Action,
    pub decision: Decision,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "PENDING",
            ApprovalStatus::Approved => "APPROVED",
            ApprovalStatus::Denied => "DENIED",
            ApprovalStatus::Expired => "EXPIRED",
        }
    }
}

impl std::str::FromStr for ApprovalStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "PENDING" => Ok(ApprovalStatus::Pending),
            "APPROVED" => Ok(ApprovalStatus::Approved),
            "DENIED" => Ok(ApprovalStatus::Denied),
            "EXPIRED" => Ok(ApprovalStatus::Expired),
            other => Err(format!("unknown approval status: {other}")),
        }
    }
}

/// A pending (or resolved) human decision for an ASK permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub id: String,
    pub client_id: String,
    pub client_name: String,
    pub project_id: Option<String>,
    pub environment_id: Option<String>,
    pub connection_id: Option<String>,
    pub action: Action,
    pub summary: String,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub client_id: Option<String>,
    pub client_name: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub environment_id: Option<String>,
    pub environment_name: Option<String>,
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub action: Action,
    pub summary: String,
    /// ALLOWED | DENIED | ASKED | ERROR
    pub decision: String,
    pub created_at: DateTime<Utc>,
}

/// Persisted user settings (shared by CLI and Desktop).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Settings {
    /// `ja` | `en` | `system`
    pub language: String,
    /// `file` | `keychain`
    pub key_backend: String,
    /// `system` | `light` | `dark`
    pub theme: String,
}
