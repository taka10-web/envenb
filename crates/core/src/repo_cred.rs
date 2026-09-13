//! SQLx queries for credentials. No business rules, no crypto.

use chrono::{DateTime, Utc};
use envfish_vault::EncryptedSecret;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::error::Result;
use crate::model::{Credential, CredentialField, CredentialKind};

fn ts(row: &SqliteRow, col: &str) -> DateTime<Utc> {
    let raw: String = row.get(col);
    DateTime::parse_from_rfc3339(&raw)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn header_from_row(row: &SqliteRow) -> Credential {
    let kind: String = row.get("kind");
    Credential {
        id: row.get("id"),
        project_id: row.get("project_id"),
        environment_id: row.get("environment_id"),
        kind: kind.parse().unwrap_or(CredentialKind::Account),
        name: row.get("name"),
        note: row.get("note"),
        fields: Vec::new(),
        created_at: ts(row, "created_at"),
        updated_at: ts(row, "updated_at"),
    }
}

pub async fn list_headers(pool: &SqlitePool, environment_id: &str) -> Result<Vec<Credential>> {
    let rows =
        sqlx::query("SELECT * FROM credentials WHERE environment_id = ? ORDER BY kind, name COLLATE NOCASE")
            .bind(environment_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.iter().map(header_from_row).collect())
}

pub async fn list_headers_for_project(pool: &SqlitePool, project_id: &str) -> Result<Vec<Credential>> {
    let rows = sqlx::query(
        "SELECT * FROM credentials WHERE project_id = ? ORDER BY environment_id, kind, name COLLATE NOCASE",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(header_from_row).collect())
}

pub async fn find_header_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Credential>> {
    let row = sqlx::query("SELECT * FROM credentials WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(header_from_row))
}

pub async fn find_header_by_name(
    pool: &SqlitePool,
    environment_id: &str,
    name: &str,
) -> Result<Option<Credential>> {
    let row = sqlx::query("SELECT * FROM credentials WHERE environment_id = ? AND name = ? COLLATE NOCASE")
        .bind(environment_id)
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(row.as_ref().map(header_from_row))
}

pub async fn insert_header(pool: &SqlitePool, c: &Credential) -> Result<()> {
    sqlx::query(
        "INSERT INTO credentials (id, project_id, environment_id, kind, name, note, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&c.id)
    .bind(&c.project_id)
    .bind(&c.environment_id)
    .bind(c.kind.as_str())
    .bind(&c.name)
    .bind(&c.note)
    .bind(c.created_at.to_rfc3339())
    .bind(c.updated_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn touch_header(pool: &SqlitePool, id: &str, note: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE credentials SET updated_at = ?, note = COALESCE(?, note) WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(note)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_header(pool: &SqlitePool, id: &str) -> Result<bool> {
    let r = sqlx::query("DELETE FROM credentials WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

/// Field metadata (never ciphertext) for listing.
pub async fn list_fields(pool: &SqlitePool, credential_id: &str) -> Result<Vec<CredentialField>> {
    let rows = sqlx::query(
        "SELECT field, secret, value FROM credential_fields WHERE credential_id = ? ORDER BY field",
    )
    .bind(credential_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            let secret: i64 = r.get("secret");
            CredentialField {
                field: r.get("field"),
                secret: secret == 1,
                value: if secret == 1 { None } else { r.get("value") },
                present: true,
            }
        })
        .collect())
}

pub async fn upsert_plain_field(
    pool: &SqlitePool,
    credential_id: &str,
    field: &str,
    value: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO credential_fields (id, credential_id, field, secret, value, ciphertext, nonce, updated_at)
         VALUES (?, ?, ?, 0, ?, NULL, NULL, ?)
         ON CONFLICT(credential_id, field) DO UPDATE SET value = excluded.value, ciphertext = NULL, nonce = NULL, secret = 0, updated_at = excluded.updated_at",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(credential_id)
    .bind(field)
    .bind(value)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns the row id that must be used as AEAD associated data for this field.
pub async fn field_row_id(pool: &SqlitePool, credential_id: &str, field: &str) -> Result<Option<String>> {
    let row = sqlx::query("SELECT id FROM credential_fields WHERE credential_id = ? AND field = ?")
        .bind(credential_id)
        .bind(field)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.get("id")))
}

pub async fn upsert_secret_field(
    pool: &SqlitePool,
    row_id: &str,
    credential_id: &str,
    field: &str,
    sealed: &EncryptedSecret,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO credential_fields (id, credential_id, field, secret, value, ciphertext, nonce, updated_at)
         VALUES (?, ?, ?, 1, NULL, ?, ?, ?)
         ON CONFLICT(credential_id, field) DO UPDATE SET ciphertext = excluded.ciphertext, nonce = excluded.nonce, value = NULL, secret = 1, updated_at = excluded.updated_at",
    )
    .bind(row_id)
    .bind(credential_id)
    .bind(field)
    .bind(&sealed.ciphertext)
    .bind(&sealed.nonce)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_sealed_field(
    pool: &SqlitePool,
    credential_id: &str,
    field: &str,
) -> Result<Option<(String, EncryptedSecret)>> {
    let row = sqlx::query(
        "SELECT id, ciphertext, nonce FROM credential_fields WHERE credential_id = ? AND field = ? AND secret = 1",
    )
    .bind(credential_id)
    .bind(field)
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

pub async fn delete_field(pool: &SqlitePool, credential_id: &str, field: &str) -> Result<bool> {
    let r = sqlx::query("DELETE FROM credential_fields WHERE credential_id = ? AND field = ?")
        .bind(credential_id)
        .bind(field)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn count_credentials(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM credentials")
        .fetch_one(pool)
        .await?;
    Ok(row.get("n"))
}
