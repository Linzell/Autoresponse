use crate::domain::{entities::Notification, error::DomainResult, NotificationSource};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EmailActionData {
    thread_id: String,
    subject: Option<String>,
    recipient: Option<String>,
    response_template: Option<String>,
}

#[derive(Debug)]
pub struct ActionExecutor;

pub type DynActionExecutor = Arc<dyn ActionExecutorTrait>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ActionExecutorTrait: Send + Sync {
    async fn execute(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_email(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_github(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_gitlab(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_jira(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_microsoft(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_google(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_linkedin(&self, notification: &Notification) -> DomainResult<()>;
    async fn handle_custom(&self, notification: &Notification, service: &str) -> DomainResult<()>;
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
impl ActionExecutorTrait for ActionExecutor {
    async fn execute(&self, notification: &Notification) -> DomainResult<()> {
        match notification.metadata.source {
            NotificationSource::Email => self.handle_email(notification).await,
            NotificationSource::Github => self.handle_github(notification).await,
            NotificationSource::Gitlab => self.handle_gitlab(notification).await,
            NotificationSource::Jira => self.handle_jira(notification).await,
            NotificationSource::Microsoft => self.handle_microsoft(notification).await,
            NotificationSource::Google => self.handle_google(notification).await,
            NotificationSource::LinkedIn => self.handle_linkedin(notification).await,
            NotificationSource::Custom(ref service) => {
                self.handle_custom(notification, service).await
            }
        }
    }

    async fn handle_email(&self, notification: &Notification) -> DomainResult<()> {
        if let Some(data) = &notification.metadata.custom_data {
            match serde_json::from_value::<EmailActionData>(data.clone()) {
                Ok(email_data) => {
                    tracing::info!(
                        "Processing email action with thread ID: {}",
                        email_data.thread_id
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to deserialize EmailActionData: {}. Data: {:?}",
                        e,
                        data
                    );
                }
            }
        }
        Ok(())
    }

    async fn handle_github(&self, notification: &Notification) -> DomainResult<()> {
        if let Some(url) = &notification.metadata.url {
            tracing::info!("Processing GitHub action for URL: {}", url);
        }
        Ok(())
    }

    async fn handle_gitlab(&self, notification: &Notification) -> DomainResult<()> {
        if let Some(url) = &notification.metadata.url {
            tracing::info!("Processing GitLab action for URL: {}", url);
        }
        Ok(())
    }

    async fn handle_jira(&self, notification: &Notification) -> DomainResult<()> {
        if let Some(external_id) = &notification.metadata.external_id {
            tracing::info!("Processing Jira action for ticket: {}", external_id);
        }
        Ok(())
    }

    async fn handle_microsoft(&self, notification: &Notification) -> DomainResult<()> {
        if let Some(_data) = &notification.metadata.custom_data {
            tracing::info!("Processing Microsoft action with custom data");
        }
        Ok(())
    }

    async fn handle_google(&self, notification: &Notification) -> DomainResult<()> {
        if let Some(_data) = &notification.metadata.custom_data {
            tracing::info!("Processing Google action with custom data");
        }
        Ok(())
    }

    async fn handle_linkedin(&self, notification: &Notification) -> DomainResult<()> {
        if let Some(url) = &notification.metadata.url {
            tracing::info!("Processing LinkedIn action for URL: {}", url);
        }
        Ok(())
    }

    async fn handle_custom(&self, notification: &Notification, service: &str) -> DomainResult<()> {
        tracing::info!("Processing custom action for service: {}", service);
        if let Some(_data) = &notification.metadata.custom_data {
            // Handle custom service integration based on service name and data
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::{NotificationMetadata, NotificationPriority},
        NotificationSource,
    };
    use serde_json::json;

    #[tokio::test]
    async fn test_execute_email_action() {
        let executor = ActionExecutor::new();
        let notification = create_test_notification(
            NotificationSource::Email,
            Some(json!({
                "thread_id": "123",
                "subject": "Test",
            })),
        );

        let result = executor.execute(&notification).await;
        assert!(result.is_ok());
    }

    fn create_test_notification(
        source: NotificationSource,
        custom_data: Option<serde_json::Value>,
    ) -> Notification {
        Notification::new(
            "Test Title".to_string(),
            "Test Content".to_string(),
            NotificationPriority::Medium,
            NotificationMetadata {
                source,
                external_id: Some("test-id".to_string()),
                url: Some("http://test.com".to_string()),
                tags: vec!["test".to_string()],
                custom_data,
            },
        )
    }
}
