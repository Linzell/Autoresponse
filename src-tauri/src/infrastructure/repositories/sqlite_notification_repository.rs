use crate::domain::{
    entities::{Notification, NotificationMetadata, NotificationSource, NotificationStatus},
    error::DomainError,
    repositories::NotificationRepository,
};
use crate::infrastructure::repositories::{
    cached_repository::{CachedRepository, Repository},
    sqlite_base::SqliteRepository,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug)]
pub struct SqliteNotificationRepository {
    connection: Arc<Mutex<Connection>>,
}

#[derive(Debug)]
pub struct CachedSqliteNotificationRepository {
    inner: CachedRepository<Notification, SqliteNotificationRepository>,
    base_repo: Arc<SqliteNotificationRepository>,
}

impl CachedSqliteNotificationRepository {
    pub fn new(repository: SqliteNotificationRepository, max_capacity: u64, ttl: Duration) -> Self {
        let base_repo = Arc::new(repository);
        Self {
            inner: CachedRepository::new(Arc::clone(&base_repo), max_capacity, ttl),
            base_repo,
        }
    }
}

#[async_trait]
impl NotificationRepository for CachedSqliteNotificationRepository {
    async fn save(&self, notification: &mut Notification) -> Result<(), DomainError> {
        let result = NotificationRepository::save(&*self.base_repo, notification).await;
        if result.is_ok() {
            self.inner.invalidate(notification.id).await;
        }
        result
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError> {
        self.inner.find_by_id(id).await
    }

    async fn find_all(&self) -> Result<Vec<Notification>, DomainError> {
        NotificationRepository::find_all(&*self.base_repo).await
    }

    async fn find_by_status(
        &self,
        status: NotificationStatus,
    ) -> Result<Vec<Notification>, DomainError> {
        self.base_repo.find_by_status(status).await
    }

    async fn find_by_source(
        &self,
        source: NotificationSource,
    ) -> Result<Vec<Notification>, DomainError> {
        self.base_repo.find_by_source(source).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let result = NotificationRepository::delete(&*self.base_repo, id).await;
        if result.is_ok() {
            self.inner.invalidate(id).await;
        }
        result
    }

    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> Result<(), DomainError> {
        let result = self.base_repo.update_status(id, status).await;
        if result.is_ok() {
            self.inner.invalidate(id).await;
        }
        result
    }
}

#[async_trait]
impl Repository<Notification> for SqliteNotificationRepository {
    async fn save(&self, entity: &mut Notification) -> Result<(), DomainError> {
        <Self as SqliteRepository<Notification>>::save(self, entity).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError> {
        <Self as SqliteRepository<Notification>>::find_by_id(self, id).await
    }

    async fn find_all(&self) -> Result<Vec<Notification>, DomainError> {
        <Self as SqliteRepository<Notification>>::find_all(self).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        <Self as SqliteRepository<Notification>>::delete(self, id).await
    }
}

impl SqliteNotificationRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DomainError> {
        let connection = Connection::open(path)
            .map_err(|e| DomainError::InternalError(format!("Failed to open database: {}", e)))?;

        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS notifications (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    priority TEXT NOT NULL,
                    status TEXT NOT NULL,
                    source TEXT NOT NULL,
                    external_id TEXT,
                    url TEXT,
                    tags TEXT,
                    custom_data TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    read_at TEXT,
                    action_taken_at TEXT
                )",
                [],
            )
            .map_err(|e| DomainError::InternalError(format!("Failed to create table: {}", e)))?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }
}

impl SqliteRepository<Notification> for SqliteNotificationRepository {
    fn table_name(&self) -> &str {
        "notifications"
    }

    fn column_names(&self) -> Vec<&str> {
        vec![
            "id",
            "title",
            "content",
            "priority",
            "status",
            "source",
            "external_id",
            "url",
            "tags",
            "custom_data",
            "created_at",
            "updated_at",
            "read_at",
            "action_taken_at",
        ]
    }

    fn connection(&self) -> &Arc<Mutex<Connection>> {
        &self.connection
    }

    fn map_row(&self, row: &Row) -> rusqlite::Result<Notification> {
        let tags_str: String = row.get("tags")?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

        let custom_data: Option<Value> = row
            .get::<_, Option<String>>("custom_data")?
            .and_then(|s| serde_json::from_str(&s).ok());

        Ok(Notification {
            id: Uuid::parse_str(&row.get::<_, String>("id")?).unwrap(),
            title: row.get("title")?,
            content: row.get("content")?,
            priority: serde_json::from_str(&row.get::<_, String>("priority")?).unwrap(),
            status: serde_json::from_str(&row.get::<_, String>("status")?).unwrap(),
            metadata: NotificationMetadata {
                source: serde_json::from_str(&row.get::<_, String>("source")?).unwrap(),
                external_id: row.get("external_id")?,
                url: row.get("url")?,
                tags,
                custom_data,
            },
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("updated_at")?)
                .unwrap()
                .with_timezone(&Utc),
            read_at: row.get::<_, Option<String>>("read_at")?.map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
            action_taken_at: row.get::<_, Option<String>>("action_taken_at")?.map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
        })
    }

    fn map_entity_to_params(
        &self,
        notification: &Notification,
    ) -> Vec<Box<dyn rusqlite::ToSql + Send>> {
        vec![
            Box::new(notification.id.to_string()),
            Box::new(notification.title.clone()),
            Box::new(notification.content.clone()),
            Box::new(serde_json::to_string(&notification.priority).unwrap()),
            Box::new(serde_json::to_string(&notification.status).unwrap()),
            Box::new(serde_json::to_string(&notification.metadata.source).unwrap()),
            Box::new(notification.metadata.external_id.clone()),
            Box::new(notification.metadata.url.clone()),
            Box::new(serde_json::to_string(&notification.metadata.tags).unwrap()),
            Box::new(
                notification
                    .metadata
                    .custom_data
                    .as_ref()
                    .map(|d| serde_json::to_string(d).unwrap()),
            ),
            Box::new(notification.created_at.to_rfc3339()),
            Box::new(notification.updated_at.to_rfc3339()),
            Box::new(notification.read_at.map(|dt| dt.to_rfc3339())),
            Box::new(notification.action_taken_at.map(|dt| dt.to_rfc3339())),
        ]
    }
}

#[async_trait]
impl NotificationRepository for SqliteNotificationRepository {
    async fn save(&self, notification: &mut Notification) -> Result<(), DomainError> {
        <Self as SqliteRepository<Notification>>::save(self, notification).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError> {
        <Self as SqliteRepository<Notification>>::find_by_id(self, id).await
    }

    async fn find_all(&self) -> Result<Vec<Notification>, DomainError> {
        <Self as SqliteRepository<Notification>>::find_all(self).await
    }

    async fn find_by_status(
        &self,
        status: NotificationStatus,
    ) -> Result<Vec<Notification>, DomainError> {
        let conn = self.connection().lock().await;
        let query = format!("SELECT * FROM {} WHERE status = ?", self.table_name());
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![serde_json::to_string(&status)?], |row| {
            self.map_row(row)
        })?;

        let mut notifications = Vec::new();
        for notification in rows {
            notifications.push(notification?);
        }
        Ok(notifications)
    }

    async fn find_by_source(
        &self,
        source: NotificationSource,
    ) -> Result<Vec<Notification>, DomainError> {
        let conn = self.connection().lock().await;
        let query = format!("SELECT * FROM {} WHERE source = ?", self.table_name());
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![serde_json::to_string(&source)?], |row| {
            self.map_row(row)
        })?;

        let mut notifications = Vec::new();
        for notification in rows {
            notifications.push(notification?);
        }
        Ok(notifications)
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        <Self as SqliteRepository<Notification>>::delete(self, id).await
    }

    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> Result<(), DomainError> {
        let conn = self.connection().lock().await;
        let query = format!(
            "UPDATE {} SET status = ?, updated_at = ? WHERE id = ?",
            self.table_name()
        );
        conn.execute(
            &query,
            params![
                serde_json::to_string(&status)?,
                Utc::now().to_rfc3339(),
                id.to_string()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::NotificationPriority;
    use std::time::Duration;

    use super::*;

    async fn create_test_notification() -> Notification {
        Notification {
            id: Uuid::new_v4(),
            title: "Test Notification".to_string(),
            content: "Test Content".to_string(),
            priority: NotificationPriority::Medium,
            status: NotificationStatus::New,
            metadata: NotificationMetadata {
                source: NotificationSource::Email,
                external_id: Some("test-123".to_string()),
                url: Some("https://example.com".to_string()),
                tags: vec!["test".to_string()],
                custom_data: Some(serde_json::json!({ "key": "value" })),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            read_at: None,
            action_taken_at: None,
        }
    }

    #[tokio::test]
    async fn test_cached_repository() {
        let base_repo = SqliteNotificationRepository::new(":memory:").unwrap();
        let repo = CachedSqliteNotificationRepository::new(base_repo, 100, Duration::from_secs(30));
        let mut notification = create_test_notification().await;

        // Test save and cache invalidation
        NotificationRepository::save(&repo, &mut notification)
            .await
            .unwrap();

        // Test find by id (should use cache)
        let found = NotificationRepository::find_by_id(&repo, notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.title, notification.title);
        assert_eq!(found.content, notification.content);

        // Test update status with cache invalidation
        repo.update_status(notification.id, NotificationStatus::Read)
            .await
            .unwrap();
        let updated = NotificationRepository::find_by_id(&repo, notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, NotificationStatus::Read);

        // Test delete with cache invalidation
        NotificationRepository::delete(&repo, notification.id)
            .await
            .unwrap();
        let deleted = NotificationRepository::find_by_id(&repo, notification.id)
            .await
            .unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_repository() {
        let repo = SqliteNotificationRepository::new(":memory:").unwrap();
        let mut notification = create_test_notification().await;

        // Test save
        NotificationRepository::save(&repo, &mut notification)
            .await
            .unwrap();

        // Test find by id
        let found = NotificationRepository::find_by_id(&repo, notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.title, notification.title);
        assert_eq!(found.content, notification.content);

        // Test find by status
        let unread = repo.find_by_status(NotificationStatus::New).await.unwrap();
        assert_eq!(unread.len(), 1);
        assert_eq!(unread[0].id, notification.id);

        // Test find by source
        let email = repo
            .find_by_source(NotificationSource::Email)
            .await
            .unwrap();
        assert_eq!(email.len(), 1);
        assert_eq!(email[0].id, notification.id);

        // Test update status
        repo.update_status(notification.id, NotificationStatus::Read)
            .await
            .unwrap();
        let updated = NotificationRepository::find_by_id(&repo, notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, NotificationStatus::Read);

        // Test delete
        NotificationRepository::delete(&repo, notification.id)
            .await
            .unwrap();
        let deleted = NotificationRepository::find_by_id(&repo, notification.id)
            .await
            .unwrap();
        assert!(deleted.is_none());
    }
}
