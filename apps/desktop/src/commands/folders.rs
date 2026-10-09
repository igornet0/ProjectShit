use project_hub_domain::{Folder, FolderId, ProjectId};
use serde::Deserialize;
use tauri::State;

use crate::commands::SharedState;
use crate::services::{FolderService, ServiceError};

#[derive(Debug, Deserialize)]
pub struct CreateFolderInput {
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenameFolderInput {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AssignFolderInput {
    pub project_id: String,
    pub folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderFoldersInput {
    pub folder_ids: Vec<String>,
}

#[tauri::command]
pub async fn folders_list(state: State<'_, SharedState>) -> Result<Vec<Folder>, String> {
    let state = state.inner();
    FolderService::list(&state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn folders_create(
    state: State<'_, SharedState>,
    input: CreateFolderInput,
) -> Result<Folder, String> {
    let state = state.inner();
    let parent_id = input
        .parent_id
        .as_deref()
        .map(parse_folder_id)
        .transpose()?;
    FolderService::create(&state, input.name, parent_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn folders_rename(
    state: State<'_, SharedState>,
    input: RenameFolderInput,
) -> Result<(), String> {
    let state = state.inner();
    let id = parse_folder_id(&input.id)?;
    FolderService::rename(&state, &id, input.name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn folders_delete(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    let state = state.inner();
    FolderService::delete(&state, &parse_folder_id(&id)?)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn folders_assign_project(
    state: State<'_, SharedState>,
    input: AssignFolderInput,
) -> Result<(), String> {
    let state = state.inner();
    let project_id = parse_project_id(&input.project_id)?;
    let folder_id = input
        .folder_id
        .as_deref()
        .map(parse_folder_id)
        .transpose()?;
    FolderService::assign_project(&state, &project_id, folder_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn folders_reorder(
    state: State<'_, SharedState>,
    input: ReorderFoldersInput,
) -> Result<(), String> {
    let state = state.inner();
    let ids: Result<Vec<FolderId>, String> = input
        .folder_ids
        .iter()
        .map(|id| parse_folder_id(id))
        .collect();
    FolderService::reorder(&state, &ids?)
        .await
        .map_err(|e| e.to_string())
}

fn parse_folder_id(id: &str) -> Result<FolderId, String> {
    uuid::Uuid::parse_str(id)
        .map(FolderId::from_uuid)
        .map_err(|e| ServiceError::NotFound(e.to_string()).to_string())
}

fn parse_project_id(id: &str) -> Result<ProjectId, String> {
    uuid::Uuid::parse_str(id)
        .map(ProjectId::from_uuid)
        .map_err(|e| ServiceError::NotFound(e.to_string()).to_string())
}
