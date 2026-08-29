#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod services;

use std::sync::Arc;

use project_hub_database::init_db;
use services::AppState;
use tauri::Manager;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use commands::SharedState;

fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "project_hub=debug,tauri=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");

            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data directory");

            let db_path = app_data_dir.join("project-hub.db");
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            let db = rt
                .block_on(init_db(&db_path))
                .expect("failed to initialize database");

            let state: SharedState = Arc::new(Mutex::new(AppState::new(db)));
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects_list,
            commands::projects_get,
            commands::projects_search,
            commands::projects_scan,
            commands::projects_refresh,
            commands::projects_get_detail,
            commands::project_roots_list,
            commands::project_roots_add,
            commands::projects_delete,
            commands::projects_open_folder,
            commands::projects_list_summaries,
            commands::projects_list_groups,
            commands::projects_mark_opened,
            commands::projects_update,
            commands::projects_open_in_editor,
            commands::editors_get_config,
            commands::editors_save_config,
            commands::editors_add_custom,
            commands::tasks_list,
            commands::tasks_list_by_project,
            commands::tasks_create,
            commands::tasks_update,
            commands::tasks_delete,
            commands::calendar_list,
            commands::calendar_create,
            commands::git_status,
            commands::commands_run,
            commands::activity_list,
            commands::activity_list_by_project,
            commands::folders_list,
            commands::folders_create,
            commands::folders_rename,
            commands::folders_delete,
            commands::folders_assign_project,
            commands::folders_reorder,
            commands::github_get_config,
            commands::github_connect,
            commands::github_oauth_start,
            commands::github_oauth_complete,
            commands::github_disconnect,
            commands::github_save_config,
            commands::github_sync_repos,
            commands::github_clone_repo,
            commands::projects_list_hub,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
