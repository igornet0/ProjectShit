use chrono::{DateTime, Utc};
use project_hub_domain::{TaskId, TaskLink};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::DbError;

const COLUMNS: &str = "task_id, github_repo, github_issue_number, github_issue_url, github_issue_state, source, external_ref, synced_at";

pub struct TaskLinkRepository;

impl TaskLinkRepository {
    pub async fn get(pool: &SqlitePool, task_id: &TaskId) -> Result<Option<TaskLink>, DbError> {
        sqlx::query_as::<_, TaskLinkRow>(&format!("SELECT {COLUMNS} FROM task_links WHERE task_id = ?"))
            .bind(task_id.to_string())
            .fetch_optional(pool)
            .await?
            .map(TaskLinkRow::into_link)
            .transpose()
    }

    pub async fn list(pool: &SqlitePool) -> Result<Vec<TaskLink>, DbError> {
        sqlx::query_as::<_, TaskLinkRow>(&format!("SELECT {COLUMNS} FROM task_links"))
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(TaskLinkRow::into_link)
            .collect()
    }

    pub async fn list_with_issues(pool: &SqlitePool) -> Result<Vec<TaskLink>, DbError> {
        sqlx::query_as::<_, TaskLinkRow>(&format!(
            "SELECT {COLUMNS} FROM task_links WHERE github_issue_number IS NOT NULL"
        ))
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(TaskLinkRow::into_link)
        .collect()
    }

    pub async fn upsert(pool: &SqlitePool, link: &TaskLink) -> Result<(), DbError> {
        sqlx::query(&format!(
            "INSERT INTO task_links ({COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(task_id) DO UPDATE SET
                github_repo = excluded.github_repo,
                github_issue_number = excluded.github_issue_number,
                github_issue_url = excluded.github_issue_url,
                github_issue_state = excluded.github_issue_state,
                source = excluded.source,
                external_ref = excluded.external_ref,
                synced_at = excluded.synced_at"
        ))
        .bind(link.task_id.to_string())
        .bind(&link.github_repo)
        .bind(link.github_issue_number)
        .bind(&link.github_issue_url)
        .bind(&link.github_issue_state)
        .bind(&link.source)
        .bind(&link.external_ref)
        .bind(link.synced_at.map(|d| d.to_rfc3339()))
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct TaskLinkRow {
    task_id: String,
    github_repo: Option<String>,
    github_issue_number: Option<i64>,
    github_issue_url: Option<String>,
    github_issue_state: Option<String>,
    source: Option<String>,
    external_ref: Option<String>,
    synced_at: Option<String>,
}

impl TaskLinkRow {
    fn into_link(self) -> Result<TaskLink, DbError> {
        let id = Uuid::parse_str(&self.task_id).map_err(|e| DbError::NotFound(e.to_string()))?;
        Ok(TaskLink {
            task_id: TaskId(id),
            github_repo: self.github_repo,
            github_issue_number: self.github_issue_number,
            github_issue_url: self.github_issue_url,
            github_issue_state: self.github_issue_state,
            source: self.source,
            external_ref: self.external_ref,
            synced_at: self
                .synced_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
        })
    }
}
