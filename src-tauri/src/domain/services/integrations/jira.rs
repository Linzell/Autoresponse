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
pub struct JiraService {
    client: Client,
    config: Arc<RwLock<Option<ServiceConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraEvent {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "webhookEvent")]
    pub webhook_event: String,
    pub issue_event_type_name: Option<String>,
    pub user: JiraUser,
    pub issue: JiraIssue,
    pub changelog: Option<JiraChangelog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraUser {
    #[serde(rename = "self")]
    pub self_url: String,
    pub name: String,
    #[serde(rename = "emailAddress")]
    pub email: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "avatarUrls")]
    pub avatar_urls: Option<JiraAvatarUrls>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraAvatarUrls {
    #[serde(rename = "48x48")]
    pub url_48: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub id: String,
    pub key: String,
    pub fields: JiraIssueFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
    pub description: Option<String>,
    pub status: JiraStatus,
    pub priority: Option<JiraPriority>,
    pub project: JiraProject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatus {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraPriority {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraProject {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraChangelog {
    pub items: Vec<JiraChangelogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraChangelogItem {
    pub field: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

impl JiraService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            config: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_headers(&self) -> DomainResult<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();

        // Clone the config data to avoid lifetime issues
        let config_guard = self
            .config
            .read()
            .map_err(|_| DomainError::InternalError("Failed to read config".to_string()))?;
        let config = config_guard
            .as_ref()
            .ok_or_else(|| {
                DomainError::ConfigurationError("Jira service not configured".to_string())
            })?
            .clone();

        // Create auth header based on config type
        let auth_header_name: String;
        let auth_header_value: String;

        match &config.auth_config {
            AuthConfig::BasicAuth(basic) => {
                auth_header_name = reqwest::header::AUTHORIZATION.to_string();
                let auth = BASE64_STANDARD.encode(format!("{}:{}", basic.username, basic.password));
                auth_header_value = format!("Basic {}", auth);
            }
            AuthConfig::ApiKey(api_key) => {
                auth_header_name = api_key
                    .header_name
                    .clone()
                    .unwrap_or_else(|| "Authorization".to_string());
                auth_header_value = api_key.key.clone();
            }
            _ => {
                return Err(DomainError::ConfigurationError(
                    "Invalid auth config for Jira".to_string(),
                ))
            }
        }

        // Insert headers using owned values
        headers.insert(
            reqwest::header::HeaderName::from_bytes(auth_header_name.as_bytes())?,
            reqwest::header::HeaderValue::from_str(&auth_header_value)?,
        );

        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
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
            DomainError::ConfigurationError("Jira service not configured".to_string())
        })?;
        Ok(config.endpoints.base_url.clone())
    }

    fn map_jira_priority_to_notification_priority(
        priority: Option<&JiraPriority>,
    ) -> NotificationPriority {
        match priority.map(|p| p.name.as_str()) {
            Some("Highest") | Some("High") => NotificationPriority::High,
            Some("Medium") => NotificationPriority::Medium,
            Some("Low") | Some("Lowest") => NotificationPriority::Low,
            _ => NotificationPriority::Medium,
        }
    }
}

#[async_trait]
impl super::IntegrationService for JiraService {
    fn service_type(&self) -> ServiceType {
        ServiceType::Jira
    }

    async fn initialize(&self, config: ServiceConfig) -> DomainResult<()> {
        if config.service_type != ServiceType::Jira {
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
            .get(&format!("{}/rest/api/3/myself", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(response.status().is_success())
    }

    async fn sync_notifications(&self) -> DomainResult<Vec<Notification>> {
        let headers = self.get_headers().await?;
        let base_url = self.get_base_url().await?;

        // Get all issues assigned to the current user or recently updated
        let issues_response: serde_json::Value = self
            .client
            .get(&format!("{}/rest/api/3/search", base_url))
            .headers(headers)
            .query(&[
                ("jql", "assignee = currentUser() OR updated >= -24h"),
                ("fields", "summary,description,status,priority,project"),
            ])
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        let issues = issues_response["issues"].as_array().ok_or_else(|| {
            DomainError::ExternalServiceError("Invalid response format".to_string())
        })?;

        let notifications = futures::future::join_all(issues.iter().map(|issue| {
            let event = JiraEvent {
                id: issue["id"].as_str().unwrap_or("").to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                webhook_event: "jira:issue_updated".to_string(),
                issue_event_type_name: Some("issue_updated".to_string()),
                user: serde_json::from_value(issue["fields"]["assignee"].clone()).unwrap(),
                issue: serde_json::from_value(issue.clone()).unwrap(),
                changelog: None,
            };
            self.create_notification_from_event(event.into())
        }))
        .await
        .into_iter()
        .collect::<DomainResult<Vec<_>>>()?;

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

        let issue = payload
            .get("issue")
            .ok_or_else(|| DomainError::InvalidInput("No issue data found".to_string()))?;

        let issue_key = format!(
            "{}-{}",
            issue["fields"]["project"]["key"].as_str().unwrap_or(""),
            issue["key"].as_str().unwrap_or("")
        );
        let title = format!(
            "Jira: {} - {}",
            issue_key,
            issue["fields"]["summary"].as_str().unwrap_or(""),
        );

        let priority = issue["fields"]["priority"]
            .as_object()
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|name| JiraPriority {
                name: name.to_string(),
            });

        let content = if let Some(description) = issue["fields"]["description"].as_str() {
            description.to_string()
        } else {
            serde_json::to_string_pretty(&event.payload)
                .map_err(|e| DomainError::InternalError(e.to_string()))?
        };

        Ok(<dyn IntegrationService>::event_to_notification(
            self,
            &event,
            title,
            content,
            Self::map_jira_priority_to_notification_priority(priority.as_ref()),
        ))
    }

    async fn send_response(&self, notification: &Notification, response: &str) -> DomainResult<()> {
        let headers = self.get_headers().await?;
        let base_url = self.get_base_url().await?;
        let issue_key = notification
            .metadata
            .custom_data
            .as_ref()
            .and_then(|data| data.get("issue_key"))
            .and_then(|key| key.as_str())
            .ok_or_else(|| DomainError::InvalidInput("No issue key found".to_string()))?;

        // Create a comment on the Jira issue
        let comment_url = format!("{}/rest/api/3/issue/{}/comment", base_url, issue_key);
        let payload = serde_json::json!({
            "body": {
                "type": "doc",
                "version": 1,
                "content": [
                    {
                        "type": "paragraph",
                        "content": [
                            {
                                "type": "text",
                                "text": response
                            }
                        ]
                    }
                ]
            }
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
        let issue_key = notification
            .metadata
            .custom_data
            .as_ref()
            .and_then(|data| data.get("issue_key"))
            .and_then(|key| key.as_str())
            .ok_or_else(|| DomainError::InvalidInput("No issue key found".to_string()))?;

        let action_url = match action_type {
            "transition" => format!("{}/rest/api/3/issue/{}/transitions", base_url, issue_key),
            "assign" => format!("{}/rest/api/3/issue/{}/assignee", base_url, issue_key),
            "update" => format!("{}/rest/api/3/issue/{}", base_url, issue_key),
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

impl From<JiraEvent> for super::IntegrationEvent {
    fn from(event: JiraEvent) -> Self {
        Self {
            id: event.id,
            event_type: event.webhook_event.clone(),
            source: NotificationSource::Jira,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.timestamp)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .into(),
            payload: serde_json::json!({
                "event_type": event.webhook_event,
                "issue_event_type": event.issue_event_type_name,
                "user": event.user,
                "issue": event.issue,
                "changelog": event.changelog,
                "issue_key": event.issue.key
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::service_config::{
        AuthConfig, AuthType, BasicAuthConfig, ServiceEndpoints,
    };
    use crate::domain::services::integrations::IntegrationService;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    impl Default for JiraUser {
        fn default() -> Self {
            Self {
                self_url: "".to_string(),
                name: "".to_string(),
                email: None,
                display_name: "".to_string(),
                avatar_urls: None,
            }
        }
    }

    impl Default for JiraIssue {
        fn default() -> Self {
            Self {
                id: "".to_string(),
                key: "".to_string(),
                fields: JiraIssueFields {
                    summary: "".to_string(),
                    description: None,
                    status: JiraStatus {
                        name: "".to_string(),
                    },
                    priority: None,
                    project: JiraProject {
                        key: "".to_string(),
                        name: "".to_string(),
                    },
                },
            }
        }
    }

    #[tokio::test]
    async fn test_jira_service_initialization() {
        let service = JiraService::new();
        let config = ServiceConfig::new(
            "Jira".to_string(),
            ServiceType::Jira,
            AuthType::BasicAuth,
            AuthConfig::BasicAuth(BasicAuthConfig {
                username: "test@example.com".to_string(),
                password: "test123".to_string(),
            }),
            ServiceEndpoints {
                base_url: "https://your-domain.atlassian.net".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(service.initialize(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_jira_service_sync_notifications() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Mock Jira search endpoint
        Mock::given(method("GET"))
            .and(path("/rest/api/3/search"))
            .and(query_param("jql", "assignee = currentUser() OR updated >= -24h"))
            .and(query_param("fields", "summary,description,status,priority,project"))
            .respond_with(ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(serde_json::json!({
                    "issues": [{
                        "id": "12345",
                        "key": "PROJ-123",
                        "fields": {
                            "summary": "Test Issue",
                            "description": "Test Description",
                            "status": {
                                "name": "In Progress"
                            },
                            "priority": {
                                "name": "High"
                            },
                            "project": {
                                "key": "PROJ",
                                "name": "Test Project"
                            },
                            "assignee": {
                                "self": "https://your-domain.atlassian.net/rest/api/3/user?accountId=123",
                                "name": "test",
                                "displayName": "Test User",
                                "emailAddress": "test@example.com"
                            }
                        }
                    }]
                })))
            .mount(&mock_server)
            .await;

        // Create service instance with mocked base URL
        let service = JiraService::new();
        let config = ServiceConfig::new(
            "Jira".to_string(),
            ServiceType::Jira,
            AuthType::BasicAuth,
            AuthConfig::BasicAuth(BasicAuthConfig {
                username: "test@example.com".to_string(),
                password: "test123".to_string(),
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
        assert_eq!(notification.metadata.source, NotificationSource::Jira);
        assert!(notification.title.contains("PROJ-123"));
        assert!(notification.content.contains("Test Description"));
        assert_eq!(notification.priority, NotificationPriority::High);
    }
}
