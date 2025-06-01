use crate::domain::{
    entities::{
        notification::{
            Notification, NotificationMetadata, NotificationPriority, NotificationSource,
        },
        service_config::{AuthConfig, ServiceConfig, ServiceType},
    },
    error::DomainResult,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationEvent {
    pub id: String,
    pub event_type: String,
    pub source: NotificationSource,
    pub created_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationCredentials {
    pub auth_config: AuthConfig,
    pub endpoints: serde_json::Map<String, serde_json::Value>,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait IntegrationService: Send + Sync + std::fmt::Debug {
    /// Returns the type of service this integration handles
    fn service_type(&self) -> ServiceType;

    /// Initialize the service with configuration
    async fn initialize(&self, config: ServiceConfig) -> DomainResult<()>;

    /// Test the connection and credentials
    async fn test_connection(&self) -> DomainResult<bool>;

    /// Sync notifications from the service
    async fn sync_notifications(&self) -> DomainResult<Vec<Notification>>;

    /// Create a notification from an integration event
    async fn create_notification_from_event(
        &self,
        event: IntegrationEvent,
    ) -> DomainResult<Notification>;

    /// Send a response back to the service
    async fn send_response(&self, notification: &Notification, response: &str) -> DomainResult<()>;

    /// Execute an action in the service
    async fn execute_action(
        &self,
        notification: &Notification,
        action_type: &str,
        payload: serde_json::Value,
    ) -> DomainResult<()>;
}

pub type DynIntegrationService = Arc<dyn IntegrationService>;

/// Common implementation for converting integration events to notifications
impl dyn IntegrationService {
    fn event_to_notification(
        &self,
        event: &IntegrationEvent,
        title: String,
        content: String,
        priority: NotificationPriority,
    ) -> Notification {
        let metadata = NotificationMetadata {
            source: event.source.clone(),
            external_id: Some(event.id.clone()),
            url: None,
            tags: vec![event.event_type.clone()],
            custom_data: Some(event.payload.clone()),
        };

        Notification::new(title, content, priority, metadata)
    }
}

pub mod github;
pub mod gitlab;
pub mod google;
pub mod jira;
pub mod linkedin;
pub mod manager;
pub mod microsoft;
pub mod service_bridge;

// Re-export individual services
pub use github::GithubService;
pub use gitlab::GitlabService;
pub use google::GoogleService;
pub use jira::JiraService;
pub use linkedin::LinkedInService;
pub use microsoft::MicrosoftService;
