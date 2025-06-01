use anyhow::Result;
use autoresponse_lib::domain::{
    entities::{Notification, NotificationPriority, NotificationSource, NotificationStatus},
    error::DomainError,
    events::{notification_events::NotificationEvent, publisher::EventPublisher},
    repositories::notification_repository::NotificationRepository,
    services::{
        Job, JobHandler, JobType, NotificationActionType, NotificationProcessor,
        NotificationService,
    },
};
use serde_json::json;
use std::sync::Arc;
use tokio;
use uuid::Uuid;

#[derive(Debug)]
struct TestNotificationRepository {
    notifications: Arc<tokio::sync::Mutex<Vec<Notification>>>,
}

impl TestNotificationRepository {
    fn new() -> Self {
        Self {
            notifications: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl NotificationRepository for TestNotificationRepository {
    async fn save(&self, notification: &mut Notification) -> Result<(), DomainError> {
        let mut notifications = self.notifications.lock().await;
        notifications.retain(|n| n.id != notification.id);
        notifications.push(notification.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError> {
        let notifications = self.notifications.lock().await;
        Ok(notifications.iter().find(|n| n.id == id).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Notification>, DomainError> {
        let notifications = self.notifications.lock().await;
        Ok(notifications.clone())
    }

    async fn find_by_status(
        &self,
        status: NotificationStatus,
    ) -> Result<Vec<Notification>, DomainError> {
        let notifications = self.notifications.lock().await;
        Ok(notifications
            .iter()
            .filter(|n| n.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_source(
        &self,
        source: NotificationSource,
    ) -> Result<Vec<Notification>, DomainError> {
        let notifications = self.notifications.lock().await;
        Ok(notifications
            .iter()
            .filter(|n| n.metadata.source == source)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut notifications = self.notifications.lock().await;
        notifications.retain(|n| n.id != id);
        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> Result<(), DomainError> {
        let mut notifications = self.notifications.lock().await;
        if let Some(notification) = notifications.iter_mut().find(|n| n.id == id) {
            notification.status = status;
            Ok(())
        } else {
            Err(DomainError::NotFoundError(format!(
                "Notification {} not found",
                id
            )))
        }
    }
}

#[derive(Debug)]
struct TestNotificationService;

#[async_trait::async_trait]
impl NotificationService for TestNotificationService {
    async fn analyze_notification_content(
        &self,
        _notification: &Notification,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }

    async fn generate_response(&self, _notification: &Notification) -> Result<String, DomainError> {
        Ok("Test response generated".to_string())
    }

    async fn execute_action(&self, _notification: &Notification) -> Result<(), DomainError> {
        Ok(())
    }

    // Implement required methods with minimal functionality
    async fn create_notification(
        &self,
        title: String,
        content: String,
        priority: NotificationPriority,
        metadata: autoresponse_lib::domain::entities::NotificationMetadata,
    ) -> Result<Notification, DomainError> {
        Ok(Notification::new(title, content, priority, metadata))
    }

    async fn get_notification(&self, _id: Uuid) -> Result<Notification, DomainError> {
        Err(DomainError::NotFoundError(
            "Notification not found".to_string(),
        ))
    }

    async fn get_all_notifications(&self) -> Result<Vec<Notification>, DomainError> {
        Ok(vec![])
    }

    async fn get_notifications_by_status(
        &self,
        _status: NotificationStatus,
    ) -> Result<Vec<Notification>, DomainError> {
        Ok(vec![])
    }

    async fn get_notifications_by_source(
        &self,
        _source: NotificationSource,
    ) -> Result<Vec<Notification>, DomainError> {
        Ok(vec![])
    }

    async fn mark_as_read(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn mark_action_required(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn mark_action_taken(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn archive_notification(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }

    async fn delete_notification(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

struct TestEventPublisher {
    events: Arc<tokio::sync::Mutex<Vec<NotificationEvent>>>,
}

impl TestEventPublisher {
    fn new() -> Self {
        Self {
            events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    async fn get_events(&self) -> Vec<NotificationEvent> {
        self.events.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl EventPublisher for TestEventPublisher {
    async fn publish_event(&self, event: NotificationEvent) -> Result<(), DomainError> {
        let mut events = self.events.lock().await;
        events.push(event);
        Ok(())
    }
}

#[tokio::test]
async fn test_notification_processing_workflow() -> Result<()> {
    let repository = Arc::new(TestNotificationRepository::new());
    let service = Arc::new(TestNotificationService);
    let event_publisher = Arc::new(TestEventPublisher::new());
    let processor =
        NotificationProcessor::new(service, repository.clone(), event_publisher.clone());

    // Create a notification in New status
    let mut notification = create_test_notification();
    notification.status = NotificationStatus::New;
    repository
        .save(&mut notification.clone())
        .await
        .expect("Failed to save notification");

    // Process the notification
    let mut job = create_processor_job(notification.id, NotificationActionType::Process);
    processor
        .handle(&mut job)
        .await
        .expect("Failed to process notification");

    // Give the notification time to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get the updated notification
    let processed_notification = repository
        .find_by_id(notification.id)
        .await?
        .expect("Notification not found");
    assert_eq!(
        processed_notification.status,
        NotificationStatus::ActionRequired,
        "Notification should be marked as requiring action"
    );

    // Verify notification processed event was published
    let events = event_publisher.get_events().await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, NotificationEvent::NotificationActionRequired { .. })),
        "Expected notification action required event"
    );

    Ok(())
}

#[tokio::test]
async fn test_response_generation() -> Result<()> {
    let repository = Arc::new(TestNotificationRepository::new());
    let service = Arc::new(TestNotificationService);
    let event_publisher = Arc::new(TestEventPublisher::new());
    let processor =
        NotificationProcessor::new(service, repository.clone(), event_publisher.clone());

    // Create and save a notification
    let mut notification = create_test_notification();
    notification.mark_action_required();
    repository
        .save(&mut notification.clone())
        .await
        .expect("Failed to save notification");

    // Generate response
    let mut job = create_processor_job(notification.id, NotificationActionType::GenerateResponse);
    processor
        .handle(&mut job)
        .await
        .expect("Failed to generate response");

    // Verify events
    let events = event_publisher.get_events().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, NotificationEvent::ResponseGenerated { .. })));

    Ok(())
}

#[tokio::test]
async fn test_action_execution() -> Result<()> {
    let repository = Arc::new(TestNotificationRepository::new());
    let service = Arc::new(TestNotificationService);
    let event_publisher = Arc::new(TestEventPublisher::new());
    let processor =
        NotificationProcessor::new(service, repository.clone(), event_publisher.clone());

    // Create and save a notification in New status
    let mut notification = create_test_notification();
    notification.status = NotificationStatus::New;
    repository
        .save(&mut notification.clone())
        .await
        .expect("Failed to save notification");

    // First process it to mark as action required
    let mut process_job = create_processor_job(notification.id, NotificationActionType::Process);
    processor
        .handle(&mut process_job)
        .await
        .expect("Failed to process notification");

    // Wait for processing to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify notification is now action required
    let notification = repository
        .find_by_id(notification.id)
        .await?
        .expect("Notification not found");
    assert_eq!(notification.status, NotificationStatus::ActionRequired);

    // Execute action
    let mut execute_job =
        create_processor_job(notification.id, NotificationActionType::ExecuteAction);
    processor
        .handle(&mut execute_job)
        .await
        .expect("Failed to execute action");

    // Verify notification status
    let processed_notification = repository
        .find_by_id(notification.id)
        .await?
        .expect("Notification not found");
    assert_eq!(
        processed_notification.status,
        NotificationStatus::ActionTaken,
        "Notification should be marked as ActionTaken after execution"
    );

    // Verify events
    let events = event_publisher.get_events().await;
    assert!(events
        .iter()
        .any(|e| matches!(e, NotificationEvent::ActionExecuted { .. })));

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let repository = Arc::new(TestNotificationRepository::new());
    let service = Arc::new(TestNotificationService);
    let event_publisher = Arc::new(TestEventPublisher::new());
    let processor =
        NotificationProcessor::new(service, repository.clone(), event_publisher.clone());

    // Test with non-existent notification
    let mut job = create_processor_job(Uuid::new_v4(), NotificationActionType::Process);
    let result = processor.handle(&mut job).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));

    // Test processing a non-New notification
    let mut notification = create_test_notification();
    notification.mark_as_read(); // Change status from New
    repository
        .save(&mut notification.clone())
        .await
        .expect("Failed to save notification");

    let mut job = create_processor_job(notification.id, NotificationActionType::Process);
    let result = processor.handle(&mut job).await;
    assert!(
        result.is_ok(),
        "Processing non-New notification should succeed with warning"
    );

    Ok(())
}

fn create_test_notification() -> Notification {
    Notification::new(
        "Test Notification".to_string(),
        "Test Content".to_string(),
        NotificationPriority::High,
        autoresponse_lib::domain::entities::NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test123".to_string()),
            url: None,
            tags: vec!["test".to_string()],
            custom_data: None,
        },
    )
}

fn create_processor_job(notification_id: Uuid, action_type: NotificationActionType) -> Job {
    Job::new(
        json!({
            "notification_id": notification_id,
            "action_type": action_type
        }),
        autoresponse_lib::domain::services::JobPriority::Normal,
        JobType::Custom("notification_processor".to_string()),
        3,
    )
}
