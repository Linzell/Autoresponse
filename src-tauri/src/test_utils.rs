use crate::domain::entities::{
    AuthConfig, AuthType, OAuth2Config, ServiceConfig, ServiceEndpoints, ServiceType,
};
use chrono::Utc;
use std::cell::RefCell;
use std::sync::Arc;
use std::thread_local;
use tauri::Manager;
use uuid::Uuid;

thread_local! {
    static TEST_APP: RefCell<Option<tauri::App<tauri::test::MockRuntime>>> = RefCell::new(None);
}

pub fn create_test_state<T: ?Sized + Send + Sync + 'static>(
    value: Arc<T>,
) -> tauri::State<'static, Arc<T>> {
    TEST_APP.with(|app| {
        let mut app_ref = app.borrow_mut();
        if app_ref.is_none() {
            *app_ref = Some(tauri::test::mock_app());
        }
        let app = app_ref.as_ref().unwrap();
        app.manage(value.clone());
        unsafe { std::mem::transmute(app.state::<Arc<T>>()) }
    })
}

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

pub fn create_test_service_config(
    service_type: ServiceType,
    auth_type: AuthType,
    auth_config: AuthConfig,
) -> ServiceConfig {
    ServiceConfig {
        id: Uuid::new_v4(),
        name: format!("Test Service {}", Uuid::new_v4()),
        service_type,
        auth_type,
        auth_config,
        endpoints: ServiceEndpoints {
            base_url: "https://api.example.com".to_string(),
            endpoints: serde_json::Map::new(),
        },
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_sync: None,
        metadata: serde_json::Value::Null,
    }
}

impl ServiceConfig {
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_oauth2_config() {
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
    fn test_create_service_config() {
        let oauth_config = create_test_oauth2_config();
        let config = create_test_service_config(
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth_config),
        );
        assert!(!config.name.is_empty());
        assert!(matches!(config.service_type, ServiceType::Github));
        assert!(matches!(config.auth_type, AuthType::OAuth2));
        assert!(config.enabled);
        assert!(config.last_sync.is_none());
    }

    #[test]
    fn test_with_id() {
        let oauth_config = create_test_oauth2_config();
        let id = Uuid::new_v4();
        let config = create_test_service_config(
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth_config),
        )
        .with_id(id);
        assert_eq!(config.id, id);
    }
}
