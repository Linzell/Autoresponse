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
        redirect_uri: "http://localhost:1420/oauth/callback".to_string(),
        auth_url: "https://github.com/login/oauth/authorize".to_string(),
        token_url: "https://github.com/login/oauth/access_token".to_string(),
        scope: vec!["repo".to_string(), "user".to_string(), "notifications".to_string()],
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

use autoresponse_lib::domain::entities::{
    Notification, NotificationMetadata, NotificationPriority, NotificationSource,
};

pub fn create_test_notification(
    title: &str,
    content: &str,
    priority: NotificationPriority,
    source: NotificationSource,
) -> Notification {
    Notification::new(
        title.to_string(),
        content.to_string(),
        priority,
        NotificationMetadata {
            source,
            external_id: Some("test-id".to_string()),
            url: Some("http://test.com".to_string()),
            tags: vec!["test".to_string()],
            custom_data: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth2_config_creation() {
        let config = create_test_oauth2_config();
        assert!(!config.client_id.is_empty());
        assert!(!config.client_secret.is_empty());
        assert_eq!(config.redirect_uri, "http://localhost:1420/oauth/callback");
        assert_eq!(config.auth_url, "https://github.com/login/oauth/authorize");
        assert_eq!(config.token_url, "https://github.com/login/oauth/access_token");
        assert!(config.scope.contains(&"repo".to_string()));
        assert!(config.scope.contains(&"user".to_string()));
        assert!(config.scope.contains(&"notifications".to_string()));
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
