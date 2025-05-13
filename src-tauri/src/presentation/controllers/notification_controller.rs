use crate::{
    domain::entities::NotificationMetadata,
    domain::services::NotificationService,
    presentation::dtos::{
        CreateNotificationRequest, NotificationError, NotificationFilterRequest,
        NotificationListResponse, NotificationResponse,
    },
};
use std::sync::Arc;
use uuid::Uuid;

pub struct NotificationController {
    service: Arc<dyn NotificationService>,
}

impl NotificationController {
    pub fn new(service: Arc<dyn NotificationService>) -> Self {
        Self { service }
    }

    pub async fn create_notification(
        &self,
        request: CreateNotificationRequest,
    ) -> Result<NotificationResponse, NotificationError> {
        let metadata = NotificationMetadata {
            source: request.source,
            external_id: request.external_id,
            url: request.url,
            tags: request.tags,
            custom_data: request.custom_data,
        };

        let notification = self
            .service
            .create_notification(request.title, request.content, request.priority, metadata)
            .await
            .map_err(NotificationError::from)?;

        Ok(notification.into())
    }

    pub async fn get_notification(
        &self,
        id: String,
    ) -> Result<NotificationResponse, NotificationError> {
        let id = Uuid::parse_str(&id).map_err(|e| NotificationError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        let notification = self
            .service
            .get_notification(id)
            .await
            .map_err(NotificationError::from)?;

        Ok(notification.into())
    }

    pub async fn get_all_notifications(
        &self,
        filter: Option<NotificationFilterRequest>,
    ) -> Result<NotificationListResponse, NotificationError> {
        let mut notifications = self
            .service
            .get_all_notifications()
            .await
            .map_err(NotificationError::from)?;

        // Apply filters if provided
        let mut total = notifications.len();
        let mut page = 1;
        let mut per_page = 20;

        if let Some(ref filter) = filter {
            if let Some(ref source) = filter.source {
                notifications.retain(|n| &n.metadata.source == source);
            }
            if let Some(ref status) = filter.status {
                notifications.retain(|n| &n.status == status);
            }
            if let Some(ref priority) = filter.priority {
                notifications.retain(|n| &n.priority == priority);
            }
            if let Some(ref tags) = filter.tags {
                notifications.retain(|n| tags.iter().all(|tag| n.metadata.tags.contains(tag)));
            }
            if let Some(from_date) = filter.from_date {
                notifications.retain(|n| n.created_at >= from_date);
            }
            if let Some(to_date) = filter.to_date {
                notifications.retain(|n| n.created_at <= to_date);
            }
            page = filter.page.unwrap_or(1);
            per_page = filter.per_page.unwrap_or(20);
            total = notifications.len();
        }
        let start = ((page - 1) * per_page) as usize;
        let end = start + per_page as usize;
        let has_more = end < total;

        let notifications = notifications
            .into_iter()
            .skip(start)
            .take(per_page as usize)
            .map(Into::into)
            .collect();

        Ok(NotificationListResponse {
            notifications,
            total,
            has_more,
        })
    }

    pub async fn mark_as_read(&self, id: String) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(&id).map_err(|e| NotificationError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .mark_as_read(id)
            .await
            .map_err(NotificationError::from)
    }

    pub async fn mark_action_required(&self, id: String) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(&id).map_err(|e| NotificationError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .mark_action_required(id)
            .await
            .map_err(NotificationError::from)
    }

    pub async fn mark_action_taken(&self, id: String) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(&id).map_err(|e| NotificationError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .mark_action_taken(id)
            .await
            .map_err(NotificationError::from)
    }

    pub async fn archive_notification(&self, id: String) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(&id).map_err(|e| NotificationError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .archive_notification(id)
            .await
            .map_err(NotificationError::from)
    }

    pub async fn delete_notification(&self, id: String) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(&id).map_err(|e| NotificationError {
            code: "INVALID_ID".to_string(),
            message: e.to_string(),
            details: vec![],
        })?;

        self.service
            .delete_notification(id)
            .await
            .map_err(NotificationError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::{NotificationPriority, NotificationSource},
        services::MockNotificationService,
    };
    use mockall::predicate;

    fn create_test_request() -> CreateNotificationRequest {
        CreateNotificationRequest {
            title: "Test Notification".to_string(),
            content: "Test Content".to_string(),
            priority: NotificationPriority::Medium,
            source: NotificationSource::Email,
            external_id: Some("test123".to_string()),
            url: Some("https://example.com".to_string()),
            tags: vec!["test".to_string()],
            custom_data: None,
        }
    }

    #[tokio::test]
    async fn test_create_notification() {
        let mut mock_service = MockNotificationService::new();
        let request = create_test_request();

        mock_service
            .expect_create_notification()
            .with(
                predicate::eq(request.title.clone()),
                predicate::eq(request.content.clone()),
                predicate::eq(request.priority.clone()),
                predicate::function(move |metadata: &NotificationMetadata| {
                    metadata.source == request.source
                        && metadata.external_id == request.external_id
                        && metadata.url == request.url
                        && metadata.tags == request.tags
                }),
            )
            .returning(|title, content, priority, metadata| {
                Ok(crate::domain::entities::Notification::new(
                    title, content, priority, metadata,
                ))
            });

        let controller = NotificationController::new(Arc::new(mock_service));
        let request = create_test_request();
        let result = controller.create_notification(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.title, "Test Notification");
        assert_eq!(response.content, "Test Content");
        assert!(matches!(response.priority, NotificationPriority::Medium));
    }

    #[tokio::test]
    async fn test_notification_not_found() {
        let mut mock_service = MockNotificationService::new();
        let id = Uuid::new_v4();

        mock_service.expect_get_notification().returning(|_| {
            Err(crate::domain::error::DomainError::NotFoundError(
                "Notification not found".to_string(),
            ))
        });

        let controller = NotificationController::new(Arc::new(mock_service));
        let result = controller.get_notification(id.to_string()).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            NotificationError { code, message: _, details: _ } if code == "NOT_FOUND"
        ));
    }
}
