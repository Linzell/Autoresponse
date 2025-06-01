use crate::domain::{
    entities::{
        notification::{Notification, NotificationPriority, NotificationSource},
        service_config::{ServiceConfig, ServiceType},
    },
    error::{DomainError, DomainResult},
    AuthConfig,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use super::IntegrationService;

#[derive(Debug)]
pub struct GitlabService {
    client: Client,
    config: Arc<RwLock<Option<ServiceConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabEvent {
    pub id: i64,
    pub project_id: i64,
    pub event_type: String,
    pub target_type: Option<String>,
    pub action_name: Option<String>,
    pub target_title: Option<String>,
    pub created_at: String,
    pub author: GitlabUser,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabUser {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub web_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabProject {
    pub id: i64,
    pub name: String,
    pub path_with_namespace: String,
    pub web_url: String,
}

impl Default for GitlabService {
    fn default() -> Self {
        Self::new()
    }
}

impl GitlabService {
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
            DomainError::ConfigurationError("GitLab service not configured".to_string())
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
            AuthConfig::ApiKey(api_key) => {
                headers.insert(
                    "PRIVATE-TOKEN",
                    reqwest::header::HeaderValue::from_str(&api_key.key)?,
                );
            }
            _ => {
                return Err(DomainError::ConfigurationError(
                    "Invalid auth config for GitLab".to_string(),
                ))
            }
        }

        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        Ok(headers)
    }

    async fn get_base_url(&self) -> DomainResult<String> {
        let config = self
            .config
            .read()
            .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
        let config = config.as_ref().ok_or_else(|| {
            DomainError::ConfigurationError("GitLab service not configured".to_string())
        })?;
        Ok(config.endpoints.base_url.clone())
    }
}

#[async_trait]
impl super::IntegrationService for GitlabService {
    fn service_type(&self) -> ServiceType {
        ServiceType::Gitlab
    }

    async fn initialize(&self, config: ServiceConfig) -> DomainResult<()> {
        if config.service_type != ServiceType::Gitlab {
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
        let base_url = self.get_base_url().await?;
        let response = self
            .client
            .get(format!("{}/api/v4/user", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(response.status().is_success())
    }

    async fn sync_notifications(&self) -> DomainResult<Vec<Notification>> {
        let headers = self.get_headers().await?;
        let base_url = self.get_base_url().await?;
        let events: Vec<GitlabEvent> = self
            .client
            .get(format!("{}/api/v4/events", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        let notifications = futures::future::try_join_all(
            events
                .into_iter()
                .map(|event| self.create_notification_from_event(event.into())),
        )
        .await?;

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

        let title = format!(
            "GitLab: {} - {}",
            payload
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Event"),
            payload
                .get("target_title")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );

        let content = serde_json::to_string_pretty(&event.payload)
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        Ok(<dyn IntegrationService>::event_to_notification(
            self,
            &event,
            title,
            content,
            NotificationPriority::Medium,
        ))
    }

    async fn send_response(&self, notification: &Notification, response: &str) -> DomainResult<()> {
        let headers = self.get_headers().await?;
        let base_url = self.get_base_url().await?;
        let external_id = notification
            .metadata
            .external_id
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No external ID found".to_string()))?;

        let metadata = notification
            .metadata
            .custom_data
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No custom data found".to_string()))?;

        let project_id = metadata
            .get("project_id")
            .ok_or_else(|| DomainError::InvalidInput("No project ID found".to_string()))?
            .as_i64()
            .ok_or_else(|| DomainError::InvalidInput("Invalid project ID".to_string()))?;

        // Create a note on the relevant GitLab resource
        let comment_url = format!(
            "{}/api/v4/projects/{}/notes/{}",
            base_url, project_id, external_id
        );
        let payload = serde_json::json!({
            "body": response
        });

        self.client
            .post(&comment_url)
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
        let base_url = self.get_base_url().await?;
        let metadata = notification
            .metadata
            .custom_data
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No custom data found".to_string()))?;

        let project_id = metadata
            .get("project_id")
            .ok_or_else(|| DomainError::InvalidInput("No project ID found".to_string()))?
            .as_i64()
            .ok_or_else(|| DomainError::InvalidInput("Invalid project ID".to_string()))?;

        let action_url = format!(
            "{}/api/v4/projects/{}/{}",
            base_url, project_id, action_type
        );

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

impl From<GitlabEvent> for super::IntegrationEvent {
    fn from(event: GitlabEvent) -> Self {
        Self {
            id: event.id.to_string(),
            event_type: event.event_type.clone(),
            source: NotificationSource::Gitlab,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.created_at)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .into(),
            payload: serde_json::json!({
                "event_type": event.event_type,
                "target_type": event.target_type,
                "action_name": event.action_name,
                "target_title": event.target_title,
                "author": event.author,
                "project_id": event.project_id,
                "data": event.data
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_gitlab_service_initialization() {
        let service = GitlabService::new();
        let config = ServiceConfig::new(
            "GitLab".to_string(),
            ServiceType::Gitlab,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["api".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://gitlab.com".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(service.initialize(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_gitlab_service_sync_notifications() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock GitLab events endpoint
        Mock::given(method("GET"))
            .and(path("/api/v4/events"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(serde_json::json!([{
                        "id": 1,
                        "project_id": 1,
                        "event_type": "push",
                        "target_type": "branch",
                        "action_name": "pushed",
                        "target_title": "main",
                        "created_at": "2024-01-01T00:00:00Z",
                        "author": {
                            "id": 1,
                            "name": "Test User",
                            "username": "test",
                            "avatar_url": "https://gitlab.com/test.png",
                            "web_url": "https://gitlab.com/test"
                        },
                        "data": {
                            "ref": "refs/heads/main",
                            "commit_count": 1
                        }
                    }])),
            )
            .mount(&mock_server)
            .await;

        // Create service instance with mocked base URL
        let service = GitlabService::new();
        let config = ServiceConfig::new(
            "GitLab".to_string(),
            ServiceType::Gitlab,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["api".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: mock_server.uri(),
                endpoints: serde_json::Map::new(),
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

        let notification = &notifications[0];
        assert_eq!(notification.metadata.source, NotificationSource::Gitlab);
        assert!(notification.title.contains("push"));
    }
}
