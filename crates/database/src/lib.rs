pub mod connection;
pub mod error;
pub mod repositories;

pub use connection::{init_db, Database};
pub use sqlx::{self, SqlitePool};
pub use error::DbError;
pub use repositories::{
    ActivityRepository, CalendarRepository, FolderRepository, GitHubRepoRepository,
    ProjectRepository, ProjectRootRepository, SettingsRepository, TaskLinkRepository, TaskRepository,
    EDITOR_SETTINGS_KEY, GITHUB_CONFIG_KEY, GITHUB_OAUTH_CLIENT_ID_KEY, GITHUB_TOKEN_KEY,
};
