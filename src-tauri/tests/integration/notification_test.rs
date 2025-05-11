use crate::common::mock::MockNotificationService;
use anyhow::Result;
use autoresponse_lib::{
    application::use_cases::notification_use_cases::{
        CreateNotificationRequest, NotificationUseCases,
    },
    domain::entities::{
        Notification, NotificationMetadata, NotificationPriority, NotificationSource,
        NotificationStatus,
    },
};
use mockall::predicate;
use std::sync::Arc;
use tokio;
use uuid::Uuid;

#[tokio::test]
async fn test_notification_lifecycle() -> Result<()> {
    let mut mock = MockNotificationService::new();
    let notification_id = Uuid::new_v4();

    mock.expect_create_notification().times(1).returning(
        move |title, content, priority, metadata| {
            let mut notification = Notification::new(title, content, priority, metadata);
            notification.id = notification_id;
            Ok(notification)
        },
    );

    mock.expect_mark_as_read().times(1).returning(|_| Ok(()));
    mock.expect_mark_action_required()
        .times(1)
        .returning(|_| Ok(()));
    mock.expect_mark_action_taken()
        .times(1)
        .returning(|_| Ok(()));
    mock.expect_archive_notification()
        .times(1)
        .returning(|_| Ok(()));
    mock.expect_delete_notification()
        .times(1)
        .returning(|_| Ok(()));

    let use_cases = NotificationUseCases::new(Arc::new(mock));

    let request = CreateNotificationRequest {
        title: "Test Notification".to_string(),
        content: "This is a test notification".to_string(),
        priority: NotificationPriority::High,
        source: NotificationSource::Email,
        external_id: Some("test123".to_string()),
        url: Some("https://example.com".to_string()),
        tags: vec!["test".to_string()],
        custom_data: None,
    };

    let notification = use_cases.create_notification(request).await?;
    assert_eq!(notification.title, "Test Notification");
    assert_eq!(notification.status, NotificationStatus::New);

    use_cases.mark_as_read(notification.id).await?;
    use_cases.mark_action_required(notification.id).await?;
    use_cases.mark_action_taken(notification.id).await?;
    use_cases.archive_notification(notification.id).await?;
    use_cases.delete_notification(notification.id).await?;

    Ok(())
}

#[tokio::test]
async fn test_recent_notifications() -> Result<()> {
    let mut mock = MockNotificationService::new();
    let now = chrono::Utc::now();
    let notifications = vec![
        create_test_notification_with_time(
            NotificationPriority::High,
            NotificationSource::Email,
            NotificationStatus::New,
            now,
        ),
        create_test_notification_with_time(
            NotificationPriority::Medium,
            NotificationSource::Github,
            NotificationStatus::Read,
            now - chrono::Duration::hours(1),
        ),
        create_test_notification_with_time(
            NotificationPriority::Low,
            NotificationSource::Jira,
            NotificationStatus::New,
            now - chrono::Duration::hours(2),
        ),
    ];

    mock.expect_get_all_notifications()
        .times(1)
        .returning(move || Ok(notifications.clone()));

    let use_cases = NotificationUseCases::new(Arc::new(mock));
    let recent = use_cases.get_recent_notifications(2).await?;

    assert_eq!(recent.len(), 2);
    assert!(recent[0].created_at > recent[1].created_at);
    assert_eq!(recent[0].priority, NotificationPriority::High);
    assert_eq!(recent[1].priority, NotificationPriority::Medium);

    Ok(())
}

#[tokio::test]
async fn test_notification_filtering() -> Result<()> {
    let mut mock = MockNotificationService::new();

    let base_notifications = vec![
        create_test_notification(
            NotificationPriority::High,
            NotificationSource::Email,
            NotificationStatus::New,
        ),
        create_test_notification(
            NotificationPriority::Medium,
            NotificationSource::Github,
            NotificationStatus::Read,
        ),
        create_test_notification(
            NotificationPriority::Critical,
            NotificationSource::Jira,
            NotificationStatus::ActionRequired,
        ),
    ];

    let email_notifications = base_notifications.clone();
    mock.expect_get_notifications_by_source()
        .with(predicate::eq(NotificationSource::Email))
        .times(1)
        .returning(move |_| {
            Ok(email_notifications
                .clone()
                .into_iter()
                .filter(|n| matches!(n.metadata.source, NotificationSource::Email))
                .collect())
        });

    let action_required_notifications = base_notifications.clone();
    mock.expect_get_notifications_by_status()
        .with(predicate::eq(NotificationStatus::ActionRequired))
        .times(1)
        .returning(move |_| {
            Ok(action_required_notifications
                .clone()
                .into_iter()
                .filter(|n| matches!(n.status, NotificationStatus::ActionRequired))
                .collect())
        });

    let new_notifications = base_notifications.clone();
    mock.expect_get_notifications_by_status()
        .with(predicate::eq(NotificationStatus::New))
        .times(1)
        .returning(move |_| {
            Ok(new_notifications
                .clone()
                .into_iter()
                .filter(|n| matches!(n.status, NotificationStatus::New))
                .collect())
        });

    let use_cases = NotificationUseCases::new(Arc::new(mock));

    let email_notifications = use_cases
        .get_notifications_by_source(NotificationSource::Email)
        .await?;
    assert_eq!(email_notifications.len(), 1);
    assert!(matches!(
        email_notifications[0].metadata.source,
        NotificationSource::Email
    ));

    let action_required = use_cases.get_action_required_notifications().await?;
    assert_eq!(action_required.len(), 1);
    assert!(matches!(
        action_required[0].status,
        NotificationStatus::ActionRequired
    ));

    let unread = use_cases.get_unread_notifications().await?;
    assert_eq!(unread.len(), 1);
    assert!(matches!(unread[0].status, NotificationStatus::New));

    Ok(())
}

fn create_test_notification(
    priority: NotificationPriority,
    source: NotificationSource,
    status: NotificationStatus,
) -> Notification {
    let mut notification = Notification::new(
        "Test Notification".to_string(),
        "Test Content".to_string(),
        priority,
        NotificationMetadata {
            source,
            external_id: None,
            url: None,
            tags: vec![],
            custom_data: None,
        },
    );

    match status {
        NotificationStatus::Read => notification.mark_as_read(),
        NotificationStatus::ActionRequired => notification.mark_action_required(),
        NotificationStatus::ActionTaken => notification.mark_action_taken(),
        NotificationStatus::Archived => notification.archive(),
        NotificationStatus::Deleted => notification.delete(),
        NotificationStatus::New => {}
    }

    notification
}

fn create_test_notification_with_time(
    priority: NotificationPriority,
    source: NotificationSource,
    status: NotificationStatus,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Notification {
    let mut notification = create_test_notification(priority, source, status);
    notification.created_at = created_at;
    notification.updated_at = created_at;
    notification
}
