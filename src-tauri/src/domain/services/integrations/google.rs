use crate::domain::{
    entities::{
        notification::{Notification, NotificationPriority, NotificationSource},
        service_config::{ServiceConfig, ServiceType},
    },
    error::{DomainError, DomainResult},
    AuthConfig,
};
use async_trait::async_trait;
use base64::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use super::IntegrationService;

#[derive(Debug)]
pub struct GoogleService {
    client: Client,
    config: Arc<RwLock<Option<ServiceConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEvent {
    pub id: String,
    pub resource_id: String,
    pub resource_uri: String,
    pub token: String,
    #[serde(rename = "expiration")]
    pub expiration_time: String,
    pub changed_fields: Vec<String>,
    pub message_details: Option<GmailMessage>,
    pub calendar_details: Option<CalendarEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessage {
    pub id: String,
    pub thread_id: String,
    pub label_ids: Vec<String>,
    pub snippet: String,
    pub payload: GmailPayload,
    pub size_estimate: i32,
    pub history_id: String,
    pub internal_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailPayload {
    pub headers: Vec<GmailHeader>,
    pub mime_type: String,
    pub body: GmailBody,
    pub parts: Option<Vec<GmailPart>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailBody {
    pub size: i32,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailPart {
    pub part_id: String,
    pub mime_type: String,
    pub filename: String,
    pub headers: Vec<GmailHeader>,
    pub body: GmailBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: CalendarDateTime,
    pub end: CalendarDateTime,
    pub attendees: Option<Vec<CalendarAttendee>>,
    pub organizer: Option<CalendarOrganizer>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarAttendee {
    pub email: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub response_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarOrganizer {
    pub email: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub self_organized: Option<bool>,
}

impl GoogleService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_base_url(&self) -> DomainResult<String> {
        let url = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Google service not configured".to_string())
            })?;
            config.endpoints.base_url.clone()
        };
        Ok(url)
    }

    async fn get_headers(&self) -> DomainResult<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth_config = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Google service not configured".to_string())
            })?;
            config.auth_config.clone()
        };

        match &auth_config {
            AuthConfig::OAuth2(oauth) => {
                let token = oauth
                    .access_token
                    .as_ref()
                    .ok_or_else(|| DomainError::UnauthorizedError("No access token".to_string()))?;
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))?,
                );
            }
            _ => {
                return Err(DomainError::ConfigurationError(
                    "Invalid auth config for Google services".to_string(),
                ))
            }
        }

        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        Ok(headers)
    }

    async fn fetch_gmail_message(&self, message_id: &str) -> DomainResult<GmailMessage> {
        let headers = self.get_headers().await?;
        let base_url = self.get_base_url().await?;
        self.client
            .get(&format!(
                "{}/gmail/v1/users/me/messages/{}",
                base_url, message_id
            ))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))
    }

    async fn fetch_calendar_event(
        &self,
        event_id: &str,
        calendar_id: &str,
    ) -> DomainResult<CalendarEvent> {
        let headers = self.get_headers().await?;
        self.client
            .get(&format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
                calendar_id, event_id
            ))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))
    }

    fn get_email_header(headers: &[GmailHeader], name: &str) -> Option<String> {
        headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.clone())
    }

    fn determine_priority(headers: &[GmailHeader]) -> NotificationPriority {
        if let Some(importance) = Self::get_email_header(headers, "Importance") {
            match importance.to_lowercase().as_str() {
                "high" => NotificationPriority::High,
                "low" => NotificationPriority::Low,
                _ => NotificationPriority::Medium,
            }
        } else {
            NotificationPriority::Medium
        }
    }
}

#[async_trait]
impl super::IntegrationService for GoogleService {
    fn service_type(&self) -> ServiceType {
        ServiceType::Google
    }

    async fn initialize(&self, config: ServiceConfig) -> DomainResult<()> {
        if config.service_type != ServiceType::Google {
            return Err(DomainError::ConfigurationError(
                "Invalid service type".to_string(),
            ));
        }

        let mut config_lock = self
            .config
            .write()
            .map_err(|_| DomainError::InternalError("Failed to write config".to_string()))?;
        *config_lock = Some(config);
        Ok(())
    }

    async fn test_connection(&self) -> DomainResult<bool> {
        let headers = self.get_headers().await?;
        let response = self
            .client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(response.status().is_success())
    }

    async fn sync_notifications(&self) -> DomainResult<Vec<Notification>> {
        let headers = self.get_headers().await?;

        // Fetch recent Gmail messages
        let base_url = self.get_base_url().await?;
        let gmail_response: serde_json::Value = self
            .client
            .get(format!("{}/gmail/v1/users/me/messages", base_url))
            .query(&[("q", "is:unread")])
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        let mut notifications = Vec::new();

        if let Some(messages) = gmail_response.get("messages").and_then(|m| m.as_array()) {
            for message in messages {
                if let Some(message_id) = message.get("id").and_then(|id| id.as_str()) {
                    if let Ok(message_details) = self.fetch_gmail_message(message_id).await {
                        let event = GoogleEvent {
                            id: message_details.id.clone(),
                            resource_id: message_id.to_string(),
                            resource_uri: format!("gmail/messages/{}", message_id),
                            token: "".to_string(),
                            expiration_time: chrono::Utc::now().to_rfc3339(),
                            changed_fields: vec!["messages".to_string()],
                            message_details: Some(message_details),
                            calendar_details: None,
                        };
                        if let Ok(notification) = self.create_notification_from_event(event.into()).await {
                            notifications.push(notification);
                        }
                    }
                }
            }
        }
        // Add Calendar events sync here if needed

        Ok(notifications)
    }

    async fn create_notification_from_event(
        &self,
        event: super::IntegrationEvent,
    ) -> DomainResult<Notification> {
        let payload = event
            .payload
            .as_object()
            .ok_or_else(|| DomainError::InvalidInput("Invalid event payload".to_string()))?;

        let (title, content, priority) = if let Some(message) = payload.get("message_details") {
            let message_id = message
                .get("id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| DomainError::InvalidInput("Missing message ID".to_string()))?;

            let gmail_message = self.fetch_gmail_message(message_id).await?;
            let headers = gmail_message.payload.headers;

            let subject = Self::get_email_header(&headers, "Subject")
                .unwrap_or_else(|| "No Subject".to_string());
            let from = Self::get_email_header(&headers, "From")
                .unwrap_or_else(|| "Unknown Sender".to_string());

            let title = format!("Gmail: {} (from: {})", subject, from);
            let content = gmail_message.snippet;
            let priority = Self::determine_priority(&headers);

            (title, content, priority)
        } else if let Some(calendar) = payload.get("calendar_details") {
            let event_id = calendar
                .get("id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| {
                    DomainError::InvalidInput("Missing calendar event ID".to_string())
                })?;

            let calendar_id = calendar
                .get("calendar_id")
                .and_then(|id| id.as_str())
                .unwrap_or("primary");
            let calendar_event = self.fetch_calendar_event(event_id, calendar_id).await?;

            let title = format!("Calendar: {}", calendar_event.summary);
            let content = calendar_event
                .description
                .unwrap_or_else(|| "No description".to_string());

            (title, content, NotificationPriority::Medium)
        } else {
            return Err(DomainError::InvalidInput(
                "Unsupported event type".to_string(),
            ));
        };

        Ok(<dyn IntegrationService>::event_to_notification(
            self, &event, title, content, priority,
        ))
    }

    async fn send_response(&self, notification: &Notification, response: &str) -> DomainResult<()> {
        let headers = self.get_headers().await?;
        let message_id = notification
            .metadata
            .external_id
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No external ID found".to_string()))?;

        let thread_id = notification
            .metadata
            .custom_data
            .as_ref()
            .and_then(|data| data.get("thread_id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| DomainError::InvalidInput("No thread ID found".to_string()))?;

        // Create draft response
        let payload = serde_json::json!({
            "raw": BASE64_STANDARD.encode(format!(
                "From: me\r\nIn-Reply-To: {}\r\nReferences: {}\r\nSubject: Re: {}\r\n\r\n{}",
                message_id,
                thread_id,
                notification.title.strip_prefix("Gmail: ").unwrap_or(&notification.title),
                response
            ))
        });

        self.client
            .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(())
    }

    async fn execute_action(
        &self,
        notification: &Notification,
        action_type: &str,
        payload: serde_json::Value,
    ) -> DomainResult<()> {
        let headers = self.get_headers().await?;
        let message_id = notification
            .metadata
            .external_id
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No external ID found".to_string()))?;

        let action_url = match action_type {
            "modify" => format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/modify",
                message_id
            ),
            "trash" => format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/trash",
                message_id
            ),
            "untrash" => format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/untrash",
                message_id
            ),
            _ => {
                return Err(DomainError::InvalidInput(
                    "Unsupported action type".to_string(),
                ))
            }
        };

        self.client
            .post(&action_url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(())
    }
}

impl From<GoogleEvent> for super::IntegrationEvent {
    fn from(event: GoogleEvent) -> Self {
        Self {
            id: event.id,
            event_type: if event.message_details.is_some() {
                "gmail.message".to_string()
            } else if event.calendar_details.is_some() {
                "calendar.event".to_string()
            } else {
                "unknown".to_string()
            },
            source: NotificationSource::Google,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.expiration_time)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .into(),
            payload: serde_json::json!({
                "resource_id": event.resource_id,
                "resource_uri": event.resource_uri,
                "changed_fields": event.changed_fields,
                "message_details": event.message_details,
                "calendar_details": event.calendar_details,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::service_config::{
        AuthConfig, AuthType, OAuth2Config, ServiceEndpoints,
    };
    use crate::domain::services::integrations::IntegrationService;
    use chrono::Utc;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{self, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_google_service_initialization() {
        let service = GoogleService::new();
        let config = ServiceConfig::new(
            "Google".to_string(),
            ServiceType::Google,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test_client".to_string(),
                client_secret: "test_secret".to_string(),
                redirect_uri: "http://localhost/callback".to_string(),
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scope: vec![
                    "https://www.googleapis.com/auth/gmail.modify".to_string(),
                    "https://www.googleapis.com/auth/calendar.events".to_string(),
                ],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://www.googleapis.com".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(service.initialize(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_google_service_sync_notifications() {
        // Start mock server
        let mock_server = MockServer::start().await;
        // Mock Gmail messages endpoint
        let messages_mock = Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages"))
            .and(query_param("q", "is:unread"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "messages": [{
                            "id": "test_id",
                            "threadId": "thread_id"
                        }]
                    })),
            );
        messages_mock.mount(&mock_server).await;

        // Mock Gmail message details endpoint
        let details_mock = Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages/test_id"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "id": "test_id",
                        "thread_id": "thread_id",
                        "snippet": "Test email content",
                        "label_ids": ["UNREAD", "INBOX", "CATEGORY_PRIMARY"],
                        "size_estimate": 100,
                        "history_id": "12345",
                        "internal_date": "1624987654321",
                        "payload": {
                            "headers": [
                                {"name": "Subject", "value": "Test Subject"},
                                {"name": "From", "value": "test@example.com"},
                                {"name": "Importance", "value": "high"}
                            ],
                            "mime_type": "text/plain",
                            "body": {
                                "size": 100,
                                "data": "VGVzdCBlbWFpbCBjb250ZW50"
                            },
                            "parts": null
                        }
                    })),
            );
        details_mock.mount(&mock_server).await;

        // Create service instance with mocked base URL
        let service = GoogleService::new();
        let config = ServiceConfig::new(
            "Google".to_string(),
            ServiceType::Google,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "http://localhost:1420/oauth/callback".to_string(),
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scope: vec![
                    "https://www.googleapis.com/auth/gmail.readonly".to_string(),
                    "https://www.googleapis.com/auth/gmail.modify".to_string(),
                    "https://www.googleapis.com/auth/calendar.readonly".to_string(),
                ],
                access_token: Some("ya29.test_token".to_string()),
                refresh_token: Some("1//test_refresh_token".to_string()),
                token_expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            }),
            ServiceEndpoints {
                base_url: mock_server.uri(),
                endpoints: serde_json::Map::new(),
            },
        );

        // Initialize service with test configuration
        service
            .initialize(config.clone())
            .await
            .expect("Failed to initialize service");

        // Test sync notifications
        let notifications = service
            .sync_notifications()
            .await
            .expect("Failed to sync notifications");
        assert!(!notifications.is_empty());
        assert_eq!(notifications.len(), 1);

        let notification = &notifications[0];
        assert_eq!(notification.metadata.source, NotificationSource::Google);
        assert_eq!(
            notification.metadata.external_id.as_ref().unwrap(),
            "test_id"
        );
        assert!(notification.title.contains("Test Subject"));
        assert!(notification.content.contains("Test email content"));
        assert_eq!(notification.priority, NotificationPriority::High);
    }
}
