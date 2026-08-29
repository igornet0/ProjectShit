use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ProjectListItem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub id: i64,
    pub full_name: String,
    pub clone_url: String,
    pub ssh_url: Option<String>,
    pub html_url: String,
    pub description: Option<String>,
    pub private: bool,
    pub default_branch: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub username: Option<String>,
    pub default_clone_dir: Option<String>,
    pub connected_at: Option<DateTime<Utc>>,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            username: None,
            default_clone_dir: None,
            connected_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubEntryKind {
    Local,
    Ghost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubProjectEntry {
    pub kind: HubEntryKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectListItem>,
    pub github_repo: GitHubRepo,
}
