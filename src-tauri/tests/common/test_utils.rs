use autoresponse_lib::domain::entities::{
    ApiKeyConfig, AuthConfig, AuthType, BasicAuthConfig, OAuth2Config, ServiceConfig,
    ServiceEndpoints, ServiceType,
};
use chrono::Utc;
use uuid::Uuid;

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
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_sync: None,
        metadata: serde_json::Value::Object(serde_json::Map::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth2_config_creation() {
        let config = create_test_oauth2_config();
        assert!(!config.client_id.is_empty());
        assert!(!config.client_secret.is_empty());
        assert_eq!(config.redirect_uri, "http://localhost:8080/callback");
    }

    #[test]
    fn test_basic_auth_config_creation() {
        let config = create_test_basic_auth_config();
        assert!(!config.username.is_empty());
        assert!(!config.password.is_empty());
    }

    #[test]
    fn test_api_key_config_creation() {
        let config = create_test_api_key_config();
        assert!(!config.key.is_empty());
        assert_eq!(config.header_name.as_deref().unwrap(), "X-Test-API-Key");
    }

    #[test]
    fn test_service_config_creation() {
        let config = create_test_service_config(
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(create_test_oauth2_config()),
        );
        assert!(!config.name.is_empty());
        assert!(matches!(config.service_type, ServiceType::Github));
        assert!(matches!(config.auth_type, AuthType::OAuth2));
        assert!(config.enabled);
        assert!(config.last_sync.is_none());
    }
}
