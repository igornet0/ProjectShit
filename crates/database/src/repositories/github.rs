use chrono::Utc;
use project_hub_domain::GitHubRepo;
use sqlx::SqlitePool;

use crate::error::DbError;

pub struct GitHubRepoRepository;

impl GitHubRepoRepository {
    pub async fn replace_all(pool: &SqlitePool, repos: &[GitHubRepo]) -> Result<(), DbError> {
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM github_repos_cache")
            .execute(&mut *tx)
            .await?;

        let synced_at = Utc::now().to_rfc3339();
        for repo in repos {
            sqlx::query(
                r#"
                INSERT INTO github_repos_cache
                (id, full_name, clone_url, ssh_url, html_url, description, private, default_branch, updated_at, synced_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(repo.id)
            .bind(&repo.full_name)
            .bind(&repo.clone_url)
            .bind(&repo.ssh_url)
            .bind(&repo.html_url)
            .bind(&repo.description)
            .bind(repo.private as i32)
            .bind(&repo.default_branch)
            .bind(repo.updated_at.to_rfc3339())
            .bind(&synced_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn list(pool: &SqlitePool) -> Result<Vec<GitHubRepo>, DbError> {
        let rows = sqlx::query_as::<_, GitHubRepoRow>(
            "SELECT id, full_name, clone_url, ssh_url, html_url, description, private, default_branch, updated_at FROM github_repos_cache ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(GitHubRepoRow::into_repo).collect()
    }
}

#[derive(sqlx::FromRow)]
struct GitHubRepoRow {
    id: i64,
    full_name: String,
    clone_url: String,
    ssh_url: Option<String>,
    html_url: String,
    description: Option<String>,
    private: i32,
    default_branch: Option<String>,
    updated_at: String,
}

impl GitHubRepoRow {
    fn into_repo(self) -> Result<GitHubRepo, DbError> {
        let updated_at = chrono::DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| DbError::Migration(e.to_string()))?;

        Ok(GitHubRepo {
            id: self.id,
            full_name: self.full_name,
            clone_url: self.clone_url,
            ssh_url: self.ssh_url,
            html_url: self.html_url,
            description: self.description,
            private: self.private != 0,
            default_branch: self.default_branch,
            updated_at,
        })
    }
}
