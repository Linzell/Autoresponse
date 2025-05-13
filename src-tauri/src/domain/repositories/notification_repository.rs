use crate::domain::{entities::Notification, error::DomainResult};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn save(&self, notification: &mut Notification) -> DomainResult<()>;
    async fn find_by_id(&self, id: uuid::Uuid) -> DomainResult<Option<Notification>>;
    async fn find_all(&self) -> DomainResult<Vec<Notification>>;
    async fn find_by_status(
        &self,
        status: crate::domain::entities::NotificationStatus,
    ) -> DomainResult<Vec<Notification>>;
    async fn find_by_source(
        &self,
        source: crate::domain::entities::NotificationSource,
    ) -> DomainResult<Vec<Notification>>;
    async fn delete(&self, id: uuid::Uuid) -> DomainResult<()>;
    async fn update_status(
        &self,
        id: uuid::Uuid,
        status: crate::domain::entities::NotificationStatus,
    ) -> DomainResult<()>;
}

pub type DynNotificationRepository = Arc<dyn NotificationRepository>;
