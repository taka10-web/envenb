//! Thin SQLx queries. No business rules and no cryptography live here.

use chrono::{DateTime, Utc};
use envenb_vault::EncryptedSecret;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::error::Result;
use crate::model::{Environment, Project, Variable, VariableKind};

fn parse_ts(row: &SqliteRow, col: &str) -> DateTime<Utc> {
    let raw: String = row.get(col);
    DateTime::parse_from_rfc3339(&raw)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn project_from_row(row: &SqliteRow) -> Project {
    Project {
        id: row.get("id"),
        name: row.get("name"),
        local_path: row.get("local_path"),
        created_at: parse_ts(row, "created_at"),
        updated_at: parse_ts(row, "updated_at"),
    }
}

fn environment_from_row(row: &SqliteRow) -> Environment {
    Environment {
        id: row.get("id"),
        project_id: row.get("project_id"),
        name: row.get("name"),
        created_at: parse_ts(row, "created_at"),
        updated_at: parse_ts(row, "updated_at"),
    }
}

// ---------- projects ----------

pub async fn list_projects(pool: &SqlitePool) -> Result<Vec<Project>> {
    let rows = sqlx::query("SELECT * FROM projects ORDER BY name COLLATE NOCASE")
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(project_from_row).collect())
}

pub async fn find_project_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Project>> {
    let row = sqlx::query("SELECT * FROM projects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(project_from_row))
}

pub async fn find_project_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Project>> {
    let row = sqlx::query("SELECT * FROM projects WHERE name = ? COLLATE NOCASE")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(project_from_row))
}

pub async fn insert_project(pool: &SqlitePool, project: &Project) -> Result<()> {
    sqlx::query("INSERT INTO projects (id, name, local_path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.local_path)
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_project(pool: &SqlitePool, id: &str) -> Result<bool> {
    let res = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

// ---------- environments ----------

pub async fn list_environments(pool: &SqlitePool, project_id: &str) -> Result<Vec<Environment>> {
    let rows = sqlx::query("SELECT * FROM environments WHERE project_id = ? ORDER BY name COLLATE NOCASE")
        .bind(project_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(environment_from_row).collect())
}

pub async fn find_environment_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Environment>> {
    let row = sqlx::query("SELECT * FROM environments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(environment_from_row))
}

pub async fn find_environment_by_name(
    pool: &SqlitePool,
    project_id: &str,
    name: &str,
) -> Result<Option<Environment>> {
    let row = sqlx::query("SELECT * FROM environments WHERE project_id = ? AND name = ? COLLATE NOCASE")
        .bind(project_id)
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(environment_from_row))
}

pub async fn insert_environment(pool: &SqlitePool, env: &Environment) -> Result<()> {
    sqlx::query(
        "INSERT INTO environments (id, project_id, name, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&env.id)
    .bind(&env.project_id)
    .bind(&env.name)
    .bind(env.created_at.to_rfc3339())
    .bind(env.updated_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_environment(pool: &SqlitePool, id: &str) -> Result<bool> {
    let res = sqlx::query("DELETE FROM environments WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

// ---------- variables (PUBLIC) + secrets (SECRET) ----------

/// Unified listing of public variables and secret *metadata* for an environment.
/// Secret values are never selected from the database here.
pub async fn list_variables(pool: &SqlitePool, environment_id: &str) -> Result<Vec<Variable>> {
    let rows = sqlx::query(
        "SELECT id, environment_id, name, 'PUBLIC' AS kind, value, created_at, updated_at
           FROM variables WHERE environment_id = ?
         UNION ALL
         SELECT id, environment_id, name, 'SECRET' AS kind, NULL AS value, created_at, updated_at
           FROM secrets WHERE environment_id = ?
         ORDER BY name COLLATE NOCASE",
    )
    .bind(environment_id)
    .bind(environment_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            let kind_raw: String = row.get("kind");
            let kind = kind_raw.parse::<VariableKind>().unwrap_or(VariableKind::Public);
            Variable {
                id: row.get("id"),
                environment_id: row.get("environment_id"),
                name: row.get("name"),
                kind,
                value: if kind == VariableKind::Public {
                    row.get("value")
                } else {
                    None
                },
                created_at: parse_ts(row, "created_at"),
                updated_at: parse_ts(row, "updated_at"),
            }
        })
        .collect())
}

/// Which table (if any) currently holds `name` in this environment.
pub async fn find_variable_kind(
    pool: &SqlitePool,
    environment_id: &str,
    name: &str,
) -> Result<Option<(String, VariableKind)>> {
    let row = sqlx::query(
        "SELECT id, 'PUBLIC' AS kind FROM variables WHERE environment_id = ? AND name = ?
         UNION ALL
         SELECT id, 'SECRET' AS kind FROM secrets WHERE environment_id = ? AND name = ?
         LIMIT 1",
    )
    .bind(environment_id)
    .bind(name)
    .bind(environment_id)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| {
        let kind: String = r.get("kind");
        (r.get("id"), kind.parse().unwrap_or(VariableKind::Public))
    }))
}

pub async fn upsert_public_variable(
    pool: &SqlitePool,
    id: &str,
    environment_id: &str,
    name: &str,
    value: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO variables (id, environment_id, name, value, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(environment_id, name) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(id)
    .bind(environment_id)
    .bind(name)
    .bind(value)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_secret(
    pool: &SqlitePool,
    id: &str,
    environment_id: &str,
    name: &str,
    sealed: &EncryptedSecret,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO secrets (id, environment_id, name, ciphertext, nonce, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(environment_id, name) DO UPDATE SET
            ciphertext = excluded.ciphertext, nonce = excluded.nonce, updated_at = excluded.updated_at",
    )
    .bind(id)
    .bind(environment_id)
    .bind(name)
    .bind(&sealed.ciphertext)
    .bind(&sealed.nonce)
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch the sealed form of a secret together with its row id (used as AEAD associated data).
pub(crate) async fn find_sealed_secret(
    pool: &SqlitePool,
    environment_id: &str,
    name: &str,
) -> Result<Option<(String, EncryptedSecret)>> {
    let row = sqlx::query("SELECT id, ciphertext, nonce FROM secrets WHERE environment_id = ? AND name = ?")
        .bind(environment_id)
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| {
        (
            r.get::<String, _>("id"),
            EncryptedSecret {
                ciphertext: r.get("ciphertext"),
                nonce: r.get("nonce"),
            },
        )
    }))
}

/// The stored value of a PUBLIC variable. There is deliberately no equivalent
/// for secrets outside the vault-backed path.
pub async fn find_public_value(
    pool: &SqlitePool,
    environment_id: &str,
    name: &str,
) -> Result<Option<String>> {
    let row = sqlx::query("SELECT value FROM variables WHERE environment_id = ? AND name = ?")
        .bind(environment_id)
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.get("value")))
}

pub async fn delete_variable_by_name(pool: &SqlitePool, environment_id: &str, name: &str) -> Result<bool> {
    let a = sqlx::query("DELETE FROM variables WHERE environment_id = ? AND name = ?")
        .bind(environment_id)
        .bind(name)
        .execute(pool)
        .await?;
    let b = sqlx::query("DELETE FROM secrets WHERE environment_id = ? AND name = ?")
        .bind(environment_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(a.rows_affected() + b.rows_affected() > 0)
}

pub async fn count_secrets(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM secrets")
        .fetch_one(pool)
        .await?;
    Ok(row.get("n"))
}
