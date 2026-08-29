use std::path::Path;

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

use crate::error::DbError;

const MIGRATION_001: &str = include_str!("../../../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../../../migrations/002_project_enhancements.sql");
const MIGRATION_003: &str = include_str!("../../../migrations/003_folders_github.sql");

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

pub async fn init_db(db_path: &Path) -> Result<Database, DbError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DbError::Migration(e.to_string()))?;
    }

    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;

    run_migrations(&pool).await?;

    info!(path = %db_path.display(), "database initialized");

    Ok(Database { pool })
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), DbError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
    )
    .execute(pool)
    .await?;

    apply_migration(pool, 1, MIGRATION_001).await?;
    apply_migration(pool, 2, MIGRATION_002).await?;
    apply_migration(pool, 3, MIGRATION_003).await?;
    Ok(())
}

async fn apply_migration(pool: &SqlitePool, version: i64, sql: &str) -> Result<(), DbError> {
    let applied: Option<(i64,)> =
        sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
            .bind(version)
            .fetch_optional(pool)
            .await?;

    if applied.is_some() {
        return Ok(());
    }

    for statement in sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        if let Err(e) = sqlx::query(statement).execute(pool).await {
            // SQLite returns error if column already exists on re-run without migration table
            let msg = e.to_string();
            if msg.contains("duplicate column") {
                continue;
            }
            return Err(DbError::Migration(msg));
        }
    }

    sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (?, datetime('now'))")
        .bind(version)
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn init_creates_tables() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = init_db(&db_path).await.unwrap();

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert!(row.0 >= 9);
    }
}
