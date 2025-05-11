use crate::{
    domain::services::ServiceConfigService,
    presentation::dtos::{
        CreateServiceConfigRequest, ServiceConfigError, ServiceConfigListResponse,
        ServiceConfigResponse, UpdateServiceAuthRequest,
    },
};
use std::sync::Arc;
use uuid::Uuid;

pub struct ServiceConfigController {
    service: Arc<dyn ServiceConfigService>,
}

impl ServiceConfigController {
    pub fn new(service: Arc<dyn ServiceConfigService>) -> Self {
        Self { service }
    }

    pub async fn create_service_config(
        &self,
        request: CreateServiceConfigRequest,
    ) -> Result<ServiceConfigResponse, ServiceConfigError> {
        let config = self
            .service
            .create_service_config(
                request.name,
                request.service_type,
                request.auth_type,
                request.auth_config,
                request.endpoints,
            )
            .await
            .map_err(ServiceConfigError::from)?;

        Ok(config.into())
    }

    pub async fn get_service_config(
        &self,
        id: String,
    ) -> Result<ServiceConfigResponse, ServiceConfigError> {
        let id = Uuid::parse_str(&id).map_err(|e| ServiceConfigError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        let config = self
            .service
            .get_service_config(id)
            .await
            .map_err(ServiceConfigError::from)?;

        Ok(config.into())
    }

    pub async fn get_all_service_configs(
        &self,
    ) -> Result<ServiceConfigListResponse, ServiceConfigError> {
        let configs = self
            .service
            .get_all_service_configs()
            .await
            .map_err(ServiceConfigError::from)?;

        let responses: Vec<_> = configs.iter().map(|c| c.clone().into()).collect();
        let total = responses.len();
        let response = ServiceConfigListResponse {
            configs: responses,
            total,
        };

        Ok(response)
    }

    pub async fn get_configs_by_service_type(
        &self,
        service_type: crate::domain::entities::ServiceType,
    ) -> Result<ServiceConfigListResponse, ServiceConfigError> {
        let configs = self
            .service
            .get_configs_by_service_type(service_type)
            .await
            .map_err(ServiceConfigError::from)?;

        let responses: Vec<_> = configs.iter().map(|c| c.clone().into()).collect();
        let total = responses.len();
        let response = ServiceConfigListResponse {
            configs: responses,
            total,
        };

        Ok(response)
    }

    pub async fn update_auth_config(
        &self,
        id: String,
        request: UpdateServiceAuthRequest,
    ) -> Result<(), ServiceConfigError> {
        let id = Uuid::parse_str(&id).map_err(|e| ServiceConfigError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .update_auth_config(id, request.auth_config)
            .await
            .map_err(ServiceConfigError::from)
    }

    pub async fn enable_service(&self, id: String) -> Result<(), ServiceConfigError> {
        let id = Uuid::parse_str(&id).map_err(|e| ServiceConfigError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .enable_service(id)
            .await
            .map_err(ServiceConfigError::from)
    }

    pub async fn disable_service(&self, id: String) -> Result<(), ServiceConfigError> {
        let id = Uuid::parse_str(&id).map_err(|e| ServiceConfigError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .disable_service(id)
            .await
            .map_err(ServiceConfigError::from)
    }

    pub async fn delete_service_config(&self, id: String) -> Result<(), ServiceConfigError> {
        let id = Uuid::parse_str(&id).map_err(|e| ServiceConfigError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .delete_service_config(id)
            .await
            .map_err(ServiceConfigError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        AuthConfig, AuthType, OAuth2Config, ServiceEndpoints, ServiceType,
    };

    use serde_json::json;

    fn create_test_request() -> CreateServiceConfigRequest {
        CreateServiceConfigRequest {
            name: "Test Service".to_string(),
            service_type: ServiceType::Github,
            auth_type: AuthType::OAuth2,
            auth_config: AuthConfig::OAuth2(OAuth2Config {
                client_id: "test_client".to_string(),
                client_secret: "test_secret".to_string(),
                redirect_uri: "http://localhost:8080/callback".to_string(),
                auth_url: "http://auth.example.com/oauth/authorize".to_string(),
                token_url: "http://auth.example.com/oauth/token".to_string(),
                scope: vec!["read".to_string(), "write".to_string()],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            }),
            endpoints: ServiceEndpoints {
                base_url: "http://api.example.com".to_string(),
                endpoints: {
                    let mut map = serde_json::Map::new();
                    map.insert(
                        "test".to_string(),
                        json!({
                            "path": "/test",
                            "method": "GET"
                        }),
                    );
                    map
                },
            },
        }
    }

    #[tokio::test]
    async fn test_create_service_config() {
        let mut mock_service = crate::domain::services::MockServiceConfigService::new();
        let request = create_test_request();

        // Clone the request values we need to compare in the closure
        let expected_name = request.name.clone();
        let expected_service_type = request.service_type.clone();
        let expected_auth_type = request.auth_type.clone();
        let expected_auth_config = request.auth_config.clone();
        let expected_endpoints = request.endpoints.clone();

        mock_service
            .expect_create_service_config()
            .withf(
                move |name: &String,
                      service_type: &ServiceType,
                      auth_type: &AuthType,
                      auth_config: &AuthConfig,
                      endpoints: &ServiceEndpoints| {
                    *name == expected_name
                        && service_type == &expected_service_type
                        && auth_type == &expected_auth_type
                        && auth_config == &expected_auth_config
                        && endpoints == &expected_endpoints
                },
            )
            .returning(|name, service_type, auth_type, auth_config, endpoints| {
                Ok(crate::domain::entities::ServiceConfig::new(
                    name,
                    service_type,
                    auth_type,
                    auth_config,
                    endpoints,
                ))
            });

        let controller = ServiceConfigController::new(Arc::new(mock_service));
        let result = controller.create_service_config(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.name, "Test Service");
        assert!(matches!(response.service_type, ServiceType::Github));
    }
}
