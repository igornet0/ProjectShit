use chrono::{DateTime, Utc};
use project_hub_domain::GitHubRepo;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("github api error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("invalid token")]
    InvalidToken,

    #[error("not connected")]
    NotConnected,

    #[error("GitHub sign-in is not available in this build")]
    OAuthNotConfigured,

    #[error("oauth authorization expired")]
    OAuthExpired,

    #[error("oauth authorization denied")]
    OAuthDenied,
}

pub struct GitHubClient {
    http: reqwest::Client,
    token: String,
}

impl GitHubClient {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            token: token.into(),
        }
    }

    pub async fn validate_token(&self) -> Result<String, GitHubError> {
        #[derive(Deserialize)]
        struct User {
            login: String,
        }

        let user: User = self.get("/user").await?;
        Ok(user.login)
    }

    pub async fn list_repos(&self) -> Result<Vec<GitHubRepo>, GitHubError> {
        let mut all = Vec::new();
        let mut page = 1u32;

        loop {
            let path = format!(
                "/user/repos?per_page=100&page={page}&sort=updated&affiliation=owner,collaborator,organization_member"
            );
            let batch: Vec<ApiRepo> = self.get(&path).await?;
            if batch.is_empty() {
                break;
            }
            let count = batch.len();
            all.extend(batch.into_iter().filter_map(|r| r.into_repo()));
            if count < 100 {
                break;
            }
            page += 1;
            if page > 20 {
                break;
            }
        }

        Ok(all)
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, GitHubError> {
        let url = format!("https://api.github.com{path}");
        let response = self
            .http
            .get(&url)
            .header(USER_AGENT, "ProjectHub/0.1")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await?;

        let status = response.status();
        if status.as_u16() == 401 {
            return Err(GitHubError::InvalidToken);
        }
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(GitHubError::Api {
                status: status.as_u16(),
                message,
            });
        }

        Ok(response.json().await?)
    }
}

#[derive(Debug, Deserialize, Clone)]
struct ApiRepo {
    id: i64,
    full_name: String,
    clone_url: String,
    ssh_url: Option<String>,
    html_url: String,
    description: Option<String>,
    private: bool,
    default_branch: Option<String>,
    updated_at: String,
}

impl ApiRepo {
    fn into_repo(self) -> Option<GitHubRepo> {
        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .ok()?
            .with_timezone(&Utc);

        Some(GitHubRepo {
            id: self.id,
            full_name: self.full_name,
            clone_url: self.clone_url,
            ssh_url: self.ssh_url,
            html_url: self.html_url,
            description: self.description,
            private: self.private,
            default_branch: self.default_branch,
            updated_at,
        })
    }
}
