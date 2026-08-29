use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::project::ProjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActivityId(pub Uuid);

impl ActivityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ActivityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ActivityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    ProjectOpened,
    GitCommit,
    CommandRun,
    TaskCompleted,
    TaskCreated,
    ProjectScanned,
    Other,
}

impl ActivityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectOpened => "project_opened",
            Self::GitCommit => "git_commit",
            Self::CommandRun => "command_run",
            Self::TaskCompleted => "task_completed",
            Self::TaskCreated => "task_created",
            Self::ProjectScanned => "project_scanned",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "project_opened" => Self::ProjectOpened,
            "git_commit" => Self::GitCommit,
            "command_run" => Self::CommandRun,
            "task_completed" => Self::TaskCompleted,
            "task_created" => Self::TaskCreated,
            "project_scanned" => Self::ProjectScanned,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub id: ActivityId,
    pub project_id: Option<ProjectId>,
    pub activity_type: ActivityType,
    pub message: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
