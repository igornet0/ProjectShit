use project_hub_domain::{CalendarEvent, Task, TaskId};
use tauri::State;

use crate::commands::SharedState;
use crate::services::inputs::parse_uuid;
use crate::services::{
    CalendarService, CreateEventInput, CreateTaskInput, IssueService, IssueSyncReport, TaskService,
    TaskWithLink, UpdateTaskInput,
};

#[tauri::command]
pub async fn tasks_list(state: State<'_, SharedState>) -> Result<Vec<Task>, String> {
    let state = state.inner();
    TaskService::list(state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_list_by_project(
    state: State<'_, SharedState>,
    project_id: String,
) -> Result<Vec<Task>, String> {
    let id = parse_uuid(&project_id).map(project_hub_domain::ProjectId::from_uuid)?;
    let state = state.inner();
    TaskService::list_by_project(state, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_create(state: State<'_, SharedState>, input: CreateTaskInput) -> Result<Task, String> {
    let state = state.inner();
    TaskService::create_from_input(state, input)
        .await
        .map(|t| t.task)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_update(state: State<'_, SharedState>, input: UpdateTaskInput) -> Result<Task, String> {
    let state = state.inner();
    TaskService::update_from_input(state, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_delete(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    let task_id = TaskId(parse_uuid(&id)?);
    let state = state.inner();
    TaskService::delete(state, &task_id)
        .await
        .map_err(|e| e.to_string())
}

/// Open a GitHub issue for the task in its project's repository.
#[tauri::command]
pub async fn tasks_create_github_issue(
    state: State<'_, SharedState>,
    id: String,
    labels: Option<Vec<String>>,
) -> Result<TaskWithLink, String> {
    let task_id = TaskId(parse_uuid(&id)?);
    let state = state.inner();
    IssueService::create_for_task(state, &task_id, labels.unwrap_or_default())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn github_issues_sync(state: State<'_, SharedState>) -> Result<IssueSyncReport, String> {
    let state = state.inner();
    IssueService::sync(state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn calendar_list(state: State<'_, SharedState>) -> Result<Vec<CalendarEvent>, String> {
    let state = state.inner();
    CalendarService::list(state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn calendar_create(
    state: State<'_, SharedState>,
    input: CreateEventInput,
) -> Result<CalendarEvent, String> {
    let state = state.inner();
    CalendarService::create_from_input(state, input)
        .await
        .map_err(|e| e.to_string())
}
