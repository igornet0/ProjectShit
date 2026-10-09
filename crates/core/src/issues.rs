//! Tasks ↔ GitHub issues, plus task links for external tools (BoardDo).

use chrono::Utc;
use project_hub_database::{
    GitHubRepoRepository, ProjectRepository, SettingsRepository, TaskLinkRepository, TaskRepository,
    GITHUB_TOKEN_KEY,
};
use project_hub_domain::{Project, ProjectId, Task, TaskId, TaskLink, TaskStatus};
use project_hub_github::{parse_github_full_name, remote_urls_for_path, GitHubClient, GitHubIssue};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{AppState, ServiceError, TaskService};

/// A task with its external link (if any).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithLink {
    #[serde(flatten)]
    pub task: Task,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<TaskLink>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueSyncReport {
    pub checked: usize,
    pub tasks_completed: usize,
    pub tasks_reopened: usize,
    pub issues_closed: usize,
    pub errors: Vec<String>,
}

pub struct TaskLinkService;

impl TaskLinkService {
    pub async fn attach(
        state: &AppState,
        task_id: &TaskId,
        source: Option<String>,
        external_ref: Option<String>,
    ) -> Result<Option<TaskLink>, ServiceError> {
        if source.is_none() && external_ref.is_none() {
            return Ok(TaskLinkRepository::get(state.db.pool(), task_id).await?);
        }
        let mut link = TaskLinkRepository::get(state.db.pool(), task_id)
            .await?
            .unwrap_or_else(|| TaskLink::new(*task_id));
        if source.is_some() {
            link.source = source;
        }
        if external_ref.is_some() {
            link.external_ref = external_ref;
        }
        TaskLinkRepository::upsert(state.db.pool(), &link).await?;
        Ok(Some(link))
    }

    pub async fn with_links(state: &AppState, tasks: Vec<Task>) -> Result<Vec<TaskWithLink>, ServiceError> {
        let links = TaskLinkRepository::list(state.db.pool()).await?;
        Ok(tasks
            .into_iter()
            .map(|task| {
                let link = links.iter().find(|l| l.task_id == task.id).cloned();
                TaskWithLink { task, link }
            })
            .collect())
    }

    pub async fn get(state: &AppState, task_id: &TaskId) -> Result<TaskWithLink, ServiceError> {
        let task = TaskRepository::get(state.db.pool(), task_id).await?;
        let link = TaskLinkRepository::get(state.db.pool(), task_id).await?;
        Ok(TaskWithLink { task, link })
    }
}

pub struct IssueService;

impl IssueService {
    async fn client(state: &AppState) -> Result<GitHubClient, ServiceError> {
        let token = SettingsRepository::get(state.db.pool(), GITHUB_TOKEN_KEY)
            .await?
            .filter(|t| !t.trim().is_empty())
            .ok_or(project_hub_github::GitHubError::NotConnected)?;
        Ok(GitHubClient::new(token))
    }

    /// `owner/repo` of the project: stored GitHub link, then the `origin` remote.
    pub async fn repo_full_name(state: &AppState, project: &Project) -> Result<String, ServiceError> {
        if let Some(id) = project.github_repo_id {
            if let Some(repo) = GitHubRepoRepository::list(state.db.pool())
                .await?
                .into_iter()
                .find(|r| r.id == id)
            {
                return Ok(repo.full_name);
            }
        }
        if let Some(name) = project.remote_url.as_deref().and_then(parse_github_full_name) {
            return Ok(name);
        }
        let root = project.root_path.clone();
        let remotes = tokio::task::spawn_blocking(move || remote_urls_for_path(&root)).await?;
        remotes
            .iter()
            .find_map(|url| parse_github_full_name(url))
            .ok_or_else(|| ServiceError::Invalid(format!("project `{}` has no GitHub remote", project.name)))
    }

    pub async fn list_for_project(state: &AppState, id: &ProjectId, issue_state: &str, limit: usize) -> Result<Vec<GitHubIssue>, ServiceError> {
        let project = ProjectRepository::get(state.db.pool(), id).await?;
        let repo = Self::repo_full_name(state, &project).await?;
        Ok(Self::client(state).await?.list_issues(&repo, issue_state, limit).await?)
    }

    /// Open a GitHub issue for the task in its project's repository.
    pub async fn create_for_task(
        state: &AppState,
        task_id: &TaskId,
        labels: Vec<String>,
    ) -> Result<TaskWithLink, ServiceError> {
        let task = TaskRepository::get(state.db.pool(), task_id).await?;
        let mut link = TaskLinkRepository::get(state.db.pool(), task_id)
            .await?
            .unwrap_or_else(|| TaskLink::new(*task_id));
        if link.github_issue_number.is_some() {
            return Ok(TaskWithLink { task, link: Some(link) });
        }
        let project_id = task
            .project_id
            .ok_or_else(|| ServiceError::Invalid("personal tasks have no repository".into()))?;
        let project = ProjectRepository::get(state.db.pool(), &project_id).await?;
        let repo = Self::repo_full_name(state, &project).await?;

        let mut body = task.description.clone().unwrap_or_default();
        body.push_str(&format!(
            "\n\n---\n_Created from Project Hub task `{}` ({}, priority {})._",
            task.id,
            task.status.as_str(),
            task.priority.as_str()
        ));
        let issue = Self::client(state)
            .await?
            .create_issue(&repo, &task.title, Some(body.trim()), &labels)
            .await?;

        link.github_repo = Some(repo);
        link.github_issue_number = Some(issue.number);
        link.github_issue_url = Some(issue.html_url.clone());
        link.github_issue_state = Some(issue.state.clone());
        link.synced_at = Some(Utc::now());
        TaskLinkRepository::upsert(state.db.pool(), &link).await?;
        info!(task = %task.id, issue = issue.number, "github.issue_created");
        Ok(TaskWithLink { task, link: Some(link) })
    }

    /// Two-way sync of linked issues:
    /// issue closed → task done; issue reopened → done task back to todo;
    /// task done/cancelled while the issue is open → close the issue.
    pub async fn sync(state: &AppState) -> Result<IssueSyncReport, ServiceError> {
        let links = TaskLinkRepository::list_with_issues(state.db.pool()).await?;
        let mut report = IssueSyncReport::default();
        if links.is_empty() {
            return Ok(report);
        }
        let client = Self::client(state).await?;
        for mut link in links {
            let (Some(repo), Some(number)) = (link.github_repo.clone(), link.github_issue_number) else {
                continue;
            };
            report.checked += 1;
            let task = match TaskRepository::get(state.db.pool(), &link.task_id).await {
                Ok(t) => t,
                Err(err) => {
                    report.errors.push(format!("task {}: {err}", link.task_id));
                    continue;
                }
            };
            let mut issue = match client.get_issue(&repo, number).await {
                Ok(i) => i,
                Err(err) => {
                    report.errors.push(format!("{repo}#{number}: {err}"));
                    continue;
                }
            };
            let task_finished = matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled);
            let previous = link.github_issue_state.clone();

            if issue.state == "closed" && !task_finished {
                let mut updated = task.clone();
                updated.status = TaskStatus::Done;
                updated.updated_at = Utc::now();
                TaskService::update(state, updated).await?;
                report.tasks_completed += 1;
            } else if issue.state == "open" && task.status == TaskStatus::Done && previous.as_deref() == Some("closed") {
                // Reopened on GitHub after we saw it closed.
                let mut updated = task.clone();
                updated.status = TaskStatus::Todo;
                updated.updated_at = Utc::now();
                TaskService::update(state, updated).await?;
                report.tasks_reopened += 1;
            } else if issue.state == "open" && task_finished {
                match client.set_issue_state(&repo, number, "closed").await {
                    Ok(i) => {
                        issue = i;
                        report.issues_closed += 1;
                    }
                    Err(err) => report.errors.push(format!("close {repo}#{number}: {err}")),
                }
            }
            link.github_issue_state = Some(issue.state);
            link.synced_at = Some(Utc::now());
            TaskLinkRepository::upsert(state.db.pool(), &link).await?;
        }
        info!(?report, "github.issues_synced");
        Ok(report)
    }
}
