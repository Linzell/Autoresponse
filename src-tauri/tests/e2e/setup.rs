use anyhow::Result;
use autoresponse_lib::domain::entities::{
    ApiKeyConfig, AuthConfig, AuthType, BasicAuthConfig, OAuth2Config, ServiceConfig,
    ServiceEndpoints, ServiceType,
};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct TestDatabase {
    pub connection: Arc<Mutex<Connection>>,
}

impl TestDatabase {
    pub async fn new() -> Result<Self> {
        let connection = Connection::open_in_memory()?;

        // Initialize schema
        connection.execute(
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
        )?;

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
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub async fn cleanup(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("DELETE FROM notifications", [])?;
        conn.execute("DELETE FROM service_configs", [])?;
        Ok(())
    }
}

pub fn create_test_oauth2_config() -> OAuth2Config {
    OAuth2Config {
        client_id: format!("test_client_{}", Uuid::new_v4()),
        client_secret: format!("test_secret_{}", Uuid::new_v4()),
        redirect_uri: "http://localhost:8080/callback".to_string(),
        auth_url: "http://auth.example.com/oauth/authorize".to_string(),
        token_url: "http://auth.example.com/oauth/token".to_string(),
        scope: vec!["read".to_string(), "write".to_string()],
        access_token: None,
        refresh_token: None,
        token_expires_at: None,
    }
}

pub fn create_test_basic_auth_config() -> BasicAuthConfig {
    BasicAuthConfig {
        username: format!("test_user_{}", Uuid::new_v4()),
        password: format!("test_pass_{}", Uuid::new_v4()),
    }
}

pub fn create_test_api_key_config() -> ApiKeyConfig {
    ApiKeyConfig {
        key: format!("test_key_{}", Uuid::new_v4()),
        header_name: Some("X-Test-API-Key".to_string()),
    }
}

pub fn create_test_service_config(
    service_type: ServiceType,
    auth_type: AuthType,
    auth_config: AuthConfig,
) -> ServiceConfig {
    let now = Utc::now();
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

    ServiceConfig {
        id: Uuid::new_v4(),
        name: format!("Test Service {}", Uuid::new_v4()),
        service_type,
        auth_type,
        auth_config,
        endpoints,
        enabled: true,
        created_at: now,
        updated_at: now,
        last_sync: None,
        metadata: serde_json::Value::Object(serde_json::Map::new()),
    }
}

pub async fn setup_test_database() -> Result<TestDatabase> {
    let db = TestDatabase::new().await?;
    db.cleanup().await?;
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_setup() {
        let db = setup_test_database().await.unwrap();
        assert!(db.cleanup().await.is_ok());
    }

    #[test]
    fn test_config_creation() {
        let oauth2_config = create_test_oauth2_config();
        assert!(!oauth2_config.client_id.is_empty());
        assert!(!oauth2_config.client_secret.is_empty());

        let basic_auth_config = create_test_basic_auth_config();
        assert!(!basic_auth_config.username.is_empty());
        assert!(!basic_auth_config.password.is_empty());

        let api_key_config = create_test_api_key_config();
        assert!(!api_key_config.key.is_empty());
        assert!(api_key_config.header_name.is_some());
    }
}
