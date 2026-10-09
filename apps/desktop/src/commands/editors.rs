use project_hub_domain::{Project, ProjectId, ProjectListItem, ProjectStatus};
use serde::Deserialize;
use tauri::State;

use crate::commands::SharedState;
use crate::services::{normalize_custom_editor, EditorConfig, EditorDefinition, EditorService, ProjectService, ServiceError};

#[derive(Debug, Deserialize)]
pub struct UpdateProjectInput {
    pub id: String,
    pub group_name: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveEditorConfigInput {
    pub default_editor_id: Option<String>,
    pub editors: Vec<EditorDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct AddCustomEditorInput {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[tauri::command]
pub async fn projects_list_summaries(
    state: State<'_, SharedState>,
    include_archived: Option<bool>,
) -> Result<Vec<ProjectListItem>, String> {
    let state = state.inner();
    ProjectService::list_summaries(&state, include_archived.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_list_groups(state: State<'_, SharedState>) -> Result<Vec<String>, String> {
    let state = state.inner();
    ProjectService::list_groups(&state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_mark_opened(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    let project_id = parse_project_id(&id)?;
    let state = state.inner();
    ProjectService::mark_opened(&state, &project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_update(
    state: State<'_, SharedState>,
    input: UpdateProjectInput,
) -> Result<Project, String> {
    let project_id = parse_project_id(&input.id)?;
    let state = state.inner();
    let mut project = ProjectService::get(&state, &project_id)
        .await
        .map_err(|e| e.to_string())?;

    if input.group_name.is_some() {
        project.group_name = input
            .group_name
            .map(|g| if g.trim().is_empty() { None } else { Some(g.trim().to_string()) })
            .flatten();
    }
    if let Some(status) = input.status {
        project.status = ProjectStatus::from_str(&status);
    }

    ProjectService::update_project(&state, project)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn projects_open_in_editor(
    state: State<'_, SharedState>,
    id: String,
    editor_id: Option<String>,
) -> Result<(), String> {
    let project_id = parse_project_id(&id)?;
    let state = state.inner();
    let project = ProjectService::get(&state, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    EditorService::open_project(&state, &project, editor_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn editors_get_config(state: State<'_, SharedState>) -> Result<EditorConfig, String> {
    let state = state.inner();
    EditorService::get_config(&state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn editors_save_config(
    state: State<'_, SharedState>,
    input: SaveEditorConfigInput,
) -> Result<EditorConfig, String> {
    let state = state.inner();
    let config = EditorConfig {
        default_editor_id: input.default_editor_id,
        editors: input.editors,
    };
    EditorService::save_config(&state, &config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub async fn editors_add_custom(
    state: State<'_, SharedState>,
    input: AddCustomEditorInput,
) -> Result<EditorConfig, String> {
    let state = state.inner();
    let mut config = EditorService::get_config(&state)
        .await
        .map_err(|e| e.to_string())?;

    let custom = normalize_custom_editor(
        input.id,
        input.name,
        input.command,
        input.args,
    );

    if let Some(existing) = config.editors.iter_mut().find(|e| e.id == custom.id) {
        *existing = custom;
    } else {
        config.editors.push(custom);
    }

    EditorService::save_config(&state, &config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(config)
}

fn parse_project_id(id: &str) -> Result<ProjectId, String> {
    uuid::Uuid::parse_str(id)
        .map(ProjectId::from_uuid)
        .map_err(|e| ServiceError::NotFound(e.to_string()).to_string())
}
