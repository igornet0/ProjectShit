use project_hub_domain::{GitHubConfig, GitHubRepo, HubProjectEntry, Project};
use serde::Deserialize;
use tauri::State;

use crate::commands::SharedState;
use crate::services::{GitHubConfigResponse, GitHubOAuthStartResponse, GitHubService, ProjectService};

#[derive(Debug, Deserialize)]
pub struct GitHubConnectInput {
    pub token: String,
    pub default_clone_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubOAuthLoginInput {
    pub default_clone_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubCloneInput {
    pub full_name: String,
    pub target_dir: Option<String>,
}

#[tauri::command]
pub async fn github_get_config(state: State<'_, SharedState>) -> Result<GitHubConfigResponse, String> {
    let state = state.lock().await;
    GitHubService::get_config(&state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_connect(
    state: State<'_, SharedState>,
    input: GitHubConnectInput,
) -> Result<GitHubConfigResponse, String> {
    let state = state.lock().await;
    GitHubService::connect(&state, input.token, input.default_clone_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_oauth_start(
    state: State<'_, SharedState>,
) -> Result<GitHubOAuthStartResponse, String> {
    let mut state = state.lock().await;
    GitHubService::oauth_start(&mut state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_oauth_complete(
    state: State<'_, SharedState>,
    input: GitHubOAuthLoginInput,
) -> Result<GitHubConfigResponse, String> {
    let mut state = state.lock().await;
    GitHubService::oauth_complete(&mut state, input.default_clone_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_disconnect(state: State<'_, SharedState>) -> Result<(), String> {
    let state = state.lock().await;
    GitHubService::disconnect(&state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_save_config(
    state: State<'_, SharedState>,
    config: GitHubConfig,
) -> Result<GitHubConfigResponse, String> {
    let state = state.lock().await;
    GitHubService::save_config(&state, &config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_sync_repos(state: State<'_, SharedState>) -> Result<Vec<GitHubRepo>, String> {
    let state = state.lock().await;
    GitHubService::sync_repos(&state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_clone_repo(
    state: State<'_, SharedState>,
    input: GitHubCloneInput,
) -> Result<Project, String> {
    let state = state.lock().await;
    GitHubService::clone_repo(&state, &input.full_name, input.target_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_list_hub(
    state: State<'_, SharedState>,
    include_archived: Option<bool>,
) -> Result<Vec<HubProjectEntry>, String> {
    let state = state.lock().await;
    ProjectService::list_hub_entries(&state, include_archived.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}
