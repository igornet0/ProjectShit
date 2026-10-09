//! `.brdd` maintenance on top of the analysis crate.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use project_hub_brdd::{BrddBundle, BrddOptions, BrddReport};
use project_hub_database::{ProjectRepository, SettingsRepository, TaskRepository};
use project_hub_domain::{Project, ProjectId, ProjectStatus};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{AppState, ServiceError};

pub const BRDD_SETTINGS_KEY: &str = "brdd.settings";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrddSettings {
    /// Refresh `.brdd` for projects touched by scan / refresh.
    #[serde(default)]
    pub auto_on_scan: bool,
    /// List `.brdd/` in `.git/info/exclude`.
    #[serde(default = "yes")]
    pub exclude_from_git: bool,
}

fn yes() -> bool {
    true
}

impl Default for BrddSettings {
    fn default() -> Self {
        Self {
            auto_on_scan: false,
            exclude_from_git: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrddRefreshItem {
    pub project_id: ProjectId,
    pub name: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrddRefreshAll {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub refreshed: usize,
    pub failed: usize,
    pub items: Vec<BrddRefreshItem>,
}

pub struct BrddService;

impl BrddService {
    pub async fn settings(state: &AppState) -> Result<BrddSettings, ServiceError> {
        Ok(SettingsRepository::get(state.db.pool(), BRDD_SETTINGS_KEY)
            .await?
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default())
    }

    pub async fn save_settings(state: &AppState, settings: &BrddSettings) -> Result<BrddSettings, ServiceError> {
        let json = serde_json::to_string(settings).map_err(|e| ServiceError::Invalid(e.to_string()))?;
        SettingsRepository::set(state.db.pool(), BRDD_SETTINGS_KEY, &json).await?;
        Ok(settings.clone())
    }

    /// Analyse one project and rewrite its `.brdd` folder.
    pub async fn refresh(state: &AppState, id: &ProjectId, force_snapshot: bool) -> Result<BrddReport, ServiceError> {
        let project = ProjectRepository::get(state.db.pool(), id).await?;
        let settings = Self::settings(state).await?;
        Self::refresh_project(state, project, &settings, force_snapshot).await
    }

    async fn refresh_project(
        state: &AppState,
        project: Project,
        settings: &BrddSettings,
        force_snapshot: bool,
    ) -> Result<BrddReport, ServiceError> {
        let tasks = TaskRepository::list_by_project(state.db.pool(), &project.id).await?;
        let opts = BrddOptions {
            exclude_from_git: settings.exclude_from_git,
            force_snapshot,
            ..Default::default()
        };
        let report = tokio::task::spawn_blocking(move || project_hub_brdd::refresh(&project, &tasks, &opts))
            .await??;
        Ok(report)
    }

    /// Refresh every active project (or the given ones), sequentially to keep
    /// disk pressure low.
    pub async fn refresh_many(state: &AppState, ids: Option<Vec<ProjectId>>) -> Result<BrddRefreshAll, ServiceError> {
        let started_at = Utc::now();
        let settings = Self::settings(state).await?;
        let projects: Vec<Project> = ProjectRepository::list(state.db.pool())
            .await?
            .into_iter()
            .filter(|p| match &ids {
                Some(ids) => ids.contains(&p.id),
                None => p.status != ProjectStatus::Archived,
            })
            .collect();
        let mut items = Vec::with_capacity(projects.len());
        for project in projects {
            let (project_id, name) = (project.id, project.name.clone());
            match Self::refresh_project(state, project, &settings, false).await {
                Ok(report) => items.push(BrddRefreshItem {
                    project_id,
                    name,
                    ok: true,
                    error: None,
                    snapshot: report.snapshot.map(|s| s.n),
                }),
                Err(err) => items.push(BrddRefreshItem {
                    project_id,
                    name,
                    ok: false,
                    error: Some(err.to_string()),
                    snapshot: None,
                }),
            }
        }
        let failed = items.iter().filter(|i| !i.ok).count();
        info!(refreshed = items.len() - failed, failed, "brdd.refresh_many");
        Ok(BrddRefreshAll {
            started_at,
            finished_at: Utc::now(),
            refreshed: items.len() - failed,
            failed,
            items,
        })
    }

    pub async fn read(state: &AppState, id: &ProjectId) -> Result<BrddBundle, ServiceError> {
        let project = ProjectRepository::get(state.db.pool(), id).await?;
        Ok(tokio::task::spawn_blocking(move || project_hub_brdd::read(&project.root_path)).await?)
    }

    pub async fn write_notes(state: &AppState, id: &ProjectId, markdown: String) -> Result<BrddBundle, ServiceError> {
        let project = ProjectRepository::get(state.db.pool(), id).await?;
        let root = project.root_path.clone();
        tokio::task::spawn_blocking(move || {
            project_hub_brdd::write_notes(&root, &markdown)?;
            Ok::<_, ServiceError>(project_hub_brdd::read(&root))
        })
        .await?
    }

    /// Fire-and-forget refresh after a scan when the user enabled it.
    pub fn spawn_auto(state: Arc<AppState>, ids: Vec<ProjectId>) {
        if ids.is_empty() {
            return;
        }
        tokio::spawn(async move {
            match Self::settings(&state).await {
                Ok(s) if s.auto_on_scan => {
                    if let Err(err) = Self::refresh_many(&state, Some(ids)).await {
                        warn!(error = %err, "brdd.auto_refresh_failed");
                    }
                }
                _ => {}
            }
        });
    }
}
