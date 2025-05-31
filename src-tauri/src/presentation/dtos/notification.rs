use crate::domain::entities::{NotificationPriority, NotificationSource, NotificationStatus};
use crate::presentation::middleware::ValidatedCommand;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateNotificationRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Title must be between 1 and 200 characters"
    ))]
    pub title: String,
    #[validate(length(
        min = 1,
        max = 5000,
        message = "Content must be between 1 and 5000 characters"
    ))]
    pub content: String,
    pub priority: NotificationPriority,
    pub source: NotificationSource,
    pub external_id: Option<String>,
    #[validate(url(message = "URL must be valid"))]
    pub url: Option<String>,
    #[validate(length(max = 10, message = "Maximum 10 tags allowed"))]
    pub tags: Vec<String>,
    pub custom_data: Option<serde_json::Value>,
}

impl ValidatedCommand for CreateNotificationRequest {}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateNotificationRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Title must be between 1 and 200 characters"
    ))]
    pub title: Option<String>,
    #[validate(length(
        min = 1,
        max = 5000,
        message = "Content must be between 1 and 5000 characters"
    ))]
    pub content: Option<String>,
    pub priority: Option<NotificationPriority>,
    #[validate(length(max = 10, message = "Maximum 10 tags allowed"))]
    pub tags: Option<Vec<String>>,
    pub custom_data: Option<serde_json::Value>,
}

impl ValidatedCommand for UpdateNotificationRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: String,
    pub title: String,
    pub content: String,
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub source: NotificationSource,
    pub external_id: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub custom_data: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    pub read_at: Option<String>,
    pub action_taken_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationListResponse {
    pub notifications: Vec<NotificationResponse>,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NotificationFilterRequest {
    pub source: Option<NotificationSource>,
    pub status: Option<NotificationStatus>,
    pub priority: Option<NotificationPriority>,
    #[validate(length(max = 10, message = "Maximum 10 tags allowed for filtering"))]
    pub tags: Option<Vec<String>>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    #[validate(range(min = 1, message = "Page must be greater than 0"))]
    pub page: Option<u32>,
    #[validate(range(
        min = 1,
        max = 100,
        message = "Items per page must be between 1 and 100"
    ))]
    pub per_page: Option<u32>,
}

impl ValidatedCommand for NotificationFilterRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationError {
    pub code: String,
    pub message: String,
    pub details: Vec<String>,
}

impl From<crate::domain::error::DomainError> for NotificationError {
    fn from(error: crate::domain::error::DomainError) -> Self {
        match error {
            crate::domain::error::DomainError::ValidationError(msg) => Self {
                code: "VALIDATION_ERROR".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::NotFoundError(msg) => Self {
                code: "NOT_FOUND".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::NotFound(msg) => Self {
                code: "NOT_FOUND".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::InvalidInput(msg) => Self {
                code: "VALIDATION_ERROR".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::InvalidOperation(msg) => Self {
                code: "INVALID_OPERATION".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::UnauthorizedError(msg) => Self {
                code: "UNAUTHORIZED".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::ConflictError(msg) => Self {
                code: "CONFLICT".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::InternalError(msg) => Self {
                code: "INTERNAL_ERROR".to_string(),
                message: msg,
                details: vec![],
            },
            crate::domain::error::DomainError::ExternalServiceError(msg) => Self {
                code: "EXTERNAL_SERVICE_ERROR".to_string(),
                message: msg,
                details: vec![],
            },
        }
    }
}

impl From<crate::domain::entities::Notification> for NotificationResponse {
    fn from(notification: crate::domain::entities::Notification) -> Self {
        Self {
            id: notification.id.to_string(),
            title: notification.title,
            content: notification.content,
            priority: notification.priority,
            status: notification.status,
            source: notification.metadata.source,
            external_id: notification.metadata.external_id,
            url: notification.metadata.url,
            tags: notification.metadata.tags,
            custom_data: notification.metadata.custom_data,
            created_at: notification.created_at.to_rfc3339(),
            updated_at: notification.updated_at.to_rfc3339(),
            read_at: notification.read_at.map(|dt| dt.to_rfc3339()),
            action_taken_at: notification.action_taken_at.map(|dt| dt.to_rfc3339()),
        }
    }
}
