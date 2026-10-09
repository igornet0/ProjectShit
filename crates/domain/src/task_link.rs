use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::task::TaskId;

/// Links a task to the outside world: a GitHub issue and/or the tool that
/// created it (`source = "boarddo"`, `external_ref = "objective:<id>/<item>"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLink {
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_issue_number: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_issue_url: Option<String>,
    /// `open` | `closed`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_issue_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<DateTime<Utc>>,
}

impl TaskLink {
    pub fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            github_repo: None,
            github_issue_number: None,
            github_issue_url: None,
            github_issue_state: None,
            source: None,
            external_ref: None,
            synced_at: None,
        }
    }
}
