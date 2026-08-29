use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectRootId(pub Uuid);

impl ProjectRootId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProjectRootId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectRootId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Cpp,
    Unknown,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Java => "java",
            Self::Cpp => "cpp",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rust" => Self::Rust,
            "python" => Self::Python,
            "javascript" | "js" => Self::JavaScript,
            "typescript" | "ts" => Self::TypeScript,
            "go" | "golang" => Self::Go,
            "java" => Self::Java,
            "cpp" | "c++" => Self::Cpp,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Application,
    Library,
    Workspace,
    Web,
    Embedded,
    Unknown,
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Library => "library",
            Self::Workspace => "workspace",
            Self::Web => "web",
            Self::Embedded => "embedded",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "application" | "app" => Self::Application,
            "library" | "lib" => Self::Library,
            "workspace" => Self::Workspace,
            "web" => Self::Web,
            "embedded" => Self::Embedded,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Paused,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "archived" => Self::Archived,
            _ => Self::Active,
        }
    }
}

/// Persisted project record — metadata only, files stay on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub root_path: PathBuf,
    pub language: Language,
    pub project_type: ProjectType,
    pub status: ProjectStatus,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub group_name: Option<String>,
    pub github_repo_id: Option<i64>,
    pub remote_url: Option<String>,
    pub last_opened_at: Option<DateTime<Utc>>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Result of project detection before persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub root_path: PathBuf,
    pub language: Language,
    pub project_type: ProjectType,
    pub has_git: bool,
    pub packages: Vec<String>,
}

/// Enriched project row for gallery/list views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectListItem {
    #[serde(flatten)]
    pub project: Project,
    pub has_git: bool,
    pub git_dirty: bool,
    /// Immediate parent project id derived from path containment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ProjectId>,
    /// Number of direct child projects nested under this path.
    pub child_count: u32,
    /// Nesting depth (0 = top-level).
    pub depth: u32,
    /// User-assigned virtual folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<crate::FolderId>,
}

/// User-configured directory to scan for projects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRoot {
    pub id: ProjectRootId,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_roundtrip() {
        assert_eq!(Language::from_str("rust"), Language::Rust);
        assert_eq!(Language::Rust.as_str(), "rust");
    }

    #[test]
    fn project_status_defaults_to_active() {
        assert_eq!(ProjectStatus::from_str("invalid"), ProjectStatus::Active);
    }
}
