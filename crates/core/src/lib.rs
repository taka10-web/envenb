//! # envenb-core
//!
//! Domain models, SQLite persistence and the application service for EnvEnb.
//!
//! Layering (top to bottom):
//!
//! - [`EnvEnb`] — the service façade used by the CLI, the Tauri desktop app and,
//!   later, the Local Agent. It is the only place that touches the vault.
//! - `repo` — thin SQLx queries. No business rules, no crypto.
//! - [`envenb_vault`] — sealing/opening secrets.
//!
//! ## Secret handling contract
//!
//! - Plaintext secrets are represented by [`SecretValue`], which cannot be
//!   `Display`ed, `Serialize`d, or `Debug`-printed in the clear.
//! - Listing APIs return [`Variable`] whose `value` is `None` for secrets.
//! - There is deliberately **no** `get_secret`-style method on [`EnvEnb`].
//!   Human-only reveal and process injection will be added as separate,
//!   clearly named, non-AI-facing entry points in later phases.

#[cfg(feature = "clipboard")]
pub mod clipboard;
mod db;
pub mod dotenv;
pub mod biometric;
pub mod env_compat;
mod error;
mod model;
mod paths;
pub mod permission;
mod repo;
mod repo_ai;
mod repo_cred;
mod secret;
mod service;
mod service_ai;
mod service_cred;
mod state;
pub mod totp;

pub use db::open_pool;
pub use error::{CoreError, Result};
pub use model::{
    Action, AiClient, Approval, ApprovalStatus, AuditEntry, Connection, ConnectionKind, Credential,
    CredentialField, CredentialKind, Decision, Environment, FieldSpec, Permission, Project, Settings,
    Variable, VariableKind,
};
pub use paths::Paths;
pub use secret::SecretValue;
pub use service::{EnvEnb, StatusReport};
pub use service_ai::{AuditRecord, ImportReport, NewConnection, PermissionScope, ProcessEnv};
pub use service_cred::{CredentialFieldInput, NewCredential};
pub use state::CliState;

pub use envenb_vault as vault;
