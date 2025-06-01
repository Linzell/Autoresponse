use super::types::{Job, JobHandler, JobType};
use crate::domain::{
    entities::notification::{Notification, NotificationStatus},
    events::{EventPublisher, NotificationEvent},
    repositories::notification_repository::NotificationRepository,
    services::notification_service::NotificationService,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationProcessingPayload {
    pub notification_id: uuid::Uuid,
    pub action_type: NotificationActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationActionType {
    Process,
    GenerateResponse,
    ExecuteAction,
}

#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Notification not found: {0}")]
    NotFound(Uuid),
    #[error("Invalid notification state: {0}")]
    InvalidState(String),
    #[error("Repository error: {0}")]
    Repository(String),
    #[error("Service error: {0}")]
    Service(String),
    #[error("Event error: {0}")]
    Event(String),
}

impl From<ProcessorError> for String {
    fn from(error: ProcessorError) -> Self {
        error.to_string()
    }
}

impl From<String> for ProcessorError {
    fn from(error: String) -> Self {
        ProcessorError::InvalidState(error)
    }
}

#[derive(Clone)]
pub struct NotificationProcessor {
    notification_service: Arc<dyn NotificationService + Send + Sync>,
    notification_repository: Arc<dyn NotificationRepository + Send + Sync>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl std::fmt::Debug for NotificationProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationProcessor")
            .field("notification_service", &"Arc<dyn NotificationService>")
            .field(
                "notification_repository",
                &"Arc<dyn NotificationRepository>",
            )
            .finish()
    }
}

impl NotificationProcessor {
    pub fn new(
        notification_service: Arc<dyn NotificationService + Send + Sync>,
        notification_repository: Arc<dyn NotificationRepository + Send + Sync>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            notification_service,
            notification_repository,
            event_publisher,
        }
    }

    async fn process_notification(&self, notification_id: Uuid) -> Result<(), ProcessorError> {
        let notification = self.get_notification(notification_id).await?;

        match notification.status {
            NotificationStatus::New => {
                // Analyze notification content and determine if action is required
                let requires_action = self
                    .notification_service
                    .analyze_notification_content(&notification)
                    .await
                    .map_err(|e| ProcessorError::Service(e.to_string()))?;

                let mut updated_notification = notification.clone();
                if requires_action {
                    updated_notification.mark_action_required();
                } else {
                    updated_notification.mark_as_read();
                }

                self.notification_repository
                    .save(&mut updated_notification)
                    .await
                    .map_err(|e| ProcessorError::Repository(e.to_string()))?;

                // Publish event
                let event = if requires_action {
                    NotificationEvent::notification_action_required(notification_id)
                } else {
                    NotificationEvent::notification_read(notification_id)
                };
                self.event_publisher
                    .publish_event(event)
                    .await
                    .map_err(|e| ProcessorError::Event(e.to_string()))?;
            }
            _ => {
                warn!("Notification {} is not in New status", notification_id);
            }
        }

        Ok(())
    }

    async fn get_notification(&self, id: Uuid) -> Result<Notification, ProcessorError> {
        self.notification_repository
            .find_by_id(id)
            .await
            .map_err(|e| ProcessorError::Repository(e.to_string()))?
            .ok_or(ProcessorError::NotFound(id))
    }

    async fn generate_response(&self, notification_id: Uuid) -> Result<(), ProcessorError> {
        let notification = self.get_notification(notification_id).await?;
        self.validate_action_required(&notification)?;

        let response = self
            .notification_service
            .generate_response(&notification)
            .await
            .map_err(|e| ProcessorError::Service(e.to_string()))?;

        info!(
            "Generated response for notification {}: {}",
            notification_id, response
        );

        // Store the generated response in notification metadata
        let mut updated_notification = notification.clone();
        if let Some(ref mut custom_data) = updated_notification.metadata.custom_data {
            if let Some(obj) = custom_data.as_object_mut() {
                obj.insert(
                    "generated_response".to_string(),
                    serde_json::Value::String(response.clone()),
                );
            }
        } else {
            updated_notification.metadata.custom_data = Some(serde_json::json!({
                "generated_response": response
            }));
        }

        self.notification_repository
            .save(&mut updated_notification)
            .await
            .map_err(|e| ProcessorError::Repository(e.to_string()))?;

        // Publish response generated event
        let event = NotificationEvent::response_generated(notification_id, response);
        self.event_publisher
            .publish_event(event)
            .await
            .map_err(|e| ProcessorError::Event(e.to_string()))?;

        Ok(())
    }

    fn validate_action_required(&self, notification: &Notification) -> Result<(), ProcessorError> {
        if notification.status != NotificationStatus::ActionRequired {
            return Err(ProcessorError::InvalidState(format!(
                "Cannot perform action on notification {} in status {:?}",
                notification.id, notification.status
            )));
        }
        Ok(())
    }

    async fn execute_action(&self, notification_id: Uuid) -> Result<(), ProcessorError> {
        let notification = self.get_notification(notification_id).await?;
        self.validate_action_required(&notification)?;

        self.notification_service
            .execute_action(&notification)
            .await
            .map_err(|e| ProcessorError::Service(e.to_string()))?;

        let mut updated_notification = notification.clone();
        updated_notification.mark_action_taken();

        self.notification_repository
            .save(&mut updated_notification)
            .await
            .map_err(|e| ProcessorError::Repository(e.to_string()))?;

        // Publish action executed event
        let event = NotificationEvent::action_executed(notification_id, true, None);
        self.event_publisher
            .publish_event(event)
            .await
            .map_err(|e| format!("Failed to publish event: {}", e))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl JobHandler for NotificationProcessor {
    async fn handle(&self, job: &mut Job) -> Result<(), String> {
        let payload: NotificationProcessingPayload = serde_json::from_value(job.payload.clone())
            .map_err(|e| format!("Invalid job payload: {}", e))?;

        let notification_id = payload.notification_id;
        let action_type = payload.action_type;

        let result = match action_type {
            NotificationActionType::Process => self.process_notification(notification_id).await,
            NotificationActionType::GenerateResponse => {
                self.generate_response(notification_id).await
            }
            NotificationActionType::ExecuteAction => self.execute_action(notification_id).await,
        };
        result.map_err(|e| e.to_string())
    }

    fn job_type(&self) -> JobType {
        JobType::NotificationProcessing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::notification::{
            Notification, NotificationMetadata, NotificationPriority, NotificationSource,
        },
        events::NoopEventPublisher,
        repositories::NotificationRepository,
        services::{background::JobPriority, NotificationService},
        DomainError, DomainResult,
    };
    use async_trait::async_trait;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    const TEST_TITLE: &str = "Test Notification";
    const TEST_CONTENT: &str = "Test Content";
    const TEST_RESPONSE: &str = "Test Response";

    #[derive(Default, Debug)]
    struct TestNotificationRepository {
        notifications: Mutex<HashMap<Uuid, Notification>>,
    }

    #[async_trait]
    impl NotificationRepository for TestNotificationRepository {
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
                .filter(|n| n.status == status)
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
                .filter(|n| n.metadata.source == source)
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
                    "Notification {} not found",
                    id
                )))
            }
        }
    }

    #[derive(Default, Debug)]
    struct TestNotificationService {}

    #[async_trait]
    impl NotificationService for TestNotificationService {
        async fn create_notification(
            &self,
            title: String,
            content: String,
            priority: NotificationPriority,
            metadata: NotificationMetadata,
        ) -> DomainResult<Notification> {
            Ok(Notification::new(title, content, priority, metadata))
        }

        async fn get_notification(&self, _id: Uuid) -> DomainResult<Notification> {
            unimplemented!()
        }

        async fn get_all_notifications(&self) -> DomainResult<Vec<Notification>> {
            Ok(vec![])
        }

        async fn get_notifications_by_status(
            &self,
            _status: NotificationStatus,
        ) -> DomainResult<Vec<Notification>> {
            Ok(vec![])
        }

        async fn get_notifications_by_source(
            &self,
            _source: NotificationSource,
        ) -> DomainResult<Vec<Notification>> {
            Ok(vec![])
        }

        async fn mark_as_read(&self, _id: Uuid) -> DomainResult<()> {
            Ok(())
        }

        async fn mark_action_required(&self, _id: Uuid) -> DomainResult<()> {
            Ok(())
        }

        async fn mark_action_taken(&self, _id: Uuid) -> DomainResult<()> {
            Ok(())
        }

        async fn archive_notification(&self, _id: Uuid) -> DomainResult<()> {
            Ok(())
        }

        async fn delete_notification(&self, _id: Uuid) -> DomainResult<()> {
            Ok(())
        }

        async fn analyze_notification_content(
            &self,
            _notification: &Notification,
        ) -> DomainResult<bool> {
            Ok(true)
        }

        async fn generate_response(&self, _notification: &Notification) -> DomainResult<String> {
            Ok(TEST_RESPONSE.to_string())
        }

        async fn execute_action(&self, _notification: &Notification) -> DomainResult<()> {
            Ok(())
        }
    }

    fn create_test_notification() -> Notification {
        Notification::new(
            TEST_TITLE.to_string(),
            TEST_CONTENT.to_string(),
            NotificationPriority::Medium,
            NotificationMetadata {
                source: NotificationSource::Email,
                external_id: None,
                url: None,
                tags: vec![],
                custom_data: None,
            },
        )
    }

    fn setup_test_environment() -> (
        Arc<TestNotificationRepository>,
        Arc<TestNotificationService>,
        NotificationProcessor,
    ) {
        let repository = Arc::new(TestNotificationRepository::default());
        let service = Arc::new(TestNotificationService::default());
        let processor = NotificationProcessor::new(
            service.clone(),
            repository.clone(),
            Arc::new(NoopEventPublisher),
        );
        (repository, service, processor)
    }

    #[tokio::test]
    async fn test_process_notification() {
        let notification = create_test_notification();
        let (repository, _service, processor) = setup_test_environment();

        // Save notification to repository
        repository.save(&mut notification.clone()).await.unwrap();

        let job_payload = NotificationProcessingPayload {
            notification_id: notification.id,
            action_type: NotificationActionType::Process,
        };

        let mut job = Job::new(
            serde_json::to_value(job_payload).unwrap(),
            super::super::types::JobPriority::Normal,
            JobType::NotificationProcessing,
            3,
        );

        processor.handle(&mut job).await.unwrap();

        // Verify notification was processed
        let processed = repository
            .find_by_id(notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(processed.status, NotificationStatus::ActionRequired);
    }

    #[tokio::test]
    async fn test_generate_response() {
        let mut notification = create_test_notification();
        let (repository, _service, processor) = setup_test_environment();

        // Setup notification in ActionRequired state
        notification.mark_action_required();
        repository.save(&mut notification.clone()).await.unwrap();

        let job_payload = NotificationProcessingPayload {
            notification_id: notification.id,
            action_type: NotificationActionType::GenerateResponse,
        };

        let mut job = Job::new(
            serde_json::to_value(job_payload).unwrap(),
            JobPriority::Normal,
            JobType::NotificationProcessing,
            3,
        );

        processor.handle(&mut job).await.unwrap();

        // Verify response was generated
        let processed = repository
            .find_by_id(notification.id)
            .await
            .unwrap()
            .unwrap();
        let response = processed.metadata.custom_data.unwrap();
        assert_eq!(response["generated_response"], TEST_RESPONSE);
    }

    #[tokio::test]
    async fn test_execute_action() {
        let mut notification = create_test_notification();
        let (repository, _service, processor) = setup_test_environment();

        // Setup notification in ActionRequired state
        notification.mark_action_required();
        repository.save(&mut notification.clone()).await.unwrap();

        let job_payload = NotificationProcessingPayload {
            notification_id: notification.id,
            action_type: NotificationActionType::ExecuteAction,
        };

        let mut job = Job::new(
            serde_json::to_value(job_payload).unwrap(),
            JobPriority::Normal,
            JobType::NotificationProcessing,
            3,
        );

        processor.handle(&mut job).await.unwrap();

        // Verify action was executed
        let processed = repository
            .find_by_id(notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(processed.status, NotificationStatus::ActionTaken);
    }
}
