use chrono::{DateTime, Utc};
use project_hub_domain::GitHubRepo;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
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

pub const GITHUB_API: &str = "https://api.github.com";

/// Issue (or pull request — GitHub lists both) as Project Hub uses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub number: i64,
    pub title: String,
    /// `open` | `closed`
    pub state: String,
    pub html_url: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub is_pull_request: bool,
}

pub struct GitHubClient {
    http: reqwest::Client,
    token: String,
    base: String,
}

impl GitHubClient {
    pub fn new(token: impl Into<String>) -> Self {
        // `PROJECT_HUB_GITHUB_API` points the client at GitHub Enterprise or a test server.
        let base = std::env::var("PROJECT_HUB_GITHUB_API")
            .ok()
            .map(|b| b.trim().trim_end_matches('/').to_string())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| GITHUB_API.to_string());
        Self::with_api_base(token, base)
    }

    pub fn with_api_base(token: impl Into<String>, base: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            token: token.into(),
            base: base.into(),
        }
    }

    /// Issues of `owner/repo` (pull requests excluded), newest first.
    pub async fn list_issues(&self, full_name: &str, state: &str, limit: usize) -> Result<Vec<GitHubIssue>, GitHubError> {
        let state = match state {
            "open" | "closed" | "all" => state,
            _ => "open",
        };
        let per_page = limit.clamp(1, 100);
        let raw: Vec<ApiIssue> = self
            .get(&format!("/repos/{full_name}/issues?state={state}&per_page={per_page}&sort=updated"))
            .await?;
        Ok(raw
            .into_iter()
            .filter_map(ApiIssue::into_issue)
            .filter(|i| !i.is_pull_request)
            .take(limit)
            .collect())
    }

    pub async fn get_issue(&self, full_name: &str, number: i64) -> Result<GitHubIssue, GitHubError> {
        let raw: ApiIssue = self.get(&format!("/repos/{full_name}/issues/{number}")).await?;
        raw.into_issue().ok_or_else(|| GitHubError::Api {
            status: 500,
            message: "malformed issue".into(),
        })
    }

    pub async fn create_issue(
        &self,
        full_name: &str,
        title: &str,
        body: Option<&str>,
        labels: &[String],
    ) -> Result<GitHubIssue, GitHubError> {
        let mut payload = serde_json::json!({ "title": title });
        if let Some(body) = body.filter(|b| !b.trim().is_empty()) {
            payload["body"] = serde_json::json!(body);
        }
        if !labels.is_empty() {
            payload["labels"] = serde_json::json!(labels);
        }
        let raw: ApiIssue = self
            .send(reqwest::Method::POST, &format!("/repos/{full_name}/issues"), &payload)
            .await?;
        raw.into_issue().ok_or_else(|| GitHubError::Api {
            status: 500,
            message: "malformed issue".into(),
        })
    }

    /// Open or close an issue.
    pub async fn set_issue_state(&self, full_name: &str, number: i64, state: &str) -> Result<GitHubIssue, GitHubError> {
        let raw: ApiIssue = self
            .send(
                reqwest::Method::PATCH,
                &format!("/repos/{full_name}/issues/{number}"),
                &serde_json::json!({ "state": state }),
            )
            .await?;
        raw.into_issue().ok_or_else(|| GitHubError::Api {
            status: 500,
            message: "malformed issue".into(),
        })
    }

    async fn send<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, GitHubError> {
        let response = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header(USER_AGENT, "ProjectHub/0.1")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .json(body)
            .send()
            .await?;
        Self::decode(response).await
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
        let url = format!("{}{path}", self.base);
        let response = self
            .http
            .get(&url)
            .header(USER_AGENT, "ProjectHub/0.1")
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await?;
        Self::decode(response).await
    }

    async fn decode<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T, GitHubError> {
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

#[derive(Debug, Deserialize)]
struct ApiIssue {
    number: i64,
    title: String,
    state: String,
    html_url: String,
    body: Option<String>,
    #[serde(default)]
    labels: Vec<ApiLabel>,
    updated_at: String,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ApiLabel {
    name: String,
}

impl ApiIssue {
    fn into_issue(self) -> Option<GitHubIssue> {
        Some(GitHubIssue {
            number: self.number,
            title: self.title,
            state: self.state,
            html_url: self.html_url,
            body: self.body,
            labels: self.labels.into_iter().map(|l| l.name).collect(),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at).ok()?.with_timezone(&Utc),
            is_pull_request: self.pull_request.is_some(),
        })
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
