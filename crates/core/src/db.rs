use std::path::Path;
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::error::Result;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open (creating if needed) the SQLite database at `path` and apply migrations.
pub async fn open_pool(path: &Path) -> Result<SqlitePool> {
    let url = format!("sqlite://{}", path.display());
    let options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    tracing::debug!(path = %path.display(), "database ready");
    Ok(pool)
}

/// In-memory database for tests.
pub async fn open_memory_pool() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
    // A single connection so every query sees the same in-memory database.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Migrations are immutable once shipped: SQLx stores a checksum of each
    /// applied migration, so editing one — even a comment — makes every
    /// existing vault refuse to open. These checksums are the ones real
    /// installations have on disk. If this test fails, restore the migration
    /// file instead of updating the expected value, and put the schema change
    /// in a new numbered migration.
    #[test]
    fn migration_checksums_never_change() {
        let actual: Vec<(i64, String)> = MIGRATOR
            .iter()
            .map(|m| {
                let hex = m.checksum.iter().map(|b| format!("{b:02x}")).collect();
                (m.version, hex)
            })
            .collect();
        let expected = [
            (
                1,
                "cca0b83af4fd598f3cbd99fdd9b68995c0db57abdac2bb3fa72dfa8b47855081e5ad16ad763506c1955412b3b7edb749",
            ),
            (
                2,
                "3e122f18c41ecba9a3634cb5e07760b6c02f5f4b2b775105aef6702cd46dcdee3ec0ca4a641f01d100047e156a8879f6",
            ),
            (
                3,
                "a7d046d64f66ccbb3cf9cb01e5a1fa33a01aa73021b7c073ef8651d27f2d95a25d1088229afd66b60a78233691fe3e05",
            ),
        ];
        assert_eq!(actual.len(), expected.len(), "a migration was added or removed");
        for ((version, got), (want_version, want)) in actual.iter().zip(expected) {
            assert_eq!(*version, want_version);
            assert_eq!(got, want, "migration {version} was modified after shipping");
        }
    }
}
