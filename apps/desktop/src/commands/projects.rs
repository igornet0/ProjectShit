use std::path::PathBuf;

use project_hub_domain::{Project, ProjectId, ProjectRoot};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::SharedState;
use crate::services::{ProjectService, ServiceError};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub projects: Vec<Project>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddRootResult {
    pub root: ProjectRoot,
    pub projects: Vec<Project>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub project: Project,
    pub packages: Vec<String>,
    pub commands: Vec<CommandDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandDto {
    pub name: String,
    pub command: String,
}

#[tauri::command]
pub async fn projects_list(state: State<'_, SharedState>) -> Result<Vec<Project>, String> {
    let state = state.lock().await;
    ProjectService::list(&state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_get(state: State<'_, SharedState>, id: String) -> Result<Project, String> {
    let project_id = parse_project_id(&id)?;
    let state = state.lock().await;
    ProjectService::get(&state, &project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_search(state: State<'_, SharedState>, query: String) -> Result<Vec<Project>, String> {
    let state = state.lock().await;
    ProjectService::search(&state, &query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_scan(state: State<'_, SharedState>, path: String) -> Result<ScanResult, String> {
    let state = state.lock().await;
    let projects = ProjectService::scan_directory(&state, PathBuf::from(&path).as_path())
        .await
        .map_err(|e| e.to_string())?;
    Ok(ScanResult { projects })
}

#[tauri::command]
pub async fn projects_refresh(state: State<'_, SharedState>, id: String) -> Result<Project, String> {
    let project_id = parse_project_id(&id)?;
    let state = state.lock().await;
    ProjectService::refresh(&state, &project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn project_roots_list(state: State<'_, SharedState>) -> Result<Vec<ProjectRoot>, String> {
    let state = state.lock().await;
    ProjectService::list_roots(&state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn project_roots_add(state: State<'_, SharedState>, path: String) -> Result<AddRootResult, String> {
    let state = state.lock().await;
    let (root, projects) = ProjectService::add_root(&state, PathBuf::from(&path).as_path())
        .await
        .map_err(|e| e.to_string())?;
    Ok(AddRootResult { root, projects })
}

#[tauri::command]
pub async fn projects_get_detail(state: State<'_, SharedState>, id: String) -> Result<ProjectDetail, String> {
    let project_id = parse_project_id(&id)?;
    let state = state.lock().await;
    let project = ProjectService::get(&state, &project_id)
        .await
        .map_err(|e| e.to_string())?;

    let commands = crate::services::CommandFacade::list_for_project(&project)
        .into_iter()
        .map(|c| CommandDto {
            name: c.name,
            command: c.command,
        })
        .collect();

    // Re-detect packages from filesystem
    let packages = state
        .scanner
        .detect_at(&project.root_path)
        .map(|i| i.packages)
        .unwrap_or_default();

    Ok(ProjectDetail {
        project,
        packages,
        commands,
    })
}

#[tauri::command]
pub async fn projects_delete(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    let project_id = parse_project_id(&id)?;
    let state = state.lock().await;
    ProjectService::delete(&state, &project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_open_folder(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    let project_id = parse_project_id(&id)?;
    let state = state.lock().await;
    let project = ProjectService::get(&state, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    ProjectService::open_folder(&project.root_path).map_err(|e| e.to_string())
}

fn parse_project_id(id: &str) -> Result<ProjectId, String> {
    uuid::Uuid::parse_str(id)
        .map(ProjectId::from_uuid)
        .map_err(|e| ServiceError::NotFound(e.to_string()).to_string())
}
