use crate::domain::{
    entities::{notification::Notification, service_config::ServiceConfig},
    error::DomainResult,
    services::DynNotificationService,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::manager::IntegrationManager;

#[derive(Debug)]
pub struct ServiceBridge {
    integration_manager: Arc<IntegrationManager>,
    notification_service: DynNotificationService,
    sync_interval: Arc<RwLock<std::time::Duration>>,
}

impl ServiceBridge {
    pub fn new(
        integration_manager: Arc<IntegrationManager>,
        notification_service: DynNotificationService,
    ) -> Self {
        Self {
            integration_manager,
            notification_service,
            sync_interval: Arc::new(RwLock::new(std::time::Duration::from_secs(300))), // 5 minutes default
        }
    }

    pub async fn initialize_service(&self, config: ServiceConfig) -> DomainResult<()> {
        self.integration_manager.initialize_service(config).await
    }

    pub async fn set_sync_interval(&self, interval: std::time::Duration) {
        let mut current = self.sync_interval.write().await;
        *current = interval;
    }

    pub async fn start_sync_loop(&self) -> DomainResult<tokio::task::JoinHandle<()>> {
        let integration_manager = Arc::clone(&self.integration_manager);
        let notification_service = Arc::clone(&self.notification_service);
        let sync_interval = Arc::clone(&self.sync_interval);

        let handle = tokio::spawn(async move {
            loop {
                match Self::sync_notifications(&integration_manager, &notification_service).await {
                    Ok(_) => log::info!("Successfully synced notifications"),
                    Err(e) => log::error!("Error syncing notifications: {}", e),
                }

                let interval = *sync_interval.read().await;
                tokio::time::sleep(interval).await;
            }
        });

        Ok(handle)
    }

    async fn sync_notifications(
        integration_manager: &IntegrationManager,
        notification_service: &DynNotificationService,
    ) -> DomainResult<()> {
        let notifications = integration_manager.sync_all_notifications().await?;

        for notification in notifications {
            if let Err(e) = notification_service
                .create_notification(
                    notification.title.clone(),
                    notification.content.clone(),
                    notification.priority,
                    notification.metadata.clone(),
                )
                .await
            {
                log::error!(
                    "Failed to create notification from service {}: {}",
                    notification.metadata.source.to_string(),
                    e
                );
            }
        }

        Ok(())
    }

    pub async fn process_notification(&self, notification: &Notification) -> DomainResult<()> {
        // Get the appropriate service for this notification
        let service = self
            .integration_manager
            .get_service_for_source(&notification.metadata.source)
            .await?;

        // Generate response using the notification service's AI capabilities
        let response = self
            .notification_service
            .generate_response(notification)
            .await?;

        // Send the response through the integration service
        service.send_response(notification, &response).await?;

        // Mark the notification as handled
        self.notification_service
            .mark_action_taken(notification.id)
            .await?;

        Ok(())
    }

    pub async fn test_connections(&self) -> DomainResult<bool> {
        let results = self.integration_manager.test_connections().await;
        let all_connected = results.values().all(|&connected| connected);

        if !all_connected {
            for (service_type, connected) in results {
                if !connected {
                    log::warn!("Service {:?} is not connected", service_type);
                }
            }
        }

        Ok(all_connected)
    }

    pub async fn execute_action(
        &self,
        notification: &Notification,
        action_type: &str,
        payload: serde_json::Value,
    ) -> DomainResult<()> {
        let service = self
            .integration_manager
            .get_service_for_source(&notification.metadata.source)
            .await?;

        service
            .execute_action(notification, action_type, payload)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::service_config::{AuthConfig, AuthType, OAuth2Config, ServiceEndpoints},
        services::MockNotificationService,
        ServiceType,
    };

    #[tokio::test]
    async fn test_service_bridge_initialization() {
        let integration_manager = Arc::new(IntegrationManager::new());
        let mock_notification_service = MockNotificationService::new();

        let bridge = ServiceBridge::new(
            integration_manager.clone(),
            Arc::new(mock_notification_service),
        );

        let config = ServiceConfig::new(
            "Test".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["test".to_string()],
                access_token: Some("test".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "test".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(bridge.initialize_service(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_service_bridge_sync_interval() {
        let integration_manager = Arc::new(IntegrationManager::new());
        let mock_notification_service = MockNotificationService::new();

        let bridge = ServiceBridge::new(integration_manager, Arc::new(mock_notification_service));

        let new_interval = std::time::Duration::from_secs(600);
        bridge.set_sync_interval(new_interval).await;

        let current_interval = *bridge.sync_interval.read().await;
        assert_eq!(current_interval, new_interval);
    }
}
