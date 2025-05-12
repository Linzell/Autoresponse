use autoresponse_lib::domain::{
    entities::{
        Notification, NotificationMetadata, NotificationPriority, NotificationSource,
        NotificationStatus,
    },
    error::DomainResult,
    services::{actions::ActionHandler, notification_service::NotificationService},
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct TestActionHandler {
    handled_notifications: Arc<Mutex<Vec<Notification>>>,
}

impl TestActionHandler {
    fn new() -> Self {
        Self {
            handled_notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ActionHandler for TestActionHandler {
    fn handle<'a>(
        &'a self,
        notification: &'a Notification,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DomainResult<()>> + Send + 'a>> {
        Box::pin(async move {
            self.handled_notifications
                .lock()
                .await
                .push(notification.clone());
            Ok(())
        })
    }
}

struct TestNotificationService {
    action_handler: TestActionHandler,
}

impl TestNotificationService {
    fn new() -> Self {
        Self {
            action_handler: TestActionHandler::new(),
        }
    }
}

#[async_trait::async_trait]
impl NotificationService for TestNotificationService {
    async fn execute_action(&self, notification: &Notification) -> DomainResult<()> {
        self.action_handler.handle(notification).await
    }

    async fn create_notification(
        &self,
        title: String,
        content: String,
        priority: NotificationPriority,
        metadata: NotificationMetadata,
    ) -> DomainResult<Notification> {
        Ok(Notification::new(title, content, priority, metadata))
    }

    async fn get_notification(&self, _id: uuid::Uuid) -> DomainResult<Notification> {
        unimplemented!("Not needed for these tests")
    }

    async fn get_all_notifications(&self) -> DomainResult<Vec<Notification>> {
        unimplemented!("Not needed for these tests")
    }

    async fn get_notifications_by_status(
        &self,
        _status: NotificationStatus,
    ) -> DomainResult<Vec<Notification>> {
        unimplemented!("Not needed for these tests")
    }

    async fn get_notifications_by_source(
        &self,
        _source: NotificationSource,
    ) -> DomainResult<Vec<Notification>> {
        unimplemented!("Not needed for these tests")
    }

    async fn mark_as_read(&self, _id: uuid::Uuid) -> DomainResult<()> {
        unimplemented!("Not needed for these tests")
    }

    async fn mark_action_required(&self, _id: uuid::Uuid) -> DomainResult<()> {
        unimplemented!("Not needed for these tests")
    }

    async fn mark_action_taken(&self, _id: uuid::Uuid) -> DomainResult<()> {
        unimplemented!("Not needed for these tests")
    }

    async fn archive_notification(&self, _id: uuid::Uuid) -> DomainResult<()> {
        unimplemented!("Not needed for these tests")
    }

    async fn delete_notification(&self, _id: uuid::Uuid) -> DomainResult<()> {
        unimplemented!("Not needed for these tests")
    }

    async fn analyze_notification_content(
        &self,
        _notification: &Notification,
    ) -> DomainResult<bool> {
        unimplemented!("Not needed for these tests")
    }

    async fn generate_response(&self, _notification: &Notification) -> DomainResult<String> {
        unimplemented!("Not needed for these tests")
    }
}

#[tokio::test]
async fn test_action_execution() {
    let service = TestNotificationService::new();

    let notification = Notification::new(
        "Test Action".to_string(),
        "Content".to_string(),
        NotificationPriority::High,
        NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test-id".to_string()),
            url: Some("http://test.com".to_string()),
            tags: vec!["test".to_string()],
            custom_data: None,
        },
    );

    let result = service.execute_action(&notification).await;
    assert!(result.is_ok());

    let handled = service.action_handler.handled_notifications.lock().await;
    assert_eq!(handled.len(), 1);
    assert_eq!(handled[0].id, notification.id);
}

#[tokio::test]
async fn test_multiple_action_sources() {
    let service = TestNotificationService::new();

    let notifications = vec![
        ("Email Test", NotificationSource::Email),
        ("GitHub Test", NotificationSource::Github),
        ("Jira Test", NotificationSource::Jira),
    ];

    for (title, source) in notifications {
        let notification = Notification::new(
            title.to_string(),
            "Test content".to_string(),
            NotificationPriority::Medium,
            NotificationMetadata {
                source,
                external_id: Some("test-id".to_string()),
                url: Some("http://test.com".to_string()),
                tags: vec!["test".to_string()],
                custom_data: None,
            },
        );

        let result = service.execute_action(&notification).await;
        assert!(result.is_ok());
    }

    let handled = service.action_handler.handled_notifications.lock().await;
    assert_eq!(handled.len(), 3);
}

#[tokio::test]
async fn test_concurrent_action_execution() {
    let service = Arc::new(TestNotificationService::new());

    let notifications: Vec<_> = (0..10)
        .map(|i| {
            Notification::new(
                format!("Test {}", i),
                "Content".to_string(),
                NotificationPriority::Medium,
                NotificationMetadata {
                    source: if i % 2 == 0 {
                        NotificationSource::Email
                    } else {
                        NotificationSource::Github
                    },
                    external_id: Some(format!("test-{}", i)),
                    url: Some("http://test.com".to_string()),
                    tags: vec!["test".to_string()],
                    custom_data: None,
                },
            )
        })
        .collect();

    let handles: Vec<_> = notifications
        .iter()
        .map(|notification| {
            let service = service.clone();
            let notification = notification.clone();
            tokio::spawn(async move { service.execute_action(&notification).await })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    let handled = service.action_handler.handled_notifications.lock().await;
    assert_eq!(handled.len(), notifications.len());
}
