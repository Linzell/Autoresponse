use crate::domain::{
    entities::{
        Notification, NotificationMetadata, NotificationPriority, NotificationSource,
        NotificationStatus,
    },
    error::DomainResult,
    services::DynNotificationService,
};
use uuid::Uuid;

pub struct NotificationUseCases {
    notification_service: DynNotificationService,
}

impl NotificationUseCases {
    pub fn new(notification_service: DynNotificationService) -> Self {
        Self {
            notification_service,
        }
    }

    pub async fn create_notification(
        &self,
        title: String,
        content: String,
        priority: NotificationPriority,
        source: NotificationSource,
        external_id: Option<String>,
        url: Option<String>,
        tags: Vec<String>,
        custom_data: Option<serde_json::Value>,
    ) -> DomainResult<Notification> {
        let metadata = NotificationMetadata {
            source,
            external_id,
            url,
            tags,
            custom_data,
        };

        self.notification_service
            .create_notification(title, content, priority, metadata)
            .await
    }

    pub async fn get_notification(&self, id: Uuid) -> DomainResult<Notification> {
        self.notification_service.get_notification(id).await
    }

    pub async fn get_notifications_by_status(
        &self,
        status: NotificationStatus,
    ) -> DomainResult<Vec<Notification>> {
        let notifications = self.notification_service.get_all_notifications().await?;
        Ok(notifications
            .into_iter()
            .filter(|n| n.status == status)
            .collect())
    }

    pub async fn get_notifications_by_source(
        &self,
        source: NotificationSource,
    ) -> DomainResult<Vec<Notification>> {
        let notifications = self.notification_service.get_all_notifications().await?;
        Ok(notifications
            .into_iter()
            .filter(|n| n.metadata.source == source)
            .collect())
    }

    pub async fn get_recent_notifications(&self, limit: usize) -> DomainResult<Vec<Notification>> {
        let mut notifications = self.notification_service.get_all_notifications().await?;
        notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(notifications.into_iter().take(limit).collect())
    }

    pub async fn get_unread_notifications(&self) -> DomainResult<Vec<Notification>> {
        let notifications = self.notification_service.get_all_notifications().await?;
        Ok(notifications
            .into_iter()
            .filter(|n| matches!(n.status, NotificationStatus::New))
            .collect())
    }

    pub async fn get_action_required_notifications(&self) -> DomainResult<Vec<Notification>> {
        self.get_notifications_by_status(NotificationStatus::ActionRequired)
            .await
    }

    pub async fn mark_as_read(&self, id: Uuid) -> DomainResult<()> {
        self.notification_service.mark_as_read(id).await
    }

    pub async fn mark_action_required(&self, id: Uuid) -> DomainResult<()> {
        self.notification_service.mark_action_required(id).await
    }

    pub async fn mark_action_taken(&self, id: Uuid) -> DomainResult<()> {
        self.notification_service.mark_action_taken(id).await
    }

    pub async fn archive_notification(&self, id: Uuid) -> DomainResult<()> {
        self.notification_service.archive_notification(id).await
    }

    pub async fn delete_notification(&self, id: Uuid) -> DomainResult<()> {
        self.notification_service.delete_notification(id).await
    }

    pub async fn bulk_mark_as_read(&self, ids: Vec<Uuid>) -> DomainResult<()> {
        for id in ids {
            self.mark_as_read(id).await?;
        }
        Ok(())
    }

    pub async fn bulk_archive(&self, ids: Vec<Uuid>) -> DomainResult<()> {
        for id in ids {
            self.archive_notification(id).await?;
        }
        Ok(())
    }

    pub async fn cleanup_old_notifications(&self, days: i64) -> DomainResult<()> {
        let notifications = self.notification_service.get_all_notifications().await?;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);

        for notification in notifications {
            if notification.created_at < cutoff {
                self.delete_notification(notification.id).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_notification() {
        let mut mock_service = crate::domain::services::MockNotificationService::new();

        mock_service
            .expect_create_notification()
            .withf(
                |title: &String,
                 content: &String,
                 priority: &NotificationPriority,
                 metadata: &NotificationMetadata| {
                    title == "Test Title"
                        && content == "Test Content"
                        && matches!(priority, NotificationPriority::Medium)
                        && matches!(metadata.source, NotificationSource::Email)
                },
            )
            .returning(|title, content, priority, metadata| {
                Ok(Notification::new(title, content, priority, metadata))
            });

        let use_cases = NotificationUseCases::new(std::sync::Arc::new(mock_service));

        let result = use_cases
            .create_notification(
                "Test Title".to_string(),
                "Test Content".to_string(),
                NotificationPriority::Medium,
                NotificationSource::Email,
                Some("test123".to_string()),
                None,
                vec!["test".to_string()],
                None,
            )
            .await;

        assert!(result.is_ok());
        let notification = result.unwrap();
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.content, "Test Content");
        assert!(matches!(
            notification.priority,
            NotificationPriority::Medium
        ));
    }
}
