use std::path::Path;

use chrono::Utc;
use project_hub_database::{
    ActivityRepository, CalendarRepository, Database, FolderRepository, GitHubRepoRepository,
    ProjectRepository, ProjectRootRepository, SettingsRepository, TaskRepository,
    EDITOR_SETTINGS_KEY, GITHUB_CONFIG_KEY, GITHUB_TOKEN_KEY,
};
use project_hub_discovery::ProjectScanner;
use project_hub_domain::{
    enrich_with_hierarchy, normalize_path, Activity, ActivityId, ActivityType, CalendarEvent,
    CalendarEventId, Folder, FolderId, GitHubConfig, HubEntryKind, HubProjectEntry, Project,
    ProjectId, ProjectInfo, ProjectListItem, ProjectRoot, ProjectRootId, ProjectStatus, Task,
    TaskId,
};
use project_hub_executor::providers::detect_commands;
use project_hub_git::GitService;
use project_hub_github::{parse_github_full_name, remote_urls_for_path};
use thiserror::Error;
use tracing::info;

mod editor;
mod github;

pub use editor::{normalize_custom_editor, EditorConfig, EditorDefinition};
pub use github::{GitHubConfigResponse, GitHubOAuthStartResponse, GitHubService};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("database error: {0}")]
    Database(#[from] project_hub_database::DbError),

    #[error("git error: {0}")]
    Git(#[from] project_hub_git::GitError),

    #[error("github error: {0}")]
    GitHub(#[from] project_hub_github::GitHubError),

    #[error("process error: {0}")]
    Process(#[from] project_hub_executor::process::ProcessError),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("not found: {0}")]
    NotFound(String),
}

pub struct AppState {
    pub db: Database,
    pub scanner: ProjectScanner,
    pub oauth_session: Option<github::PendingOAuthSession>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            scanner: ProjectScanner::new(),
            oauth_session: None,
        }
    }
}

pub struct ProjectService;

impl ProjectService {
    pub async fn list_summaries(
        state: &AppState,
        include_archived: bool,
    ) -> Result<Vec<ProjectListItem>, ServiceError> {
        let projects = ProjectRepository::list(state.db.pool()).await?;
        let folder_map = FolderRepository::folder_map(state.db.pool()).await?;
        let mut items = Vec::with_capacity(projects.len());

        for project in projects {
            if !include_archived && project.status == ProjectStatus::Archived {
                continue;
            }
            let has_git = GitService::has_git(&project.root_path);
            let git_dirty = has_git && GitService::is_dirty(&project.root_path);
            items.push(ProjectListItem {
                project,
                has_git,
                git_dirty,
                parent_id: None,
                child_count: 0,
                depth: 0,
                folder_id: None,
            });
        }

        for item in &mut items {
            item.folder_id = folder_map.get(&item.project.id.to_string()).copied();
        }

        enrich_with_hierarchy(&mut items);
        Ok(items)
    }

    pub async fn list_hub_entries(
        state: &AppState,
        include_archived: bool,
    ) -> Result<Vec<HubProjectEntry>, ServiceError> {
        let local_items = Self::list_summaries(state, include_archived).await?;
        let github_repos = GitHubRepoRepository::list(state.db.pool()).await?;

        if github_repos.is_empty() {
            return Ok(Vec::new());
        }

        let mut matched_ids = std::collections::HashSet::new();
        let mut entries = Vec::new();

        for item in &local_items {
            if let Some(repo) = match_local_to_github(item, &github_repos) {
                matched_ids.insert(repo.id);
                entries.push(HubProjectEntry {
                    kind: HubEntryKind::Local,
                    project: Some(item.clone()),
                    github_repo: repo,
                });
            }
        }

        for repo in github_repos {
            if !matched_ids.contains(&repo.id) {
                entries.push(HubProjectEntry {
                    kind: HubEntryKind::Ghost,
                    project: None,
                    github_repo: repo,
                });
            }
        }

        Ok(entries)
    }

    pub async fn list_groups(state: &AppState) -> Result<Vec<String>, ServiceError> {
        Ok(ProjectRepository::list_groups(state.db.pool()).await?)
    }

    pub async fn mark_opened(state: &AppState, id: &ProjectId) -> Result<(), ServiceError> {
        let now = Utc::now();
        ProjectRepository::mark_opened(state.db.pool(), id, now).await?;

        let activity = Activity {
            id: ActivityId::new(),
            project_id: Some(*id),
            activity_type: ActivityType::ProjectOpened,
            message: Some("Project opened".into()),
            metadata: None,
            created_at: now,
        };
        ActivityRepository::create(state.db.pool(), &activity).await?;
        Ok(())
    }

    pub async fn update_project(state: &AppState, project: Project) -> Result<Project, ServiceError> {
        let mut updated = project;
        updated.updated_at = Utc::now();
        ProjectRepository::update_metadata(state.db.pool(), &updated).await?;
        Ok(updated)
    }

    pub async fn list(state: &AppState) -> Result<Vec<Project>, ServiceError> {
        Ok(ProjectRepository::list(state.db.pool()).await?)
    }

    pub async fn get(state: &AppState, id: &ProjectId) -> Result<Project, ServiceError> {
        Ok(ProjectRepository::get(state.db.pool(), id).await?)
    }

    pub async fn search(state: &AppState, query: &str) -> Result<Vec<Project>, ServiceError> {
        Ok(ProjectRepository::search(state.db.pool(), query).await?)
    }

    pub async fn scan_directory(state: &AppState, path: &Path) -> Result<Vec<Project>, ServiceError> {
        if !path.exists() {
            return Err(ServiceError::InvalidPath(path.display().to_string()));
        }

        let detected = state.scanner.scan_directory(path);
        let mut saved = Vec::new();

        for info in detected {
            let project = upsert_discovered_project(state, &info).await?;
            link_project_github_remote(state, &project).await?;

            let activity = Activity {
                id: ActivityId::new(),
                project_id: Some(project.id),
                activity_type: ActivityType::ProjectScanned,
                message: Some(format!("Discovered project: {}", project.name)),
                metadata: None,
                created_at: Utc::now(),
            };
            ActivityRepository::create(state.db.pool(), &activity).await?;

            saved.push(project);
        }

        info!(path = %path.display(), count = saved.len(), "directory scanned");
        Ok(saved)
    }

    pub async fn refresh(state: &AppState, id: &ProjectId) -> Result<Project, ServiceError> {
        let existing = ProjectRepository::get(state.db.pool(), id).await?;
        let path = existing.root_path.clone();

        if let Some(info) = state.scanner.detect_at(&path) {
            let project = upsert_discovered_project(state, &info).await?;
            link_project_github_remote(state, &project).await?;
            Ok(project)
        } else {
            Ok(existing)
        }
    }

    pub async fn list_roots(state: &AppState) -> Result<Vec<ProjectRoot>, ServiceError> {
        Ok(ProjectRootRepository::list(state.db.pool()).await?)
    }

    pub async fn add_root(state: &AppState, path: &Path) -> Result<(ProjectRoot, Vec<Project>), ServiceError> {
        if !path.exists() {
            return Err(ServiceError::InvalidPath(path.display().to_string()));
        }

        let normalized = normalize_path(path);

        if let Some(existing) = ProjectRootRepository::find_by_path(state.db.pool(), &normalized).await? {
            info!(path = %normalized.display(), "project root already registered, rescanning");
            let projects = Self::scan_directory(state, &existing.path).await?;
            return Ok((existing, projects));
        }

        let root = ProjectRoot {
            id: ProjectRootId::new(),
            path: normalized,
            created_at: Utc::now(),
        };

        ProjectRootRepository::create(state.db.pool(), &root).await?;
        let projects = Self::scan_directory(state, &root.path).await?;

        Ok((root, projects))
    }

    pub async fn delete(state: &AppState, id: &ProjectId) -> Result<(), ServiceError> {
        let project = ProjectRepository::get(state.db.pool(), id).await?;
        ProjectRepository::delete(state.db.pool(), id).await?;

        let activity = Activity {
            id: ActivityId::new(),
            project_id: None,
            activity_type: ActivityType::Other,
            message: Some(format!("Removed project from hub: {}", project.name)),
            metadata: None,
            created_at: Utc::now(),
        };
        ActivityRepository::create(state.db.pool(), &activity).await?;

        info!(name = %project.name, "project removed from hub");
        Ok(())
    }

    pub fn open_folder(path: &Path) -> Result<(), ServiceError> {
        if !path.exists() {
            return Err(ServiceError::InvalidPath(path.display().to_string()));
        }
        open::that(path).map_err(|e| ServiceError::InvalidPath(e.to_string()))
    }
}

pub struct FolderService;

impl FolderService {
    pub async fn list(state: &AppState) -> Result<Vec<Folder>, ServiceError> {
        Ok(FolderRepository::list(state.db.pool()).await?)
    }

    pub async fn create(
        state: &AppState,
        name: String,
        parent_id: Option<FolderId>,
    ) -> Result<Folder, ServiceError> {
        let sort_order = FolderRepository::next_sort_order(state.db.pool()).await?;
        let folder = Folder {
            id: FolderId::new(),
            name,
            parent_id,
            sort_order,
            created_at: Utc::now(),
            project_count: 0,
        };
        FolderRepository::create(state.db.pool(), &folder).await?;
        Ok(folder)
    }

    pub async fn rename(state: &AppState, id: &FolderId, name: String) -> Result<(), ServiceError> {
        FolderRepository::rename(state.db.pool(), id, &name).await?;
        Ok(())
    }

    pub async fn delete(state: &AppState, id: &FolderId) -> Result<(), ServiceError> {
        FolderRepository::delete(state.db.pool(), id).await?;
        Ok(())
    }

    pub async fn reorder(state: &AppState, ids: &[FolderId]) -> Result<(), ServiceError> {
        FolderRepository::reorder(state.db.pool(), ids).await?;
        Ok(())
    }

    pub async fn assign_project(
        state: &AppState,
        project_id: &ProjectId,
        folder_id: Option<FolderId>,
    ) -> Result<(), ServiceError> {
        if let Some(folder_id) = folder_id {
            FolderRepository::assign_project(state.db.pool(), project_id, &folder_id).await?;
        } else {
            FolderRepository::unassign_project(state.db.pool(), project_id).await?;
        }
        Ok(())
    }
}

pub struct TaskService;

impl TaskService {
    pub async fn list(state: &AppState) -> Result<Vec<Task>, ServiceError> {
        Ok(TaskRepository::list(state.db.pool()).await?)
    }

    pub async fn list_by_project(state: &AppState, project_id: &ProjectId) -> Result<Vec<Task>, ServiceError> {
        Ok(TaskRepository::list_by_project(state.db.pool(), project_id).await?)
    }

    pub async fn create(state: &AppState, task: Task) -> Result<Task, ServiceError> {
        TaskRepository::create(state.db.pool(), &task).await?;

        let activity = Activity {
            id: ActivityId::new(),
            project_id: Some(task.project_id),
            activity_type: ActivityType::TaskCreated,
            message: Some(format!("Task created: {}", task.title)),
            metadata: None,
            created_at: Utc::now(),
        };
        ActivityRepository::create(state.db.pool(), &activity).await?;

        Ok(task)
    }

    pub async fn update(state: &AppState, task: Task) -> Result<Task, ServiceError> {
        TaskRepository::update(state.db.pool(), &task).await?;

        if task.status == project_hub_domain::TaskStatus::Done {
            let activity = Activity {
                id: ActivityId::new(),
                project_id: Some(task.project_id),
                activity_type: ActivityType::TaskCompleted,
                message: Some(format!("Task completed: {}", task.title)),
                metadata: None,
                created_at: Utc::now(),
            };
            ActivityRepository::create(state.db.pool(), &activity).await?;
        }

        Ok(task)
    }

    pub async fn delete(state: &AppState, id: &TaskId) -> Result<(), ServiceError> {
        TaskRepository::delete(state.db.pool(), id).await?;
        Ok(())
    }
}

pub struct CalendarService;

impl CalendarService {
    pub async fn list(state: &AppState) -> Result<Vec<CalendarEvent>, ServiceError> {
        Ok(CalendarRepository::list(state.db.pool()).await?)
    }

    pub async fn create(state: &AppState, event: CalendarEvent) -> Result<CalendarEvent, ServiceError> {
        CalendarRepository::create(state.db.pool(), &event).await?;
        Ok(event)
    }

    pub async fn delete(state: &AppState, id: &CalendarEventId) -> Result<(), ServiceError> {
        CalendarRepository::delete(state.db.pool(), id).await?;
        Ok(())
    }
}

pub struct ActivityService;

impl ActivityService {
    pub async fn list(state: &AppState, limit: i64) -> Result<Vec<Activity>, ServiceError> {
        Ok(ActivityRepository::list(state.db.pool(), limit).await?)
    }

    pub async fn list_by_project(
        state: &AppState,
        project_id: &ProjectId,
        limit: i64,
    ) -> Result<Vec<Activity>, ServiceError> {
        Ok(ActivityRepository::list_by_project(state.db.pool(), project_id, limit).await?)
    }
}

pub struct GitFacade;

impl GitFacade {
    pub fn status(project: &Project) -> Result<project_hub_git::GitStatus, ServiceError> {
        Ok(GitService::get_status(&project.root_path)?)
    }

    pub fn history(project: &Project, limit: usize) -> Result<Vec<project_hub_git::GitCommitInfo>, ServiceError> {
        Ok(GitService::get_recent_commits(&project.root_path, limit)?)
    }
}

pub struct EditorService;

impl EditorService {
    pub async fn get_config(state: &AppState) -> Result<EditorConfig, ServiceError> {
        let raw = SettingsRepository::get(state.db.pool(), EDITOR_SETTINGS_KEY).await?;
        let parsed = raw
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|e| ServiceError::InvalidPath(e.to_string()))?;
        Ok(editor::merge_editor_config(parsed))
    }

    pub async fn save_config(state: &AppState, config: &EditorConfig) -> Result<(), ServiceError> {
        let json = serde_json::to_string(config)
            .map_err(|e| ServiceError::InvalidPath(e.to_string()))?;
        SettingsRepository::set(state.db.pool(), EDITOR_SETTINGS_KEY, &json).await?;
        Ok(())
    }

    pub async fn open_project(
        state: &AppState,
        project: &Project,
        editor_id: Option<&str>,
    ) -> Result<(), ServiceError> {
        let config = Self::get_config(state).await?;
        let editor = editor::find_editor(&config, editor_id)
            .ok_or_else(|| ServiceError::NotFound("editor not configured".into()))?;
        editor::open_in_editor(editor, &project.root_path)
            .await
            .map_err(|e| ServiceError::InvalidPath(e))?;
        Ok(())
    }
}

pub struct CommandFacade;

impl CommandFacade {
    pub fn list_for_project(project: &Project) -> Vec<project_hub_domain::ProjectCommand> {
        detect_commands(project)
    }

    pub async fn run(project: &Project, command: &str) -> Result<project_hub_executor::process::ProcessOutput, ServiceError> {
        let output = project_hub_executor::CommandService::run(project, command).await?;
        Ok(output)
    }
}

fn info_to_project(info: &ProjectInfo) -> Project {
    let now = Utc::now();
    Project {
        id: ProjectId::new(),
        name: info.name.clone(),
        root_path: normalize_path(&info.root_path),
        language: info.language,
        project_type: info.project_type,
        status: ProjectStatus::Active,
        description: None,
        icon: None,
        group_name: None,
        github_repo_id: None,
        remote_url: None,
        last_opened_at: None,
        last_modified_at: compute_last_modified(&info.root_path),
        created_at: now,
        updated_at: now,
    }
}

async fn upsert_discovered_project(
    state: &AppState,
    info: &ProjectInfo,
) -> Result<Project, ServiceError> {
    let root_path = normalize_path(&info.root_path);
    let path_key = root_path.to_string_lossy().to_string();
    let mut project = info_to_project(info);
    project.root_path = root_path.clone();
    project.last_modified_at = compute_last_modified(&root_path);

    if let Some(existing) = ProjectRepository::get_by_path(state.db.pool(), &path_key).await? {
        project.id = existing.id;
        project.status = existing.status;
        project.description = existing.description.clone();
        project.icon = existing.icon.clone();
        project.group_name = existing.group_name.clone();
        project.github_repo_id = existing.github_repo_id;
        project.remote_url = existing.remote_url.clone();
        project.last_opened_at = existing.last_opened_at;
        project.created_at = existing.created_at;
    }

    project.updated_at = Utc::now();
    ProjectRepository::upsert(state.db.pool(), &project).await?;
    Ok(project)
}

pub(crate) async fn link_project_github_remote(state: &AppState, project: &Project) -> Result<(), ServiceError> {
    let urls = remote_urls_for_path(&project.root_path);
    let github_repos = GitHubRepoRepository::list(state.db.pool()).await.unwrap_or_default();

    for url in urls {
        if let Some(full_name) = parse_github_full_name(&url) {
            if let Some(repo) = github_repos.iter().find(|r| r.full_name == full_name) {
                let mut updated = project.clone();
                updated.github_repo_id = Some(repo.id);
                updated.remote_url = Some(url);
                updated.updated_at = Utc::now();
                ProjectRepository::update_metadata(state.db.pool(), &updated).await?;
                break;
            }
        }
    }

    Ok(())
}

fn match_local_to_github(
    item: &ProjectListItem,
    github_repos: &[project_hub_domain::GitHubRepo],
) -> Option<project_hub_domain::GitHubRepo> {
    if let Some(id) = item.project.github_repo_id {
        if let Some(repo) = github_repos.iter().find(|r| r.id == id) {
            return Some(repo.clone());
        }
    }

    if let Some(url) = &item.project.remote_url {
        if let Some(full_name) = parse_github_full_name(url) {
            if let Some(repo) = github_repos.iter().find(|r| r.full_name == full_name) {
                return Some(repo.clone());
            }
        }
    }

    for url in remote_urls_for_path(&item.project.root_path) {
        if let Some(full_name) = parse_github_full_name(&url) {
            if let Some(repo) = github_repos.iter().find(|r| r.full_name == full_name) {
                return Some(repo.clone());
            }
        }
    }

    None
}

fn compute_last_modified(path: &Path) -> Option<chrono::DateTime<Utc>> {
    let mut latest = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(chrono::DateTime::<Utc>::from);

    if GitService::has_git(path) {
        if let Ok(Some(commit)) = GitService::get_last_commit(path) {
            if latest.is_none_or(|l| commit.date > l) {
                latest = Some(commit.date);
            }
        }
    }

    latest
}
