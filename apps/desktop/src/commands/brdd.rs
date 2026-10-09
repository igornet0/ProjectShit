use project_hub_brdd::{BrddBundle, BrddReport};
use project_hub_domain::ProjectId;
use tauri::State;

use crate::commands::SharedState;
use crate::services::inputs::parse_uuid;
use crate::services::{BrddRefreshAll, BrddService, BrddSettings};

fn parse_project(id: &str) -> Result<ProjectId, String> {
    parse_uuid(id).map(ProjectId::from_uuid)
}

#[tauri::command]
pub async fn brdd_get(state: State<'_, SharedState>, project_id: String) -> Result<BrddBundle, String> {
    let id = parse_project(&project_id)?;
    BrddService::read(state.inner(), &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn brdd_refresh(
    state: State<'_, SharedState>,
    project_id: String,
    force_snapshot: Option<bool>,
) -> Result<BrddReport, String> {
    let id = parse_project(&project_id)?;
    BrddService::refresh(state.inner(), &id, force_snapshot.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn brdd_refresh_all(state: State<'_, SharedState>) -> Result<BrddRefreshAll, String> {
    BrddService::refresh_many(state.inner(), None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn brdd_save_notes(
    state: State<'_, SharedState>,
    project_id: String,
    markdown: String,
) -> Result<BrddBundle, String> {
    let id = parse_project(&project_id)?;
    BrddService::write_notes(state.inner(), &id, markdown)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn brdd_get_settings(state: State<'_, SharedState>) -> Result<BrddSettings, String> {
    BrddService::settings(state.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn brdd_save_settings(
    state: State<'_, SharedState>,
    settings: BrddSettings,
) -> Result<BrddSettings, String> {
    BrddService::save_settings(state.inner(), &settings)
        .await
        .map_err(|e| e.to_string())
}
