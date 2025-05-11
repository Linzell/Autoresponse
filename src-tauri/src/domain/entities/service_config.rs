use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    Email,
    Github,
    Gitlab,
    Jira,
    Google,
    Microsoft,
    LinkedIn,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    OAuth2,
    BasicAuth,
    ApiKey,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub auth_url: String,
    pub token_url: String,
    pub scope: Vec<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BasicAuthConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiKeyConfig {
    pub key: String,
    pub header_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomAuthConfig {
    pub auth_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthConfig {
    OAuth2(OAuth2Config),
    BasicAuth(BasicAuthConfig),
    ApiKey(ApiKeyConfig),
    Custom(CustomAuthConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceEndpoints {
    pub base_url: String,
    pub endpoints: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceConfig {
    pub id: Uuid,
    pub name: String,
    pub service_type: ServiceType,
    pub auth_type: AuthType,
    pub auth_config: AuthConfig,
    pub endpoints: ServiceEndpoints,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl ServiceConfig {
    pub fn new(
        name: String,
        service_type: ServiceType,
        auth_type: AuthType,
        auth_config: AuthConfig,
        endpoints: ServiceEndpoints,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
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

    pub fn update_auth_config(&mut self, auth_config: AuthConfig) {
        self.auth_config = auth_config;
        self.updated_at = Utc::now();
    }

    pub fn update_endpoints(&mut self, endpoints: ServiceEndpoints) {
        self.endpoints = endpoints;
        self.updated_at = Utc::now();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.updated_at = Utc::now();
    }

    pub fn update_last_sync(&mut self) {
        self.last_sync = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_oauth2_config() -> OAuth2Config {
        OAuth2Config {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
            auth_url: "http://auth.example.com/oauth/authorize".to_string(),
            token_url: "http://auth.example.com/oauth/token".to_string(),
            scope: vec!["read".to_string(), "write".to_string()],
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        }
    }

    fn create_test_endpoints() -> ServiceEndpoints {
        let mut endpoints = serde_json::Map::new();
        endpoints.insert(
            "notifications".to_string(),
            serde_json::json!({
                "path": "/api/notifications",
                "method": "GET"
            }),
        );

        ServiceEndpoints {
            base_url: "http://api.example.com".to_string(),
            endpoints,
        }
    }

    #[test]
    fn test_service_config_creation() {
        let oauth2_config = create_test_oauth2_config();
        let endpoints = create_test_endpoints();

        let service_config = ServiceConfig::new(
            "Test Service".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config),
            endpoints,
        );

        assert_eq!(service_config.name, "Test Service");
        assert!(matches!(service_config.service_type, ServiceType::Github));
        assert!(matches!(service_config.auth_type, AuthType::OAuth2));
        assert!(service_config.enabled);
        assert!(service_config.last_sync.is_none());
    }

    #[test]
    fn test_service_config_updates() {
        let oauth2_config = create_test_oauth2_config();
        let endpoints = create_test_endpoints();

        let mut service_config = ServiceConfig::new(
            "Test Service".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config.clone()),
            endpoints.clone(),
        );

        // Test updating auth config
        let mut new_oauth2_config = oauth2_config;
        new_oauth2_config.access_token = Some("new_token".to_string());
        service_config.update_auth_config(AuthConfig::OAuth2(new_oauth2_config));
        assert!(matches!(service_config.auth_config,
            AuthConfig::OAuth2(ref config) if config.access_token.as_deref() == Some("new_token")));

        // Test updating endpoints
        let mut new_endpoints = endpoints;
        new_endpoints.base_url = "http://new.example.com".to_string();
        service_config.update_endpoints(new_endpoints);
        assert_eq!(service_config.endpoints.base_url, "http://new.example.com");

        // Test updating enabled status
        service_config.set_enabled(false);
        assert!(!service_config.enabled);

        // Test updating last sync
        assert!(service_config.last_sync.is_none());
        service_config.update_last_sync();
        assert!(service_config.last_sync.is_some());

        // Test updating metadata
        let metadata = serde_json::json!({
            "key": "value"
        });
        service_config.update_metadata(metadata.clone());
        assert_eq!(service_config.metadata, metadata);
    }
}
