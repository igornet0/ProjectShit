use chrono::{DateTime, Utc};
use project_hub_domain::{
    normalize_path, Language, Project, ProjectId, ProjectStatus, ProjectType,
};
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

use crate::error::DbError;

const PROJECT_COLUMNS: &str = "id, name, root_path, language, project_type, status, description, icon, group_name, github_repo_id, remote_url, last_opened_at, last_modified_at, created_at, updated_at";

pub struct ProjectRepository;

impl ProjectRepository {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Project>, DbError> {
        let rows = sqlx::query_as::<_, ProjectRow>(&format!(
            "SELECT {PROJECT_COLUMNS} FROM projects ORDER BY last_opened_at DESC NULLS LAST, last_modified_at DESC NULLS LAST, name"
        ))
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(ProjectRow::into_project).collect()
    }

    pub async fn list_groups(pool: &SqlitePool) -> Result<Vec<String>, DbError> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT DISTINCT group_name FROM projects WHERE group_name IS NOT NULL AND group_name != '' ORDER BY group_name",
        )
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn get(pool: &SqlitePool, id: &ProjectId) -> Result<Project, DbError> {
        let row = sqlx::query_as::<_, ProjectRow>(&format!(
            "SELECT {PROJECT_COLUMNS} FROM projects WHERE id = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))?;

        row.into_project()
    }

    pub async fn get_by_path(pool: &SqlitePool, root_path: &str) -> Result<Option<Project>, DbError> {
        if let Some(project) = Self::get_by_path_exact(pool, root_path).await? {
            return Ok(Some(project));
        }

        Self::get_by_path_exact(pool, &normalize_path(Path::new(root_path)).to_string_lossy()).await
    }

    async fn get_by_path_exact(pool: &SqlitePool, root_path: &str) -> Result<Option<Project>, DbError> {
        let row = sqlx::query_as::<_, ProjectRow>(&format!(
            "SELECT {PROJECT_COLUMNS} FROM projects WHERE root_path = ?"
        ))
        .bind(root_path)
        .fetch_optional(pool)
        .await?;

        row.map(|r| r.into_project()).transpose()
    }

    pub async fn upsert(pool: &SqlitePool, project: &Project) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO projects (id, name, root_path, language, project_type, status, description, icon, group_name, github_repo_id, remote_url, last_opened_at, last_modified_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(root_path) DO UPDATE SET
                name = excluded.name,
                language = excluded.language,
                project_type = excluded.project_type,
                description = excluded.description,
                icon = excluded.icon,
                github_repo_id = COALESCE(excluded.github_repo_id, projects.github_repo_id),
                remote_url = COALESCE(excluded.remote_url, projects.remote_url),
                last_modified_at = COALESCE(excluded.last_modified_at, projects.last_modified_at),
                updated_at = excluded.updated_at
            "#,
        )
        .bind(project.id.to_string())
        .bind(&project.name)
        .bind(normalize_path(&project.root_path).to_string_lossy().as_ref())
        .bind(project.language.as_str())
        .bind(project.project_type.as_str())
        .bind(project.status.as_str())
        .bind(&project.description)
        .bind(&project.icon)
        .bind(&project.group_name)
        .bind(project.github_repo_id)
        .bind(&project.remote_url)
        .bind(project.last_opened_at.map(|d| d.to_rfc3339()))
        .bind(project.last_modified_at.map(|d| d.to_rfc3339()))
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_metadata(pool: &SqlitePool, project: &Project) -> Result<(), DbError> {
        let result = sqlx::query(
            r#"
            UPDATE projects SET
                name = ?, language = ?, project_type = ?, status = ?,
                description = ?, icon = ?, group_name = ?,
                github_repo_id = ?, remote_url = ?,
                last_opened_at = ?, last_modified_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&project.name)
        .bind(project.language.as_str())
        .bind(project.project_type.as_str())
        .bind(project.status.as_str())
        .bind(&project.description)
        .bind(&project.icon)
        .bind(&project.group_name)
        .bind(project.github_repo_id)
        .bind(&project.remote_url)
        .bind(project.last_opened_at.map(|d| d.to_rfc3339()))
        .bind(project.last_modified_at.map(|d| d.to_rfc3339()))
        .bind(project.updated_at.to_rfc3339())
        .bind(project.id.to_string())
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(project.id.to_string()));
        }
        Ok(())
    }

    pub async fn mark_opened(pool: &SqlitePool, id: &ProjectId, at: DateTime<Utc>) -> Result<(), DbError> {
        let result = sqlx::query(
            "UPDATE projects SET last_opened_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(at.to_rfc3339())
        .bind(at.to_rfc3339())
        .bind(id.to_string())
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &ProjectId) -> Result<(), DbError> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub async fn search(pool: &SqlitePool, query: &str) -> Result<Vec<Project>, DbError> {
        let pattern = format!("%{}%", query.to_lowercase());
        let rows = sqlx::query_as::<_, ProjectRow>(&format!(
            "SELECT {PROJECT_COLUMNS} FROM projects WHERE LOWER(name) LIKE ? OR LOWER(root_path) LIKE ? ORDER BY last_opened_at DESC NULLS LAST, name"
        ))
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(ProjectRow::into_project).collect()
    }
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    root_path: String,
    language: String,
    project_type: String,
    status: String,
    description: Option<String>,
    icon: Option<String>,
    group_name: Option<String>,
    github_repo_id: Option<i64>,
    remote_url: Option<String>,
    last_opened_at: Option<String>,
    last_modified_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ProjectRow {
    fn into_project(self) -> Result<Project, DbError> {
        let id = Uuid::parse_str(&self.id)
            .map(ProjectId::from_uuid)
            .map_err(|e| DbError::Migration(e.to_string()))?;

        Ok(Project {
            id,
            name: self.name,
            root_path: self.root_path.into(),
            language: Language::from_str(&self.language),
            project_type: ProjectType::from_str(&self.project_type),
            status: ProjectStatus::from_str(&self.status),
            description: self.description,
            icon: self.icon,
            group_name: self.group_name,
            github_repo_id: self.github_repo_id,
            remote_url: self.remote_url,
            last_opened_at: self.last_opened_at.map(|s| parse_datetime(&s)).transpose()?,
            last_modified_at: self.last_modified_at.map(|s| parse_datetime(&s)).transpose()?,
            created_at: parse_datetime(&self.created_at)?,
            updated_at: parse_datetime(&self.updated_at)?,
        })
    }
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DbError::Migration(format!("invalid datetime: {e}")))
}
