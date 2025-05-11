use crate::domain::{
    entities::{AuthConfig, AuthType, ServiceConfig, ServiceEndpoints, ServiceType},
    error::DomainResult,
    services::DynServiceConfigService,
};
use uuid::Uuid;

pub struct ServiceConfigUseCases {
    service_config_service: DynServiceConfigService,
}

impl ServiceConfigUseCases {
    pub fn new(service_config_service: DynServiceConfigService) -> Self {
        Self {
            service_config_service,
        }
    }

    pub async fn create_service_config(
        &self,
        name: String,
        service_type: ServiceType,
        auth_type: AuthType,
        auth_config: AuthConfig,
        endpoints: ServiceEndpoints,
    ) -> DomainResult<ServiceConfig> {
        self.service_config_service
            .create_service_config(name, service_type, auth_type, auth_config, endpoints)
            .await
    }

    pub async fn get_service_config(&self, id: Uuid) -> DomainResult<ServiceConfig> {
        self.service_config_service.get_service_config(id).await
    }

    pub async fn get_all_service_configs(&self) -> DomainResult<Vec<ServiceConfig>> {
        self.service_config_service.get_all_service_configs().await
    }

    pub async fn get_enabled_service_configs(&self) -> DomainResult<Vec<ServiceConfig>> {
        self.service_config_service.get_enabled_configs().await
    }

    pub async fn get_service_configs_by_type(
        &self,
        service_type: ServiceType,
    ) -> DomainResult<Vec<ServiceConfig>> {
        self.service_config_service
            .get_configs_by_service_type(service_type)
            .await
    }

    pub async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> DomainResult<()> {
        self.service_config_service
            .update_auth_config(id, auth_config)
            .await
    }

    pub async fn enable_service(&self, id: Uuid) -> DomainResult<()> {
        self.service_config_service.enable_service(id).await
    }

    pub async fn disable_service(&self, id: Uuid) -> DomainResult<()> {
        self.service_config_service.disable_service(id).await
    }

    pub async fn delete_service_config(&self, id: Uuid) -> DomainResult<()> {
        self.service_config_service.delete_service_config(id).await
    }

    pub async fn validate_service_connection(&self, id: Uuid) -> DomainResult<bool> {
        let _config = self.get_service_config(id).await?;
        // Implement actual validation logic here
        Ok(true)
    }

    pub async fn rotate_auth_credentials(&self, id: Uuid) -> DomainResult<()> {
        let _config = self.get_service_config(id).await?;
        // Implement credential rotation logic here
        Ok(())
    }

    pub async fn update_last_sync(&self, id: Uuid) -> DomainResult<()> {
        self.service_config_service.update_last_sync(id).await
    }

    pub async fn get_configs_requiring_sync(
        &self,
        threshold_hours: i64,
    ) -> DomainResult<Vec<ServiceConfig>> {
        let configs = self.get_enabled_service_configs().await?;
        let threshold = chrono::Utc::now() - chrono::Duration::hours(threshold_hours);

        Ok(configs
            .into_iter()
            .filter(|config| {
                config
                    .last_sync
                    .map(|last_sync| last_sync < threshold)
                    .unwrap_or(true)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_service_config() {
        let mut mock_service = crate::domain::services::MockServiceConfigService::new();

        mock_service
            .expect_create_service_config()
            .withf(
                |name: &String,
                 service_type: &ServiceType,
                 auth_type: &AuthType,
                 auth_config: &AuthConfig,
                 endpoints: &ServiceEndpoints| {
                    name == "Test Service"
                        && matches!(service_type, ServiceType::Github)
                        && matches!(auth_type, AuthType::OAuth2)
                        && matches!(auth_config, AuthConfig::OAuth2(_))
                        && endpoints.base_url == "http://api.example.com"
                },
            )
            .returning(|name, service_type, auth_type, auth_config, endpoints| {
                Ok(ServiceConfig::new(
                    name,
                    service_type,
                    auth_type,
                    auth_config,
                    endpoints,
                ))
            });

        let use_cases = ServiceConfigUseCases::new(std::sync::Arc::new(mock_service));

        let result = use_cases
            .create_service_config(
                "Test Service".to_string(),
                ServiceType::Github,
                AuthType::OAuth2,
                AuthConfig::OAuth2(crate::domain::entities::OAuth2Config {
                    client_id: "test_client".to_string(),
                    client_secret: "test_secret".to_string(),
                    redirect_uri: "http://localhost:8080/callback".to_string(),
                    auth_url: "http://auth.example.com/oauth/authorize".to_string(),
                    token_url: "http://auth.example.com/oauth/token".to_string(),
                    scope: vec!["read".to_string(), "write".to_string()],
                    access_token: None,
                    refresh_token: None,
                    token_expires_at: None,
                })
                .clone(),
                ServiceEndpoints {
                    base_url: "http://api.example.com".to_string(),
                    endpoints: serde_json::Map::new(),
                }
                .clone(),
            )
            .await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, "Test Service");
        assert!(matches!(config.service_type, ServiceType::Github));
    }
}
