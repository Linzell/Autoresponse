use super::setup_test_env;
use autoresponse_lib::{
    domain::{
        entities::{
            Notification, NotificationMetadata, NotificationPriority, NotificationSource,
            NotificationStatus,
        },
        services::{
            background::{BackgroundJobManager, Job, JobHandler, JobType},
            notification_service::{DefaultNotificationService, NotificationService},
        },
    },
    infrastructure::repositories::sqlite_notification_repository::SqliteNotificationRepository,
};
use std::sync::Arc;
use tokio;

#[derive(Debug)]
struct TestJobHandler;

impl JobHandler for TestJobHandler {
    fn handle(&self, _job: &mut Job) -> Result<(), String> {
        Ok(())
    }

    fn job_type(&self) -> JobType {
        JobType::NotificationProcessing
    }
}

async fn setup_test_service() -> Arc<dyn NotificationService> {
    setup_test_env();

    let notification_repo = SqliteNotificationRepository::new(":memory:").unwrap();
    let job_manager = Arc::new(BackgroundJobManager::new());

    // Register notification processing handler
    job_manager
        .register_handler(Arc::new(TestJobHandler))
        .await
        .unwrap();

    Arc::new(DefaultNotificationService::new(
        Arc::new(notification_repo),
        job_manager,
    ))
}

#[tokio::test]
async fn test_email_notification_workflow() {
    let service = setup_test_service().await;

    let notification = Notification::new(
        "Test Email".to_string(),
        "Email content".to_string(),
        NotificationPriority::High,
        NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test-123".to_string()),
            url: None,
            tags: vec!["email".to_string(), "test".to_string()],
            custom_data: Some(serde_json::json!({
                "thread_id": "thread-123",
                "subject": "Test Subject",
                "recipient": "test@example.com"
            })),
        },
    );

    let created = service
        .create_notification(
            notification.title.clone(),
            notification.content.clone(),
            notification.priority.clone(),
            notification.metadata.clone(),
        )
        .await
        .unwrap();

    let result = service.execute_action(&created).await;
    assert!(result.is_ok());

    let updated = service.get_notification(created.id).await.unwrap();
    assert_eq!(updated.id, created.id);
}

#[tokio::test]
async fn test_github_notification_workflow() {
    let service = setup_test_service().await;

    let notification = Notification::new(
        "PR Review".to_string(),
        "New pull request needs review".to_string(),
        NotificationPriority::Medium,
        NotificationMetadata {
            source: NotificationSource::Github,
            external_id: Some("pr-123".to_string()),
            url: Some("https://github.com/org/repo/pull/123".to_string()),
            tags: vec!["github".to_string(), "pr".to_string()],
            custom_data: Some(serde_json::json!({
                "repo": "org/repo",
                "pr_number": 123,
                "action": "review_requested"
            })),
        },
    );

    let created = service
        .create_notification(
            notification.title,
            notification.content,
            notification.priority,
            notification.metadata,
        )
        .await
        .unwrap();

    // Test complete workflow
    let result = service.mark_action_required(created.id).await;
    assert!(result.is_ok());

    let result = service.generate_response(&created).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.is_empty());

    let result = service.execute_action(&created).await;
    assert!(result.is_ok());

    let result = service.mark_action_taken(created.id).await;
    assert!(result.is_ok());

    let final_state = service.get_notification(created.id).await.unwrap();
    assert!(matches!(
        final_state.status,
        NotificationStatus::ActionTaken
    ));
}

#[tokio::test]
async fn test_jira_notification_chain() {
    let service = setup_test_service().await;

    // Create a chain of related notifications
    let notifications = vec![
        (
            "PROJ-123 Created",
            "Action required: New task created",
            Some(serde_json::json!({"status": "Open", "action_required": true})),
        ),
        (
            "PROJ-123 Updated",
            "Review required: Task assigned for urgent review",
            Some(serde_json::json!({"status": "In Progress", "action_required": true})),
        ),
        (
            "PROJ-123 Completed",
            "Action required: Task completed, needs final review",
            Some(serde_json::json!({"status": "Done", "review_required": true})),
        ),
    ];

    let mut created_ids = Vec::new();
    for (title, content, data) in notifications {
        let notification = Notification::new(
            title.to_string(),
            content.to_string(),
            NotificationPriority::Medium,
            NotificationMetadata {
                source: NotificationSource::Jira,
                external_id: Some("PROJ-123".to_string()),
                url: Some("https://jira.company.com/browse/PROJ-123".to_string()),
                tags: vec!["jira".to_string()],
                custom_data: data,
            },
        );

        let created = service
            .create_notification(
                notification.title,
                notification.content,
                notification.priority,
                notification.metadata,
            )
            .await
            .unwrap();

        let result = service.execute_action(&created).await;
        assert!(result.is_ok());

        created_ids.push(created.id);
    }

    // Verify all notifications were processed
    for id in created_ids {
        let notification = service.get_notification(id).await.unwrap();
        assert!(service
            .analyze_notification_content(&notification)
            .await
            .unwrap());
    }
}

#[tokio::test]
async fn test_notification_error_handling() {
    let service = setup_test_service().await;

    // Test with invalid notification data
    let notification = Notification::new(
        "Invalid Data".to_string(),
        "Test content".to_string(),
        NotificationPriority::Low,
        NotificationMetadata {
            source: NotificationSource::Email,
            external_id: None,
            url: None,
            tags: vec![],
            custom_data: Some(serde_json::json!({
                "invalid_field": "value"
            })),
        },
    );

    let created = service
        .create_notification(
            notification.title,
            notification.content,
            notification.priority,
            notification.metadata,
        )
        .await
        .unwrap();

    // Even with invalid data, execution should not fail
    let result = service.execute_action(&created).await;
    assert!(result.is_ok());
}
