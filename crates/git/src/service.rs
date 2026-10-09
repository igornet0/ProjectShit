use std::path::Path;

use crate::repository::GitRepository;
use crate::status::{GitCommitInfo, GitStatus};

pub struct GitService;

impl GitService {
    pub fn get_status(path: &Path) -> Result<GitStatus, crate::repository::GitError> {
        let repo = GitRepository::open(path)?;
        repo.status()
    }

    pub fn is_dirty(path: &Path) -> bool {
        GitRepository::open(path)
            .and_then(|repo| repo.is_dirty_fast())
            .unwrap_or(false)
    }

    pub fn has_git(path: &Path) -> bool {
        path.join(".git").exists()
    }

    pub fn get_last_commit(path: &Path) -> Result<Option<GitCommitInfo>, crate::repository::GitError> {
        let repo = GitRepository::open(path)?;
        repo.last_commit()
    }

    pub fn get_recent_commits(
        path: &Path,
        limit: usize,
    ) -> Result<Vec<GitCommitInfo>, crate::repository::GitError> {
        let repo = GitRepository::open(path)?;
        repo.recent_commits(limit)
    }
}
