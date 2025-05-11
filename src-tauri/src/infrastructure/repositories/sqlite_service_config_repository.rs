use crate::domain::{
    entities::{AuthConfig, ServiceConfig, ServiceType},
    error::{DomainError, DomainResult},
    repositories::ServiceConfigRepository,
};
use crate::infrastructure::repositories::sqlite_base::SqliteRepository;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use serde_json;
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct SqliteServiceConfigRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteServiceConfigRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DomainError> {
        let connection = Connection::open(path).map_err(|e| {
            DomainError::InternalError(format!("Failed to open database connection: {}", e))
        })?;

        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS service_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                service_type TEXT NOT NULL,
                auth_type TEXT NOT NULL,
                auth_config TEXT NOT NULL,
                endpoints TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_sync TEXT,
                metadata TEXT
            )",
                [],
            )
            .map_err(|e| DomainError::InternalError(format!("Failed to create table: {}", e)))?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }
}

impl SqliteRepository<ServiceConfig> for SqliteServiceConfigRepository {
    fn table_name(&self) -> &str {
        "service_configs"
    }

    fn column_names(&self) -> Vec<&str> {
        vec![
            "id",
            "name",
            "service_type",
            "auth_type",
            "auth_config",
            "endpoints",
            "enabled",
            "created_at",
            "updated_at",
            "last_sync",
            "metadata",
        ]
    }

    fn connection(&self) -> &Arc<Mutex<Connection>> {
        &self.connection
    }

    fn map_row(&self, row: &Row) -> rusqlite::Result<ServiceConfig> {
        Ok(ServiceConfig {
            id: Uuid::parse_str(&row.get::<_, String>("id")?).unwrap(),
            name: row.get("name")?,
            service_type: serde_json::from_str(&row.get::<_, String>("service_type")?).unwrap(),
            auth_type: serde_json::from_str(&row.get::<_, String>("auth_type")?).unwrap(),
            auth_config: serde_json::from_str(&row.get::<_, String>("auth_config")?).unwrap(),
            endpoints: serde_json::from_str(&row.get::<_, String>("endpoints")?).unwrap(),
            enabled: row.get("enabled")?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("created_at")?)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>("updated_at")?)
                .unwrap()
                .with_timezone(&Utc),
            last_sync: row.get::<_, Option<String>>("last_sync")?.map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
            metadata: serde_json::Value::Null,
        })
    }

    fn map_entity_to_params(&self, config: &ServiceConfig) -> Vec<Box<dyn rusqlite::ToSql + Send>> {
        vec![
            Box::new(config.id.to_string()),
            Box::new(config.name.clone()),
            Box::new(serde_json::to_string(&config.service_type).unwrap()),
            Box::new(serde_json::to_string(&config.auth_type).unwrap()),
            Box::new(serde_json::to_string(&config.auth_config).unwrap()),
            Box::new(serde_json::to_string(&config.endpoints).unwrap()),
            Box::new(config.enabled),
            Box::new(config.created_at.to_rfc3339()),
            Box::new(config.updated_at.to_rfc3339()),
            Box::new(config.last_sync.map(|dt| dt.to_rfc3339())),
            Box::new(serde_json::to_string(&config.metadata).unwrap()),
        ]
    }
}

#[async_trait]
impl ServiceConfigRepository for SqliteServiceConfigRepository {
    async fn save(&self, config: &mut ServiceConfig) -> DomainResult<()> {
        <Self as SqliteRepository<ServiceConfig>>::save(self, config).await
    }

    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<ServiceConfig>> {
        <Self as SqliteRepository<ServiceConfig>>::find_by_id(self, id).await
    }

    async fn find_all(&self) -> DomainResult<Vec<ServiceConfig>> {
        <Self as SqliteRepository<ServiceConfig>>::find_all(self).await
    }

    async fn find_by_service_type(
        &self,
        service_type: ServiceType,
    ) -> DomainResult<Vec<ServiceConfig>> {
        let conn = self.connection().lock().await;
        let query = format!("SELECT * FROM {} WHERE service_type = ?", self.table_name());
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![serde_json::to_string(&service_type)?], |row| {
            self.map_row(row)
        })?;

        let mut configs = Vec::new();
        for config in rows {
            configs.push(config?);
        }
        Ok(configs)
    }

    async fn find_enabled(&self) -> DomainResult<Vec<ServiceConfig>> {
        let conn = self.connection().lock().await;
        let query = format!("SELECT * FROM {} WHERE enabled = 1", self.table_name());
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| self.map_row(row))?;

        let mut configs = Vec::new();
        for config in rows {
            configs.push(config?);
        }
        Ok(configs)
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        <Self as SqliteRepository<ServiceConfig>>::delete(self, id).await
    }

    async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> DomainResult<()> {
        let conn = self.connection().lock().await;
        let query = format!(
            "UPDATE {} SET auth_config = ?, updated_at = ? WHERE id = ?",
            self.table_name()
        );
        conn.execute(
            &query,
            params![
                serde_json::to_string(&auth_config)?,
                Utc::now().to_rfc3339(),
                id.to_string()
            ],
        )?;
        Ok(())
    }

    async fn update_enabled_status(&self, id: Uuid, enabled: bool) -> DomainResult<()> {
        let conn = self.connection().lock().await;
        let query = format!(
            "UPDATE {} SET enabled = ?, updated_at = ? WHERE id = ?",
            self.table_name()
        );
        conn.execute(
            &query,
            params![enabled, Utc::now().to_rfc3339(), id.to_string()],
        )?;
        Ok(())
    }

    async fn update_last_sync(&self, id: Uuid) -> DomainResult<()> {
        let conn = self.connection().lock().await;
        let query = format!(
            "UPDATE {} SET last_sync = ?, updated_at = ? WHERE id = ?",
            self.table_name()
        );
        conn.execute(
            &query,
            params![
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
                id.to_string()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{AuthType, OAuth2Config, ServiceEndpoints};

    use super::*;

    async fn create_test_config() -> ServiceConfig {
        let oauth2_config = OAuth2Config {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:1420".to_string(),
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            scope: vec!["test".to_string()],
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        };

        let mut endpoints = serde_json::Map::new();
        endpoints.insert(
            "test".to_string(),
            serde_json::json!({
                "path": "/test",
                "method": "GET"
            }),
        );

        ServiceConfig {
            id: Uuid::new_v4(),
            name: "Test Config".to_string(),
            service_type: ServiceType::Github,
            auth_type: AuthType::OAuth2,
            auth_config: AuthConfig::OAuth2(oauth2_config),
            endpoints: ServiceEndpoints {
                base_url: "https://api.github.com".to_string(),
                endpoints,
            },
            enabled: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_sync: None,
            metadata: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn test_sqlite_repository() {
        let repo = SqliteServiceConfigRepository::new(":memory:").unwrap();
        let mut config = create_test_config().await;

        // Test save
        ServiceConfigRepository::save(&repo, &mut config)
            .await
            .unwrap();

        // Test find by id
        let found = ServiceConfigRepository::find_by_id(&repo, config.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, config.name);

        // Test find all
        let all = ServiceConfigRepository::find_all(&repo).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, config.name);

        // Test update auth config
        let new_auth_config = AuthConfig::OAuth2(OAuth2Config {
            client_id: "new_client".to_string(),
            client_secret: "new_secret".to_string(),
            redirect_uri: "http://localhost:1420".to_string(),
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            scope: vec!["new_scope".to_string()],
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        });
        repo.update_auth_config(config.id, new_auth_config.clone())
            .await
            .unwrap();

        // Test update enabled status
        repo.update_enabled_status(config.id, true).await.unwrap();

        // Test find enabled
        let enabled = repo.find_enabled().await.unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, config.id);

        // Test update last sync
        repo.update_last_sync(config.id).await.unwrap();

        // Test delete
        ServiceConfigRepository::delete(&repo, config.id)
            .await
            .unwrap();
        let not_found = ServiceConfigRepository::find_by_id(&repo, config.id)
            .await
            .unwrap();
        assert!(not_found.is_none());
    }
}
