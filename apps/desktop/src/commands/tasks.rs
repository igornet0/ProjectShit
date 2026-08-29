use chrono::{DateTime, Utc};
use project_hub_domain::{
    CalendarEvent, CalendarEventId, Task, TaskId, TaskPriority, TaskStatus,
};
use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

use crate::commands::SharedState;
use crate::services::{CalendarService, TaskService};

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventInput {
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub title: String,
    pub start_at: String,
    pub end_at: Option<String>,
}

#[tauri::command]
pub async fn tasks_list(state: State<'_, SharedState>) -> Result<Vec<Task>, String> {
    let state = state.lock().await;
    TaskService::list(&state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_list_by_project(
    state: State<'_, SharedState>,
    project_id: String,
) -> Result<Vec<Task>, String> {
    let id = parse_uuid(&project_id).map(project_hub_domain::ProjectId::from_uuid)?;
    let state = state.lock().await;
    TaskService::list_by_project(&state, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_create(state: State<'_, SharedState>, input: CreateTaskInput) -> Result<Task, String> {
    let now = Utc::now();
    let task = Task {
        id: TaskId::new(),
        project_id: project_hub_domain::ProjectId::from_uuid(parse_uuid(&input.project_id)?),
        title: input.title,
        description: input.description,
        status: TaskStatus::Todo,
        priority: input
            .priority
            .map(|p| TaskPriority::from_str(&p))
            .unwrap_or(TaskPriority::Medium),
        due_at: input.due_at.map(|d| parse_datetime(&d)).transpose()?,
        created_at: now,
        updated_at: now,
    };

    let state = state.lock().await;
    TaskService::create(&state, task).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_update(state: State<'_, SharedState>, input: UpdateTaskInput) -> Result<Task, String> {
    let state = state.lock().await;
    let task_id = TaskId(parse_uuid(&input.id)?);

    let mut task = project_hub_database::TaskRepository::get(state.db.pool(), &task_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(title) = input.title {
        task.title = title;
    }
    if input.description.is_some() {
        task.description = input.description;
    }
    if let Some(status) = input.status {
        task.status = TaskStatus::from_str(&status);
    }
    if let Some(priority) = input.priority {
        task.priority = TaskPriority::from_str(&priority);
    }
    if input.due_at.is_some() {
        task.due_at = input
            .due_at
            .map(|d| parse_datetime(&d))
            .transpose()?;
    }
    task.updated_at = Utc::now();

    TaskService::update(&state, task)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tasks_delete(state: State<'_, SharedState>, id: String) -> Result<(), String> {
    let task_id = TaskId(parse_uuid(&id)?);
    let state = state.lock().await;
    TaskService::delete(&state, &task_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn calendar_list(state: State<'_, SharedState>) -> Result<Vec<CalendarEvent>, String> {
    let state = state.lock().await;
    CalendarService::list(&state).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn calendar_create(
    state: State<'_, SharedState>,
    input: CreateEventInput,
) -> Result<CalendarEvent, String> {
    let event = CalendarEvent {
        id: CalendarEventId::new(),
        project_id: input
            .project_id
            .map(|p| parse_uuid(&p).map(project_hub_domain::ProjectId::from_uuid))
            .transpose()?,
        task_id: input
            .task_id
            .map(|t| parse_uuid(&t).map(TaskId))
            .transpose()?,
        title: input.title,
        start_at: parse_datetime(&input.start_at)?,
        end_at: input
            .end_at
            .map(|d| parse_datetime(&d))
            .transpose()?,
        created_at: Utc::now(),
    };

    let state = state.lock().await;
    CalendarService::create(&state, event)
        .await
        .map_err(|e| e.to_string())
}

fn parse_uuid(s: &str) -> Result<Uuid, String> {
    Uuid::parse_str(s).map_err(|e| e.to_string())
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| e.to_string())
}
