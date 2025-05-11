use crate::domain::error::{DomainError, DomainResult};
use rusqlite::{params, Connection, Error as SqliteError, Result as SqliteResult};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait SqliteRepository<T>
where
    T: Debug + Send + Sync,
    Self: Send + Sync,
{
    /// Name of the table for the entity
    fn table_name(&self) -> &str;

    /// Get column names for the table
    fn column_names(&self) -> Vec<&str>;

    /// Connection instance for database operations
    fn connection(&self) -> &Arc<Mutex<Connection>>;

    /// Maps a database row to the entity type
    fn map_row(&self, row: &rusqlite::Row) -> SqliteResult<T>;

    /// Maps an entity to SQL parameters for insert/update operations
    fn map_entity_to_params(&self, entity: &T) -> Vec<Box<dyn rusqlite::ToSql + Send>>;

    /// Default error mapping implementation
    fn map_db_error(&self, error: SqliteError) -> DomainError {
        DomainError::InternalError(format!("Database error: {}", error))
    }

    /// Generic save operation
    async fn save(&self, entity: &T) -> DomainResult<()> {
        let conn = self.connection().lock().await;
        let params = self.map_entity_to_params(entity);
        let columns = self.column_names();

        let param_placeholders = std::iter::repeat_n("?", columns.len())
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            self.table_name(),
            columns.join(", "),
            param_placeholders
        );

        conn.execute(&query, rusqlite::params_from_iter(params))
            .map_err(|e| self.map_db_error(e))?;

        Ok(())
    }

    /// Generic find by ID operation
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<T>> {
        let conn = self.connection().lock().await;
        let query = format!("SELECT * FROM {} WHERE id = ?", self.table_name());
        let mut stmt = conn.prepare(&query).map_err(|e| self.map_db_error(e))?;

        let rows = stmt
            .query_map(params![id.to_string()], |row| self.map_row(row))
            .map_err(|e| self.map_db_error(e))?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row.map_err(|e| self.map_db_error(e))?);
        }

        Ok(entities.into_iter().next())
    }

    /// Generic find all operation
    async fn find_all(&self) -> DomainResult<Vec<T>> {
        let conn = self.connection().lock().await;
        let query = format!("SELECT * FROM {}", self.table_name());
        let mut stmt = conn.prepare(&query).map_err(|e| self.map_db_error(e))?;

        let rows = stmt
            .query_map([], |row| self.map_row(row))
            .map_err(|e| self.map_db_error(e))?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row.map_err(|e| self.map_db_error(e))?);
        }

        Ok(entities)
    }

    /// Generic delete operation
    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let conn = self.connection().lock().await;
        let query = format!("DELETE FROM {} WHERE id = ?", self.table_name());
        conn.execute(&query, params![id.to_string()])
            .map_err(|e| self.map_db_error(e))?;

        Ok(())
    }

    /// Generic count operation
    async fn count(&self) -> DomainResult<i64> {
        let conn = self.connection().lock().await;
        let query = format!("SELECT COUNT(*) FROM {}", self.table_name());
        let count: i64 = conn
            .query_row(&query, [], |row| row.get(0))
            .map_err(|e| self.map_db_error(e))?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct TestEntity {
        id: Uuid,
        name: String,
    }

    struct TestRepository {
        connection: Arc<Mutex<Connection>>,
    }

    #[async_trait::async_trait]
    impl SqliteRepository<TestEntity> for TestRepository {
        fn table_name(&self) -> &str {
            "test_entities"
        }

        fn column_names(&self) -> Vec<&str> {
            vec!["id", "name"]
        }

        fn connection(&self) -> &Arc<Mutex<Connection>> {
            &self.connection
        }

        fn map_row(&self, row: &rusqlite::Row) -> SqliteResult<TestEntity> {
            Ok(TestEntity {
                id: Uuid::parse_str(row.get::<_, String>("id")?.as_str()).unwrap(),
                name: row.get("name")?,
            })
        }

        fn map_entity_to_params(
            &self,
            entity: &TestEntity,
        ) -> Vec<Box<dyn rusqlite::ToSql + Send>> {
            vec![
                Box::new(entity.id.to_string()),
                Box::new(entity.name.clone()),
            ]
        }
    }

    #[tokio::test]
    async fn test_sqlite_repository() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE test_entities (id TEXT PRIMARY KEY, name TEXT NOT NULL)",
            [],
        )
        .unwrap();

        let repo = TestRepository {
            connection: Arc::new(Mutex::new(conn)),
        };

        let test_entity = TestEntity {
            id: Uuid::new_v4(),
            name: "Test Entity".to_string(),
        };

        // Test save
        repo.save(&test_entity).await.unwrap();

        // Test find by id
        let found = repo.find_by_id(test_entity.id).await.unwrap().unwrap();
        assert_eq!(found.name, test_entity.name);

        // Test find all
        let all = repo.find_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, test_entity.name);

        // Test count
        let count = repo.count().await.unwrap();
        assert_eq!(count, 1);

        // Test delete
        repo.delete(test_entity.id).await.unwrap();
        let count = repo.count().await.unwrap();
        assert_eq!(count, 0);
    }
}
