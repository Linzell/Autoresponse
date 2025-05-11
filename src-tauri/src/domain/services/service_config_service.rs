use crate::domain::{
    entities::{AuthConfig, AuthType, ServiceConfig, ServiceEndpoints, ServiceType},
    error::{DomainError, DomainResult},
    repositories::DynServiceConfigRepository,
};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ServiceConfigService: Send + Sync {
    async fn create_service_config(
        &self,
        name: String,
        service_type: ServiceType,
        auth_type: AuthType,
        auth_config: AuthConfig,
        endpoints: ServiceEndpoints,
    ) -> DomainResult<ServiceConfig>;

    async fn get_service_config(&self, id: Uuid) -> DomainResult<ServiceConfig>;
    async fn get_all_service_configs(&self) -> DomainResult<Vec<ServiceConfig>>;
    async fn get_configs_by_service_type(
        &self,
        service_type: ServiceType,
    ) -> DomainResult<Vec<ServiceConfig>>;
    async fn get_enabled_configs(&self) -> DomainResult<Vec<ServiceConfig>>;
    async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> DomainResult<()>;
    async fn enable_service(&self, id: Uuid) -> DomainResult<()>;
    async fn disable_service(&self, id: Uuid) -> DomainResult<()>;
    async fn update_last_sync(&self, id: Uuid) -> DomainResult<()>;
    async fn delete_service_config(&self, id: Uuid) -> DomainResult<()>;
}

pub struct DefaultServiceConfigService {
    repository: DynServiceConfigRepository,
}

impl DefaultServiceConfigService {
    pub fn new(repository: DynServiceConfigRepository) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl ServiceConfigService for DefaultServiceConfigService {
    async fn create_service_config(
        &self,
        name: String,
        service_type: ServiceType,
        auth_type: AuthType,
        auth_config: AuthConfig,
        endpoints: ServiceEndpoints,
    ) -> DomainResult<ServiceConfig> {
        let mut config = ServiceConfig::new(name, service_type, auth_type, auth_config, endpoints);
        self.repository.save(&mut config).await?;
        Ok(config)
    }

    async fn get_service_config(&self, id: Uuid) -> DomainResult<ServiceConfig> {
        self.repository.find_by_id(id).await?.ok_or_else(|| {
            DomainError::NotFoundError(format!("Service config with id {} not found", id))
        })
    }

    async fn get_all_service_configs(&self) -> DomainResult<Vec<ServiceConfig>> {
        self.repository.find_all().await
    }

    async fn get_configs_by_service_type(
        &self,
        service_type: ServiceType,
    ) -> DomainResult<Vec<ServiceConfig>> {
        self.repository.find_by_service_type(service_type).await
    }

    async fn get_enabled_configs(&self) -> DomainResult<Vec<ServiceConfig>> {
        self.repository.find_enabled().await
    }

    async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> DomainResult<()> {
        // Verify the service exists first
        self.get_service_config(id).await?;
        self.repository.update_auth_config(id, auth_config).await
    }

    async fn enable_service(&self, id: Uuid) -> DomainResult<()> {
        // Verify the service exists first
        self.get_service_config(id).await?;
        self.repository.update_enabled_status(id, true).await
    }

    async fn disable_service(&self, id: Uuid) -> DomainResult<()> {
        // Verify the service exists first
        self.get_service_config(id).await?;
        self.repository.update_enabled_status(id, false).await
    }

    async fn update_last_sync(&self, id: Uuid) -> DomainResult<()> {
        // Verify the service exists first
        self.get_service_config(id).await?;
        self.repository.update_last_sync(id).await
    }

    async fn delete_service_config(&self, id: Uuid) -> DomainResult<()> {
        // Verify the service exists first
        self.get_service_config(id).await?;
        self.repository.delete(id).await
    }
}

pub type DynServiceConfigService = Arc<dyn ServiceConfigService>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::OAuth2Config;
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        Repository {}

        #[async_trait]
        impl crate::domain::repositories::ServiceConfigRepository for Repository {
            async fn save(&self, config: &mut ServiceConfig) -> DomainResult<()>;
            async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<ServiceConfig>>;
            async fn find_all(&self) -> DomainResult<Vec<ServiceConfig>>;
            async fn find_by_service_type(&self, service_type: ServiceType) -> DomainResult<Vec<ServiceConfig>>;
            async fn find_enabled(&self) -> DomainResult<Vec<ServiceConfig>>;
            async fn delete(&self, id: Uuid) -> DomainResult<()>;
            async fn update_auth_config(&self, id: Uuid, auth_config: AuthConfig) -> DomainResult<()>;
            async fn update_enabled_status(&self, id: Uuid, enabled: bool) -> DomainResult<()>;
            async fn update_last_sync(&self, id: Uuid) -> DomainResult<()>;
        }
    }

    fn create_test_config() -> ServiceConfig {
        let oauth2_config = OAuth2Config {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
            auth_url: "http://auth.example.com/oauth/authorize".to_string(),
            token_url: "http://auth.example.com/oauth/token".to_string(),
            scope: vec!["read".to_string(), "write".to_string()],
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        };

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

        ServiceConfig::new(
            "Test Service".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth2_config),
            endpoints,
        )
    }

    #[tokio::test]
    async fn test_service_config_service() {
        let mut repository = MockRepository::new();
        let config = create_test_config();
        let config_id = config.id;

        // Test create_service_config
        repository.expect_save().returning(|_| Ok(()));

        let service = DefaultServiceConfigService::new(Arc::new(repository));

        let created_config = service
            .create_service_config(
                config.name.clone(),
                config.service_type.clone(),
                config.auth_type.clone(),
                config.auth_config.clone(),
                config.endpoints.clone(),
            )
            .await
            .unwrap();

        assert_eq!(created_config.name, config.name);
        assert!(matches!(created_config.service_type, ServiceType::Github));

        // Create a new mock repository for the remaining tests
        let mut repository = MockRepository::new();

        // Test get_service_config
        repository
            .expect_find_by_id()
            .with(mockall::predicate::eq(config_id))
            .returning(move |_| Ok(Some(config.clone())));

        let service = DefaultServiceConfigService::new(Arc::new(repository));

        let found_config = service.get_service_config(config_id).await.unwrap();
        assert_eq!(found_config.id, config_id);

        // Create a new mock repository for the update tests
        let mut repository = MockRepository::new();
        let config = create_test_config();

        // Setup expectations for the update operations
        repository
            .expect_find_by_id()
            .returning(move |_| Ok(Some(config.clone())));

        repository
            .expect_update_enabled_status()
            .returning(|_, _| Ok(()));

        repository
            .expect_update_auth_config()
            .with(
                mockall::predicate::eq(config_id),
                mockall::predicate::eq(AuthConfig::OAuth2(OAuth2Config {
                    access_token: Some("new_token".to_string()),
                    client_id: "test_client".to_string(),
                    client_secret: "test_secret".to_string(),
                    redirect_uri: "http://localhost:8080/callback".to_string(),
                    auth_url: "http://auth.example.com/oauth/authorize".to_string(),
                    token_url: "http://auth.example.com/oauth/token".to_string(),
                    scope: vec!["read".to_string(), "write".to_string()],
                    refresh_token: None,
                    token_expires_at: None,
                })),
            )
            .returning(|_, _| Ok(()));

        repository.expect_update_last_sync().returning(|_| Ok(()));

        let service = DefaultServiceConfigService::new(Arc::new(repository));

        // Test enable/disable service
        service.enable_service(config_id).await.unwrap();
        service.disable_service(config_id).await.unwrap();

        // Test update auth config
        service
            .update_auth_config(
                config_id,
                AuthConfig::OAuth2(OAuth2Config {
                    access_token: Some("new_token".to_string()),
                    client_id: "test_client".to_string(),
                    client_secret: "test_secret".to_string(),
                    redirect_uri: "http://localhost:8080/callback".to_string(),
                    auth_url: "http://auth.example.com/oauth/authorize".to_string(),
                    token_url: "http://auth.example.com/oauth/token".to_string(),
                    scope: vec!["read".to_string(), "write".to_string()],
                    refresh_token: None,
                    token_expires_at: None,
                }),
            )
            .await
            .unwrap();

        // Test update last sync
        service.update_last_sync(config_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_service_config_not_found() {
        let mut repository = MockRepository::new();
        let id = Uuid::new_v4();

        repository.expect_find_by_id().returning(|_| Ok(None));

        let service = DefaultServiceConfigService::new(Arc::new(repository));

        let result = service.get_service_config(id).await;
        assert!(matches!(result, Err(DomainError::NotFoundError(_))));
    }
}
