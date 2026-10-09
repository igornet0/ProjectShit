//! REST API (`/api/v1`). JSON in, JSON out; errors are `{ "error", "code" }`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use project_hub_core::inputs::parse_uuid;
use project_hub_core::{
    ActivityService, AppState, BrddService, BrddSettings, CodeService, CommandFacade, CreateTaskInput,
    GitFacade, GitHubService, IssueService, ProjectService, ServiceError, TaskLinkService, TaskService,
    UpdateTaskInput,
};
use project_hub_database::{DbError, TaskLinkRepository};
use project_hub_domain::{ProjectId, TaskId, TaskStatus};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::{CommandPolicy, Config};

pub const API_VERSION: &str = "v1";
const MAX_COMMAND_OUTPUT: usize = 64 * 1024;

pub struct ApiState {
    pub app: Arc<AppState>,
    pub config: Config,
}

type Shared = State<Arc<ApiState>>;
type ApiResult<T> = Result<Json<T>, ApiError>;

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    fn bad(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        let message = err.to_string();
        match err {
            ServiceError::NotFound(_) | ServiceError::Database(DbError::NotFound(_)) => {
                Self::new(StatusCode::NOT_FOUND, "not_found", message)
            }
            ServiceError::InvalidPath(_) | ServiceError::Invalid(_) => Self::bad(message),
            ServiceError::GitHub(project_hub_github::GitHubError::NotConnected) => {
                Self::new(StatusCode::CONFLICT, "github_not_connected", message)
            }
            ServiceError::GitHub(_) => Self::new(StatusCode::BAD_GATEWAY, "github_error", message),
            ServiceError::Brdd(project_hub_brdd::BrddError::MissingRoot(_)) => {
                Self::new(StatusCode::NOT_FOUND, "folder_missing", message)
            }
            _ => Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message, "code": self.code }))).into_response()
    }
}

pub fn router(state: Arc<ApiState>) -> Router {
    let api = Router::new()
        .route("/projects", get(list_projects))
        .route("/projects/scan", post(scan))
        .route("/projects/{id}", get(get_project))
        .route("/projects/{id}/detail", get(project_detail))
        .route("/projects/{id}/refresh", post(refresh_project))
        .route("/projects/{id}/git", get(project_git))
        .route("/projects/{id}/tree", get(project_tree))
        .route("/projects/{id}/file", get(project_file))
        .route("/projects/{id}/search", get(project_search))
        .route("/projects/{id}/commands", get(project_commands))
        .route("/projects/{id}/commands/run", post(run_command))
        .route("/projects/{id}/brdd", get(brdd_get))
        .route("/projects/{id}/brdd/refresh", post(brdd_refresh))
        .route("/projects/{id}/brdd/notes", axum::routing::put(brdd_notes))
        .route("/projects/{id}/tasks", get(project_tasks))
        .route("/projects/{id}/issues", get(project_issues))
        .route("/brdd/refresh-all", post(brdd_refresh_all))
        .route("/brdd/settings", get(brdd_settings).put(brdd_save_settings))
        .route("/roots", get(list_roots).post(add_root))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/{id}", get(get_task).patch(update_task).delete(delete_task))
        .route("/tasks/{id}/github-issue", post(task_issue))
        .route("/activity", get(activity))
        .route("/github", get(github_config))
        .route("/github/connect", post(github_connect))
        .route("/github/sync", post(github_sync))
        .route("/github/issues/sync", post(github_issues_sync))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth));

    Router::new()
        .route("/api/v1/health", get(health))
        .nest("/api/v1", api)
        .with_state(state)
}

/// Bearer token check (`Authorization: Bearer …` or `X-Project-Hub-Token`).
async fn auth(State(state): Shared, headers: HeaderMap, req: Request, next: Next) -> Response {
    let Some(expected) = state.config.token.as_deref() else {
        return next.run(req).await;
    };
    let given = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| headers.get("x-project-hub-token").and_then(|v| v.to_str().ok()));
    if given.is_some_and(|g| constant_eq(g.trim(), expected)) {
        next.run(req).await
    } else {
        ApiError::new(StatusCode::UNAUTHORIZED, "unauthorized", "missing or invalid token").into_response()
    }
}

fn constant_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn project_id(raw: &str) -> Result<ProjectId, ApiError> {
    parse_uuid(raw).map(ProjectId::from_uuid).map_err(ApiError::bad)
}

fn task_id(raw: &str) -> Result<TaskId, ApiError> {
    parse_uuid(raw).map(TaskId).map_err(ApiError::bad)
}

async fn health(State(state): Shared, headers: HeaderMap) -> Json<Value> {
    let authorized = state.config.token.as_deref().is_none_or(|t| {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|g| constant_eq(g.trim(), t))
    });
    let mut body = json!({
        "status": "ok",
        "service": "project-hub-server",
        "version": env!("CARGO_PKG_VERSION"),
        "api": API_VERSION,
        "auth": state.config.token.is_some(),
        "features": {
            "brdd": true,
            "code": true,
            "github": true,
            "commands": state.config.commands.as_str(),
        },
    });
    if authorized {
        let pool = state.app.db.pool();
        let count = |sql: &'static str| async move {
            sqlx_count(pool, sql).await.unwrap_or(-1)
        };
        body["db_path"] = json!(state.config.db_path);
        body["projects"] = json!(count("SELECT COUNT(*) FROM projects").await);
        body["tasks"] = json!(count("SELECT COUNT(*) FROM tasks").await);
    }
    Json(body)
}

async fn sqlx_count(pool: &project_hub_database::SqlitePool, sql: &str) -> Result<i64, DbError> {
    let row: (i64,) = project_hub_database::sqlx::query_as(sql).fetch_one(pool).await?;
    Ok(row.0)
}

// ─── Projects ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ListQuery {
    q: Option<String>,
    #[serde(default)]
    include_archived: bool,
    limit: Option<usize>,
    #[serde(default)]
    offset: usize,
}

async fn list_projects(State(state): Shared, Query(q): Query<ListQuery>) -> ApiResult<Value> {
    let mut items = ProjectService::list_summaries(&state.app, q.include_archived).await?;
    if let Some(needle) = q.q.as_deref().map(str::to_lowercase).filter(|s| !s.trim().is_empty()) {
        items.retain(|i| {
            i.project.name.to_lowercase().contains(&needle)
                || i.project.root_path.to_string_lossy().to_lowercase().contains(&needle)
                || i.project.group_name.as_deref().is_some_and(|g| g.to_lowercase().contains(&needle))
        });
    }
    let total = items.len();
    let limit = q.limit.unwrap_or(200).clamp(1, 2000);
    let page: Vec<_> = items.into_iter().skip(q.offset).take(limit).collect();
    Ok(Json(json!({ "total": total, "offset": q.offset, "items": page })))
}

async fn get_project(State(state): Shared, Path(id): Path<String>) -> ApiResult<Value> {
    let project = ProjectService::get(&state.app, &project_id(&id)?).await?;
    Ok(Json(json!(project)))
}

/// Project + commands + packages + git + `.brdd` digest + tasks — one call for agents.
async fn project_detail(State(state): Shared, Path(id): Path<String>) -> ApiResult<Value> {
    let id = project_id(&id)?;
    let project = ProjectService::get(&state.app, &id).await?;
    let tasks = TaskService::list_by_project(&state.app, &id).await?;
    let tasks = TaskLinkService::with_links(&state.app, tasks).await?;
    let scanner = Arc::clone(&state.app.scanner);
    let root = project.root_path.clone();
    let p = project.clone();
    let (commands, packages, git, brdd) = tokio::task::spawn_blocking(move || {
        let commands: Vec<Value> = CommandFacade::list_for_project(&p)
            .into_iter()
            .map(|c| json!({ "name": c.name, "command": c.command }))
            .collect();
        let packages = scanner.detect_at(&root).map(|i| i.packages).unwrap_or_default();
        let git = GitFacade::status(&p).ok().map(|status| {
            json!({ "status": status, "recent_commits": GitFacade::history(&p, 10).unwrap_or_default() })
        });
        let bundle = project_hub_brdd::read(&root);
        let brdd = json!({
            "exists": bundle.exists,
            "version": bundle.analysis.as_ref().and_then(|a| a.version.clone()),
            "stack": bundle.analysis.as_ref().map(|a| a.stack.clone()).unwrap_or_default(),
            "analyzed_at": bundle.analysis.as_ref().map(|a| a.analyzed_at),
            "snapshots": bundle.versions.len(),
            "has_notes": bundle.notes_md.is_some(),
        });
        (commands, packages, git, brdd)
    })
    .await
    .map_err(ServiceError::from)?;
    let github_repo = IssueService::repo_full_name(&state.app, &project).await.ok();
    Ok(Json(json!({
        "project": project,
        "exists": project.root_path.is_dir(),
        "packages": packages,
        "commands": commands,
        "git": git,
        "brdd": brdd,
        "github_repo": github_repo,
        "tasks": tasks,
    })))
}

#[derive(Deserialize)]
struct PathBody {
    path: String,
}

async fn scan(State(state): Shared, Json(body): Json<PathBody>) -> ApiResult<Value> {
    let projects = ProjectService::scan_directory(&state.app, &PathBuf::from(&body.path)).await?;
    BrddService::spawn_auto(state.app.clone(), projects.iter().map(|p| p.id).collect());
    Ok(Json(json!({ "projects": projects })))
}

async fn refresh_project(State(state): Shared, Path(id): Path<String>) -> ApiResult<Value> {
    let project = ProjectService::refresh(&state.app, &project_id(&id)?).await?;
    BrddService::spawn_auto(state.app.clone(), vec![project.id]);
    Ok(Json(json!(project)))
}

async fn list_roots(State(state): Shared) -> ApiResult<Value> {
    Ok(Json(json!(ProjectService::list_roots(&state.app).await?)))
}

async fn add_root(State(state): Shared, Json(body): Json<PathBody>) -> ApiResult<Value> {
    let (root, projects) = ProjectService::add_root(&state.app, &PathBuf::from(&body.path)).await?;
    BrddService::spawn_auto(state.app.clone(), projects.iter().map(|p| p.id).collect());
    Ok(Json(json!({ "root": root, "projects": projects })))
}

#[derive(Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

async fn project_git(State(state): Shared, Path(id): Path<String>, Query(q): Query<LimitQuery>) -> ApiResult<Value> {
    let project = ProjectService::get(&state.app, &project_id(&id)?).await?;
    let limit = q.limit.unwrap_or(20).clamp(1, 200);
    let (status, commits) = tokio::task::spawn_blocking(move || {
        Ok::<_, ServiceError>((GitFacade::status(&project)?, GitFacade::history(&project, limit)?))
    })
    .await
    .map_err(ServiceError::from)??;
    Ok(Json(json!({ "status": status, "recent_commits": commits })))
}

// ─── Code ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TreeQuery {
    #[serde(default)]
    path: String,
    depth: Option<usize>,
}

async fn project_tree(State(state): Shared, Path(id): Path<String>, Query(q): Query<TreeQuery>) -> ApiResult<Value> {
    let project = ProjectService::get(&state.app, &project_id(&id)?).await?;
    let tree = tokio::task::spawn_blocking(move || CodeService::tree(&project.root_path, &q.path, q.depth.unwrap_or(2)))
        .await
        .map_err(ServiceError::from)??;
    Ok(Json(json!(tree)))
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
    max_bytes: Option<u64>,
}

async fn project_file(State(state): Shared, Path(id): Path<String>, Query(q): Query<FileQuery>) -> ApiResult<Value> {
    let project = ProjectService::get(&state.app, &project_id(&id)?).await?;
    let file = tokio::task::spawn_blocking(move || CodeService::read_file(&project.root_path, &q.path, q.max_bytes))
        .await
        .map_err(ServiceError::from)??;
    Ok(Json(json!(file)))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
}

async fn project_search(State(state): Shared, Path(id): Path<String>, Query(q): Query<SearchQuery>) -> ApiResult<Value> {
    let project = ProjectService::get(&state.app, &project_id(&id)?).await?;
    let found = tokio::task::spawn_blocking(move || CodeService::search(&project.root_path, &q.q, q.limit.unwrap_or(100)))
        .await
        .map_err(ServiceError::from)??;
    Ok(Json(json!(found)))
}

async fn project_commands(State(state): Shared, Path(id): Path<String>) -> ApiResult<Value> {
    let project = ProjectService::get(&state.app, &project_id(&id)?).await?;
    let commands: Vec<Value> = CommandFacade::list_for_project(&project)
        .into_iter()
        .map(|c| json!({ "name": c.name, "command": c.command }))
        .collect();
    Ok(Json(json!({ "policy": state.config.commands.as_str(), "commands": commands })))
}

#[derive(Deserialize)]
struct RunBody {
    command: String,
    timeout_secs: Option<u64>,
}

async fn run_command(State(state): Shared, Path(id): Path<String>, Json(body): Json<RunBody>) -> ApiResult<Value> {
    let project = ProjectService::get(&state.app, &project_id(&id)?).await?;
    let command = body.command.trim().to_string();
    match state.config.commands {
        CommandPolicy::Off => {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "commands_disabled",
                "command execution is disabled (start the server with --commands detected|any)",
            ))
        }
        CommandPolicy::Detected => {
            let allowed = CommandFacade::list_for_project(&project)
                .into_iter()
                .any(|c| c.command == command || c.name == command);
            if !allowed {
                return Err(ApiError::new(
                    StatusCode::FORBIDDEN,
                    "command_not_allowed",
                    format!("`{command}` is not a detected command of this project"),
                ));
            }
        }
        CommandPolicy::Any => {}
    }
    // A detected command may be addressed by its name.
    let resolved = CommandFacade::list_for_project(&project)
        .into_iter()
        .find(|c| c.name == command)
        .map(|c| c.command)
        .unwrap_or(command);
    let timeout = Duration::from_secs(body.timeout_secs.unwrap_or(300).clamp(1, 1800));
    let started = std::time::Instant::now();
    let output = tokio::time::timeout(timeout, CommandFacade::run(&project, &resolved))
        .await
        .map_err(|_| ApiError::new(StatusCode::GATEWAY_TIMEOUT, "timeout", format!("`{resolved}` timed out")))??;
    Ok(Json(json!({
        "command": resolved,
        "exit_code": output.exit_code,
        "ok": output.exit_code == 0,
        "duration_ms": started.elapsed().as_millis() as u64,
        "stdout": tail(&output.stdout),
        "stderr": tail(&output.stderr),
    })))
}

fn tail(s: &str) -> String {
    if s.len() <= MAX_COMMAND_OUTPUT {
        return s.to_string();
    }
    let mut start = s.len() - MAX_COMMAND_OUTPUT;
    while !s.is_char_boundary(start) {
        start += 1;
    }
    format!("…{}", &s[start..])
}

// ─── .brdd ──────────────────────────────────────────────────────────────────

async fn brdd_get(State(state): Shared, Path(id): Path<String>) -> ApiResult<Value> {
    Ok(Json(json!(BrddService::read(&state.app, &project_id(&id)?).await?)))
}

#[derive(Deserialize, Default)]
struct BrddRefreshBody {
    #[serde(default)]
    force_snapshot: bool,
}

async fn brdd_refresh(State(state): Shared, Path(id): Path<String>, body: Option<Json<BrddRefreshBody>>) -> ApiResult<Value> {
    let force = body.map(|Json(b)| b.force_snapshot).unwrap_or(false);
    Ok(Json(json!(BrddService::refresh(&state.app, &project_id(&id)?, force).await?)))
}

#[derive(Deserialize)]
struct NotesBody {
    markdown: String,
}

async fn brdd_notes(State(state): Shared, Path(id): Path<String>, Json(body): Json<NotesBody>) -> ApiResult<Value> {
    Ok(Json(json!(BrddService::write_notes(&state.app, &project_id(&id)?, body.markdown).await?)))
}

#[derive(Deserialize, Default)]
struct RefreshAllBody {
    project_ids: Option<Vec<String>>,
}

async fn brdd_refresh_all(State(state): Shared, body: Option<Json<RefreshAllBody>>) -> ApiResult<Value> {
    let ids = body
        .and_then(|Json(b)| b.project_ids)
        .map(|ids| ids.iter().map(|i| project_id(i)).collect::<Result<Vec<_>, _>>())
        .transpose()?;
    Ok(Json(json!(BrddService::refresh_many(&state.app, ids).await?)))
}

async fn brdd_settings(State(state): Shared) -> ApiResult<Value> {
    Ok(Json(json!(BrddService::settings(&state.app).await?)))
}

async fn brdd_save_settings(State(state): Shared, Json(body): Json<BrddSettings>) -> ApiResult<Value> {
    Ok(Json(json!(BrddService::save_settings(&state.app, &body).await?)))
}

// ─── Tasks ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TaskQuery {
    project_id: Option<String>,
    status: Option<String>,
    /// Comma-separated task ids (tracking many tasks in one call).
    ids: Option<String>,
    /// Only tasks linked from this source (e.g. `boarddo`).
    source: Option<String>,
}

async fn list_tasks(State(state): Shared, Query(q): Query<TaskQuery>) -> ApiResult<Value> {
    let mut tasks = match q.project_id.as_deref().filter(|s| !s.is_empty()) {
        Some(pid) => TaskService::list_by_project(&state.app, &project_id(pid)?).await?,
        None => TaskService::list(&state.app).await?,
    };
    if let Some(status) = q.status.as_deref().filter(|s| !s.is_empty()) {
        let wanted: Vec<TaskStatus> = status.split(',').map(TaskStatus::from_str).collect();
        tasks.retain(|t| wanted.contains(&t.status));
    }
    if let Some(ids) = q.ids.as_deref().filter(|s| !s.is_empty()) {
        let wanted: Vec<String> = ids.split(',').map(|s| s.trim().to_string()).collect();
        tasks.retain(|t| wanted.contains(&t.id.to_string()));
    }
    let mut items = TaskLinkService::with_links(&state.app, tasks).await?;
    if let Some(source) = q.source.as_deref().filter(|s| !s.is_empty()) {
        items.retain(|t| t.link.as_ref().and_then(|l| l.source.as_deref()) == Some(source));
    }
    Ok(Json(json!(items)))
}

async fn project_tasks(State(state): Shared, Path(id): Path<String>) -> ApiResult<Value> {
    let tasks = TaskService::list_by_project(&state.app, &project_id(&id)?).await?;
    Ok(Json(json!(TaskLinkService::with_links(&state.app, tasks).await?)))
}

async fn create_task(State(state): Shared, Json(body): Json<CreateTaskInput>) -> Result<(StatusCode, Json<Value>), ApiError> {
    let created = TaskService::create_from_input(&state.app, body).await?;
    Ok((StatusCode::CREATED, Json(json!(created))))
}

async fn get_task(State(state): Shared, Path(id): Path<String>) -> ApiResult<Value> {
    Ok(Json(json!(TaskLinkService::get(&state.app, &task_id(&id)?).await?)))
}

async fn update_task(State(state): Shared, Path(id): Path<String>, Json(mut body): Json<UpdateTaskInput>) -> ApiResult<Value> {
    body.id = id.clone();
    TaskService::update_from_input(&state.app, body).await?;
    Ok(Json(json!(TaskLinkService::get(&state.app, &task_id(&id)?).await?)))
}

async fn delete_task(State(state): Shared, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    TaskService::delete(&state.app, &task_id(&id)?).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, Default)]
struct IssueBody {
    #[serde(default)]
    labels: Vec<String>,
}

async fn task_issue(State(state): Shared, Path(id): Path<String>, body: Option<Json<IssueBody>>) -> ApiResult<Value> {
    let labels = body.map(|Json(b)| b.labels).unwrap_or_default();
    Ok(Json(json!(IssueService::create_for_task(&state.app, &task_id(&id)?, labels).await?)))
}

#[derive(Deserialize)]
struct IssuesQuery {
    state: Option<String>,
    limit: Option<usize>,
}

async fn project_issues(State(state): Shared, Path(id): Path<String>, Query(q): Query<IssuesQuery>) -> ApiResult<Value> {
    let issues = IssueService::list_for_project(
        &state.app,
        &project_id(&id)?,
        q.state.as_deref().unwrap_or("open"),
        q.limit.unwrap_or(30),
    )
    .await?;
    Ok(Json(json!(issues)))
}

// ─── Activity / GitHub ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ActivityQuery {
    project_id: Option<String>,
    limit: Option<i64>,
}

async fn activity(State(state): Shared, Query(q): Query<ActivityQuery>) -> ApiResult<Value> {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let items = match q.project_id.as_deref().filter(|s| !s.is_empty()) {
        Some(pid) => ActivityService::list_by_project(&state.app, &project_id(pid)?, limit).await?,
        None => ActivityService::list(&state.app, limit).await?,
    };
    Ok(Json(json!(items)))
}

async fn github_config(State(state): Shared) -> ApiResult<Value> {
    let config = GitHubService::get_config(&state.app).await?;
    let linked = TaskLinkRepository::list_with_issues(state.app.db.pool())
        .await
        .map(|l| l.len())
        .unwrap_or(0);
    Ok(Json(json!({ "config": config, "linked_issues": linked })))
}

#[derive(Deserialize)]
struct ConnectBody {
    token: String,
    default_clone_dir: Option<String>,
}

async fn github_connect(State(state): Shared, Json(body): Json<ConnectBody>) -> ApiResult<Value> {
    Ok(Json(json!(GitHubService::connect(&state.app, body.token, body.default_clone_dir).await?)))
}

async fn github_sync(State(state): Shared) -> ApiResult<Value> {
    let repos = GitHubService::sync_repos(&state.app).await?;
    Ok(Json(json!({ "repos": repos.len() })))
}

async fn github_issues_sync(State(state): Shared) -> ApiResult<Value> {
    Ok(Json(json!(IssueService::sync(&state.app).await?)))
}
