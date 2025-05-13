use crate::domain::entities::{AuthConfig, AuthType, ServiceEndpoints, ServiceType};
use crate::presentation::middleware::ValidatedCommand;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateServiceConfigRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,
    pub service_type: ServiceType,
    pub auth_type: AuthType,
    pub auth_config: AuthConfig,
    pub endpoints: ServiceEndpoints,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateServiceAuthRequest {
    pub auth_config: AuthConfig,
}

impl ValidatedCommand for CreateServiceConfigRequest {}
impl ValidatedCommand for UpdateServiceAuthRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceConfigResponse {
    pub id: String,
    pub name: String,
    pub service_type: ServiceType,
    pub auth_type: AuthType,
    pub endpoints: ServiceEndpoints,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_sync: Option<String>,
}

impl From<crate::domain::entities::ServiceConfig> for ServiceConfigResponse {
    fn from(config: crate::domain::entities::ServiceConfig) -> Self {
        Self {
            id: config.id.to_string(),
            name: config.name,
            service_type: config.service_type,
            auth_type: config.auth_type,
            endpoints: config.endpoints,
            enabled: config.enabled,
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
            last_sync: config.last_sync.map(|dt| dt.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceConfigListResponse {
    pub configs: Vec<ServiceConfigResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceConfigError {
    pub code: String,
    pub message: String,
    pub details: Vec<String>,
}

impl From<crate::domain::error::DomainError> for ServiceConfigError {
    fn from(error: crate::domain::error::DomainError) -> Self {
        match error {
            crate::domain::error::DomainError::ValidationError(msg) => Self {
                code: "VALIDATION_ERROR".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::NotFoundError(msg) => Self {
                code: "NOT_FOUND".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::UnauthorizedError(msg) => Self {
                code: "UNAUTHORIZED".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::ConflictError(msg) => Self {
                code: "CONFLICT".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::InternalError(msg) => Self {
                code: "INTERNAL_ERROR".to_string(),
                message: msg,
                details: vec![],
            },
        }
    }
}

impl From<crate::presentation::middleware::ValidationMiddlewareError> for ServiceConfigError {
    fn from(error: crate::presentation::middleware::ValidationMiddlewareError) -> Self {
        match error {
            crate::presentation::middleware::ValidationMiddlewareError::ValidationFailed(
                errors,
            ) => {
                let mut details = Vec::new();
                for (field, field_errors) in errors.field_errors() {
                    for error in field_errors {
                        details.push(format!(
                            "{}: {}",
                            field,
                            error
                                .message
                                .as_ref()
                                .map_or("Invalid value".to_string(), |m| m.to_string())
                        ));
                    }
                }
                Self {
                    code: "VALIDATION_ERROR".to_string(),
                    message: "Request validation failed".to_string(),
                    details,
                }
            }
            crate::presentation::middleware::ValidationMiddlewareError::DeserializationFailed(
                msg,
            ) => Self {
                code: "DESERIALIZATION_ERROR".to_string(),
                message: msg,
                details: vec![],
            },
        }
    }
}
