use crate::domain::{
    entities::{
        notification::{
            Notification, NotificationMetadata, NotificationPriority, NotificationSource,
            NotificationStatus,
        },
        service_config::{ServiceConfig, ServiceType},
    },
    error::{DomainError, DomainResult},
    AuthConfig,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use super::IntegrationService;

#[derive(Debug)]
pub struct MicrosoftService {
    client: Client,
    config: Arc<RwLock<Option<ServiceConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftEvent {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "resourceData")]
    pub resource_data: serde_json::Value,
    #[serde(rename = "changeType")]
    pub change_type: String,
    #[serde(rename = "clientState")]
    pub client_state: Option<String>,
    #[serde(rename = "subscriptionExpirationDateTime")]
    pub subscription_expiration: String,
    #[serde(rename = "resource")]
    pub resource: String,
    #[serde(rename = "tenantId")]
    pub tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftMessage {
    pub id: String,
    pub subject: Option<String>,
    #[serde(rename = "bodyPreview")]
    pub body_preview: Option<String>,
    pub importance: Option<String>,
    pub from: Option<MicrosoftEmailAddress>,
    #[serde(rename = "receivedDateTime")]
    pub received_date_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftEmailAddress {
    #[serde(rename = "emailAddress")]
    pub email: MicrosoftEmailAddressDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MicrosoftEmailAddressDetails {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftTeamsMessage {
    pub id: String,
    #[serde(rename = "messageType")]
    pub message_type: String,
    pub content: Option<String>,
    #[serde(rename = "channelIdentity")]
    pub channel: Option<MicrosoftTeamsChannel>,
    pub from: MicrosoftTeamsFrom,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftTeamsChannel {
    #[serde(rename = "channelId")]
    pub channel_id: String,
    #[serde(rename = "teamId")]
    pub team_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftTeamsFrom {
    pub user: MicrosoftTeamsUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftTeamsUser {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "userIdentityType")]
    pub identity_type: String,
}

impl Default for MicrosoftService {
    fn default() -> Self {
        Self::new()
    }
}

impl MicrosoftService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_headers(&self) -> DomainResult<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        let config = self
            .config
            .read()
            .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
        let config = config.as_ref().ok_or_else(|| {
            DomainError::ConfigurationError("Microsoft service not configured".to_string())
        })?;

        match &config.auth_config {
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
                    "Invalid auth config for Microsoft services".to_string(),
                ))
            }
        }

        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        Ok(headers)
    }

    fn map_importance_to_priority(&self, importance: &str) -> NotificationPriority {
        match importance {
            "high" | "High" => NotificationPriority::High,
            "low" | "Low" => NotificationPriority::Low,
            _ => NotificationPriority::Medium,
        }
    }

    async fn fetch_message_details(&self, resource_url: &str) -> DomainResult<MicrosoftMessage> {
        let headers = self.get_headers().await?;

        let base_url = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Microsoft service not configured".to_string())
            })?;

            let url = config
                .endpoints
                .endpoints
                .get("graph")
                .and_then(|v| v.as_str())
                .unwrap_or(&config.endpoints.base_url);
            format!(
                "{}/{}",
                url.trim_end_matches('/'),
                resource_url.trim_start_matches('/')
            )
        };

        let response = self
            .client
            .get(&base_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalServiceError(format!("Failed to fetch message details: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalServiceError(format!(
                "Failed to fetch message details. Status: {}",
                response.status()
            )));
        }

        let body = response.text().await.map_err(|e| {
            DomainError::ExternalServiceError(format!("Failed to read message details body: {}", e))
        })?;

        serde_json::from_str(&body).map_err(|e| {
            DomainError::ExternalServiceError(format!("Failed to parse message details: {}", e))
        })
    }

    async fn fetch_teams_message_details(
        &self,
        resource_url: &str,
    ) -> DomainResult<MicrosoftTeamsMessage> {
        let headers = self.get_headers().await?;

        let base_url = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Microsoft service not configured".to_string())
            })?;

            let url = config
                .endpoints
                .endpoints
                .get("graph")
                .and_then(|v| v.as_str())
                .unwrap_or(&config.endpoints.base_url);
            format!(
                "{}/{}",
                url.trim_end_matches('/'),
                resource_url.trim_start_matches('/')
            )
        };

        self.client
            .get(base_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))
    }
}

#[async_trait]
impl super::IntegrationService for MicrosoftService {
    fn service_type(&self) -> ServiceType {
        ServiceType::Microsoft
    }

    async fn initialize(&self, config: ServiceConfig) -> DomainResult<()> {
        if config.service_type != ServiceType::Microsoft {
            return Err(DomainError::ConfigurationError(
                "Invalid service type".to_string(),
            ));
        }

        let base_url = config
            .endpoints
            .endpoints
            .get("graph")
            .and_then(|v| v.as_str())
            .unwrap_or(&config.endpoints.base_url)
            .trim_end_matches('/')
            .to_string();

        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(DomainError::ConfigurationError(
                "Invalid base URL".to_string(),
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

        let base_url = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Microsoft service not configured".to_string())
            })?;

            config
                .endpoints
                .endpoints
                .get("graph")
                .and_then(|v| v.as_str())
                .unwrap_or(&config.endpoints.base_url)
                .trim_end_matches('/')
                .to_string()
        };

        let response = self
            .client
            .get(format!("{}/me", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(response.status().is_success())
    }

    async fn sync_notifications(&self) -> DomainResult<Vec<Notification>> {
        let headers = self.get_headers().await?;

        let base_url = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Microsoft service not configured".to_string())
            })?;

            config
                .endpoints
                .endpoints
                .get("graph")
                .and_then(|v| v.as_str())
                .unwrap_or(&config.endpoints.base_url)
                .trim_end_matches('/')
                .to_string()
        };

        // Fetch email messages
        let messages_url = format!("{}/me/messages", base_url);

        let response = self
            .client
            .get(&messages_url)
            .query(&[("$filter", "isRead eq false")])
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalServiceError(format!("Failed to fetch messages: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalServiceError(format!(
                "Failed to fetch messages. Status: {}",
                response.status()
            )));
        }

        println!("Response status: {}", response.status());
        let body = response.text().await.map_err(|e| {
            DomainError::ExternalServiceError(format!("Failed to read response body: {}", e))
        })?;
        println!("Response body: {}", body);

        let messages: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            DomainError::ExternalServiceError(format!("Failed to parse messages response: {}", e))
        })?;

        let mut notifications = Vec::new();

        // Process email messages
        if let Some(value) = messages.get("value").and_then(|v| v.as_array()) {
            for message in value {
                if let Ok(msg) = serde_json::from_value::<MicrosoftMessage>(message.clone()) {
                    // Create event directly from the message data we already have
                    let notification = Notification {
                        id: uuid::Uuid::new_v4(),
                        title: msg.subject.clone().unwrap_or_default(),
                        content: msg.body_preview.clone().unwrap_or_default(),
                        created_at: DateTime::parse_from_rfc3339(&msg.received_date_time)
                            .unwrap_or_else(|_| Utc::now().into())
                            .into(),
                        updated_at: Utc::now(),
                        read_at: None,
                        action_taken_at: None,
                        priority: self
                            .map_importance_to_priority(msg.importance.as_deref().unwrap_or("")),
                        metadata: NotificationMetadata {
                            source: NotificationSource::Microsoft,
                            external_id: Some(msg.id.clone()),
                            custom_data: Some(serde_json::json!({
                                "from_address": msg.from.as_ref().map(|f| f.email.address.clone()).unwrap_or_default(),
                                "from_name": msg.from.as_ref().map(|f| f.email.name.clone()).unwrap_or_default(),
                            })),
                            tags: Vec::new(),
                            url: None,
                        },
                        status: NotificationStatus::New,
                    };
                    notifications.push(notification);
                }
            }
        }

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

        let resource = payload
            .get("resource")
            .and_then(|r| r.as_str())
            .ok_or_else(|| DomainError::InvalidInput("No resource found".to_string()))?;

        let (title, content, priority) = if resource.starts_with("messages/") {
            let message = self.fetch_message_details(resource).await?;
            (
                format!(
                    "Microsoft: {}",
                    message
                        .subject
                        .unwrap_or_else(|| String::from("No Subject"))
                ),
                message
                    .body_preview
                    .unwrap_or_else(|| String::from("No preview available")),
                self.map_importance_to_priority(message.importance.as_deref().unwrap_or("")),
            )
        } else if resource.starts_with("teams/") {
            let teams_message = self.fetch_teams_message_details(resource).await?;
            (
                format!("Microsoft Teams: {}", teams_message.message_type),
                teams_message
                    .content
                    .unwrap_or_else(|| String::from("No content available")),
                NotificationPriority::Medium,
            )
        } else {
            return Err(DomainError::InvalidInput(
                "Unsupported resource type".to_string(),
            ));
        };

        Ok(<dyn IntegrationService>::event_to_notification(
            self, &event, title, content, priority,
        ))
    }

    async fn send_response(&self, notification: &Notification, response: &str) -> DomainResult<()> {
        let headers = self.get_headers().await?;
        let resource_id = notification
            .metadata
            .external_id
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No external ID found".to_string()))?;

        let base_url = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Microsoft service not configured".to_string())
            })?;

            config
                .endpoints
                .endpoints
                .get("graph")
                .and_then(|v| v.as_str())
                .unwrap_or(&config.endpoints.base_url)
                .trim_end_matches('/')
                .to_string()
        };

        let payload = serde_json::json!({
            "message": {
                "subject": "Re: Automated Response",
                "body": {
                    "contentType": "text",
                    "content": response
                },
                "toRecipients": [
                    {
                        "emailAddress": {
                            "address": notification.metadata.custom_data.as_ref()
                                .and_then(|data| data.get("from_address"))
                                .and_then(|addr| addr.as_str())
                                .unwrap_or("")
                        }
                    }
                ]
            }
        });

        let reply_url = format!(
            "{}/v1.0/me/messages/{}/reply",
            base_url.trim_end_matches('/'),
            resource_id
        );

        self.client
            .post(&reply_url)
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
        let resource_id = notification
            .metadata
            .external_id
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No external ID found".to_string()))?;

        let base_url = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("Microsoft service not configured".to_string())
            })?;

            config
                .endpoints
                .endpoints
                .get("graph")
                .and_then(|v| v.as_str())
                .unwrap_or(&config.endpoints.base_url)
                .trim_end_matches('/')
                .to_string()
        };

        let action_url = match action_type {
            "move" => format!(
                "{}/me/messages/{}/move",
                base_url.trim_end_matches('/'),
                resource_id
            ),
            "forward" => format!(
                "{}/me/messages/{}/forward",
                base_url.trim_end_matches('/'),
                resource_id
            ),
            "markRead" => format!(
                "{}/me/messages/{}",
                base_url.trim_end_matches('/'),
                resource_id
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

impl From<MicrosoftEvent> for super::IntegrationEvent {
    fn from(event: MicrosoftEvent) -> Self {
        Self {
            id: event.id,
            event_type: event.change_type.clone(),
            source: NotificationSource::Microsoft,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.subscription_expiration)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .into(),
            payload: serde_json::json!({
                "resource": event.resource,
                "change_type": event.change_type,
                "resource_data": event.resource_data,
                "tenant_id": event.tenant_id,
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
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{self, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_microsoft_service_initialization() {
        let service = MicrosoftService::new();
        let config = ServiceConfig::new(
            "Microsoft".to_string(),
            ServiceType::Microsoft,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test_client".to_string(),
                client_secret: "test_secret".to_string(),
                redirect_uri: "http://localhost/callback".to_string(),
                auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
                    .to_string(),
                token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                scope: vec!["https://graph.microsoft.com/.default".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://graph.microsoft.com/v1.0".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(service.initialize(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_microsoft_service_sync_notifications() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock Microsoft Graph messages endpoint
        Mock::given(method("GET"))
            .and(path("/v1.0/me/messages"))
            .and(query_param("$filter", "isRead eq false"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "value": [{
                            "id": "test_id",
                            "subject": "Test Email",
                            "bodyPreview": "Test content",
                            "importance": "high",
                            "isRead": false,
                            "from": {
                                "emailAddress": {
                                    "name": "Test User",
                                    "address": "test@example.com"
                                }
                            },
                            "receivedDateTime": "2024-01-01T00:00:00Z"
                        }]
                    })),
            )
            .mount(&mock_server)
            .await;

        // Mock message details endpoint
        Mock::given(method("GET"))
            .and(path("/v1.0/me/messages/test_id"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!({
                        "id": "test_id",
                        "subject": "Test Email",
                        "body": {
                            "content": "Test content",
                            "contentType": "text"
                        },
                        "importance": "high",
                        "isRead": false,
                        "from": {
                            "emailAddress": {
                                "name": "Test User",
                                "address": "test@example.com"
                            }
                        },
                        "receivedDateTime": "2024-01-01T00:00:00Z"
                    })),
            )
            .mount(&mock_server)
            .await;

        // Create service instance with mocked base URL
        let service = MicrosoftService::new();
        let mock_uri = mock_server.uri();

        // Create service config with v1.0 API version in base URL
        let mut endpoints = serde_json::Map::new();
        endpoints.insert(
            "graph".to_string(),
            serde_json::Value::String(format!("{}/v1.0", mock_uri)),
        );

        let config = ServiceConfig::new(
            "Microsoft".to_string(),
            ServiceType::Microsoft,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["https://graph.microsoft.com/.default".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: format!("{}/v1.0", mock_uri),
                endpoints,
            },
        );

        // Initialize service with test configuration
        service
            .initialize(config)
            .await
            .expect("Failed to initialize service");

        // Test sync notifications
        let notifications = service
            .sync_notifications()
            .await
            .expect("Failed to sync notifications");
        assert!(!notifications.is_empty());
        assert_eq!(notifications.len(), 1); // Should have email notification

        // Verify email notification
        let email_notification = notifications.first().expect("Email notification not found");
        assert_eq!(
            email_notification.metadata.source,
            NotificationSource::Microsoft
        );
        assert!(email_notification.content.contains("Test content"));
        assert_eq!(email_notification.priority, NotificationPriority::High);
    }
}
