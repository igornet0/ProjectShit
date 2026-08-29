pub mod activity;
pub mod calendar;
pub mod command;
pub mod error;
pub mod folder;
pub mod github;
pub mod hierarchy;
pub mod project;
pub mod task;

pub use activity::{Activity, ActivityId, ActivityType};
pub use calendar::{CalendarEvent, CalendarEventId};
pub use command::{ProjectCommand, ProjectCommandId};
pub use error::DomainError;
pub use folder::{Folder, FolderAssignment, FolderId};
pub use github::{GitHubConfig, GitHubRepo, HubEntryKind, HubProjectEntry};
pub use hierarchy::{enrich_with_hierarchy, is_strict_child, normalize_path};
pub use project::{
    Language, Project, ProjectId, ProjectInfo, ProjectListItem, ProjectRoot, ProjectRootId,
    ProjectStatus, ProjectType,
};
pub use task::{Task, TaskId, TaskPriority, TaskStatus};
