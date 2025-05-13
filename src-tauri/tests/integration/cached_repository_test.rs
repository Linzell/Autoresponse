use anyhow::Result;
use autoresponse_lib::{
    domain::{
        entities::{Notification, NotificationMetadata, NotificationPriority, NotificationSource},
        repositories::NotificationRepository,
        NotificationStatus,
    },
    infrastructure::repositories::sqlite_notification_repository::{
        CachedSqliteNotificationRepository, SqliteNotificationRepository,
    },
};
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_cache_hit_and_miss() -> Result<()> {
    let base_repo = SqliteNotificationRepository::new(":memory:")?;
    let repo = CachedSqliteNotificationRepository::new(base_repo, 100, Duration::from_secs(60));

    let mut notification = Notification::new(
        "Cache Test".to_string(),
        "Testing cache functionality".to_string(),
        NotificationPriority::Medium,
        NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test-123".to_string()),
            url: None,
            tags: vec!["test".to_string()],
            custom_data: None,
        },
    );

    // Save notification
    NotificationRepository::save(&repo, &mut notification).await?;

    // First fetch - should hit database
    let found1 = NotificationRepository::find_by_id(&repo, notification.id).await?;
    assert!(found1.is_some());
    assert_eq!(found1.unwrap().title, "Cache Test");

    // Second fetch - should hit cache
    let found2 = NotificationRepository::find_by_id(&repo, notification.id).await?;
    assert!(found2.is_some());
    assert_eq!(found2.unwrap().title, "Cache Test");

    Ok(())
}

#[tokio::test]
async fn test_cache_invalidation() -> Result<()> {
    let base_repo = SqliteNotificationRepository::new(":memory:")?;
    let repo = CachedSqliteNotificationRepository::new(base_repo, 100, Duration::from_secs(60));

    let mut notification = Notification::new(
        "Initial Title".to_string(),
        "Testing cache invalidation".to_string(),
        NotificationPriority::High,
        NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test-456".to_string()),
            url: None,
            tags: vec!["test".to_string()],
            custom_data: None,
        },
    );

    // Save notification
    NotificationRepository::save(&repo, &mut notification).await?;

    // Cache should be populated after read
    let _ = NotificationRepository::find_by_id(&repo, notification.id).await?;

    // Update status (should invalidate cache)
    repo.update_status(notification.id, NotificationStatus::Read)
        .await?;

    // Next fetch should hit database with updated status
    let updated = NotificationRepository::find_by_id(&repo, notification.id)
        .await?
        .unwrap();
    assert_eq!(updated.status, NotificationStatus::Read);

    Ok(())
}

#[tokio::test]
async fn test_cache_ttl() -> Result<()> {
    let base_repo = SqliteNotificationRepository::new(":memory:")?;
    let repo = CachedSqliteNotificationRepository::new(
        base_repo,
        100,
        Duration::from_millis(100), // Very short TTL for testing
    );

    let mut notification = Notification::new(
        "TTL Test".to_string(),
        "Testing cache TTL".to_string(),
        NotificationPriority::Low,
        NotificationMetadata {
            source: NotificationSource::Email,
            external_id: Some("test-789".to_string()),
            url: None,
            tags: vec!["test".to_string()],
            custom_data: None,
        },
    );

    // Save and read to populate cache
    NotificationRepository::save(&repo, &mut notification).await?;
    let _ = NotificationRepository::find_by_id(&repo, notification.id).await?;

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Next read should still work (from database)
    let found = NotificationRepository::find_by_id(&repo, notification.id).await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().title, "TTL Test");

    Ok(())
}

#[tokio::test]
async fn test_bulk_operations() -> Result<()> {
    let base_repo = SqliteNotificationRepository::new(":memory:")?;
    let repo = CachedSqliteNotificationRepository::new(base_repo, 100, Duration::from_secs(60));

    // Create multiple notifications
    let mut notifications = vec![];
    for i in 0..5 {
        let mut notification = Notification::new(
            format!("Bulk Test {}", i),
            "Testing bulk operations".to_string(),
            NotificationPriority::Medium,
            NotificationMetadata {
                source: NotificationSource::Email,
                external_id: Some(format!("bulk-{}", i)),
                url: None,
                tags: vec!["bulk".to_string()],
                custom_data: None,
            },
        );
        NotificationRepository::save(&repo, &mut notification).await?;
        notifications.push(notification);
    }

    // Test find_all
    let all = NotificationRepository::find_all(&repo).await?;
    assert_eq!(all.len(), 5);

    // Test find_by_status
    let new_status = repo.find_by_status(NotificationStatus::New).await?;
    assert_eq!(new_status.len(), 5);

    // Update all to read
    for notification in &notifications {
        repo.update_status(notification.id, NotificationStatus::Read)
            .await?;
    }

    // Verify all are read
    let read_status = repo.find_by_status(NotificationStatus::Read).await?;
    assert_eq!(read_status.len(), 5);

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let base_repo = SqliteNotificationRepository::new(":memory:")?;
    let repo = CachedSqliteNotificationRepository::new(base_repo, 100, Duration::from_secs(60));

    // Test with invalid UUID
    let result = NotificationRepository::find_by_id(&repo, uuid::Uuid::nil()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    // Test delete non-existent
    let result = NotificationRepository::delete(&repo, uuid::Uuid::new_v4()).await;
    assert!(result.is_ok());

    Ok(())
}
