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
pub struct GithubService {
    client: Client,
    config: Arc<RwLock<Option<ServiceConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubEvent {
    pub id: String,
    pub r#type: String,
    pub repository: GithubRepository,
    pub sender: GithubUser,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRepository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
    pub html_url: String,
}

impl Default for GithubService {
    fn default() -> Self {
        Self::new()
    }
}

impl GithubService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_base_url(&self) -> DomainResult<String> {
        let config = self
            .config
            .read()
            .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
        let config = config.as_ref().ok_or_else(|| {
            DomainError::ConfigurationError("GitHub service not configured".to_string())
        })?;
        Ok(config.endpoints.base_url.clone())
    }

    async fn get_headers(&self) -> DomainResult<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth_config = {
            let config = self
                .config
                .read()
                .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
            let config = config.as_ref().ok_or_else(|| {
                DomainError::ConfigurationError("GitHub service not configured".to_string())
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
                    "Invalid auth config for GitHub".to_string(),
                ))
            }
        }

        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github.v3+json"),
        );

        Ok(headers)
    }
}

#[async_trait]
impl super::IntegrationService for GithubService {
    fn service_type(&self) -> ServiceType {
        ServiceType::Github
    }

    async fn initialize(&self, config: ServiceConfig) -> DomainResult<()> {
        if config.service_type != ServiceType::Github {
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
        let headers = {
            let h = self.get_headers().await?;
            h
        };
        let base_url = {
            let b = self.get_base_url().await?;
            b
        };
        let response = self
            .client
            .get(format!("{}/user", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(response.status().is_success())
    }

    async fn sync_notifications(&self) -> DomainResult<Vec<Notification>> {
        let headers = {
            let h = self.get_headers().await?;
            h
        };
        let base_url = {
            let b = self.get_base_url().await?;
            b
        };
        let events: Vec<GithubEvent> = self
            .client
            .get(format!("{}/events", base_url))
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
        let title = format!("GitHub: {}", event.event_type);

        // Extract repository information if available
        let repository_info = event
            .payload
            .get("repository")
            .and_then(|r| r.as_object())
            .map(|r| {
                format!(
                    "Repository: {}\nURL: {}\n\n",
                    r.get("full_name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown"),
                    r.get("html_url")
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown")
                )
            })
            .unwrap_or_default();

        let content = format!(
            "{}\n{}",
            repository_info,
            serde_json::to_string_pretty(&event.payload)
                .map_err(|e| DomainError::InternalError(e.to_string()))?
        );

        Ok(<dyn IntegrationService>::event_to_notification(
            self,
            &event,
            title,
            content,
            NotificationPriority::Medium,
        ))
    }

    async fn send_response(&self, notification: &Notification, response: &str) -> DomainResult<()> {
        let headers = {
            let h = self.get_headers().await?;
            h
        };
        let external_id = notification
            .metadata
            .external_id
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No external ID found".to_string()))?;

        // Create a comment on the relevant GitHub resource
        let base_url = self.get_base_url().await?;
        let comment_url = format!("{}/repos/comments/{}", base_url, external_id);
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
        let headers = {
            let h = self.get_headers().await?;
            h
        };
        let base_url = {
            let b = self.get_base_url().await?;
            b
        };
        let external_id = notification
            .metadata
            .external_id
            .as_ref()
            .ok_or_else(|| DomainError::InvalidInput("No external ID found".to_string()))?;

        let action_url = format!("{}/repos/actions/{}/{}", base_url, external_id, action_type);

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

impl From<GithubEvent> for super::IntegrationEvent {
    fn from(event: GithubEvent) -> Self {
        let mut payload = event.payload;

        // Include repository information in the payload
        if let serde_json::Value::Object(ref mut map) = payload {
            map.insert(
                "repository".to_string(),
                serde_json::to_value(&event.repository).unwrap_or_default(),
            );
        }

        Self {
            id: event.id,
            event_type: event.r#type,
            source: NotificationSource::Github,
            created_at: chrono::Utc::now(),
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::service_config::{AuthConfig, AuthType, OAuth2Config};
    use crate::domain::services::integrations::IntegrationService;
    use crate::domain::ServiceEndpoints;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{self, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_github_service_initialization() {
        let service = GithubService::new();
        let config = ServiceConfig::new(
            "GitHub".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["repo".to_string()],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://api.github.com".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(service.initialize(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_github_service_sync_notifications() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock GitHub events endpoint
        Mock::given(method("GET"))
            .and(path("/events"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(json!([{
                        "id": "test123",
                        "type": "PushEvent",
                        "repository": {
                            "id": 1,
                            "name": "test/test",
                            "full_name": "test/test",
                            "html_url": "https://github.com/test/test"
                        },
                        "sender": {
                            "id": 1,
                            "login": "test",
                            "avatar_url": "https://github.com/test.png",
                            "html_url": "https://github.com/test"
                        },
                        "payload": {
                            "push_id": 1,
                            "size": 1,
                            "distinct_size": 1,
                            "ref": "refs/heads/main",
                            "head": "abc123",
                            "before": "def456",
                            "commits": []
                        }
                    }])),
            )
            .mount(&mock_server)
            .await;

        // Create service instance with mocked base URL
        let service = GithubService::new();
        let config = ServiceConfig::new(
            "GitHub".to_string(),
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["repo".to_string()],
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
        assert_eq!(notification.metadata.source, NotificationSource::Github);
        assert!(notification.title.contains("PushEvent"));
        assert!(notification.content.contains("test/test"));
        assert_eq!(notification.priority, NotificationPriority::Medium);
    }
}
