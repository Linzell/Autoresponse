use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::notification::{NotificationPriority, NotificationSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationEvent {
    NotificationCreated {
        notification_id: Uuid,
        title: String,
        content: String,
        priority: NotificationPriority,
        source: NotificationSource,
        created_at: DateTime<Utc>,
    },
    NotificationProcessed {
        notification_id: Uuid,
        requires_action: bool,
        processed_at: DateTime<Utc>,
    },
    NotificationActionRequired {
        notification_id: Uuid,
        marked_at: DateTime<Utc>,
    },
    NotificationActionTaken {
        notification_id: Uuid,
        marked_at: DateTime<Utc>,
    },
    NotificationRead {
        notification_id: Uuid,
        read_at: DateTime<Utc>,
    },
    NotificationArchived {
        notification_id: Uuid,
        archived_at: DateTime<Utc>,
    },
    NotificationDeleted {
        notification_id: Uuid,
        deleted_at: DateTime<Utc>,
    },
    ResponseGenerated {
        notification_id: Uuid,
        response: String,
        generated_at: DateTime<Utc>,
    },
    ActionExecuted {
        notification_id: Uuid,
        executed_at: DateTime<Utc>,
        success: bool,
        error: Option<String>,
    },
}

impl NotificationEvent {
    pub fn notification_processed(notification_id: Uuid, requires_action: bool) -> Self {
        Self::NotificationProcessed {
            notification_id,
            requires_action,
            processed_at: Utc::now(),
        }
    }

    pub fn notification_action_required(notification_id: Uuid) -> Self {
        Self::NotificationActionRequired {
            notification_id,
            marked_at: Utc::now(),
        }
    }

    pub fn notification_read(notification_id: Uuid) -> Self {
        Self::NotificationRead {
            notification_id,
            read_at: Utc::now(),
        }
    }

    pub fn response_generated(notification_id: Uuid, response: String) -> Self {
        Self::ResponseGenerated {
            notification_id,
            response,
            generated_at: Utc::now(),
        }
    }

    pub fn action_executed(notification_id: Uuid, success: bool, error: Option<String>) -> Self {
        Self::ActionExecuted {
            notification_id,
            executed_at: Utc::now(),
            success,
            error,
        }
    }
}
