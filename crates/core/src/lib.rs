//! Project Hub application services — shared by the Tauri desktop shell and
//! the headless HTTP server (`apps/server`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use project_hub_database::{
    ActivityRepository, CalendarRepository, Database, DbError, FolderRepository, GitHubRepoRepository,
    ProjectRepository, ProjectRootRepository, SettingsRepository, TaskRepository,
    EDITOR_SETTINGS_KEY,
};
use project_hub_discovery::ProjectScanner;
use project_hub_domain::{
    enrich_with_hierarchy, normalize_path, Activity, ActivityId, ActivityType, CalendarEvent,
    CalendarEventId, Folder, FolderId, HubEntryKind, HubProjectEntry, Project,
    ProjectId, ProjectInfo, ProjectListItem, ProjectRoot, ProjectRootId, ProjectStatus, Task,
    TaskId, TaskStatus,
};
use project_hub_executor::providers::detect_commands;
use project_hub_git::GitService;
use project_hub_github::{parse_github_full_name, remote_urls_for_path};
use rayon::prelude::*;
use thiserror::Error;
use tracing::info;

pub mod brdd;
pub mod code;
mod editor;
mod github;
pub mod inputs;
pub mod issues;

pub use brdd::{BrddRefreshAll, BrddService, BrddSettings};
pub use code::CodeService;
pub use editor::{normalize_custom_editor, EditorConfig, EditorDefinition};
pub use github::{GitHubConfigResponse, GitHubOAuthStartResponse, GitHubService};
pub use inputs::{CreateEventInput, CreateTaskInput, RecurrenceInput, UpdateTaskInput};
pub use issues::{IssueService, IssueSyncReport, TaskLinkService, TaskWithLink};

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

    #[error("background task failed: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error(".brdd error: {0}")]
    Brdd(#[from] project_hub_brdd::BrddError),

    #[error("{0}")]
    Invalid(String),
}

/// Shared application state. Every field is safe to use concurrently, so
/// commands no longer serialize behind one global lock.
pub struct AppState {
    pub db: Database,
    pub scanner: Arc<ProjectScanner>,
    pub oauth_session: Mutex<Option<github::PendingOAuthSession>>,
    git_cache: GitDirtyCache,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            scanner: Arc::new(ProjectScanner::new()),
            oauth_session: Mutex::new(None),
            git_cache: GitDirtyCache::default(),
        }
    }
}

/// Short-lived cache of per-repo dirty flags so navigating between pages does
/// not re-run `git status` on every project each time.
#[derive(Default)]
struct GitDirtyCache {
    entries: Mutex<HashMap<PathBuf, (Instant, bool)>>,
}

impl GitDirtyCache {
    const TTL: Duration = Duration::from_secs(15);

    fn get(&self, path: &Path) -> Option<bool> {
        let entries = self.entries.lock().unwrap();
        entries
            .get(path)
            .filter(|(at, _)| at.elapsed() < Self::TTL)
            .map(|(_, dirty)| *dirty)
    }

    fn insert_many(&self, values: impl IntoIterator<Item = (PathBuf, bool)>) {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|_, (at, _)| at.elapsed() < Self::TTL);
        entries.extend(values.into_iter().map(|(path, dirty)| (path, (now, dirty))));
    }

    fn invalidate(&self, path: &Path) {
        self.entries.lock().unwrap().remove(path);
    }
}

/// Filesystem facts gathered for a discovered project off the async runtime.
struct DiscoveredProject {
    info: ProjectInfo,
    root_path: PathBuf,
    last_modified_at: Option<chrono::DateTime<Utc>>,
    origin_url: Option<String>,
}

fn probe_discovered(infos: Vec<ProjectInfo>) -> Vec<DiscoveredProject> {
    infos
        .into_par_iter()
        .map(|info| {
            let root_path = normalize_path(&info.root_path);
            DiscoveredProject {
                last_modified_at: compute_last_modified(&root_path),
                origin_url: remote_urls_for_path(&root_path).into_iter().next(),
                root_path,
                info,
            }
        })
        .collect()
}

pub struct ProjectService;

impl ProjectService {
    pub async fn list_summaries(
        state: &AppState,
        include_archived: bool,
    ) -> Result<Vec<ProjectListItem>, ServiceError> {
        let projects = ProjectRepository::list(state.db.pool()).await?;
        let folder_map = FolderRepository::folder_map(state.db.pool()).await?;
        let projects: Vec<Project> = projects
            .into_iter()
            .filter(|p| include_archived || p.status != ProjectStatus::Archived)
            .collect();

        let git_flags = Self::git_flags(state, &projects).await?;

        let mut items: Vec<ProjectListItem> = projects
            .into_iter()
            .zip(git_flags)
            .map(|(project, (has_git, git_dirty))| {
                let folder_id = folder_map.get(&project.id.to_string()).copied();
                ProjectListItem {
                    project,
                    has_git,
                    git_dirty,
                    parent_id: None,
                    child_count: 0,
                    depth: 0,
                    folder_id,
                }
            })
            .collect();

        enrich_with_hierarchy(&mut items);
        Ok(items)
    }

    /// `(has_git, git_dirty)` per project, in input order. Cache misses are
    /// computed in parallel on the blocking pool.
    async fn git_flags(state: &AppState, projects: &[Project]) -> Result<Vec<(bool, bool)>, ServiceError> {
        let mut flags: Vec<Option<(bool, bool)>> = projects
            .iter()
            .map(|p| state.git_cache.get(&p.root_path).map(|dirty| (true, dirty)))
            .collect();

        let misses: Vec<(usize, PathBuf)> = flags
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_none())
            .map(|(i, _)| (i, projects[i].root_path.clone()))
            .collect();

        if !misses.is_empty() {
            let computed = tokio::task::spawn_blocking(move || {
                misses
                    .into_par_iter()
                    .map(|(i, path)| {
                        let has_git = GitService::has_git(&path);
                        let dirty = has_git && GitService::is_dirty(&path);
                        (i, path, has_git, dirty)
                    })
                    .collect::<Vec<_>>()
            })
            .await?;

            state.git_cache.insert_many(
                computed
                    .iter()
                    .filter(|(_, _, has_git, _)| *has_git)
                    .map(|(_, path, _, dirty)| (path.clone(), *dirty)),
            );
            for (i, _, has_git, dirty) in computed {
                flags[i] = Some((has_git, dirty));
            }
        }

        Ok(flags.into_iter().map(|f| f.unwrap_or((false, false))).collect())
    }

    pub async fn list_hub_entries(
        state: &AppState,
        include_archived: bool,
    ) -> Result<Vec<HubProjectEntry>, ServiceError> {
        let github_repos = GitHubRepoRepository::list(state.db.pool()).await?;
        if github_repos.is_empty() {
            return Ok(Vec::new());
        }

        let local_items = Self::list_summaries(state, include_archived).await?;
        let by_id: HashMap<i64, &project_hub_domain::GitHubRepo> =
            github_repos.iter().map(|r| (r.id, r)).collect();
        let by_full_name: HashMap<&str, &project_hub_domain::GitHubRepo> =
            github_repos.iter().map(|r| (r.full_name.as_str(), r)).collect();

        let mut matches: Vec<Option<i64>> = local_items
            .iter()
            .map(|item| match_stored_github(item, &by_id, &by_full_name))
            .collect();

        // Only projects without a stored link fall back to reading .git/config.
        let unresolved: Vec<(usize, PathBuf)> = local_items
            .iter()
            .enumerate()
            .filter(|(i, item)| matches[*i].is_none() && item.has_git)
            .map(|(i, item)| (i, item.project.root_path.clone()))
            .collect();
        if !unresolved.is_empty() {
            let origins = tokio::task::spawn_blocking(move || {
                unresolved
                    .into_par_iter()
                    .map(|(i, path)| (i, remote_urls_for_path(&path)))
                    .collect::<Vec<_>>()
            })
            .await?;
            for (i, urls) in origins {
                matches[i] = urls
                    .iter()
                    .filter_map(|url| parse_github_full_name(url))
                    .find_map(|name| by_full_name.get(name.as_str()).map(|r| r.id));
            }
        }

        let mut matched_ids = HashSet::new();
        let mut entries = Vec::new();

        for (item, repo_id) in local_items.into_iter().zip(matches) {
            if let Some(repo) = repo_id.and_then(|id| by_id.get(&id)) {
                matched_ids.insert(repo.id);
                entries.push(HubProjectEntry {
                    kind: HubEntryKind::Local,
                    project: Some(item),
                    github_repo: (*repo).clone(),
                });
            }
        }

        for repo in &github_repos {
            if !matched_ids.contains(&repo.id) {
                entries.push(HubProjectEntry {
                    kind: HubEntryKind::Ghost,
                    project: None,
                    github_repo: repo.clone(),
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

        let scanner = Arc::clone(&state.scanner);
        let root = path.to_path_buf();
        let discovered = tokio::task::spawn_blocking(move || {
            probe_discovered(scanner.scan_directory(&root))
        })
        .await?;

        let pool = state.db.pool();
        let mut existing: HashMap<PathBuf, Project> = ProjectRepository::list(pool)
            .await?
            .into_iter()
            .map(|p| (p.root_path.clone(), p))
            .collect();
        let github_repos = GitHubRepoRepository::list(pool).await?;

        // One transaction for the whole scan instead of a commit per row.
        let mut tx = pool.begin().await.map_err(DbError::from)?;
        let mut saved = Vec::with_capacity(discovered.len());

        for found in discovered {
            let mut project = merge_discovered(&found, existing.remove(&found.root_path));
            apply_github_link(&mut project, found.origin_url.as_deref(), &github_repos);
            ProjectRepository::upsert(&mut *tx, &project).await?;

            let activity = Activity {
                id: ActivityId::new(),
                project_id: Some(project.id),
                activity_type: ActivityType::ProjectScanned,
                message: Some(format!("Discovered project: {}", project.name)),
                metadata: None,
                created_at: Utc::now(),
            };
            ActivityRepository::create(&mut *tx, &activity).await?;

            state.git_cache.invalidate(&project.root_path);
            saved.push(project);
        }

        tx.commit().await.map_err(DbError::from)?;

        info!(path = %path.display(), count = saved.len(), "directory scanned");
        Ok(saved)
    }

    pub async fn refresh(state: &AppState, id: &ProjectId) -> Result<Project, ServiceError> {
        let existing = ProjectRepository::get(state.db.pool(), id).await?;
        let path = existing.root_path.clone();

        let scanner = Arc::clone(&state.scanner);
        let found = tokio::task::spawn_blocking(move || {
            scanner
                .detect_at(&path)
                .map(|info| probe_discovered(vec![info]))
                .and_then(|mut v| v.pop())
        })
        .await?;

        let Some(found) = found else {
            return Ok(existing);
        };

        let github_repos = GitHubRepoRepository::list(state.db.pool()).await?;
        let existing_by_path = ProjectRepository::get_by_path(
            state.db.pool(),
            &found.root_path.to_string_lossy(),
        )
        .await?;
        let mut project = merge_discovered(&found, existing_by_path);
        apply_github_link(&mut project, found.origin_url.as_deref(), &github_repos);
        ProjectRepository::upsert(state.db.pool(), &project).await?;
        state.git_cache.invalidate(&project.root_path);
        Ok(project)
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
            project_id: task.project_id,
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
        Self::spawn_next_occurrence(state, &task).await?;

        if task.status == TaskStatus::Done {
            let activity = Activity {
                id: ActivityId::new(),
                project_id: task.project_id,
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

    /// Validate an API / IPC payload, create the task and its link (if any).
    pub async fn create_from_input(state: &AppState, input: CreateTaskInput) -> Result<TaskWithLink, ServiceError> {
        let source = input.source.clone().filter(|s| !s.trim().is_empty());
        let external_ref = input.external_ref.clone().filter(|s| !s.trim().is_empty());
        let task = input.into_task().map_err(ServiceError::Invalid)?;
        if let Some(project_id) = task.project_id {
            // Fail early with a clear message instead of a foreign-key error.
            ProjectRepository::get(state.db.pool(), &project_id).await?;
        }
        let task = Self::create(state, task).await?;
        let link = TaskLinkService::attach(state, &task.id, source, external_ref).await?;
        Ok(TaskWithLink { task, link })
    }

    pub async fn update_from_input(state: &AppState, input: UpdateTaskInput) -> Result<Task, ServiceError> {
        let id = TaskId(inputs::parse_uuid(&input.id).map_err(ServiceError::Invalid)?);
        let task = TaskRepository::get(state.db.pool(), &id).await?;
        let task = input.apply(task).map_err(ServiceError::Invalid)?;
        Self::update(state, task).await
    }

    /// Completing an instance of a recurring series creates the next one.
    /// Idempotent: nothing is spawned if a later instance already exists, so
    /// toggling done → todo → done does not duplicate occurrences.
    async fn spawn_next_occurrence(state: &AppState, task: &Task) -> Result<(), ServiceError> {
        let (TaskStatus::Done, Some(recurrence), Some(due_at)) =
            (task.status, task.recurrence.as_ref(), task.due_at)
        else {
            return Ok(());
        };
        let series_id = task.series_id.unwrap_or(task.id.0);
        if TaskRepository::series_has_instance_after(state.db.pool(), &series_id, due_at).await? {
            return Ok(());
        }
        let Some(next_due) = recurrence.next_after(due_at) else {
            return Ok(());
        };

        let now = Utc::now();
        let next = Task {
            id: TaskId::new(),
            status: TaskStatus::Todo,
            due_at: Some(next_due),
            series_id: Some(series_id),
            created_at: now,
            updated_at: now,
            ..task.clone()
        };
        TaskRepository::create(state.db.pool(), &next).await?;
        info!(series = %series_id, due = %next_due, "next recurring task occurrence created");
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

    pub async fn create_from_input(state: &AppState, input: CreateEventInput) -> Result<CalendarEvent, ServiceError> {
        let event = input.into_event().map_err(ServiceError::Invalid)?;
        Self::create(state, event).await
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

/// Build the project row for a discovered path, keeping user-owned fields
/// from the existing row when there is one.
fn merge_discovered(found: &DiscoveredProject, existing: Option<Project>) -> Project {
    let now = Utc::now();
    let info = &found.info;
    let mut project = Project {
        id: ProjectId::new(),
        name: info.name.clone(),
        root_path: found.root_path.clone(),
        language: info.language,
        project_type: info.project_type,
        status: ProjectStatus::Active,
        description: None,
        icon: None,
        group_name: None,
        github_repo_id: None,
        remote_url: None,
        last_opened_at: None,
        last_modified_at: found.last_modified_at,
        created_at: now,
        updated_at: now,
    };

    if let Some(existing) = existing {
        project.id = existing.id;
        project.status = existing.status;
        project.description = existing.description;
        project.icon = existing.icon;
        project.group_name = existing.group_name;
        project.github_repo_id = existing.github_repo_id;
        project.remote_url = existing.remote_url;
        project.last_opened_at = existing.last_opened_at;
        project.created_at = existing.created_at;
    }

    project
}

fn apply_github_link(
    project: &mut Project,
    origin_url: Option<&str>,
    github_repos: &[project_hub_domain::GitHubRepo],
) {
    let Some(url) = origin_url else { return };
    let Some(full_name) = parse_github_full_name(url) else { return };
    if let Some(repo) = github_repos.iter().find(|r| r.full_name == full_name) {
        project.github_repo_id = Some(repo.id);
        project.remote_url = Some(url.to_string());
    }
}

/// Link every stored project to its GitHub repo by `origin` remote. Remotes are
/// read in parallel off the runtime; only changed rows are written.
pub async fn link_all_github_remotes(state: &AppState) -> Result<usize, ServiceError> {
    let pool = state.db.pool();
    let github_repos = GitHubRepoRepository::list(pool).await?;
    if github_repos.is_empty() {
        return Ok(0);
    }

    let projects = ProjectRepository::list(pool).await?;
    let origins = tokio::task::spawn_blocking(move || {
        projects
            .into_par_iter()
            .map(|p| {
                let origin = remote_urls_for_path(&p.root_path).into_iter().next();
                (p, origin)
            })
            .collect::<Vec<_>>()
    })
    .await?;

    let mut tx = pool.begin().await.map_err(DbError::from)?;
    let mut linked = 0;
    for (project, origin) in origins {
        let mut updated = project.clone();
        apply_github_link(&mut updated, origin.as_deref(), &github_repos);
        if updated.github_repo_id != project.github_repo_id || updated.remote_url != project.remote_url {
            updated.updated_at = Utc::now();
            ProjectRepository::update_metadata(&mut *tx, &updated).await?;
            linked += 1;
        }
    }
    tx.commit().await.map_err(DbError::from)?;
    Ok(linked)
}

fn match_stored_github(
    item: &ProjectListItem,
    by_id: &HashMap<i64, &project_hub_domain::GitHubRepo>,
    by_full_name: &HashMap<&str, &project_hub_domain::GitHubRepo>,
) -> Option<i64> {
    if let Some(id) = item.project.github_repo_id.filter(|id| by_id.contains_key(id)) {
        return Some(id);
    }

    item.project
        .remote_url
        .as_deref()
        .and_then(parse_github_full_name)
        .and_then(|name| by_full_name.get(name.as_str()).map(|r| r.id))
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
