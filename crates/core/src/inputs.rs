//! Request shapes shared by Tauri commands and the HTTP API.

use chrono::{DateTime, Utc};
use project_hub_domain::{
    CalendarEvent, CalendarEventId, ProjectId, Recurrence, RecurrenceFrequency, Task, TaskId,
    TaskPriority, TaskStatus,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateTaskInput {
    /// Omitted or empty for a personal task.
    #[serde(default)]
    pub project_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub recurrence: Option<RecurrenceInput>,
    /// Who created the task (`boarddo`, `api`, …) — stored as a task link.
    #[serde(default)]
    pub source: Option<String>,
    /// Caller's own reference, e.g. `objective:<id>/<item>`.
    #[serde(default)]
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecurrenceInput {
    pub frequency: String,
    #[serde(default)]
    pub interval: Option<u32>,
    #[serde(default)]
    pub until: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTaskInput {
    /// Filled from the URL by the HTTP API.
    #[serde(default)]
    pub id: String,
    /// `Some("")` moves the task to personal; `None` leaves it unchanged.
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub clear_due_at: bool,
    /// Stops repeating: this instance keeps its date, no further ones spawn.
    #[serde(default)]
    pub clear_recurrence: bool,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateEventInput {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    pub title: String,
    pub start_at: String,
    #[serde(default)]
    pub end_at: Option<String>,
}

pub fn parse_uuid(s: &str) -> Result<Uuid, String> {
    Uuid::parse_str(s.trim()).map_err(|e| format!("invalid id `{s}`: {e}"))
}

pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(s.trim())
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("invalid date `{s}`: {e}"))
}

fn parse_project(id: Option<String>) -> Result<Option<ProjectId>, String> {
    id.filter(|id| !id.trim().is_empty())
        .map(|id| parse_uuid(&id).map(ProjectId::from_uuid))
        .transpose()
}

fn parse_recurrence(input: RecurrenceInput, due_at: Option<DateTime<Utc>>) -> Result<Recurrence, String> {
    let start = due_at.ok_or("a recurring task needs a due date")?;
    let frequency = RecurrenceFrequency::parse(&input.frequency)
        .ok_or_else(|| format!("unknown recurrence frequency: {}", input.frequency))?;
    let until = input.until.map(|d| parse_datetime(&d)).transpose()?;
    if until.is_some_and(|until| until < start) {
        return Err("recurrence end date is before the first due date".into());
    }
    Ok(Recurrence {
        frequency,
        interval: input.interval.unwrap_or(1).clamp(1, 365),
        start,
        until,
    })
}

impl CreateTaskInput {
    pub fn into_task(self) -> Result<Task, String> {
        let title = self.title.trim().to_string();
        if title.is_empty() {
            return Err("task title is required".into());
        }
        let now = Utc::now();
        let id = TaskId::new();
        let due_at = self.due_at.map(|d| parse_datetime(&d)).transpose()?;
        let recurrence = self.recurrence.map(|r| parse_recurrence(r, due_at)).transpose()?;
        Ok(Task {
            id,
            project_id: parse_project(self.project_id)?,
            title,
            description: self.description.filter(|d| !d.trim().is_empty()),
            status: self.status.map(|s| TaskStatus::from_str(&s)).unwrap_or(TaskStatus::Todo),
            priority: self
                .priority
                .map(|p| TaskPriority::from_str(&p))
                .unwrap_or(TaskPriority::Medium),
            due_at,
            series_id: recurrence.as_ref().map(|_| id.0),
            recurrence,
            created_at: now,
            updated_at: now,
        })
    }
}

impl UpdateTaskInput {
    /// Apply the patch onto an existing task.
    pub fn apply(self, mut task: Task) -> Result<Task, String> {
        if let Some(project_id) = self.project_id {
            task.project_id = parse_project(Some(project_id))?;
        }
        if let Some(title) = self.title.filter(|t| !t.trim().is_empty()) {
            task.title = title.trim().to_string();
        }
        if self.description.is_some() {
            task.description = self.description.filter(|d| !d.trim().is_empty());
        }
        if let Some(status) = self.status {
            task.status = TaskStatus::from_str(&status);
        }
        if let Some(priority) = self.priority {
            task.priority = TaskPriority::from_str(&priority);
        }
        if self.due_at.is_some() {
            task.due_at = self.due_at.map(|d| parse_datetime(&d)).transpose()?;
        }
        if self.clear_due_at {
            task.due_at = None;
        }
        if self.clear_recurrence || task.due_at.is_none() {
            task.recurrence = None;
        }
        task.updated_at = Utc::now();
        Ok(task)
    }
}

impl CreateEventInput {
    pub fn into_event(self) -> Result<CalendarEvent, String> {
        Ok(CalendarEvent {
            id: CalendarEventId::new(),
            project_id: parse_project(self.project_id)?,
            task_id: self
                .task_id
                .filter(|t| !t.is_empty())
                .map(|t| parse_uuid(&t).map(TaskId))
                .transpose()?,
            title: self.title,
            start_at: parse_datetime(&self.start_at)?,
            end_at: self.end_at.map(|d| parse_datetime(&d)).transpose()?,
            created_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_patch() {
        let task = CreateTaskInput {
            project_id: Some(String::new()),
            title: "  Fix login ".into(),
            priority: Some("high".into()),
            due_at: Some("2026-10-10T12:00:00Z".into()),
            recurrence: Some(RecurrenceInput {
                frequency: "weekly".into(),
                ..Default::default()
            }),
            ..Default::default()
        }
        .into_task()
        .unwrap();
        assert_eq!(task.title, "Fix login");
        assert!(task.project_id.is_none());
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.series_id, Some(task.id.0));

        let patched = UpdateTaskInput {
            status: Some("done".into()),
            clear_due_at: true,
            ..Default::default()
        }
        .apply(task)
        .unwrap();
        assert_eq!(patched.status, TaskStatus::Done);
        assert!(patched.due_at.is_none() && patched.recurrence.is_none());

        assert!(CreateTaskInput { title: " ".into(), ..Default::default() }.into_task().is_err());
        assert!(CreateTaskInput {
            title: "x".into(),
            recurrence: Some(RecurrenceInput { frequency: "daily".into(), ..Default::default() }),
            ..Default::default()
        }
        .into_task()
        .is_err());
    }
}
