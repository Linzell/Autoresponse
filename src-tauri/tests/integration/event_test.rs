use autoresponse_lib::domain::{
    entities::{NotificationPriority, NotificationSource},
    error::DomainError,
    events::{
        notification_events::NotificationEvent,
        publisher::{EventPublisher, NoopEventPublisher},
    },
};
use chrono::Utc;
use std::sync::Arc;
use tokio;
use uuid::Uuid;

struct TestEventPublisher {
    events: Arc<tokio::sync::Mutex<Vec<NotificationEvent>>>,
}

impl TestEventPublisher {
    fn new() -> Self {
        Self {
            events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    async fn get_published_events(&self) -> Vec<NotificationEvent> {
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
async fn test_notification_creation_event() -> Result<(), DomainError> {
    let publisher = Arc::new(TestEventPublisher::new());
    let notification_id = Uuid::new_v4();
    let now = Utc::now();

    let event = NotificationEvent::NotificationCreated {
        notification_id,
        title: "Test Notification".to_string(),
        content: "Test Content".to_string(),
        priority: NotificationPriority::High,
        source: NotificationSource::Email,
        created_at: now,
    };

    publisher.publish_event(event.clone()).await?;

    let events = publisher.get_published_events().await;
    assert_eq!(events.len(), 1);

    if let NotificationEvent::NotificationCreated {
        notification_id: id,
        title,
        content,
        priority,
        source,
        created_at,
    } = &events[0]
    {
        assert_eq!(*id, notification_id);
        assert_eq!(title, "Test Notification");
        assert_eq!(content, "Test Content");
        assert_eq!(*priority, NotificationPriority::High);
        assert_eq!(*source, NotificationSource::Email);
        assert_eq!(*created_at, now);
    } else {
        panic!("Wrong event type received");
    }

    Ok(())
}

#[tokio::test]
async fn test_notification_state_change_events() -> Result<(), DomainError> {
    let publisher = Arc::new(TestEventPublisher::new());
    let notification_id = Uuid::new_v4();

    // Generate and publish multiple state change events
    let events = vec![
        NotificationEvent::notification_processed(notification_id, true),
        NotificationEvent::notification_action_required(notification_id),
        NotificationEvent::notification_read(notification_id),
        NotificationEvent::response_generated(notification_id, "Test Response".to_string()),
        NotificationEvent::action_executed(notification_id, true, None),
    ];

    for event in events {
        publisher.publish_event(event).await?;
    }

    let published_events = publisher.get_published_events().await;
    assert_eq!(published_events.len(), 5);

    // Verify event sequence
    match &published_events[0] {
        NotificationEvent::NotificationProcessed {
            notification_id: id,
            requires_action,
            ..
        } => {
            assert_eq!(*id, notification_id);
            assert!(*requires_action);
        }
        _ => panic!("Wrong event type at index 0"),
    }

    match &published_events[1] {
        NotificationEvent::NotificationActionRequired {
            notification_id: id,
            ..
        } => {
            assert_eq!(*id, notification_id);
        }
        _ => panic!("Wrong event type at index 1"),
    }

    match &published_events[2] {
        NotificationEvent::NotificationRead {
            notification_id: id,
            ..
        } => {
            assert_eq!(*id, notification_id);
        }
        _ => panic!("Wrong event type at index 2"),
    }

    match &published_events[3] {
        NotificationEvent::ResponseGenerated {
            notification_id: id,
            response,
            ..
        } => {
            assert_eq!(*id, notification_id);
            assert_eq!(response, "Test Response");
        }
        _ => panic!("Wrong event type at index 3"),
    }

    match &published_events[4] {
        NotificationEvent::ActionExecuted {
            notification_id: id,
            success,
            error,
            ..
        } => {
            assert_eq!(*id, notification_id);
            assert!(*success);
            assert!(error.is_none());
        }
        _ => panic!("Wrong event type at index 4"),
    }

    Ok(())
}

#[tokio::test]
async fn test_event_error_handling() -> Result<(), DomainError> {
    struct ErrorEventPublisher;

    #[async_trait::async_trait]
    impl EventPublisher for ErrorEventPublisher {
        async fn publish_event(&self, _event: NotificationEvent) -> Result<(), DomainError> {
            Err(DomainError::InternalError(
                "Failed to publish event".to_string(),
            ))
        }
    }

    let publisher = Arc::new(ErrorEventPublisher);
    let notification_id = Uuid::new_v4();
    let event = NotificationEvent::notification_read(notification_id);

    let result = publisher.publish_event(event).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_noop_event_publisher() -> Result<(), DomainError> {
    let publisher = NoopEventPublisher::default();
    let notification_id = Uuid::new_v4();
    let event = NotificationEvent::notification_read(notification_id);

    let result = publisher.publish_event(event).await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_event_publishing() -> Result<(), DomainError> {
    let publisher = Arc::new(TestEventPublisher::new());
    let mut handles = Vec::new();

    // Spawn multiple tasks publishing events concurrently
    for i in 0..10 {
        let publisher = publisher.clone();
        let handle = tokio::spawn(async move {
            let notification_id = Uuid::new_v4();
            let event = NotificationEvent::NotificationCreated {
                notification_id,
                title: format!("Test Notification {}", i),
                content: format!("Test Content {}", i),
                priority: NotificationPriority::Medium,
                source: NotificationSource::Email,
                created_at: Utc::now(),
            };
            publisher.publish_event(event).await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.map_err(|e| DomainError::InternalError(e.to_string()))??;
    }

    let events = publisher.get_published_events().await;
    assert_eq!(events.len(), 10);

    Ok(())
}
