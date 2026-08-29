use project_hub_git::{GitCommitInfo, GitStatus};
use project_hub_executor::process::ProcessOutput;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::SharedState;
use crate::services::{ActivityService, CommandFacade, GitFacade, ProjectService};

#[derive(Debug, Serialize, Deserialize)]
pub struct GitStatusResponse {
    pub status: GitStatus,
    pub recent_commits: Vec<GitCommitInfo>,
}

#[tauri::command]
pub async fn git_status(state: State<'_, SharedState>, project_id: String) -> Result<GitStatusResponse, String> {
    let id = uuid::Uuid::parse_str(&project_id)
        .map(project_hub_domain::ProjectId::from_uuid)
        .map_err(|e| e.to_string())?;

    let state = state.lock().await;
    let project = ProjectService::get(&state, &id)
        .await
        .map_err(|e| e.to_string())?;

    let status = GitFacade::status(&project).map_err(|e| e.to_string())?;
    let recent_commits = GitFacade::history(&project, 10).map_err(|e| e.to_string())?;

    Ok(GitStatusResponse {
        status,
        recent_commits,
    })
}

#[tauri::command]
pub async fn commands_run(
    state: State<'_, SharedState>,
    project_id: String,
    command: String,
) -> Result<ProcessOutput, String> {
    let id = uuid::Uuid::parse_str(&project_id)
        .map(project_hub_domain::ProjectId::from_uuid)
        .map_err(|e| e.to_string())?;

    let state = state.lock().await;
    let project = ProjectService::get(&state, &id)
        .await
        .map_err(|e| e.to_string())?;

    CommandFacade::run(&project, &command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn activity_list(
    state: State<'_, SharedState>,
    limit: Option<i64>,
) -> Result<Vec<project_hub_domain::Activity>, String> {
    let state = state.lock().await;
    ActivityService::list(&state, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn activity_list_by_project(
    state: State<'_, SharedState>,
    project_id: String,
    limit: Option<i64>,
) -> Result<Vec<project_hub_domain::Activity>, String> {
    let id = uuid::Uuid::parse_str(&project_id)
        .map(project_hub_domain::ProjectId::from_uuid)
        .map_err(|e| e.to_string())?;

    let state = state.lock().await;
    ActivityService::list_by_project(&state, &id, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}
