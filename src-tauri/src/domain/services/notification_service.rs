use crate::domain::{
    entities::{Notification, NotificationMetadata, NotificationPriority, NotificationStatus},
    error::{DomainError, DomainResult},
    repositories::DynNotificationRepository,
};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn create_notification(
        &self,
        title: String,
        content: String,
        priority: NotificationPriority,
        metadata: NotificationMetadata,
    ) -> DomainResult<Notification>;

    async fn get_notification(&self, id: Uuid) -> DomainResult<Notification>;
    async fn get_all_notifications(&self) -> DomainResult<Vec<Notification>>;
    async fn get_notifications_by_status(
        &self,
        status: NotificationStatus,
    ) -> DomainResult<Vec<Notification>>;
    async fn mark_as_read(&self, id: Uuid) -> DomainResult<()>;
    async fn mark_action_required(&self, id: Uuid) -> DomainResult<()>;
    async fn mark_action_taken(&self, id: Uuid) -> DomainResult<()>;
    async fn archive_notification(&self, id: Uuid) -> DomainResult<()>;
    async fn delete_notification(&self, id: Uuid) -> DomainResult<()>;
}

pub struct DefaultNotificationService {
    repository: DynNotificationRepository,
}

impl DefaultNotificationService {
    pub fn new(repository: DynNotificationRepository) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl NotificationService for DefaultNotificationService {
    async fn create_notification(
        &self,
        title: String,
        content: String,
        priority: NotificationPriority,
        metadata: NotificationMetadata,
    ) -> DomainResult<Notification> {
        let mut notification = Notification::new(title, content, priority, metadata);
        self.repository.save(&mut notification).await?;
        Ok(notification)
    }

    async fn get_notification(&self, id: Uuid) -> DomainResult<Notification> {
        self.repository.find_by_id(id).await?.ok_or_else(|| {
            DomainError::NotFoundError(format!("Notification with id {} not found", id))
        })
    }

    async fn get_all_notifications(&self) -> DomainResult<Vec<Notification>> {
        self.repository.find_all().await
    }

    async fn get_notifications_by_status(
        &self,
        status: NotificationStatus,
    ) -> DomainResult<Vec<Notification>> {
        self.repository.find_by_status(status).await
    }

    async fn mark_as_read(&self, id: Uuid) -> DomainResult<()> {
        let mut notification = self.get_notification(id).await?;
        notification.mark_as_read();
        self.repository.save(&mut notification).await
    }

    async fn mark_action_required(&self, id: Uuid) -> DomainResult<()> {
        let mut notification = self.get_notification(id).await?;
        notification.mark_action_required();
        self.repository.save(&mut notification).await
    }

    async fn mark_action_taken(&self, id: Uuid) -> DomainResult<()> {
        let mut notification = self.get_notification(id).await?;
        notification.mark_action_taken();
        self.repository.save(&mut notification).await
    }

    async fn archive_notification(&self, id: Uuid) -> DomainResult<()> {
        let mut notification = self.get_notification(id).await?;
        notification.archive();
        self.repository.save(&mut notification).await
    }

    async fn delete_notification(&self, id: Uuid) -> DomainResult<()> {
        self.repository.delete(id).await
    }
}

pub type DynNotificationService = Arc<dyn NotificationService>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::NotificationSource;
    use crate::domain::repositories::NotificationRepository;
    use async_trait::async_trait;
    use mockall::mock;
    use std::collections::HashMap;
    use std::sync::Mutex;

    mock! {
        Repository {}

        #[async_trait]
        impl NotificationRepository for Repository {
            async fn save(&self, notification: &mut Notification) -> DomainResult<()>;
            async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<Notification>>;
            async fn find_all(&self) -> DomainResult<Vec<Notification>>;
            async fn find_by_status(&self, status: NotificationStatus) -> DomainResult<Vec<Notification>>;
            async fn find_by_source(&self, source: NotificationSource) -> DomainResult<Vec<Notification>>;
            async fn delete(&self, id: Uuid) -> DomainResult<()>;
            async fn update_status(&self, id: Uuid, status: NotificationStatus) -> DomainResult<()>;
        }
    }

    struct TestRepository {
        notifications: Mutex<HashMap<Uuid, Notification>>,
    }

    #[async_trait]
    impl NotificationRepository for TestRepository {
        async fn save(&self, notification: &mut Notification) -> DomainResult<()> {
            let mut notifications = self.notifications.lock().unwrap();
            notifications.insert(notification.id, notification.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<Notification>> {
            let notifications = self.notifications.lock().unwrap();
            Ok(notifications.get(&id).cloned())
        }

        async fn find_all(&self) -> DomainResult<Vec<Notification>> {
            let notifications = self.notifications.lock().unwrap();
            Ok(notifications.values().cloned().collect())
        }

        async fn find_by_status(
            &self,
            status: NotificationStatus,
        ) -> DomainResult<Vec<Notification>> {
            let notifications = self.notifications.lock().unwrap();
            Ok(notifications
                .values()
                .filter(|n| matches!(&n.status, s if *s == status))
                .cloned()
                .collect())
        }

        async fn find_by_source(
            &self,
            source: NotificationSource,
        ) -> DomainResult<Vec<Notification>> {
            let notifications = self.notifications.lock().unwrap();
            Ok(notifications
                .values()
                .filter(|n| matches!(&n.metadata.source, s if *s == source))
                .cloned()
                .collect())
        }

        async fn delete(&self, id: Uuid) -> DomainResult<()> {
            let mut notifications = self.notifications.lock().unwrap();
            notifications.remove(&id);
            Ok(())
        }

        async fn update_status(&self, id: Uuid, status: NotificationStatus) -> DomainResult<()> {
            let mut notifications = self.notifications.lock().unwrap();
            if let Some(notification) = notifications.get_mut(&id) {
                notification.status = status;
                notification.updated_at = chrono::Utc::now();
                Ok(())
            } else {
                Err(DomainError::NotFoundError(format!(
                    "Notification with id {} not found",
                    id
                )))
            }
        }
    }

    #[tokio::test]
    async fn test_notification_lifecycle() {
        let repository = Arc::new(TestRepository {
            notifications: Mutex::new(HashMap::new()),
        });
        let service = DefaultNotificationService::new(repository);

        // Create a notification
        let metadata = NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test123".to_string()),
            url: Some("https://example.com".to_string()),
            tags: vec!["test".to_string()],
            custom_data: None,
        };

        let notification = service
            .create_notification(
                "Test".to_string(),
                "Test Content".to_string(),
                NotificationPriority::Medium,
                metadata,
            )
            .await
            .unwrap();

        // Verify creation
        let retrieved = service.get_notification(notification.id).await.unwrap();
        assert_eq!(retrieved.title, "Test");
        assert_eq!(retrieved.status, NotificationStatus::New);

        // Test status changes
        service.mark_as_read(notification.id).await.unwrap();
        let retrieved = service.get_notification(notification.id).await.unwrap();
        assert_eq!(retrieved.status, NotificationStatus::Read);

        service.mark_action_required(notification.id).await.unwrap();
        let retrieved = service.get_notification(notification.id).await.unwrap();
        assert_eq!(retrieved.status, NotificationStatus::ActionRequired);

        service.mark_action_taken(notification.id).await.unwrap();
        let retrieved = service.get_notification(notification.id).await.unwrap();
        assert_eq!(retrieved.status, NotificationStatus::ActionTaken);

        service.archive_notification(notification.id).await.unwrap();
        let retrieved = service.get_notification(notification.id).await.unwrap();
        assert_eq!(retrieved.status, NotificationStatus::Archived);

        // Test deletion
        service.delete_notification(notification.id).await.unwrap();
        let result = service.get_notification(notification.id).await;
        assert!(result.is_err());
    }
}
