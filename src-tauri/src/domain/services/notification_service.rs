use crate::domain::{
    entities::{
        Notification, NotificationMetadata, NotificationPreferences, NotificationPriority,
        NotificationStatus,
    },
    error::{DomainError, DomainResult},
    repositories::DynNotificationRepository,
    services::{
        actions::executor::DynActionExecutor,
        ai::DynAIService,
        background::{
            manager::DynBackgroundJobManager,
            types::{Job, JobPriority, JobType},
        },
    },
    NotificationSource,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
use mockall::automock;

use super::integrations::service_bridge::ServiceBridge;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait NotificationService: Send + Sync + std::fmt::Debug {
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
    async fn get_notifications_by_source(
        &self,
        source: NotificationSource,
    ) -> DomainResult<Vec<Notification>>;
    async fn mark_as_read(&self, id: Uuid) -> DomainResult<()>;
    async fn mark_action_required(&self, id: Uuid) -> DomainResult<()>;
    async fn mark_action_taken(&self, id: Uuid) -> DomainResult<()>;
    async fn archive_notification(&self, id: Uuid) -> DomainResult<()>;
    async fn delete_notification(&self, id: Uuid) -> DomainResult<()>;

    async fn analyze_notification_content(&self, notification: &Notification)
        -> DomainResult<bool>;
    async fn generate_response(&self, notification: &Notification) -> DomainResult<String>;
    async fn execute_action(&self, notification: &Notification) -> DomainResult<()>;
    async fn save_preferences(&self, preferences: NotificationPreferences) -> DomainResult<()>;
}

#[derive(Debug)]
pub struct DefaultNotificationService {
    repository: DynNotificationRepository,
    job_manager: DynBackgroundJobManager,
    action_executor: DynActionExecutor,
    ai_service: DynAIService,
    service_bridge: Option<Arc<ServiceBridge>>,
}

impl DefaultNotificationService {
    pub fn new(
        repository: DynNotificationRepository,
        job_manager: DynBackgroundJobManager,
        action_executor: DynActionExecutor,
        ai_service: DynAIService,
    ) -> Self {
        Self {
            repository,
            job_manager,
            action_executor,
            ai_service,
            service_bridge: None,
        }
    }

    pub fn with_service_bridge(mut self, service_bridge: Arc<ServiceBridge>) -> Self {
        self.service_bridge = Some(service_bridge);
        self
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

        // Submit background job for processing
        let job = Job::new(
            json!({
                "notification_id": notification.id,
                "action_type": "Process"
            }),
            JobPriority::Normal,
            JobType::NotificationProcessing,
            3,
        );

        let _ = self
            .job_manager
            .submit_job(job)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;
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

    async fn get_notifications_by_source(
        &self,
        source: NotificationSource,
    ) -> DomainResult<Vec<Notification>> {
        self.repository.find_by_source(source).await
    }

    async fn mark_as_read(&self, id: Uuid) -> DomainResult<()> {
        let mut notification = self.get_notification(id).await?;
        notification.mark_as_read();
        self.repository.save(&mut notification).await
    }

    async fn mark_action_required(&self, id: Uuid) -> DomainResult<()> {
        let mut notification = self.get_notification(id).await?;
        notification.mark_action_required();
        self.repository.save(&mut notification).await?;

        // Submit job for response generation
        let job = Job::new(
            json!({
                "notification_id": id,
                "action_type": "GenerateResponse"
            }),
            JobPriority::High,
            JobType::NotificationProcessing,
            3,
        );

        let _ = self
            .job_manager
            .submit_job(job)
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;
        Ok(())
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

    async fn analyze_notification_content(
        &self,
        notification: &Notification,
    ) -> DomainResult<bool> {
        // Check if service bridge can handle this notification
        if let Some(bridge) = &self.service_bridge {
            if let Ok(()) = bridge.process_notification(notification).await {
                return Ok(true);
            }
        }

        // Fall back to AI analysis
        let analysis = self
            .ai_service
            .analyze_content(&notification.content)
            .await?;
        Ok(analysis.requires_action)
    }

    async fn generate_response(&self, notification: &Notification) -> DomainResult<String> {
        // Generate response using AI service
        let context = format!(
            "Source: {}\nTitle: {}\nContent: {}\n",
            notification.metadata.source, notification.title, notification.content
        );

        self.ai_service.generate_response(&context).await
    }

    async fn execute_action(&self, notification: &Notification) -> DomainResult<()> {
        // Try service-specific action first if service bridge is available
        if let Some(bridge) = &self.service_bridge {
            if let Ok(()) = bridge
                .execute_action(notification, "default", serde_json::json!({}))
                .await
            {
                return Ok(());
            }
        }

        // Fall back to default action executor
        self.action_executor.execute(notification).await
    }

    async fn save_preferences(&self, preferences: NotificationPreferences) -> DomainResult<()> {
        // Create a ProjectDirs instance and keep it around
        let proj_dirs =
            directories::ProjectDirs::from("com", "autoresponse", "app").ok_or_else(|| {
                DomainError::ConfigurationError("Failed to get config directory".into())
            })?;

        let config_dir = proj_dirs.config_dir();

        std::fs::create_dir_all(config_dir).map_err(|e| {
            DomainError::ConfigurationError(format!("Failed to create config directory: {}", e))
        })?;

        let preferences_file = config_dir.join("notification_preferences.json");
        let preferences_json = serde_json::to_string_pretty(&preferences).map_err(|e| {
            DomainError::ConfigurationError(format!("Failed to serialize preferences: {}", e))
        })?;

        std::fs::write(preferences_file, preferences_json).map_err(|e| {
            DomainError::ConfigurationError(format!("Failed to save preferences: {}", e))
        })?;

        Ok(())
    }
}

pub type DynNotificationService = Arc<dyn NotificationService>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::NotificationSource;
    use crate::domain::events::NoopEventPublisher;
    use crate::domain::repositories::NotificationRepository;
    use crate::domain::services::actions::executor::MockActionExecutor;
    use crate::domain::services::actions::ActionExecutor;
    use crate::domain::services::ai::{AIAnalysis, MockAIService, PriorityLevel};
    use crate::domain::services::background::{
        manager::BackgroundJobManagerTrait, NotificationProcessor,
    };
    use crate::domain::services::BackgroundJobManager;
    use async_trait::async_trait;
    use mockall::mock;
    use std::collections::HashMap;
    use std::sync::Mutex;

    mock! {
        #[derive(Debug)]
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

    #[derive(Debug)]
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
        let notification_service = Arc::new(MockNotificationService::new());
        let job_manager = Arc::new(BackgroundJobManager::new());

        // Register notification processor
        let processor = Arc::new(NotificationProcessor::new(
            notification_service.clone(),
            repository.clone(),
            Arc::new(NoopEventPublisher),
        ));
        job_manager.register_handler(processor).await.unwrap();

        let service = DefaultNotificationService::new(
            repository,
            job_manager,
            Arc::new(ActionExecutor::new()),
            Arc::new(MockAIService::new()),
        );

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

    #[tokio::test]
    async fn test_analyze_notification_content() {
        let repository = Arc::new(TestRepository {
            notifications: Mutex::new(HashMap::new()),
        });
        let job_manager = Arc::new(BackgroundJobManager::new());

        let mut mock_ai = MockAIService::new();
        mock_ai.expect_analyze_content().returning(|content| {
            Ok(AIAnalysis {
                requires_action: content.to_lowercase().contains("urgent"),
                priority_level: PriorityLevel::High,
                summary: "Test summary".to_string(),
                suggested_actions: vec!["Action 1".to_string()],
            })
        });

        let mut mock_executor = MockActionExecutor::new();
        mock_executor.expect_execute().returning(|_| Ok(()));

        let service = DefaultNotificationService::new(
            repository.clone(),
            job_manager.clone(),
            Arc::new(mock_executor),
            Arc::new(mock_ai),
        );

        let metadata = NotificationMetadata {
            source: NotificationSource::Email,
            external_id: None,
            url: None,
            tags: vec![],
            custom_data: None,
        };

        // Test with action keywords
        let notification = Notification::new(
            "Action Required".to_string(),
            "This is an urgent task that requires your attention".to_string(),
            NotificationPriority::High,
            metadata.clone(),
        );
        let requires_action = service
            .analyze_notification_content(&notification)
            .await
            .unwrap();
        assert!(requires_action);

        // Test without action keywords
        let notification = Notification::new(
            "Simple Update".to_string(),
            "This is a regular update message".to_string(),
            NotificationPriority::Low,
            metadata,
        );
        let requires_action = service
            .analyze_notification_content(&notification)
            .await
            .unwrap();
        assert!(!requires_action);
    }

    #[tokio::test]
    async fn test_generate_response() {
        let repository = Arc::new(TestRepository {
            notifications: Mutex::new(HashMap::new()),
        });
        let job_manager = Arc::new(BackgroundJobManager::new());
        let mut mock_ai = MockAIService::new();

        // Set up mock expectations for each source
        mock_ai
            .expect_generate_response()
            .returning(|context| {
                let response = match context {
                    s if s.contains("Source: Email") => "This is an email response",
                    s if s.contains("Source: Github") => "This is a GitHub response",
                    s if s.contains("Source: Gitlab") => "This is a GitLab response",
                    s if s.contains("Source: Jira") => "This is a Jira response",
                    s if s.contains("Source: Microsoft") => "This is a Microsoft response",
                    s if s.contains("Source: Google") => "This is a Google response",
                    s if s.contains("Source: LinkedIn") => "This is a LinkedIn response",
                    _ => "This is a general notification response",
                };
                Ok(response.to_string())
            })
            .times(8); // Once for each source type we'll test

        let service = DefaultNotificationService::new(
            repository,
            job_manager,
            Arc::new(ActionExecutor::new()),
            Arc::new(mock_ai),
        );

        // Test response generation for different sources
        let sources = vec![
            NotificationSource::Email,
            NotificationSource::Github,
            NotificationSource::Gitlab,
            NotificationSource::Jira,
            NotificationSource::Microsoft,
            NotificationSource::Google,
            NotificationSource::LinkedIn,
            NotificationSource::Custom("Test".to_string()),
        ];

        for source in sources {
            let metadata = NotificationMetadata {
                source: source.clone(),
                external_id: None,
                url: None,
                tags: vec![],
                custom_data: None,
            };

            let notification = Notification::new(
                "Test".to_string(),
                "Content".to_string(),
                NotificationPriority::Medium,
                metadata,
            );

            let response = service.generate_response(&notification).await.unwrap();
            assert!(!response.is_empty());

            // Verify source-specific response
            match source {
                NotificationSource::Email => assert!(response.contains("email response")),
                NotificationSource::Github => assert!(response.contains("GitHub response")),
                NotificationSource::Gitlab => assert!(response.contains("GitLab response")),
                NotificationSource::Jira => assert!(response.contains("Jira response")),
                NotificationSource::Microsoft => assert!(response.contains("Microsoft response")),
                NotificationSource::Google => assert!(response.contains("Google response")),
                NotificationSource::LinkedIn => assert!(response.contains("LinkedIn response")),
                NotificationSource::Custom(_) => {
                    assert!(response.contains("general notification response"))
                }
            }
        }
    }

    #[tokio::test]
    async fn test_execute_action() {
        let repository = Arc::new(TestRepository {
            notifications: Mutex::new(HashMap::new()),
        });
        let job_manager = Arc::new(BackgroundJobManager::new());
        let service = DefaultNotificationService::new(
            repository,
            job_manager,
            Arc::new(ActionExecutor::new()),
            Arc::new(MockAIService::new()),
        );

        let metadata = NotificationMetadata {
            source: NotificationSource::Email,
            external_id: None,
            url: None,
            tags: vec![],
            custom_data: None,
        };

        let notification = Notification::new(
            "Test Action".to_string(),
            "Content requiring action".to_string(),
            NotificationPriority::High,
            metadata,
        );

        // Verify action execution doesn't fail
        let result = service.execute_action(&notification).await;
        assert!(result.is_ok());
    }
}
