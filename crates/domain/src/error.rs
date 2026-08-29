use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("validation error: {0}")]
    Validation(String),
}
