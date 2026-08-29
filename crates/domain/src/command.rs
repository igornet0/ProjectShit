use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::project::ProjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectCommandId(pub Uuid);

impl ProjectCommandId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProjectCommandId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectCommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCommand {
    pub id: ProjectCommandId,
    pub project_id: ProjectId,
    pub name: String,
    pub command: String,
    pub created_at: DateTime<Utc>,
}
