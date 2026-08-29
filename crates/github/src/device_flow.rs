use std::time::{Duration, Instant};

use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;

use crate::GitHubError;

#[derive(Debug, Clone)]
pub struct DeviceFlowSession {
    pub user_code: String,
    pub verification_uri: String,
    device_code: String,
    interval: u64,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn start_device_flow(client_id: &str) -> Result<DeviceFlowSession, GitHubError> {
    let http = reqwest::Client::new();
    let response = http
        .post("https://github.com/login/device/code")
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, "ProjectHub/0.1")
        .form(&[("client_id", client_id), ("scope", "read:user repo")])
        .send()
        .await?;

    let status = response.status();
    let body: DeviceCodeResponse = response.json().await.map_err(|e| GitHubError::Api {
        status: status.as_u16(),
        message: e.to_string(),
    })?;

    if let Some(error) = body.error {
        return Err(GitHubError::Api {
            status: 400,
            message: body
                .error_description
                .unwrap_or_else(|| error.clone()),
        });
    }

    Ok(DeviceFlowSession {
        user_code: body.user_code,
        verification_uri: body.verification_uri,
        device_code: body.device_code,
        interval: body.interval.max(5),
        expires_at: Instant::now() + Duration::from_secs(body.expires_in),
    })
}

pub async fn poll_access_token(
    client_id: &str,
    session: &DeviceFlowSession,
) -> Result<String, GitHubError> {
    let http = reqwest::Client::new();
    let mut interval = session.interval;

    loop {
        if Instant::now() >= session.expires_at {
            return Err(GitHubError::OAuthExpired);
        }

        tokio::time::sleep(Duration::from_secs(interval)).await;

        let response = http
            .post("https://github.com/login/oauth/access_token")
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, "ProjectHub/0.1")
            .form(&[
                ("client_id", client_id),
                ("device_code", &session.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;

        let status = response.status();
        let body: TokenResponse = response.json().await.map_err(|e| GitHubError::Api {
            status: status.as_u16(),
            message: e.to_string(),
        })?;

        if let Some(token) = body.access_token.filter(|t| !t.is_empty()) {
            return Ok(token);
        }

        match body.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval += 5;
                continue;
            }
            Some("expired_token") => return Err(GitHubError::OAuthExpired),
            Some("access_denied") => return Err(GitHubError::OAuthDenied),
            Some(error) => {
                return Err(GitHubError::Api {
                    status: 400,
                    message: body
                        .error_description
                        .unwrap_or_else(|| error.to_string()),
                });
            }
            None => continue,
        }
    }
}

pub fn verification_url(session: &DeviceFlowSession) -> String {
    format!(
        "{}?user_code={}",
        session.verification_uri, session.user_code
    )
}
