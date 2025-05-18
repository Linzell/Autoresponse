use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::{
    AuthConfig, AuthType, OAuth2Config, ServiceConfig, ServiceType,
};
use crate::infrastructure::repositories::service_config_repository::ServiceConfigRepository;
use super::oauth::{DynOAuthService, TokenResponse};

pub struct OAuthCoordinator {
    oauth_service: DynOAuthService,
    config_repository: Arc<ServiceConfigRepository>,
}

#[async_trait]
pub trait OAuth2Flow: Send + Sync {
    async fn initiate_auth(&self, service_type: ServiceType) -> Result<String, String>;
    async fn handle_callback(&self, service_type: ServiceType, code: String) -> Result<Uuid, String>;
    async fn refresh_auth(&self, config_id: Uuid) -> Result<(), String>;
    async fn get_service_config(&self, config_id: Uuid) -> Result<ServiceConfig, String>;
}

impl OAuthCoordinator {
    pub fn new(oauth_service: DynOAuthService, config_repository: Arc<ServiceConfigRepository>) -> Self {
        Self {
            oauth_service,
            config_repository,
        }
    }

    async fn update_token_info(
        &self,
        config_id: &Uuid,
        token_response: TokenResponse,
    ) -> Result<(), String> {
        let config = self.config_repository
            .find_by_id(config_id)
            .map_err(|e| format!("Failed to fetch config: {}", e))?
            .ok_or_else(|| "Service configuration not found".to_string())?;

        if let AuthConfig::OAuth2(mut oauth2_config) = config.auth_config {
            oauth2_config.access_token = Some(token_response.access_token);
            oauth2_config.refresh_token = token_response.refresh_token;
            oauth2_config.token_expires_at = token_response
                .expires_in
                .map(|expires_in| Utc::now() + chrono::Duration::seconds(expires_in));

            self.config_repository
                .update(
                    config_id,
                    Some(AuthConfig::OAuth2(oauth2_config)),
                    None,
                    None,
                    None,
                )
                .map_err(|e| format!("Failed to update config: {}", e))?;
        }

        Ok(())
    }
}

#[async_trait]
impl OAuth2Flow for OAuthCoordinator {
    async fn initiate_auth(&self, service_type: ServiceType) -> Result<String, String> {
        self.oauth_service.get_authorization_url(service_type).await
    }

    async fn handle_callback(&self, service_type: ServiceType, code: String) -> Result<Uuid, String> {
        let token_response = self.oauth_service
            .exchange_code_for_token(code, service_type.clone())
            .await?;

        let oauth2_config = OAuth2Config {
            client_id: String::new(), // Will be populated from environment
            client_secret: String::new(), // Will be populated from environment
            redirect_uri: "http://localhost:1420/oauth/callback".to_string(),
            auth_url: String::new(), // Will be populated based on service type
            token_url: String::new(), // Will be populated based on service type
            scope: vec![],
            access_token: Some(token_response.access_token.clone()),
            refresh_token: token_response.refresh_token.clone(),
            token_expires_at: token_response
                .expires_in
                .map(|expires_in| Utc::now() + chrono::Duration::seconds(expires_in)),
        };

        let service_config = ServiceConfig::new(
            format!("{:?} Integration", service_type),
            service_type,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config),
            Default::default(), // Endpoints will be configured separately
        );

        let saved_config = self.config_repository
            .save(service_config)
            .map_err(|e| format!("Failed to save service configuration: {}", e))?;

        Ok(saved_config.id)
    }

    async fn refresh_auth(&self, config_id: Uuid) -> Result<(), String> {
        let config = self.config_repository
            .find_by_id(&config_id)
            .map_err(|e| format!("Failed to fetch config: {}", e))?
            .ok_or_else(|| "Service configuration not found".to_string())?;

        let refresh_token = match &config.auth_config {
            AuthConfig::OAuth2(oauth2_config) => oauth2_config
                .refresh_token
                .clone()
                .ok_or_else(|| "No refresh token available".to_string())?,
            _ => return Err("Not an OAuth2 configuration".to_string()),
        };

        let token_response = self.oauth_service
            .refresh_token(refresh_token, config.service_type)
            .await?;

        self.update_token_info(&config_id, token_response).await
    }

    async fn get_service_config(&self, config_id: Uuid) -> Result<ServiceConfig, String> {
        self.config_repository
            .find_by_id(&config_id)
            .map_err(|e| format!("Failed to fetch config: {}", e))?
            .ok_or_else(|| "Service configuration not found".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use crate::infrastructure::services::oauth::MockOAuthService;

    mock! {
        ServiceConfigRepository {
            fn save(&self, config: ServiceConfig) -> Result<ServiceConfig, String>;
            fn find_by_id(&self, id: &Uuid) -> Result<Option<ServiceConfig>, String>;
            fn update(
                &self,
                id: &Uuid,
                auth_config: Option<AuthConfig>,
                endpoints: Option<crate::domain::entities::ServiceEndpoints>,
                enabled: Option<bool>,
                metadata: Option<serde_json::Value>,
            ) -> Result<ServiceConfig, String>;
        }
    }

    #[tokio::test]
    async fn test_initiate_auth() {
        let mut oauth_service = MockOAuthService::new();
        oauth_service
            .expect_get_authorization_url()
            .return_once(|_| "https://example.com/auth".to_string());

        let config_repo = Arc::new(ServiceConfigRepository::new());
        let coordinator = OAuthCoordinator::new(Arc::new(oauth_service), config_repo);

        let url = coordinator.initiate_auth(ServiceType::Github).await.unwrap();
        assert_eq!(url, "https://example.com/auth");
    }

    #[tokio::test]
    async fn test_handle_callback() {
        let mut oauth_service = MockOAuthService::new();
        oauth_service
            .expect_exchange_code_for_token()
            .return_once(|_, _| {
                Ok(TokenResponse {
                    access_token: "access_token".to_string(),
                    token_type: "Bearer".to_string(),
                    expires_in: Some(3600),
                    refresh_token: Some("refresh_token".to_string()),
                    scope: Some("read write".to_string()),
                })
            });

        let config_repo = Arc::new(ServiceConfigRepository::new());
        let coordinator = OAuthCoordinator::new(Arc::new(oauth_service), config_repo);

        let config_id = coordinator
            .handle_callback(ServiceType::Github, "code".to_string())
            .await
            .unwrap();

        let config = coordinator.get_service_config(config_id).await.unwrap();
        assert!(matches!(config.service_type, ServiceType::Github));
        assert!(matches!(config.auth_type, AuthType::OAuth2));
    }
}