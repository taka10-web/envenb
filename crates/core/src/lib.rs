//! # envfish-core
//!
//! Domain models, SQLite persistence and the application service for EnvFish.
//!
//! Layering (top to bottom):
//!
//! - [`EnvFish`] — the service façade used by the CLI, the Tauri desktop app and,
//!   later, the Local Agent. It is the only place that touches the vault.
//! - `repo` — thin SQLx queries. No business rules, no crypto.
//! - [`envfish_vault`] — sealing/opening secrets.
//!
//! ## Secret handling contract
//!
//! - Plaintext secrets are represented by [`SecretValue`], which cannot be
//!   `Display`ed, `Serialize`d, or `Debug`-printed in the clear.
//! - Listing APIs return [`Variable`] whose `value` is `None` for secrets.
//! - There is deliberately **no** `get_secret`-style method on [`EnvFish`].
//!   Human-only reveal and process injection will be added as separate,
//!   clearly named, non-AI-facing entry points in later phases.

mod db;
pub mod dotenv;
mod error;
mod model;
mod paths;
pub mod permission;
mod repo;
mod repo_ai;
mod secret;
mod service;
mod service_ai;
mod state;

pub use db::open_pool;
pub use error::{CoreError, Result};
pub use model::{
    Action, AiClient, Approval, ApprovalStatus, AuditEntry, Connection, ConnectionKind, Decision,
    Environment, Permission, Project, Settings, Variable, VariableKind,
};
pub use paths::Paths;
pub use secret::SecretValue;
pub use service::{EnvFish, StatusReport};
pub use service_ai::{AuditRecord, ImportReport, NewConnection, PermissionScope, ProcessEnv};
pub use state::CliState;

pub use envfish_vault as vault;
