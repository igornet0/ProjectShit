use chrono::{DateTime, Utc};
use project_hub_domain::{Folder, FolderId, ProjectId};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::DbError;

pub struct FolderRepository;

impl FolderRepository {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Folder>, DbError> {
        let rows = sqlx::query_as::<_, FolderRow>(
            r#"
            SELECT f.id, f.name, f.parent_id, f.sort_order, f.created_at,
                   (SELECT COUNT(*) FROM project_folder_assignments pfa WHERE pfa.folder_id = f.id) AS project_count
            FROM folders f
            ORDER BY f.sort_order, f.name
            "#,
        )
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(FolderRow::into_folder).collect()
    }

    pub async fn create(pool: &SqlitePool, folder: &Folder) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO folders (id, name, parent_id, sort_order, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(folder.id.to_string())
        .bind(&folder.name)
        .bind(folder.parent_id.map(|id| id.to_string()))
        .bind(folder.sort_order)
        .bind(folder.created_at.to_rfc3339())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn rename(pool: &SqlitePool, id: &FolderId, name: &str) -> Result<(), DbError> {
        let result = sqlx::query("UPDATE folders SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id.to_string())
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &FolderId) -> Result<(), DbError> {
        let result = sqlx::query("DELETE FROM folders WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub async fn next_sort_order(pool: &SqlitePool) -> Result<i32, DbError> {
        let row: (i64,) =
            sqlx::query_as("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM folders")
                .fetch_one(pool)
                .await?;
        Ok(row.0 as i32)
    }

    pub async fn reorder(pool: &SqlitePool, ids: &[FolderId]) -> Result<(), DbError> {
        for (index, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE folders SET sort_order = ? WHERE id = ?")
                .bind(index as i32)
                .bind(id.to_string())
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    pub async fn assign_project(
        pool: &SqlitePool,
        project_id: &ProjectId,
        folder_id: &FolderId,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO project_folder_assignments (project_id, folder_id, sort_order)
            VALUES (?, ?, 0)
            ON CONFLICT(project_id) DO UPDATE SET folder_id = excluded.folder_id
            "#,
        )
        .bind(project_id.to_string())
        .bind(folder_id.to_string())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn unassign_project(pool: &SqlitePool, project_id: &ProjectId) -> Result<(), DbError> {
        sqlx::query("DELETE FROM project_folder_assignments WHERE project_id = ?")
            .bind(project_id.to_string())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn folder_map(pool: &SqlitePool) -> Result<std::collections::HashMap<String, FolderId>, DbError> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT project_id, folder_id FROM project_folder_assignments",
        )
        .fetch_all(pool)
        .await?;

        let mut map = std::collections::HashMap::new();
        for (project_id, folder_id) in rows {
            let fid = Uuid::parse_str(&folder_id)
                .map(FolderId::from_uuid)
                .map_err(|e| DbError::Migration(e.to_string()))?;
            map.insert(project_id, fid);
        }
        Ok(map)
    }
}

#[derive(sqlx::FromRow)]
struct FolderRow {
    id: String,
    name: String,
    parent_id: Option<String>,
    sort_order: i32,
    created_at: String,
    project_count: i64,
}

impl FolderRow {
    fn into_folder(self) -> Result<Folder, DbError> {
        let id = Uuid::parse_str(&self.id)
            .map(FolderId::from_uuid)
            .map_err(|e| DbError::Migration(e.to_string()))?;
        let parent_id = self
            .parent_id
            .map(|s| {
                Uuid::parse_str(&s)
                    .map(FolderId::from_uuid)
                    .map_err(|e| DbError::Migration(e.to_string()))
            })
            .transpose()?;
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .or_else(|_| DateTime::parse_from_str(&self.created_at, "%+"))
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| DbError::Migration(e.to_string()))?;

        Ok(Folder {
            id,
            name: self.name,
            parent_id,
            sort_order: self.sort_order,
            created_at,
            project_count: self.project_count as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_db;
    use project_hub_domain::{Folder, FolderId};
    use tempfile::tempdir;

    #[tokio::test]
    async fn folder_roundtrip_list_and_json() {
        let dir = tempdir().unwrap();
        let db = init_db(&dir.path().join("test.db")).await.unwrap();
        let folder = Folder {
            id: FolderId::new(),
            name: "Work".into(),
            parent_id: None,
            sort_order: 0,
            created_at: chrono::Utc::now(),
            project_count: 0,
        };
        FolderRepository::create(db.pool(), &folder).await.unwrap();
        let list = FolderRepository::list(db.pool()).await.unwrap();
        assert_eq!(list.len(), 1);
        let json = serde_json::to_string(&list[0]).unwrap();
        assert!(json.contains("\"id\":\""));
    }

    #[tokio::test]
    async fn folder_reorder_persists() {
        let dir = tempdir().unwrap();
        let db = init_db(&dir.path().join("test.db")).await.unwrap();
        let a = Folder {
            id: FolderId::new(),
            name: "A".into(),
            parent_id: None,
            sort_order: 0,
            created_at: chrono::Utc::now(),
            project_count: 0,
        };
        let b = Folder {
            id: FolderId::new(),
            name: "B".into(),
            parent_id: None,
            sort_order: 1,
            created_at: chrono::Utc::now(),
            project_count: 0,
        };
        FolderRepository::create(db.pool(), &a).await.unwrap();
        FolderRepository::create(db.pool(), &b).await.unwrap();
        FolderRepository::reorder(db.pool(), &[b.id, a.id])
            .await
            .unwrap();
        let list = FolderRepository::list(db.pool()).await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "B");
        assert_eq!(list[1].name, "A");
    }
}
