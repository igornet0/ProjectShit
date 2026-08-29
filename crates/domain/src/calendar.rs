use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::project::ProjectId;
use crate::task::TaskId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CalendarEventId(pub Uuid);

impl CalendarEventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CalendarEventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CalendarEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: CalendarEventId,
    pub project_id: Option<ProjectId>,
    pub task_id: Option<TaskId>,
    pub title: String,
    pub start_at: DateTime<Utc>,
    pub end_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
