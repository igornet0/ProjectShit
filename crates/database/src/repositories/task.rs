use chrono::{DateTime, Utc};
use project_hub_domain::{ProjectId, Recurrence, RecurrenceFrequency, Task, TaskId, TaskPriority, TaskStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::DbError;

const TASK_COLUMNS: &str = "id, project_id, title, description, status, priority, due_at, recurrence_frequency, recurrence_interval, recurrence_start, recurrence_until, series_id, created_at, updated_at";

pub struct TaskRepository;

impl TaskRepository {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Task>, DbError> {
        let rows = sqlx::query_as::<_, TaskRow>(
            &format!("SELECT {TASK_COLUMNS} FROM tasks ORDER BY created_at DESC"),
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(TaskRow::into_task).collect()
    }

    pub async fn list_by_project(pool: &SqlitePool, project_id: &ProjectId) -> Result<Vec<Task>, DbError> {
        let rows = sqlx::query_as::<_, TaskRow>(
            &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE project_id = ? ORDER BY created_at DESC"),
        )
        .bind(project_id.to_string())
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(TaskRow::into_task).collect()
    }

    pub async fn get(pool: &SqlitePool, id: &TaskId) -> Result<Task, DbError> {
        let row = sqlx::query_as::<_, TaskRow>(
            &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?"),
        )
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))?;

        row.into_task()
    }

    pub async fn create(pool: &SqlitePool, task: &Task) -> Result<(), DbError> {
        sqlx::query(
            &format!("INSERT INTO tasks ({TASK_COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"),
        )
        .bind(task.id.to_string())
        .bind(task.project_id.map(|id| id.to_string()))
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.due_at.map(|d| d.to_rfc3339()))
        .bind(task.recurrence.as_ref().map(|r| r.frequency.as_str()))
        .bind(task.recurrence.as_ref().map(|r| i64::from(r.interval)))
        .bind(task.recurrence.as_ref().map(|r| r.start.to_rfc3339()))
        .bind(task.recurrence.as_ref().and_then(|r| r.until).map(|d| d.to_rfc3339()))
        .bind(task.series_id.map(|id| id.to_string()))
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update(pool: &SqlitePool, task: &Task) -> Result<(), DbError> {
        let result = sqlx::query(
            "UPDATE tasks SET project_id = ?, title = ?, description = ?, status = ?, priority = ?, due_at = ?, \
             recurrence_frequency = ?, recurrence_interval = ?, recurrence_start = ?, recurrence_until = ?, \
             series_id = ?, updated_at = ? WHERE id = ?",
        )
        .bind(task.project_id.map(|id| id.to_string()))
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.due_at.map(|d| d.to_rfc3339()))
        .bind(task.recurrence.as_ref().map(|r| r.frequency.as_str()))
        .bind(task.recurrence.as_ref().map(|r| i64::from(r.interval)))
        .bind(task.recurrence.as_ref().map(|r| r.start.to_rfc3339()))
        .bind(task.recurrence.as_ref().and_then(|r| r.until).map(|d| d.to_rfc3339()))
        .bind(task.series_id.map(|id| id.to_string()))
        .bind(task.updated_at.to_rfc3339())
        .bind(task.id.to_string())
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(task.id.to_string()));
        }
        Ok(())
    }

    /// True when the series already has an instance due after `due_at`, i.e.
    /// the next occurrence was spawned before.
    pub async fn series_has_instance_after(
        pool: &SqlitePool,
        series_id: &Uuid,
        due_at: DateTime<Utc>,
    ) -> Result<bool, DbError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE series_id = ? AND due_at > ?)",
        )
        .bind(series_id.to_string())
        .bind(due_at.to_rfc3339())
        .fetch_one(pool)
        .await?;
        Ok(row.0 != 0)
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
    project_id: Option<String>,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    due_at: Option<String>,
    recurrence_frequency: Option<String>,
    recurrence_interval: Option<i64>,
    recurrence_start: Option<String>,
    recurrence_until: Option<String>,
    series_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TaskRow {
    fn into_task(self) -> Result<Task, DbError> {
        let id = Uuid::parse_str(&self.id)
            .map(TaskId)
            .map_err(|e| DbError::Migration(e.to_string()))?;
        let project_id = self
            .project_id
            .map(|id| {
                Uuid::parse_str(&id)
                    .map(ProjectId::from_uuid)
                    .map_err(|e| DbError::Migration(e.to_string()))
            })
            .transpose()?;

        let recurrence = match (
            self.recurrence_frequency.as_deref().and_then(RecurrenceFrequency::parse),
            self.recurrence_start,
        ) {
            (Some(frequency), Some(start)) => Some(Recurrence {
                frequency,
                interval: self.recurrence_interval.unwrap_or(1).clamp(1, i64::from(u32::MAX)) as u32,
                start: parse_datetime(&start)?,
                until: self.recurrence_until.map(|s| parse_datetime(&s)).transpose()?,
            }),
            _ => None,
        };
        let series_id = self
            .series_id
            .map(|s| Uuid::parse_str(&s).map_err(|e| DbError::Migration(e.to_string())))
            .transpose()?;

        Ok(Task {
            id,
            project_id,
            title: self.title,
            description: self.description,
            status: TaskStatus::from_str(&self.status),
            priority: TaskPriority::from_str(&self.priority),
            due_at: self.due_at.map(|s| parse_datetime(&s)).transpose()?,
            recurrence,
            series_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_db;
    use chrono::TimeZone;
    use tempfile::tempdir;

    #[tokio::test]
    async fn recurring_task_roundtrip_and_series_lookup() {
        let dir = tempdir().unwrap();
        let db = init_db(&dir.path().join("test.db")).await.unwrap();
        let due = Utc.with_ymd_and_hms(2026, 1, 31, 16, 59, 59).unwrap();
        let id = TaskId::new();
        let task = Task {
            id,
            project_id: None,
            title: "Quarterly report".into(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            due_at: Some(due),
            recurrence: Some(Recurrence {
                frequency: RecurrenceFrequency::Quarterly,
                interval: 1,
                start: due,
                until: Some(due + chrono::Duration::days(400)),
            }),
            series_id: Some(id.0),
            created_at: due,
            updated_at: due,
        };
        TaskRepository::create(db.pool(), &task).await.unwrap();

        let loaded = TaskRepository::get(db.pool(), &id).await.unwrap();
        assert_eq!(loaded, task);
        assert!(!TaskRepository::series_has_instance_after(db.pool(), &id.0, due).await.unwrap());

        let next_due = task.recurrence.as_ref().unwrap().next_after(due).unwrap();
        let next = Task { id: TaskId::new(), due_at: Some(next_due), ..task.clone() };
        TaskRepository::create(db.pool(), &next).await.unwrap();
        assert!(TaskRepository::series_has_instance_after(db.pool(), &id.0, due).await.unwrap());
    }
}
