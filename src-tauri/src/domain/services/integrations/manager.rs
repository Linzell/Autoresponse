use crate::domain::{
    entities::{
        notification::NotificationSource,
        service_config::{ServiceConfig, ServiceType},
    },
    error::{DomainError, DomainResult},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{
    DynIntegrationService, GithubService, GitlabService, GoogleService, JiraService,
    LinkedInService, MicrosoftService,
};

/// Manages the lifecycle and coordination of integration services
#[derive(Debug)]
pub struct IntegrationManager {
    services: Arc<RwLock<HashMap<ServiceType, DynIntegrationService>>>,
}

impl Default for IntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrationManager {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize a service with its configuration
    pub async fn initialize_service(&self, config: ServiceConfig) -> DomainResult<()> {
        let service: DynIntegrationService = match config.service_type {
            ServiceType::Github => Arc::new(GithubService::new()),
            ServiceType::Gitlab => Arc::new(GitlabService::new()),
            ServiceType::Jira => Arc::new(JiraService::new()),
            ServiceType::Microsoft => Arc::new(MicrosoftService::new()),
            ServiceType::Google => Arc::new(GoogleService::new()),
            ServiceType::LinkedIn => Arc::new(LinkedInService::new()),
            ServiceType::Custom(_) => {
                return Err(DomainError::ConfigurationError(
                    "Custom services not supported".to_string(),
                ))
            }
        };

        service.initialize(config.clone()).await?;

        let mut services = self.services.write().await;
        services.insert(config.service_type, service);

        Ok(())
    }

    /// Get a service by its type
    pub async fn get_service(
        &self,
        service_type: &ServiceType,
    ) -> DomainResult<DynIntegrationService> {
        let services = self.services.read().await;
        services
            .get(service_type)
            .cloned()
            .ok_or_else(|| DomainError::NotFound("Service not found".to_string()))
    }

    /// Get a service by notification source
    pub async fn get_service_for_source(
        &self,
        source: &NotificationSource,
    ) -> DomainResult<DynIntegrationService> {
        let service_type = match source {
            NotificationSource::Github => ServiceType::Github,
            NotificationSource::Gitlab => ServiceType::Gitlab,
            NotificationSource::Jira => ServiceType::Jira,
            NotificationSource::Microsoft => ServiceType::Microsoft,
            NotificationSource::Google => ServiceType::Google,
            NotificationSource::LinkedIn => ServiceType::LinkedIn,
            NotificationSource::Custom(name) => ServiceType::Custom(name.clone()),
            NotificationSource::Email => ServiceType::Microsoft, // Default to Microsoft for email
        };

        self.get_service(&service_type).await
    }

    /// Test all initialized service connections
    pub async fn test_connections(&self) -> HashMap<ServiceType, bool> {
        let services = self.services.read().await;
        let mut results = HashMap::new();

        for (service_type, service) in services.iter() {
            let is_connected = service.test_connection().await.unwrap_or(false);
            results.insert(service_type.clone(), is_connected);
        }

        results
    }

    /// Sync notifications from all initialized services
    pub async fn sync_all_notifications(
        &self,
    ) -> DomainResult<Vec<crate::domain::entities::notification::Notification>> {
        let services = self.services.read().await;
        let mut all_notifications = Vec::new();

        for service in services.values() {
            match service.sync_notifications().await {
                Ok(notifications) => all_notifications.extend(notifications),
                Err(e) => log::error!("Error syncing notifications: {}", e),
            }
        }

        Ok(all_notifications)
    }

    /// Check if a service type is initialized
    pub async fn is_service_initialized(&self, service_type: &ServiceType) -> bool {
        let services = self.services.read().await;
        services.contains_key(service_type)
    }

    /// Remove a service
    pub async fn remove_service(&self, service_type: &ServiceType) -> DomainResult<()> {
        let mut services = self.services.write().await;
        services
            .remove(service_type)
            .ok_or_else(|| DomainError::NotFound("Service not found".to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::service_config::{
        AuthConfig, AuthType, OAuth2Config, ServiceEndpoints,
    };

    #[tokio::test]
    async fn test_service_initialization() {
        let manager = IntegrationManager::new();

        let config = ServiceConfig::new(
            "GitHub".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["repo".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://api.github.com".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(manager.initialize_service(config).await.is_ok());
        assert!(manager.is_service_initialized(&ServiceType::Github).await);
        assert!(manager.get_service(&ServiceType::Github).await.is_ok());
    }

    #[tokio::test]
    async fn test_service_removal() {
        let manager = IntegrationManager::new();

        let config = ServiceConfig::new(
            "GitHub".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["repo".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://api.github.com".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(manager.initialize_service(config).await.is_ok());
        assert!(manager.remove_service(&ServiceType::Github).await.is_ok());
        assert!(!manager.is_service_initialized(&ServiceType::Github).await);
    }

    #[tokio::test]
    async fn test_service_source_mapping() {
        let manager = IntegrationManager::new();

        let config = ServiceConfig::new(
            "GitHub".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["repo".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://api.github.com".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(manager.initialize_service(config).await.is_ok());
        assert!(manager
            .get_service_for_source(&NotificationSource::Github)
            .await
            .is_ok());
    }
}
