use chrono::{DateTime, Utc};
use git2::{BranchType, StatusOptions};
use serde::{Deserialize, Serialize};

use crate::repository::{GitError, GitRepository};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub modified: usize,
    pub staged: usize,
    pub untracked: usize,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: DateTime<Utc>,
}

impl GitRepository {
    pub fn status(&self) -> Result<GitStatus, GitError> {
        let branch = self.current_branch()?;
        let (modified, staged, untracked) = self.count_changes()?;
        let (ahead, behind) = self.ahead_behind()?;

        Ok(GitStatus {
            branch,
            modified,
            staged,
            untracked,
            ahead,
            behind,
        })
    }

    /// Cheap dirty check for list views: no untracked-dir recursion, no
    /// submodules, no ahead/behind graph walk.
    pub fn is_dirty_fast(&self) -> Result<bool, GitError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(false)
            .exclude_submodules(true)
            .include_ignored(false);

        let statuses = self.repo().statuses(Some(&mut opts))?;
        Ok(!statuses.is_empty())
    }

    pub fn origin_url(&self) -> Option<String> {
        self.repo()
            .find_remote("origin")
            .ok()
            .and_then(|r| r.url().map(String::from))
    }

    pub fn last_commit(&self) -> Result<Option<GitCommitInfo>, GitError> {
        let head = match self.repo().head() {
            Ok(h) => h,
            Err(_) => return Ok(None),
        };

        let commit = head.peel_to_commit()?;
        Ok(Some(commit_to_info(&commit)))
    }

    pub fn recent_commits(&self, limit: usize) -> Result<Vec<GitCommitInfo>, GitError> {
        let mut revwalk = self.repo().revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo().find_commit(oid)?;
            commits.push(commit_to_info(&commit));
        }
        Ok(commits)
    }

    fn current_branch(&self) -> Result<String, GitError> {
        let head = self.repo().head()?;
        if head.is_branch() {
            Ok(head
                .shorthand()
                .unwrap_or("HEAD")
                .to_string())
        } else {
            Ok("detached".to_string())
        }
    }

    fn count_changes(&self) -> Result<(usize, usize, usize), GitError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = self.repo().statuses(Some(&mut opts))?;

        let mut modified = 0;
        let mut staged = 0;
        let mut untracked = 0;

        for entry in statuses.iter() {
            let s = entry.status();
            if s.is_wt_new() {
                untracked += 1;
            }
            if s.is_wt_modified() || s.is_wt_deleted() || s.is_wt_renamed() || s.is_wt_typechange() {
                modified += 1;
            }
            if s.is_index_new() || s.is_index_modified() || s.is_index_deleted() || s.is_index_renamed() {
                staged += 1;
            }
        }

        Ok((modified, staged, untracked))
    }

    fn ahead_behind(&self) -> Result<(usize, usize), GitError> {
        let head = match self.repo().head() {
            Ok(h) => h,
            Err(_) => return Ok((0, 0)),
        };

        let branch_name = match head.shorthand() {
            Some(n) => n,
            None => return Ok((0, 0)),
        };

        let branch = match self.repo().find_branch(branch_name, BranchType::Local) {
            Ok(b) => b,
            Err(_) => return Ok((0, 0)),
        };

        let upstream = match branch.upstream() {
            Ok(u) => u,
            Err(_) => return Ok((0, 0)),
        };

        let local_oid = match branch.get().target() {
            Some(oid) => oid,
            None => return Ok((0, 0)),
        };
        let upstream_oid = match upstream.get().target() {
            Some(oid) => oid,
            None => return Ok((0, 0)),
        };

        match self.repo().graph_ahead_behind(local_oid, upstream_oid) {
            Ok((ahead, behind)) => Ok((ahead, behind)),
            Err(_) => Ok((0, 0)),
        }
    }
}

pub(crate) fn commit_to_info(commit: &git2::Commit) -> GitCommitInfo {
    let time = commit.time();
    let date = DateTime::from_timestamp(time.seconds(), 0).unwrap_or_else(Utc::now);

    GitCommitInfo {
        hash: commit.id().to_string()[..7].to_string(),
        message: commit.summary().unwrap_or("").to_string(),
        author: commit.author().name().unwrap_or("unknown").to_string(),
        date,
    }
}
