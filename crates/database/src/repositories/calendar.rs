use chrono::{DateTime, Utc};
use project_hub_domain::{CalendarEvent, CalendarEventId, ProjectId, TaskId};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::DbError;

pub struct CalendarRepository;

impl CalendarRepository {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<CalendarEvent>, DbError> {
        let rows = sqlx::query_as::<_, CalendarEventRow>(
            "SELECT id, project_id, task_id, title, start_at, end_at, created_at FROM calendar_events ORDER BY start_at",
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(CalendarEventRow::into_event).collect()
    }

    pub async fn list_range(
        pool: &SqlitePool,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, DbError> {
        let rows = sqlx::query_as::<_, CalendarEventRow>(
            "SELECT id, project_id, task_id, title, start_at, end_at, created_at FROM calendar_events WHERE start_at >= ? AND start_at <= ? ORDER BY start_at",
        )
        .bind(from.to_rfc3339())
        .bind(to.to_rfc3339())
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(CalendarEventRow::into_event).collect()
    }

    pub async fn create(pool: &SqlitePool, event: &CalendarEvent) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO calendar_events (id, project_id, task_id, title, start_at, end_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(event.id.to_string())
        .bind(event.project_id.map(|p| p.to_string()))
        .bind(event.task_id.map(|t| t.to_string()))
        .bind(&event.title)
        .bind(event.start_at.to_rfc3339())
        .bind(event.end_at.map(|d| d.to_rfc3339()))
        .bind(event.created_at.to_rfc3339())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &CalendarEventId) -> Result<(), DbError> {
        let result = sqlx::query("DELETE FROM calendar_events WHERE id = ?")
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
struct CalendarEventRow {
    id: String,
    project_id: Option<String>,
    task_id: Option<String>,
    title: String,
    start_at: String,
    end_at: Option<String>,
    created_at: String,
}

impl CalendarEventRow {
    fn into_event(self) -> Result<CalendarEvent, DbError> {
        let id = Uuid::parse_str(&self.id)
            .map(CalendarEventId)
            .map_err(|e| DbError::Migration(e.to_string()))?;

        let project_id = self
            .project_id
            .map(|s| Uuid::parse_str(&s).map(ProjectId::from_uuid))
            .transpose()
            .map_err(|e| DbError::Migration(e.to_string()))?;

        let task_id = self
            .task_id
            .map(|s| Uuid::parse_str(&s).map(TaskId))
            .transpose()
            .map_err(|e| DbError::Migration(e.to_string()))?;

        Ok(CalendarEvent {
            id,
            project_id,
            task_id,
            title: self.title,
            start_at: parse_datetime(&self.start_at)?,
            end_at: self.end_at.map(|s| parse_datetime(&s)).transpose()?,
            created_at: parse_datetime(&self.created_at)?,
        })
    }
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DbError::Migration(format!("invalid datetime: {e}")))
}
