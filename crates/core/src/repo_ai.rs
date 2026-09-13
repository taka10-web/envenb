//! SQLx queries for Phase 2 entities: settings, connections, AI clients,
//! permissions, approvals, audit log. No business rules, no crypto.

use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::error::Result;
use crate::model::{
    Action, AiClient, Approval, ApprovalStatus, AuditEntry, Connection, ConnectionKind, Decision, Permission,
};

fn ts(row: &SqliteRow, col: &str) -> DateTime<Utc> {
    let raw: String = row.get(col);
    DateTime::parse_from_rfc3339(&raw)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn opt_ts(row: &SqliteRow, col: &str) -> Option<DateTime<Utc>> {
    let raw: Option<String> = row.get(col);
    raw.and_then(|r| DateTime::parse_from_rfc3339(&r).ok())
        .map(|d| d.with_timezone(&Utc))
}

fn rfc(t: DateTime<Utc>) -> String {
    t.to_rfc3339()
}

// ---------- settings ----------

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.get("value")))
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value)
    .bind(rfc(Utc::now()))
    .execute(pool)
    .await?;
    Ok(())
}

// ---------- connections ----------

fn connection_from_row(row: &SqliteRow) -> Connection {
    let kind_raw: String = row.get("kind");
    let metadata_raw: String = row.get("metadata");
    Connection {
        id: row.get("id"),
        project_id: row.get("project_id"),
        environment_id: row.get("environment_id"),
        kind: kind_raw.parse().unwrap_or(ConnectionKind::GenericHttp),
        name: row.get("name"),
        base_url: row.get("base_url"),
        auth_secret: row.get("auth_secret"),
        auth_style: row.get("auth_style"),
        metadata: serde_json::from_str(&metadata_raw)
            .unwrap_or(serde_json::Value::Object(Default::default())),
        created_at: ts(row, "created_at"),
        updated_at: ts(row, "updated_at"),
    }
}

pub async fn list_connections_for_env(pool: &SqlitePool, environment_id: &str) -> Result<Vec<Connection>> {
    let rows = sqlx::query("SELECT * FROM connections WHERE environment_id = ? ORDER BY name COLLATE NOCASE")
        .bind(environment_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(connection_from_row).collect())
}

pub async fn list_connections_for_project(pool: &SqlitePool, project_id: &str) -> Result<Vec<Connection>> {
    let rows = sqlx::query(
        "SELECT * FROM connections WHERE project_id = ? ORDER BY environment_id, name COLLATE NOCASE",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(connection_from_row).collect())
}

pub async fn find_connection_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Connection>> {
    let row = sqlx::query("SELECT * FROM connections WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(connection_from_row))
}

pub async fn find_connection_by_name(
    pool: &SqlitePool,
    environment_id: &str,
    name: &str,
) -> Result<Option<Connection>> {
    let row = sqlx::query("SELECT * FROM connections WHERE environment_id = ? AND name = ? COLLATE NOCASE")
        .bind(environment_id)
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(connection_from_row))
}

pub async fn insert_connection(pool: &SqlitePool, c: &Connection) -> Result<()> {
    sqlx::query(
        "INSERT INTO connections (id, project_id, environment_id, kind, name, base_url, auth_secret, auth_style, metadata, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&c.id)
    .bind(&c.project_id)
    .bind(&c.environment_id)
    .bind(c.kind.as_str())
    .bind(&c.name)
    .bind(&c.base_url)
    .bind(&c.auth_secret)
    .bind(&c.auth_style)
    .bind(c.metadata.to_string())
    .bind(rfc(c.created_at))
    .bind(rfc(c.updated_at))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_connection(pool: &SqlitePool, id: &str) -> Result<bool> {
    let r = sqlx::query("DELETE FROM connections WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---------- ai clients ----------

fn client_from_row(row: &SqliteRow) -> AiClient {
    AiClient {
        id: row.get("id"),
        name: row.get("name"),
        kind: row.get("kind"),
        created_at: ts(row, "created_at"),
        last_seen_at: opt_ts(row, "last_seen_at"),
    }
}

pub async fn list_ai_clients(pool: &SqlitePool) -> Result<Vec<AiClient>> {
    let rows = sqlx::query("SELECT * FROM ai_clients ORDER BY name COLLATE NOCASE")
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(client_from_row).collect())
}

pub async fn find_ai_client(pool: &SqlitePool, id_or_name: &str) -> Result<Option<AiClient>> {
    let row = sqlx::query("SELECT * FROM ai_clients WHERE id = ? OR name = ? COLLATE NOCASE LIMIT 1")
        .bind(id_or_name)
        .bind(id_or_name)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(client_from_row))
}

pub async fn insert_ai_client(pool: &SqlitePool, c: &AiClient) -> Result<()> {
    sqlx::query("INSERT INTO ai_clients (id, name, kind, created_at, last_seen_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&c.id)
        .bind(&c.name)
        .bind(&c.kind)
        .bind(rfc(c.created_at))
        .bind(c.last_seen_at.map(rfc))
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn touch_ai_client(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("UPDATE ai_clients SET last_seen_at = ? WHERE id = ?")
        .bind(rfc(Utc::now()))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_ai_client(pool: &SqlitePool, id: &str) -> Result<bool> {
    let r = sqlx::query("DELETE FROM ai_clients WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---------- permissions ----------

fn permission_from_row(row: &SqliteRow) -> Permission {
    let action: String = row.get("action");
    let decision: String = row.get("decision");
    Permission {
        id: row.get("id"),
        client_id: row.get("client_id"),
        project_id: row.get("project_id"),
        environment_id: row.get("environment_id"),
        connection_id: row.get("connection_id"),
        action: action.parse().unwrap_or(Action::Read),
        decision: decision.parse().unwrap_or(Decision::Deny),
        created_at: ts(row, "created_at"),
        updated_at: ts(row, "updated_at"),
    }
}

pub async fn list_permissions(pool: &SqlitePool) -> Result<Vec<Permission>> {
    let rows = sqlx::query("SELECT * FROM permissions ORDER BY updated_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(permission_from_row).collect())
}

/// Find a rule with exactly this scope and action (for upsert semantics).
pub async fn find_permission_exact(
    pool: &SqlitePool,
    client_id: Option<&str>,
    project_id: Option<&str>,
    environment_id: Option<&str>,
    connection_id: Option<&str>,
    action: Action,
) -> Result<Option<Permission>> {
    let row = sqlx::query(
        "SELECT * FROM permissions WHERE client_id IS ? AND project_id IS ? AND environment_id IS ? AND connection_id IS ? AND action = ?",
    )
    .bind(client_id)
    .bind(project_id)
    .bind(environment_id)
    .bind(connection_id)
    .bind(action.as_str())
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(permission_from_row))
}

pub async fn insert_permission(pool: &SqlitePool, p: &Permission) -> Result<()> {
    sqlx::query(
        "INSERT INTO permissions (id, client_id, project_id, environment_id, connection_id, action, decision, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&p.id)
    .bind(&p.client_id)
    .bind(&p.project_id)
    .bind(&p.environment_id)
    .bind(&p.connection_id)
    .bind(p.action.as_str())
    .bind(p.decision.as_str())
    .bind(rfc(p.created_at))
    .bind(rfc(p.updated_at))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_permission_decision(pool: &SqlitePool, id: &str, decision: Decision) -> Result<()> {
    sqlx::query("UPDATE permissions SET decision = ?, updated_at = ? WHERE id = ?")
        .bind(decision.as_str())
        .bind(rfc(Utc::now()))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_permission(pool: &SqlitePool, id: &str) -> Result<bool> {
    let r = sqlx::query("DELETE FROM permissions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

// ---------- approvals ----------

fn approval_from_row(row: &SqliteRow) -> Approval {
    let action: String = row.get("action");
    let status: String = row.get("status");
    Approval {
        id: row.get("id"),
        client_id: row.get("client_id"),
        client_name: row.get("client_name"),
        project_id: row.get("project_id"),
        environment_id: row.get("environment_id"),
        connection_id: row.get("connection_id"),
        action: action.parse().unwrap_or(Action::Read),
        summary: row.get("summary"),
        status: status.parse().unwrap_or(ApprovalStatus::Pending),
        created_at: ts(row, "created_at"),
        resolved_at: opt_ts(row, "resolved_at"),
        expires_at: ts(row, "expires_at"),
    }
}

pub async fn insert_approval(pool: &SqlitePool, a: &Approval) -> Result<()> {
    sqlx::query(
        "INSERT INTO approvals (id, client_id, client_name, project_id, environment_id, connection_id, action, summary, status, created_at, resolved_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&a.id)
    .bind(&a.client_id)
    .bind(&a.client_name)
    .bind(&a.project_id)
    .bind(&a.environment_id)
    .bind(&a.connection_id)
    .bind(a.action.as_str())
    .bind(&a.summary)
    .bind(a.status.as_str())
    .bind(rfc(a.created_at))
    .bind(a.resolved_at.map(rfc))
    .bind(rfc(a.expires_at))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_approval(pool: &SqlitePool, id: &str) -> Result<Option<Approval>> {
    let row = sqlx::query("SELECT * FROM approvals WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(approval_from_row))
}

pub async fn list_approvals(
    pool: &SqlitePool,
    status: Option<ApprovalStatus>,
    limit: i64,
) -> Result<Vec<Approval>> {
    let rows = match status {
        Some(s) => {
            sqlx::query("SELECT * FROM approvals WHERE status = ? ORDER BY created_at DESC LIMIT ?")
                .bind(s.as_str())
                .bind(limit)
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query("SELECT * FROM approvals ORDER BY created_at DESC LIMIT ?")
                .bind(limit)
                .fetch_all(pool)
                .await?
        }
    };
    Ok(rows.iter().map(approval_from_row).collect())
}

/// Resolve only if still pending; returns whether a row changed.
pub async fn resolve_approval(pool: &SqlitePool, id: &str, status: ApprovalStatus) -> Result<bool> {
    let r =
        sqlx::query("UPDATE approvals SET status = ?, resolved_at = ? WHERE id = ? AND status = 'PENDING'")
            .bind(status.as_str())
            .bind(rfc(Utc::now()))
            .bind(id)
            .execute(pool)
            .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn expire_approvals(pool: &SqlitePool) -> Result<u64> {
    let r = sqlx::query("UPDATE approvals SET status = 'EXPIRED', resolved_at = ? WHERE status = 'PENDING' AND expires_at < ?")
        .bind(rfc(Utc::now()))
        .bind(rfc(Utc::now()))
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

// ---------- audit ----------

fn audit_from_row(row: &SqliteRow) -> AuditEntry {
    let action: String = row.get("action");
    AuditEntry {
        id: row.get("id"),
        client_id: row.get("client_id"),
        client_name: row.get("client_name"),
        project_id: row.get("project_id"),
        project_name: row.get("project_name"),
        environment_id: row.get("environment_id"),
        environment_name: row.get("environment_name"),
        connection_id: row.get("connection_id"),
        connection_name: row.get("connection_name"),
        action: action.parse().unwrap_or(Action::Read),
        summary: row.get("summary"),
        decision: row.get("decision"),
        created_at: ts(row, "created_at"),
    }
}

pub async fn insert_audit(pool: &SqlitePool, e: &AuditEntry) -> Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs (id, client_id, client_name, project_id, project_name, environment_id, environment_name, connection_id, connection_name, action, summary, decision, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&e.id)
    .bind(&e.client_id)
    .bind(&e.client_name)
    .bind(&e.project_id)
    .bind(&e.project_name)
    .bind(&e.environment_id)
    .bind(&e.environment_name)
    .bind(&e.connection_id)
    .bind(&e.connection_name)
    .bind(e.action.as_str())
    .bind(&e.summary)
    .bind(&e.decision)
    .bind(rfc(e.created_at))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_audit(pool: &SqlitePool, limit: i64) -> Result<Vec<AuditEntry>> {
    let rows = sqlx::query("SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(audit_from_row).collect())
}
