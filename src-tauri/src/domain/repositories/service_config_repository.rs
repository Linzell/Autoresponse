use crate::domain::{entities::ServiceConfig, error::DomainResult};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait ServiceConfigRepository: Send + Sync {
    async fn save(&self, config: &mut ServiceConfig) -> DomainResult<()>;
    async fn find_by_id(&self, id: uuid::Uuid) -> DomainResult<Option<ServiceConfig>>;
    async fn find_all(&self) -> DomainResult<Vec<ServiceConfig>>;
    async fn find_by_service_type(
        &self,
        service_type: crate::domain::entities::ServiceType,
    ) -> DomainResult<Vec<ServiceConfig>>;
    async fn find_enabled(&self) -> DomainResult<Vec<ServiceConfig>>;
    async fn delete(&self, id: uuid::Uuid) -> DomainResult<()>;
    async fn update_auth_config(
        &self,
        id: uuid::Uuid,
        auth_config: crate::domain::entities::AuthConfig,
    ) -> DomainResult<()>;
    async fn update_enabled_status(&self, id: uuid::Uuid, enabled: bool) -> DomainResult<()>;
    async fn update_last_sync(&self, id: uuid::Uuid) -> DomainResult<()>;
}

pub type DynServiceConfigRepository = Arc<dyn ServiceConfigRepository>;
