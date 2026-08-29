use chrono::{DateTime, Utc};
use project_hub_domain::{normalize_path, ProjectRoot, ProjectRootId};
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

use crate::error::DbError;

pub struct ProjectRootRepository;

impl ProjectRootRepository {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<ProjectRoot>, DbError> {
        let rows = sqlx::query_as::<_, ProjectRootRow>(
            "SELECT id, path, created_at FROM project_roots ORDER BY path",
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(ProjectRootRow::into_root).collect()
    }

    pub async fn find_by_path(pool: &SqlitePool, path: &Path) -> Result<Option<ProjectRoot>, DbError> {
        let target = normalize_path(path);
        let roots = Self::list(pool).await?;
        Ok(roots
            .into_iter()
            .find(|root| normalize_path(&root.path) == target))
    }

    pub async fn create(pool: &SqlitePool, root: &ProjectRoot) -> Result<(), DbError> {
        sqlx::query("INSERT INTO project_roots (id, path, created_at) VALUES (?, ?, ?)")
            .bind(root.id.to_string())
            .bind(normalize_path(&root.path).to_string_lossy().as_ref())
            .bind(root.created_at.to_rfc3339())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &ProjectRootId) -> Result<(), DbError> {
        let result = sqlx::query("DELETE FROM project_roots WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(id.to_string()));
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ProjectRootRow {
    id: String,
    path: String,
    created_at: String,
}

impl ProjectRootRow {
    fn into_root(self) -> Result<ProjectRoot, DbError> {
        let id = Uuid::parse_str(&self.id)
            .map(ProjectRootId)
            .map_err(|e| DbError::Migration(e.to_string()))?;

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| DbError::Migration(e.to_string()))?;

        Ok(ProjectRoot {
            id,
            path: self.path.into(),
            created_at,
        })
    }
}
