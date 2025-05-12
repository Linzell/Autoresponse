use async_trait::async_trait;
use std::sync::Arc;

use super::NotificationEvent;
use crate::domain::error::DomainResult;

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish_event(&self, event: NotificationEvent) -> DomainResult<()>;
}

pub type DynEventPublisher = Arc<dyn EventPublisher>;

#[derive(Default)]
pub struct NoopEventPublisher;

#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish_event(&self, _event: NotificationEvent) -> DomainResult<()> {
        Ok(())
    }
}