use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("migration error: {0}")]
    Migration(String),
}
