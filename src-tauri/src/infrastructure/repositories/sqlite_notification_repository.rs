use async_trait::async_trait;
use rusqlite::{params, Connection, Result as SqliteResult};

use std::path::Path;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::{
    entities::{
        Notification, NotificationMetadata, NotificationPriority, NotificationSource,
        NotificationStatus,
    },
    error::{DomainError, DomainResult},
    repositories::NotificationRepository,
};

pub struct SqliteNotificationRepository {
    connection: Mutex<Connection>,
}

impl SqliteNotificationRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let connection = Connection::open(path)?;

        // Initialize the database schema
        connection.execute(
            "CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                priority TEXT NOT NULL,
                status TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                read_at TEXT,
                action_taken_at TEXT
            )",
            [],
        )?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn map_db_error(err: rusqlite::Error) -> DomainError {
        DomainError::InternalError(format!("Database error: {}", err))
    }
}

#[async_trait]
impl NotificationRepository for SqliteNotificationRepository {
    async fn save(&self, notification: &mut Notification) -> DomainResult<()> {
        let connection = self.connection.lock().await;

        let metadata_json = serde_json::to_string(&notification.metadata)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        connection
            .execute(
                "INSERT OR REPLACE INTO notifications (
                    id, title, content, priority, status, metadata,
                    created_at, updated_at, read_at, action_taken_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    notification.id.to_string(),
                    notification.title,
                    notification.content,
                    serde_json::to_string(&notification.priority).unwrap(),
                    serde_json::to_string(&notification.status).unwrap(),
                    metadata_json,
                    notification.created_at.to_rfc3339(),
                    notification.updated_at.to_rfc3339(),
                    notification.read_at.map(|dt| dt.to_rfc3339()),
                    notification.action_taken_at.map(|dt| dt.to_rfc3339()),
                ],
            )
            .map_err(Self::map_db_error)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<Notification>> {
        let connection = self.connection.lock().await;

        let result = connection.query_row(
            "SELECT * FROM notifications WHERE id = ?1",
            params![id.to_string()],
            |row| {
                let metadata_str: String = row.get(5)?;
                let metadata: NotificationMetadata =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let priority_str: String = row.get(3)?;
                let priority: NotificationPriority =
                    serde_json::from_str(&priority_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let status_str: String = row.get(4)?;
                let status: NotificationStatus =
                    serde_json::from_str(&status_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(Notification {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    priority,
                    status,
                    metadata,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    read_at: if let Ok(dt) = row.get::<_, Option<String>>(8) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                    action_taken_at: if let Ok(dt) = row.get::<_, Option<String>>(9) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                })
            },
        );

        match result {
            Ok(notification) => Ok(Some(notification)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::map_db_error(e)),
        }
    }

    async fn find_all(&self) -> DomainResult<Vec<Notification>> {
        let connection = self.connection.lock().await;
        let mut stmt = connection
            .prepare(
                "SELECT * FROM notifications WHERE status != 'Deleted' ORDER BY created_at DESC",
            )
            .map_err(Self::map_db_error)?;

        let notifications = stmt
            .query_map([], |row| {
                let metadata_str: String = row.get(5)?;
                let metadata: NotificationMetadata =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let priority_str: String = row.get(3)?;
                let priority: NotificationPriority =
                    serde_json::from_str(&priority_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let status_str: String = row.get(4)?;
                let status: NotificationStatus =
                    serde_json::from_str(&status_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(Notification {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    priority,
                    status,
                    metadata,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    read_at: if let Ok(dt) = row.get::<_, Option<String>>(8) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                    action_taken_at: if let Ok(dt) = row.get::<_, Option<String>>(9) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                })
            })
            .map_err(Self::map_db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::map_db_error)?;

        Ok(notifications)
    }

    async fn find_by_status(&self, status: NotificationStatus) -> DomainResult<Vec<Notification>> {
        let connection = self.connection.lock().await;
        let mut stmt = connection
            .prepare("SELECT * FROM notifications WHERE status = json(?1) ORDER BY created_at DESC")
            .map_err(Self::map_db_error)?;

        let status_str = serde_json::to_string(&status)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        let notifications = stmt
            .query_map([&status_str], |row| {
                let metadata_str: String = row.get(5)?;
                let metadata: NotificationMetadata =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let priority_str: String = row.get(3)?;
                let priority: NotificationPriority =
                    serde_json::from_str(&priority_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(Notification {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    priority,
                    status: status.clone(),
                    metadata,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    read_at: if let Ok(dt) = row.get::<_, Option<String>>(8) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                    action_taken_at: if let Ok(dt) = row.get::<_, Option<String>>(9) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                })
            })
            .map_err(Self::map_db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::map_db_error)?;

        Ok(notifications)
    }

    async fn find_by_source(&self, source: NotificationSource) -> DomainResult<Vec<Notification>> {
        let connection = self.connection.lock().await;
        let mut stmt = connection
            .prepare("SELECT * FROM notifications WHERE json_extract(metadata, '$.source') = ?1 ORDER BY created_at DESC")
            .map_err(Self::map_db_error)?;

        let source_str = serde_json::to_string(&source)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        let notifications = stmt
            .query_map([source_str], |row| {
                let metadata_str: String = row.get(5)?;
                let metadata: NotificationMetadata =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let priority_str: String = row.get(3)?;
                let priority: NotificationPriority =
                    serde_json::from_str(&priority_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let status_str: String = row.get(4)?;
                let status: NotificationStatus =
                    serde_json::from_str(&status_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(Notification {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    priority,
                    status,
                    metadata,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    read_at: if let Ok(dt) = row.get::<_, Option<String>>(8) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                    action_taken_at: if let Ok(dt) = row.get::<_, Option<String>>(9) {
                        dt.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()?
                        .map(|dt| dt.into())
                    } else {
                        None
                    },
                })
            })
            .map_err(Self::map_db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::map_db_error)?;

        Ok(notifications)
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let connection = self.connection.lock().await;
        connection
            .execute(
                "DELETE FROM notifications WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(Self::map_db_error)?;
        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> DomainResult<()> {
        let mut notification = self.find_by_id(id).await?.ok_or_else(|| {
            DomainError::NotFoundError(format!("Notification with id {} not found", id))
        })?;

        notification.status = status;
        notification.updated_at = chrono::Utc::now();

        self.save(&mut notification).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_sqlite_repository() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repository = SqliteNotificationRepository::new(db_path).unwrap();

        // Test creating a notification
        let metadata = NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test123".to_string()),
            url: Some("https://example.com".to_string()),
            tags: vec!["test".to_string()],
            custom_data: None,
        };

        let mut notification = Notification::new(
            "Test Title".to_string(),
            "Test Content".to_string(),
            NotificationPriority::Medium,
            metadata,
        );

        // Test save
        repository.save(&mut notification).await.unwrap();

        // Test find_by_id
        let found = repository
            .find_by_id(notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.title, "Test Title");
        assert_eq!(found.content, "Test Content");

        // Test update_status
        repository
            .update_status(notification.id, NotificationStatus::Read)
            .await
            .unwrap();
        let updated = repository
            .find_by_id(notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, NotificationStatus::Read);

        // Test find_by_status
        let read_notifications = repository
            .find_by_status(NotificationStatus::Read)
            .await
            .unwrap();
        assert_eq!(read_notifications.len(), 1);
        assert_eq!(read_notifications[0].id, notification.id);

        // Test find_by_source
        let email_notifications = repository
            .find_by_source(NotificationSource::Email)
            .await
            .unwrap();
        assert_eq!(email_notifications.len(), 1);
        assert_eq!(email_notifications[0].id, notification.id);

        // Test delete
        repository.delete(notification.id).await.unwrap();
        let not_found = repository.find_by_id(notification.id).await.unwrap();
        assert!(not_found.is_none());
    }
}
