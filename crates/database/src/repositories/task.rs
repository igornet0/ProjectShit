use chrono::{DateTime, Utc};
use project_hub_domain::{ProjectId, Task, TaskId, TaskPriority, TaskStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::DbError;

pub struct TaskRepository;

impl TaskRepository {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Task>, DbError> {
        let rows = sqlx::query_as::<_, TaskRow>(
            "SELECT id, project_id, title, description, status, priority, due_at, created_at, updated_at FROM tasks ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(TaskRow::into_task).collect()
    }

    pub async fn list_by_project(pool: &SqlitePool, project_id: &ProjectId) -> Result<Vec<Task>, DbError> {
        let rows = sqlx::query_as::<_, TaskRow>(
            "SELECT id, project_id, title, description, status, priority, due_at, created_at, updated_at FROM tasks WHERE project_id = ? ORDER BY created_at DESC",
        )
        .bind(project_id.to_string())
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(TaskRow::into_task).collect()
    }

    pub async fn get(pool: &SqlitePool, id: &TaskId) -> Result<Task, DbError> {
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT id, project_id, title, description, status, priority, due_at, created_at, updated_at FROM tasks WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))?;

        row.into_task()
    }

    pub async fn create(pool: &SqlitePool, task: &Task) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO tasks (id, project_id, title, description, status, priority, due_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(task.id.to_string())
        .bind(task.project_id.to_string())
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.due_at.map(|d| d.to_rfc3339()))
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update(pool: &SqlitePool, task: &Task) -> Result<(), DbError> {
        let result = sqlx::query(
            "UPDATE tasks SET title = ?, description = ?, status = ?, priority = ?, due_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.due_at.map(|d| d.to_rfc3339()))
        .bind(task.updated_at.to_rfc3339())
        .bind(task.id.to_string())
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(task.id.to_string()));
        }
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &TaskId) -> Result<(), DbError> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
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
struct TaskRow {
    id: String,
    project_id: String,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    due_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TaskRow {
    fn into_task(self) -> Result<Task, DbError> {
        let id = Uuid::parse_str(&self.id)
            .map(TaskId)
            .map_err(|e| DbError::Migration(e.to_string()))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map(ProjectId::from_uuid)
            .map_err(|e| DbError::Migration(e.to_string()))?;

        Ok(Task {
            id,
            project_id,
            title: self.title,
            description: self.description,
            status: TaskStatus::from_str(&self.status),
            priority: TaskPriority::from_str(&self.priority),
            due_at: self.due_at.map(|s| parse_datetime(&s)).transpose()?,
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
