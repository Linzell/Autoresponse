use crate::domain::{
    entities::{AuthConfig, AuthType, OAuth2Config, ServiceConfig, ServiceEndpoints, ServiceType},
    repositories::ServiceConfigRepository,
};
use crate::infrastructure::services::oauth::DynOAuthService;
use crate::presentation::dtos::ValidationError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub service_type: ServiceType,
}

#[tauri::command]
pub async fn save_oauth_config(
    credentials: OAuthCredentials,
    config_repo: tauri::State<'_, Arc<dyn ServiceConfigRepository>>,
) -> Result<ServiceConfig, ValidationError> {
    let oauth_config = OAuth2Config {
        client_id: credentials.client_id,
        client_secret: credentials.client_secret,
        redirect_uri: "http://localhost:1420/oauth/callback".to_string(),
        auth_url: String::new(),
        token_url: String::new(),
        scope: vec![],
        access_token: None,
        refresh_token: None,
        token_expires_at: None,
    };

    let mut config = ServiceConfig {
        id: Uuid::new_v4(),
        name: format!("{:?} Integration", credentials.service_type),
        service_type: credentials.service_type,
        auth_type: AuthType::OAuth2,
        auth_config: AuthConfig::OAuth2(oauth_config),
        endpoints: ServiceEndpoints {
            base_url: String::new(),
            endpoints: serde_json::Map::new(),
        },
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_sync: None,
        metadata: serde_json::Value::Null,
    };

    config_repo
        .save(&mut config)
        .await
        .map_err(|e| ValidationError::from_message(&e.to_string()))?;
    Ok(config)
}

#[tauri::command]
pub async fn get_service_configs(
    config_repo: tauri::State<'_, Arc<dyn ServiceConfigRepository>>,
) -> Result<Vec<ServiceConfig>, ValidationError> {
    config_repo
        .find_all()
        .await
        .map_err(|e| ValidationError::from_message(&e.to_string()))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_oauth_service_config(
    config_id: String,
    config_repo: tauri::State<'_, Arc<dyn ServiceConfigRepository>>,
) -> Result<(), ValidationError> {
    let id =
        Uuid::parse_str(&config_id).map_err(|e| ValidationError::from_message(&e.to_string()))?;
    config_repo
        .delete(id)
        .await
        .map_err(|e| ValidationError::from_message(&e.to_string()))
}

#[tauri::command]
pub async fn start_oauth_flow(
    service_type: ServiceType,
    oauth_service: tauri::State<'_, DynOAuthService>,
) -> Result<String, ValidationError> {
    oauth_service
        .get_authorization_url(service_type)
        .await
        .map_err(|e| ValidationError::from_message(&e.to_string()))
}

#[tauri::command]
pub async fn handle_oauth_callback(
    code: String,
    service_type: ServiceType,
    config_id: String,
    oauth_service: tauri::State<'_, DynOAuthService>,
    config_repo: tauri::State<'_, Arc<dyn ServiceConfigRepository>>,
) -> Result<(), ValidationError> {
    let token_response = oauth_service
        .exchange_code_for_token(code, service_type.clone())
        .await
        .map_err(|e| ValidationError::from_message(&e.to_string()))?;

    let id =
        Uuid::parse_str(&config_id).map_err(|e| ValidationError::from_message(&e.to_string()))?;
    let config = config_repo
        .find_by_id(id)
        .await
        .map_err(|e| ValidationError::from_message(&e.to_string()))?
        .ok_or_else(|| ValidationError::from_message("Service configuration not found"))?;

    if let AuthConfig::OAuth2(mut oauth_config) = config.auth_config {
        oauth_config.access_token = Some(token_response.access_token);
        oauth_config.refresh_token = token_response.refresh_token;
        oauth_config.token_expires_at = token_response
            .expires_in
            .map(|expires_in| Utc::now() + chrono::Duration::seconds(expires_in));

        config_repo
            .update_auth_config(id, AuthConfig::OAuth2(oauth_config))
            .await
            .map_err(|e| ValidationError::from_message(&e.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        create_test_oauth2_config, create_test_service_config, create_test_state,
    };
    use crate::{
        domain::error::DomainError,
        infrastructure::services::oauth::{OAuthService, TokenResponse},
    };
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        ServiceConfigRepository {}
        #[async_trait]
        impl ServiceConfigRepository for ServiceConfigRepository {
            async fn save(&self, config: &mut ServiceConfig) -> Result<(), DomainError>;
            async fn find_by_id(&self, id: Uuid) -> Result<Option<ServiceConfig>, DomainError>;
            async fn find_all(&self) -> Result<Vec<ServiceConfig>, DomainError>;
            async fn find_by_service_type(&self, service_type: ServiceType) -> Result<Vec<ServiceConfig>, DomainError>;
            async fn find_enabled(&self) -> Result<Vec<ServiceConfig>, DomainError>;
            async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
            async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> Result<(), DomainError>;
            async fn update_enabled_status(&self, id: Uuid, enabled: bool) -> Result<(), DomainError>;
            async fn update_last_sync(&self, id: Uuid) -> Result<(), DomainError>;
        }
    }

    mock! {
        OAuthService {}
        #[async_trait]
        impl OAuthService for OAuthService {
            async fn get_authorization_url(&self, service_type: ServiceType) -> Result<String, DomainError>;
            async fn exchange_code_for_token(&self, code: String, service_type: ServiceType) -> Result<TokenResponse, DomainError>;
            async fn refresh_token(&self, refresh_token: String, service_type: ServiceType) -> Result<TokenResponse, DomainError>;
        }
    }

    #[tokio::test]
    async fn test_save_oauth_config() {
        let mut mock_repo = MockServiceConfigRepository::new();
        mock_repo.expect_save().returning(|_| Ok(()));

        let credentials = OAuthCredentials {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            service_type: ServiceType::Github,
        };

        let mock_repo = Arc::new(mock_repo) as Arc<dyn ServiceConfigRepository>;
        let result = save_oauth_config(credentials, create_test_state(mock_repo)).await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.service_type, ServiceType::Github);
        assert!(matches!(config.auth_type, AuthType::OAuth2));
        if let AuthConfig::OAuth2(oauth_config) = config.auth_config {
            assert_eq!(oauth_config.client_id, "test_client");
            assert_eq!(oauth_config.client_secret, "test_secret");
        } else {
            panic!("Expected OAuth2 config");
        }
    }

    #[tokio::test]
    async fn test_get_service_configs() {
        let mut mock_repo = MockServiceConfigRepository::new();
        let oauth_config = create_test_oauth2_config();
        let test_config = create_test_service_config(
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth_config),
        );

        mock_repo
            .expect_find_all()
            .returning(move || Ok(vec![test_config.clone()]));

        let mock_repo = Arc::new(mock_repo) as Arc<dyn ServiceConfigRepository>;
        let result = get_service_configs(create_test_state(mock_repo))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].service_type, ServiceType::Github);
    }

    #[tokio::test]
    async fn test_delete_oauth_service_config() {
        let mut mock_repo = MockServiceConfigRepository::new();
        let id = Uuid::new_v4();

        mock_repo
            .expect_delete()
            .with(mockall::predicate::eq(id))
            .returning(|_| Ok(()));

        let mock_repo = Arc::new(mock_repo) as Arc<dyn ServiceConfigRepository>;
        let result =
            delete_oauth_service_config(id.to_string(), create_test_state(mock_repo)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_oauth_flow() {
        let mut mock_service = MockOAuthService::new();
        mock_service
            .expect_get_authorization_url()
            .with(mockall::predicate::eq(ServiceType::Github))
            .returning(|_| Ok("https://test.com/auth".to_string()));

        let mock_service = Arc::new(mock_service) as DynOAuthService;
        let result = start_oauth_flow(ServiceType::Github, create_test_state(mock_service))
            .await
            .unwrap();

        assert_eq!(result, "https://test.com/auth");
    }

    #[tokio::test]
    async fn test_handle_oauth_callback() {
        let mut mock_service = MockOAuthService::new();
        let mut mock_repo = MockServiceConfigRepository::new();
        let config_id = Uuid::new_v4();

        let oauth_config = create_test_oauth2_config();
        let test_config = create_test_service_config(
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth_config),
        )
        .with_id(config_id);

        mock_service
            .expect_exchange_code_for_token()
            .with(
                mockall::predicate::eq("test_code".to_string()),
                mockall::predicate::eq(ServiceType::Github),
            )
            .returning(|_, _| {
                Ok(TokenResponse {
                    access_token: "test_token".to_string(),
                    token_type: "Bearer".to_string(),
                    expires_in: Some(3600),
                    refresh_token: Some("refresh_token".to_string()),
                    scope: Some("test".to_string()),
                })
            });

        mock_repo
            .expect_find_by_id()
            .with(mockall::predicate::eq(config_id))
            .returning(move |_| Ok(Some(test_config.clone())));

        mock_repo
            .expect_update_auth_config()
            .returning(|_, _| Ok(()));

        let result = handle_oauth_callback(
            "test_code".to_string(),
            ServiceType::Github,
            config_id.to_string(),
            create_test_state(Arc::new(mock_service) as DynOAuthService),
            create_test_state(Arc::new(mock_repo) as Arc<dyn ServiceConfigRepository>),
        )
        .await;

        assert!(result.is_ok());
    }
}
