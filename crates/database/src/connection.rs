use std::path::Path;

use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use tracing::info;

use crate::error::DbError;

const MIGRATION_001: &str = include_str!("../../../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../../../migrations/002_project_enhancements.sql");
const MIGRATION_003: &str = include_str!("../../../migrations/003_folders_github.sql");
const MIGRATION_004: &str = include_str!("../../../migrations/004_personal_tasks.sql");
const MIGRATION_005: &str = include_str!("../../../migrations/005_recurring_tasks.sql");
const MIGRATION_006: &str = include_str!("../../../migrations/006_task_links.sql");

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

    let url = format!("sqlite:{}", db_path.display());
    // WAL lets readers run alongside a writer; NORMAL sync is safe under WAL
    // and avoids an fsync per statement.
    let options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
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
    apply_migration(pool, 4, MIGRATION_004).await?;
    apply_migration(pool, 5, MIGRATION_005).await?;
    apply_migration(pool, 6, MIGRATION_006).await?;
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

    // One connection for the whole file: PRAGMAs and BEGIN/COMMIT are
    // connection-scoped.
    let mut conn = pool.acquire().await?;
    for statement in sql
        .split(';')
        .map(strip_sql_comments)
        .filter(|s| !s.is_empty())
    {
        if let Err(e) = sqlx::query(&statement).execute(&mut *conn).await {
            // SQLite returns error if column already exists on re-run without migration table
            let msg = e.to_string();
            if msg.contains("duplicate column") {
                continue;
            }
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            let _ = sqlx::query("PRAGMA foreign_keys = ON").execute(&mut *conn).await;
            return Err(DbError::Migration(msg));
        }
    }
    drop(conn);

    sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (?, datetime('now'))")
        .bind(version)
        .execute(pool)
        .await?;

    Ok(())
}

fn strip_sql_comments(statement: &str) -> String {
    statement
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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

    #[tokio::test]
    async fn task_links_roundtrip_and_cascade() {
        use crate::{TaskLinkRepository, TaskRepository};
        use project_hub_domain::{Task, TaskId, TaskLink, TaskPriority, TaskStatus};

        let dir = tempdir().unwrap();
        let db = init_db(&dir.path().join("links.db")).await.unwrap();
        let now = chrono::Utc::now();
        let task = Task {
            id: TaskId::new(),
            project_id: None,
            title: "linked".into(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            due_at: None,
            recurrence: None,
            series_id: None,
            created_at: now,
            updated_at: now,
        };
        TaskRepository::create(db.pool(), &task).await.unwrap();
        let mut link = TaskLink::new(task.id);
        link.github_repo = Some("octo/demo".into());
        link.github_issue_number = Some(7);
        link.source = Some("boarddo".into());
        TaskLinkRepository::upsert(db.pool(), &link).await.unwrap();
        link.github_issue_state = Some("closed".into());
        TaskLinkRepository::upsert(db.pool(), &link).await.unwrap();

        let got = TaskLinkRepository::get(db.pool(), &task.id).await.unwrap().unwrap();
        assert_eq!(got.github_issue_state.as_deref(), Some("closed"));
        assert_eq!(TaskLinkRepository::list_with_issues(db.pool()).await.unwrap().len(), 1);

        TaskRepository::delete(db.pool(), &task.id).await.unwrap();
        assert!(TaskLinkRepository::get(db.pool(), &task.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn migration_004_keeps_tasks_and_event_links() {
        let dir = tempdir().unwrap();
        let url = format!("sqlite:{}?mode=rwc", dir.path().join("old.db").display());
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .after_connect(|conn, _| {
                Box::pin(async move {
                    sqlx::query("PRAGMA foreign_keys = ON").execute(conn).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        apply_migration(&pool, 1, MIGRATION_001).await.unwrap();
        apply_migration(&pool, 2, MIGRATION_002).await.unwrap();
        apply_migration(&pool, 3, MIGRATION_003).await.unwrap();

        let now = "2026-01-01T00:00:00+00:00";
        sqlx::query("INSERT INTO projects (id, name, root_path, language, project_type, status, created_at, updated_at) VALUES ('p1', 'demo', '/tmp/demo', 'rust', 'application', 'active', ?, ?)")
            .bind(now).bind(now).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tasks (id, project_id, title, created_at, updated_at) VALUES ('t1', 'p1', 'old task', ?, ?)")
            .bind(now).bind(now).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO calendar_events (id, task_id, title, start_at, created_at) VALUES ('e1', 't1', 'ev', ?, ?)")
            .bind(now).bind(now).execute(&pool).await.unwrap();

        apply_migration(&pool, 4, MIGRATION_004).await.unwrap();

        let (project_id,): (Option<String>,) =
            sqlx::query_as("SELECT project_id FROM tasks WHERE id = 't1'").fetch_one(&pool).await.unwrap();
        assert_eq!(project_id.as_deref(), Some("p1"));
        let (task_id,): (Option<String>,) =
            sqlx::query_as("SELECT task_id FROM calendar_events WHERE id = 'e1'").fetch_one(&pool).await.unwrap();
        assert_eq!(task_id.as_deref(), Some("t1"));

        sqlx::query("INSERT INTO tasks (id, project_id, title, created_at, updated_at) VALUES ('t2', NULL, 'personal', ?, ?)")
            .bind(now).bind(now).execute(&pool).await.unwrap();
        let (fk,): (i64,) = sqlx::query_as("PRAGMA foreign_keys").fetch_one(&pool).await.unwrap();
        assert_eq!(fk, 1);
    }
}
