use async_trait::async_trait;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::{
    entities::{AuthConfig, AuthType, ServiceConfig, ServiceEndpoints, ServiceType},
    error::{DomainError, DomainResult},
    repositories::ServiceConfigRepository,
};

pub struct SqliteServiceConfigRepository {
    connection: Mutex<Connection>,
}

impl SqliteServiceConfigRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let connection = Connection::open(path)?;

        // Initialize the database schema
        connection.execute(
            "CREATE TABLE IF NOT EXISTS service_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                service_type TEXT NOT NULL,
                auth_type TEXT NOT NULL,
                auth_config TEXT NOT NULL,
                endpoints TEXT NOT NULL,
                enabled BOOLEAN NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_sync TEXT,
                metadata TEXT NOT NULL
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
impl ServiceConfigRepository for SqliteServiceConfigRepository {
    async fn save(&self, config: &mut ServiceConfig) -> DomainResult<()> {
        let connection = self.connection.lock().await;

        let auth_config_json = serde_json::to_string(&config.auth_config)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        let endpoints_json = serde_json::to_string(&config.endpoints)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        let service_type_json = serde_json::to_string(&config.service_type)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        let auth_type_json = serde_json::to_string(&config.auth_type)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        let metadata_json = serde_json::to_string(&config.metadata)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        connection
            .execute(
                "INSERT OR REPLACE INTO service_configs (
                    id, name, service_type, auth_type, auth_config, endpoints,
                    enabled, created_at, updated_at, last_sync, metadata
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    config.id.to_string(),
                    config.name,
                    service_type_json,
                    auth_type_json,
                    auth_config_json,
                    endpoints_json,
                    config.enabled,
                    config.created_at.to_rfc3339(),
                    config.updated_at.to_rfc3339(),
                    config.last_sync.map(|dt| dt.to_rfc3339()),
                    metadata_json,
                ],
            )
            .map_err(Self::map_db_error)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<ServiceConfig>> {
        let connection = self.connection.lock().await;

        let result = connection.query_row(
            "SELECT * FROM service_configs WHERE id = ?1",
            params![id.to_string()],
            |row| {
                let service_type_str: String = row.get(2)?;
                let auth_type_str: String = row.get(3)?;
                let auth_config_str: String = row.get(4)?;
                let endpoints_str: String = row.get(5)?;
                let metadata_str: String = row.get(10)?;

                let service_type: ServiceType =
                    serde_json::from_str(&service_type_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let auth_type: AuthType = serde_json::from_str(&auth_type_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let auth_config: AuthConfig =
                    serde_json::from_str(&auth_config_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let endpoints: ServiceEndpoints =
                    serde_json::from_str(&endpoints_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let metadata: serde_json::Value =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(ServiceConfig {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    name: row.get(1)?,
                    service_type,
                    auth_type,
                    auth_config,
                    endpoints,
                    enabled: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    last_sync: if let Ok(dt) = row.get::<_, Option<String>>(9) {
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
                    metadata,
                })
            },
        );

        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::map_db_error(e)),
        }
    }

    async fn find_all(&self) -> DomainResult<Vec<ServiceConfig>> {
        let connection = self.connection.lock().await;
        let mut stmt = connection
            .prepare("SELECT * FROM service_configs ORDER BY created_at DESC")
            .map_err(Self::map_db_error)?;

        let configs = stmt
            .query_map([], |row| {
                let service_type_str: String = row.get(2)?;
                let auth_type_str: String = row.get(3)?;
                let auth_config_str: String = row.get(4)?;
                let endpoints_str: String = row.get(5)?;
                let metadata_str: String = row.get(10)?;

                let service_type: ServiceType =
                    serde_json::from_str(&service_type_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let auth_type: AuthType = serde_json::from_str(&auth_type_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let auth_config: AuthConfig =
                    serde_json::from_str(&auth_config_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let endpoints: ServiceEndpoints =
                    serde_json::from_str(&endpoints_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let metadata: serde_json::Value =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(ServiceConfig {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    name: row.get(1)?,
                    service_type,
                    auth_type,
                    auth_config,
                    endpoints,
                    enabled: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    last_sync: if let Ok(dt) = row.get::<_, Option<String>>(9) {
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
                    metadata,
                })
            })
            .map_err(Self::map_db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::map_db_error)?;

        Ok(configs)
    }

    async fn find_by_service_type(
        &self,
        service_type: ServiceType,
    ) -> DomainResult<Vec<ServiceConfig>> {
        let connection = self.connection.lock().await;
        let service_type_str = serde_json::to_string(&service_type)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        let mut stmt = connection
            .prepare(
                "SELECT * FROM service_configs WHERE service_type = ?1 ORDER BY created_at DESC",
            )
            .map_err(Self::map_db_error)?;

        let configs = stmt
            .query_map([service_type_str], |row| {
                let auth_type_str: String = row.get(3)?;
                let auth_config_str: String = row.get(4)?;
                let endpoints_str: String = row.get(5)?;
                let metadata_str: String = row.get(10)?;

                let auth_type: AuthType = serde_json::from_str(&auth_type_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let auth_config: AuthConfig =
                    serde_json::from_str(&auth_config_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let endpoints: ServiceEndpoints =
                    serde_json::from_str(&endpoints_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let metadata: serde_json::Value =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(ServiceConfig {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    name: row.get(1)?,
                    service_type: service_type.clone(),
                    auth_type,
                    auth_config,
                    endpoints,
                    enabled: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    last_sync: if let Ok(dt) = row.get::<_, Option<String>>(9) {
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
                    metadata,
                })
            })
            .map_err(Self::map_db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::map_db_error)?;

        Ok(configs)
    }

    async fn find_enabled(&self) -> DomainResult<Vec<ServiceConfig>> {
        let connection = self.connection.lock().await;
        let mut stmt = connection
            .prepare("SELECT * FROM service_configs WHERE enabled = TRUE ORDER BY created_at DESC")
            .map_err(Self::map_db_error)?;

        let configs = stmt
            .query_map([], |row| {
                let service_type_str: String = row.get(2)?;
                let auth_type_str: String = row.get(3)?;
                let auth_config_str: String = row.get(4)?;
                let endpoints_str: String = row.get(5)?;
                let metadata_str: String = row.get(10)?;

                let service_type: ServiceType =
                    serde_json::from_str(&service_type_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let auth_type: AuthType = serde_json::from_str(&auth_type_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let auth_config: AuthConfig =
                    serde_json::from_str(&auth_config_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let endpoints: ServiceEndpoints =
                    serde_json::from_str(&endpoints_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let metadata: serde_json::Value =
                    serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                Ok(ServiceConfig {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    name: row.get(1)?,
                    service_type,
                    auth_type,
                    auth_config,
                    endpoints,
                    enabled: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .into(),
                    last_sync: if let Ok(dt) = row.get::<_, Option<String>>(9) {
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
                    metadata,
                })
            })
            .map_err(Self::map_db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::map_db_error)?;

        Ok(configs)
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let connection = self.connection.lock().await;
        connection
            .execute(
                "DELETE FROM service_configs WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(Self::map_db_error)?;
        Ok(())
    }

    async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> DomainResult<()> {
        let connection = self.connection.lock().await;
        let auth_config_json = serde_json::to_string(&auth_config)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization error: {}", e)))?;

        connection
            .execute(
                "UPDATE service_configs SET auth_config = ?1, updated_at = ?2 WHERE id = ?3",
                params![
                    auth_config_json,
                    chrono::Utc::now().to_rfc3339(),
                    id.to_string(),
                ],
            )
            .map_err(Self::map_db_error)?;

        Ok(())
    }

    async fn update_enabled_status(&self, id: Uuid, enabled: bool) -> DomainResult<()> {
        let connection = self.connection.lock().await;
        connection
            .execute(
                "UPDATE service_configs SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![enabled, chrono::Utc::now().to_rfc3339(), id.to_string(),],
            )
            .map_err(Self::map_db_error)?;

        Ok(())
    }

    async fn update_last_sync(&self, id: Uuid) -> DomainResult<()> {
        let connection = self.connection.lock().await;
        let now = chrono::Utc::now();

        connection
            .execute(
                "UPDATE service_configs SET last_sync = ?1, updated_at = ?2 WHERE id = ?3",
                params![now.to_rfc3339(), now.to_rfc3339(), id.to_string(),],
            )
            .map_err(Self::map_db_error)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::OAuth2Config;
    use tempfile::tempdir;

    async fn create_test_config(repository: &SqliteServiceConfigRepository) -> ServiceConfig {
        let oauth2_config = OAuth2Config {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
            auth_url: "http://auth.example.com/oauth/authorize".to_string(),
            token_url: "http://auth.example.com/oauth/token".to_string(),
            scope: vec!["read".to_string(), "write".to_string()],
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        };

        let endpoints = ServiceEndpoints {
            base_url: "http://api.example.com".to_string(),
            endpoints: {
                let mut map = serde_json::Map::new();
                map.insert(
                    "test".to_string(),
                    serde_json::json!({
                        "path": "/test",
                        "method": "GET"
                    }),
                );
                map
            },
        };

        let mut config = ServiceConfig::new(
            "Test Service".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config),
            endpoints,
        );

        repository.save(&mut config).await.unwrap();
        config
    }

    #[tokio::test]
    async fn test_sqlite_repository() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repository = SqliteServiceConfigRepository::new(db_path).unwrap();

        // Test creating a service config
        let config = create_test_config(&repository).await;

        // Test find_by_id
        let found = repository.find_by_id(config.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Test Service");
        assert!(matches!(found.service_type, ServiceType::Github));

        // Test find_all
        let all_configs = repository.find_all().await.unwrap();
        assert_eq!(all_configs.len(), 1);

        // Test find_by_service_type
        let github_configs = repository
            .find_by_service_type(ServiceType::Github)
            .await
            .unwrap();
        assert_eq!(github_configs.len(), 1);
        assert_eq!(github_configs[0].id, config.id);

        // Test update_enabled_status
        repository
            .update_enabled_status(config.id, false)
            .await
            .unwrap();
        let updated = repository.find_by_id(config.id).await.unwrap().unwrap();
        assert!(!updated.enabled);

        // Test find_enabled
        let enabled_configs = repository.find_enabled().await.unwrap();
        assert_eq!(enabled_configs.len(), 0);

        // Test update_last_sync
        repository.update_last_sync(config.id).await.unwrap();
        let updated = repository.find_by_id(config.id).await.unwrap().unwrap();
        assert!(updated.last_sync.is_some());

        // Test delete
        repository.delete(config.id).await.unwrap();
        let not_found = repository.find_by_id(config.id).await.unwrap();
        assert!(not_found.is_none());
    }
}
