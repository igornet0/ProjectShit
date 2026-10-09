use chrono::{DateTime, Utc};
use project_hub_domain::{Activity, ActivityId, ActivityType, ProjectId};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::DbError;

pub struct ActivityRepository;

impl ActivityRepository {
    pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<Activity>, DbError> {
        let rows = sqlx::query_as::<_, ActivityRow>(
            "SELECT id, project_id, activity_type, message, metadata, created_at FROM activities ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(ActivityRow::into_activity).collect()
    }

    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: &ProjectId,
        limit: i64,
    ) -> Result<Vec<Activity>, DbError> {
        let rows = sqlx::query_as::<_, ActivityRow>(
            "SELECT id, project_id, activity_type, message, metadata, created_at FROM activities WHERE project_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(project_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(ActivityRow::into_activity).collect()
    }

    pub async fn create<'e, E>(executor: E, activity: &Activity) -> Result<(), DbError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let metadata = activity
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| DbError::Migration(e.to_string()))?;

        sqlx::query(
            "INSERT INTO activities (id, project_id, activity_type, message, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(activity.id.to_string())
        .bind(activity.project_id.map(|p| p.to_string()))
        .bind(activity.activity_type.as_str())
        .bind(&activity.message)
        .bind(metadata)
        .bind(activity.created_at.to_rfc3339())
        .execute(executor)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: String,
    project_id: Option<String>,
    activity_type: String,
    message: Option<String>,
    metadata: Option<String>,
    created_at: String,
}

impl ActivityRow {
    fn into_activity(self) -> Result<Activity, DbError> {
        let id = Uuid::parse_str(&self.id)
            .map(ActivityId)
            .map_err(|e| DbError::Migration(e.to_string()))?;

        let project_id = self
            .project_id
            .map(|s| Uuid::parse_str(&s).map(ProjectId::from_uuid))
            .transpose()
            .map_err(|e| DbError::Migration(e.to_string()))?;

        let metadata = self
            .metadata
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| DbError::Migration(e.to_string()))?;

        Ok(Activity {
            id,
            project_id,
            activity_type: ActivityType::from_str(&self.activity_type),
            message: self.message,
            metadata,
            created_at: parse_datetime(&self.created_at)?,
        })
    }
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, DbError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DbError::Migration(format!("invalid datetime: {e}")))
}
