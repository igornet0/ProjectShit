use std::path::Path;

use git2::Repository;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("not a git repository: {0}")]
    NotARepo(String),

    #[error("git error: {0}")]
    Git2(#[from] git2::Error),
}

pub struct GitRepository {
    repo: Repository,
    path: std::path::PathBuf,
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        if !path.join(".git").exists() {
            return Err(GitError::NotARepo(path.display().to_string()));
        }
        let repo = Repository::open(path)?;
        Ok(Self {
            repo,
            path: path.to_path_buf(),
        })
    }

    pub fn repo(&self) -> &Repository {
        &self.repo
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
