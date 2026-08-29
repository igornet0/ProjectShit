use std::path::{Path, PathBuf};

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("watch error: {0}")]
    Watch(String),
}

#[async_trait]
pub trait FileWatcher: Send + Sync {
    async fn watch(&self, path: &Path) -> Result<(), WatcherError>;
    async fn unwatch(&self, path: &Path) -> Result<(), WatcherError>;
}

/// No-op watcher for MVP — real implementation in v0.9.
pub struct NoopFileWatcher;

#[async_trait]
impl FileWatcher for NoopFileWatcher {
    async fn watch(&self, path: &Path) -> Result<(), WatcherError> {
        tracing::debug!(path = %path.display(), "noop watch registered");
        Ok(())
    }

    async fn unwatch(&self, path: &Path) -> Result<(), WatcherError> {
        tracing::debug!(path = %path.display(), "noop watch removed");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub path: PathBuf,
    pub kind: WatchEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    Created,
    Modified,
    Deleted,
}
