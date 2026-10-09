use std::path::{Path, PathBuf};

use chrono::Utc;
use project_hub_database::{
    GitHubRepoRepository, ProjectRepository, SettingsRepository, GITHUB_CONFIG_KEY,
    GITHUB_TOKEN_KEY,
};
use project_hub_domain::{GitHubConfig, GitHubRepo, Project};
use project_hub_github::{
    poll_access_token, start_device_flow, verification_url, DeviceFlowSession, GitHubClient,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::info;

use crate::{AppState, ProjectService, ServiceError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfigResponse {
    #[serde(flatten)]
    pub config: GitHubConfig,
    pub connected: bool,
}

pub struct GitHubService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOAuthStartResponse {
    pub user_code: String,
    pub verification_uri: String,
}

#[derive(Debug, Clone)]
pub struct PendingOAuthSession {
    pub client_id: String,
    pub session: DeviceFlowSession,
}

impl GitHubService {
    pub async fn get_config(state: &AppState) -> Result<GitHubConfigResponse, ServiceError> {
        let token = SettingsRepository::get(state.db.pool(), GITHUB_TOKEN_KEY).await?;
        let raw = SettingsRepository::get(state.db.pool(), GITHUB_CONFIG_KEY).await?;
        let config: GitHubConfig = raw
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|e| ServiceError::InvalidPath(e.to_string()))?
            .unwrap_or_default();

        Ok(GitHubConfigResponse {
            config,
            connected: token.filter(|t| !t.is_empty()).is_some(),
        })
    }

    pub async fn oauth_start(state: &AppState) -> Result<GitHubOAuthStartResponse, ServiceError> {
        let client_id = Self::oauth_client_id()?;
        let session = start_device_flow(&client_id).await?;
        let user_code = session.user_code.clone();
        let verification_uri = session.verification_uri.clone();

        open::that(verification_url(&session))
            .map_err(|e| ServiceError::InvalidPath(e.to_string()))?;

        *state.oauth_session.lock().unwrap() = Some(PendingOAuthSession {
            client_id,
            session,
        });

        Ok(GitHubOAuthStartResponse {
            user_code,
            verification_uri,
        })
    }

    pub async fn oauth_complete(
        state: &AppState,
        default_clone_dir: Option<String>,
    ) -> Result<GitHubConfigResponse, ServiceError> {
        // Take the session and release the lock before polling: the user may
        // spend minutes in the browser.
        let pending = state
            .oauth_session
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| ServiceError::InvalidPath("no pending GitHub authorization".into()))?;

        let token = poll_access_token(&pending.client_id, &pending.session).await?;
        Self::connect(state, token, default_clone_dir).await
    }

    fn oauth_client_id() -> Result<String, ServiceError> {
        const BUILT_IN: &str = match option_env!("PROJECT_HUB_GITHUB_OAUTH_CLIENT_ID") {
            Some(id) => id,
            None => "",
        };
        if !BUILT_IN.is_empty() {
            return Ok(BUILT_IN.to_string());
        }

        if let Ok(id) = std::env::var("GITHUB_OAUTH_CLIENT_ID") {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        Err(project_hub_github::GitHubError::OAuthNotConfigured.into())
    }

    pub async fn connect(state: &AppState, token: String, default_clone_dir: Option<String>) -> Result<GitHubConfigResponse, ServiceError> {
        let client = GitHubClient::new(token.trim());
        let username = client.validate_token().await?;

        SettingsRepository::set(state.db.pool(), GITHUB_TOKEN_KEY, token.trim()).await?;

        let config = GitHubConfig {
            username: Some(username),
            default_clone_dir,
            connected_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&config)
            .map_err(|e| ServiceError::InvalidPath(e.to_string()))?;
        SettingsRepository::set(state.db.pool(), GITHUB_CONFIG_KEY, &json).await?;

        Self::sync_repos(state).await?;

        Self::get_config(state).await
    }

    pub async fn disconnect(state: &AppState) -> Result<(), ServiceError> {
        SettingsRepository::set(state.db.pool(), GITHUB_TOKEN_KEY, "").await?;
        SettingsRepository::set(state.db.pool(), GITHUB_CONFIG_KEY, "").await?;
        GitHubRepoRepository::replace_all(state.db.pool(), &[]).await?;
        Ok(())
    }

    pub async fn save_config(state: &AppState, config: &GitHubConfig) -> Result<GitHubConfigResponse, ServiceError> {
        let existing = Self::get_config(state).await?;
        let merged = GitHubConfig {
            username: config.username.clone().or(existing.config.username),
            default_clone_dir: config.default_clone_dir.clone(),
            connected_at: existing.config.connected_at,
        };
        let json = serde_json::to_string(&merged)
            .map_err(|e| ServiceError::InvalidPath(e.to_string()))?;
        SettingsRepository::set(state.db.pool(), GITHUB_CONFIG_KEY, &json).await?;
        Self::get_config(state).await
    }

    pub async fn sync_repos(state: &AppState) -> Result<Vec<GitHubRepo>, ServiceError> {
        let token = SettingsRepository::get(state.db.pool(), GITHUB_TOKEN_KEY)
            .await?
            .filter(|t| !t.is_empty())
            .ok_or(project_hub_github::GitHubError::NotConnected)?;

        let client = GitHubClient::new(token);
        let repos = client.list_repos().await?;
        GitHubRepoRepository::replace_all(state.db.pool(), &repos).await?;

        crate::link_all_github_remotes(state).await?;

        info!(count = repos.len(), "github repos synced");
        Ok(repos)
    }

    pub async fn clone_repo(
        state: &AppState,
        full_name: &str,
        target_dir: Option<String>,
    ) -> Result<Project, ServiceError> {
        let repos = GitHubRepoRepository::list(state.db.pool()).await?;
        let repo = repos
            .iter()
            .find(|r| r.full_name == full_name)
            .ok_or_else(|| ServiceError::NotFound(format!("github repo {full_name}")))?;

        let config = Self::get_config(state).await?;
        let base = config
            .config
            .default_clone_dir
            .map(PathBuf::from)
            .unwrap_or_else(default_clone_dir);

        let repo_dir = full_name.split('/').next_back().unwrap_or(full_name);
        let target = target_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| base.join(repo_dir));

        if target.exists() {
            return Err(ServiceError::InvalidPath(format!(
                "target already exists: {}",
                target.display()
            )));
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ServiceError::InvalidPath(e.to_string()))?;
        }

        let status = Command::new("git")
            .args(["clone", &repo.clone_url, &target.to_string_lossy()])
            .status()
            .await
            .map_err(|e| ServiceError::InvalidPath(e.to_string()))?;

        if !status.success() {
            return Err(ServiceError::InvalidPath(format!(
                "git clone failed for {full_name}"
            )));
        }

        let projects = ProjectService::scan_directory(state, &target).await?;
        let mut cloned = projects
            .into_iter()
            .find(|p| normalize_path_eq(&p.root_path, &target))
            .ok_or_else(|| ServiceError::NotFound("cloned project not detected".into()))?;

        cloned.github_repo_id = Some(repo.id);
        cloned.remote_url = Some(repo.clone_url.clone());
        cloned.updated_at = Utc::now();
        ProjectRepository::update_metadata(state.db.pool(), &cloned).await?;

        Ok(cloned)
    }
}

fn default_clone_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Projects")
}

fn normalize_path_eq(a: &Path, b: &Path) -> bool {
    project_hub_domain::normalize_path(a) == project_hub_domain::normalize_path(b)
}
