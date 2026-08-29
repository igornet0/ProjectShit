use sqlx::SqlitePool;

use crate::error::DbError;

pub const EDITOR_SETTINGS_KEY: &str = "editor_config";
pub const GITHUB_TOKEN_KEY: &str = "github_token";
pub const GITHUB_CONFIG_KEY: &str = "github_config";
pub const GITHUB_OAUTH_CLIENT_ID_KEY: &str = "github_oauth_client_id";

pub struct SettingsRepository;

impl SettingsRepository {
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, DbError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
                .bind(key)
                .fetch_optional(pool)
                .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
        Ok(())
    }
}
