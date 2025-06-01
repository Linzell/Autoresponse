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
pub struct LinkedInService {
    client: Client,
    config: Arc<RwLock<Option<ServiceConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInEvent {
    pub id: String,
    #[serde(rename = "serviceUpdateUrn")]
    pub service_update_urn: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub data: LinkedInEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInEventData {
    #[serde(rename = "messageDetails")]
    pub message_details: Option<LinkedInMessage>,
    #[serde(rename = "jobDetails")]
    pub job_details: Option<LinkedInJob>,
    #[serde(rename = "connectionDetails")]
    pub connection_details: Option<LinkedInConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInMessage {
    pub subject: Option<String>,
    pub body: String,
    pub sender: LinkedInProfile,
    #[serde(rename = "messageType")]
    pub message_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInJob {
    #[serde(rename = "jobPosting")]
    pub posting: JobPosting,
    #[serde(rename = "matchScore")]
    pub match_score: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPosting {
    pub id: String,
    pub title: String,
    pub description: String,
    pub company: Company,
    pub location: Location,
    #[serde(rename = "listedAt")]
    pub listed_at: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInConnection {
    pub profile: LinkedInProfile,
    pub status: String,
    #[serde(rename = "sharedConnections")]
    pub shared_connections: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInProfile {
    pub id: String,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub headline: String,
    #[serde(rename = "publicProfileUrl")]
    pub profile_url: String,
    #[serde(rename = "profilePicture")]
    pub picture_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: String,
    pub name: String,
    #[serde(rename = "logoUrl")]
    pub logo_url: Option<String>,
    pub industry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub country: String,
    pub city: String,
    pub region: String,
    #[serde(rename = "postalCode")]
    pub postal_code: String,
}

impl Default for LinkedInService {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkedInService {
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
                DomainError::ConfigurationError("LinkedIn service not configured".to_string())
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
                DomainError::ConfigurationError("LinkedIn service not configured".to_string())
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
                    "Invalid auth config for LinkedIn services".to_string(),
                ))
            }
        }

        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "X-Restli-Protocol-Version",
            reqwest::header::HeaderValue::from_static("2.0.0"),
        );

        Ok(headers)
    }

    fn determine_priority(&self, event: &LinkedInEvent) -> NotificationPriority {
        match event.event_type.as_str() {
            "JOB_APPLICATION_UPDATE" => NotificationPriority::High,
            "CONNECTION_REQUEST" | "JOB_RECOMMENDATION" | "MESSAGE_RECEIVED" => {
                NotificationPriority::Medium
            }
            _ => NotificationPriority::Low,
        }
    }
}

#[async_trait]
impl super::IntegrationService for LinkedInService {
    fn service_type(&self) -> ServiceType {
        ServiceType::LinkedIn
    }

    async fn initialize(&self, config: ServiceConfig) -> DomainResult<()> {
        if config.service_type != ServiceType::LinkedIn {
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
        let base_url = self.get_base_url().await?;
        let headers = self.get_headers().await?;
        let response = self
            .client
            .get(format!("{}/api/v2/messages", base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        Ok(response.status().is_success())
    }

    async fn sync_notifications(&self) -> DomainResult<Vec<Notification>> {
        // Get headers and base URL before any async operations
        let headers = {
            let h = self.get_headers().await?;
            h
        };
        let base_url = {
            let b = self.get_base_url().await?;
            b
        };
        let mut notifications = Vec::new();

        // Fetch messages
        let messages_response: serde_json::Value = self
            .client
            .get(format!("{}/api/v2/messages", base_url))
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        if let Some(messages) = messages_response.get("elements").and_then(|e| e.as_array()) {
            for message in messages {
                let event = LinkedInEvent {
                    id: message["id"].as_str().unwrap_or("").to_string(),
                    service_update_urn: format!(
                        "urn:li:message:{}",
                        message["id"].as_str().unwrap_or("")
                    ),
                    event_type: "MESSAGE_RECEIVED".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    data: LinkedInEventData {
                        message_details: Some(serde_json::from_value(
                            message["messageDetails"].clone(),
                        )?),
                        connection_details: None,
                        job_details: None,
                    },
                };
                notifications.push(self.create_notification_from_event(event.into()).await?);
            }
        }

        // Fetch job recommendations
        let jobs_response: serde_json::Value = self
            .client
            .get(format!("{}/api/v2/jobs/recommendations", base_url))
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        tracing::debug!("LinkedIn jobs response: {:?}", jobs_response);
        if let Some(jobs) = jobs_response.get("elements").and_then(|e| e.as_array()) {
            for job in jobs {
                tracing::debug!("Processing job: {:?}", job);

                let job_posting = job.get("jobPosting").ok_or_else(|| {
                    tracing::error!("Missing jobPosting field in job data");
                    DomainError::InvalidInput("Missing jobPosting field".to_string())
                })?;
                tracing::debug!("Job posting data: {:?}", job_posting);

                let job_details = LinkedInJob {
                    posting: serde_json::from_value(job_posting.clone()).map_err(|e| {
                        tracing::error!("Failed to parse job posting: {:?}", e);
                        DomainError::InvalidInput(format!("Invalid job posting format: {}", e))
                    })?,
                    match_score: Some(job["matchScore"].as_f64().unwrap_or_else(|| {
                        tracing::warn!("Missing or invalid matchScore, defaulting to 0.0");
                        0.0
                    })),
                    status: job["status"]
                        .as_str()
                        .unwrap_or_else(|| {
                            tracing::warn!("Missing status, defaulting to UNKNOWN");
                            "UNKNOWN"
                        })
                        .to_string(),
                };
                tracing::debug!("Created job details: {:?}", job_details);

                let event = LinkedInEvent {
                    id: job["id"]
                        .as_str()
                        .unwrap_or_else(|| {
                            tracing::warn!("Missing job id, using empty string");
                            ""
                        })
                        .to_string(),
                    service_update_urn: format!(
                        "urn:li:job:{}",
                        job_posting
                            .get("id")
                            .and_then(|id| id.as_str())
                            .unwrap_or_else(|| {
                                tracing::warn!("Missing job posting id, using empty string");
                                ""
                            })
                    ),
                    event_type: "JOB_RECOMMENDATION".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    data: LinkedInEventData {
                        job_details: Some(job_details),
                        message_details: None,
                        connection_details: None,
                    },
                };
                notifications.push(self.create_notification_from_event(event.into()).await?);
            }
        }

        // Fetch connection invitations
        let connections_response: serde_json::Value = self
            .client
            .get(format!("{}/api/v2/invitations", base_url))
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?
            .json()
            .await
            .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;

        tracing::debug!("LinkedIn connections response: {:?}", connections_response);
        if let Some(connections) = connections_response
            .get("elements")
            .and_then(|e| e.as_array())
        {
            for connection in connections {
                tracing::debug!("Processing connection: {:?}", connection);
                let event = LinkedInEvent {
                    id: connection["invitation"]["id"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    service_update_urn: format!(
                        "urn:li:invitation:{}",
                        connection["invitation"]["id"].as_str().unwrap_or("")
                    ),
                    event_type: "CONNECTION_REQUEST".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    data: LinkedInEventData {
                        connection_details: Some(serde_json::from_value(
                            connection["invitation"].clone(),
                        )?),
                        message_details: None,
                        job_details: None,
                    },
                };
                notifications.push(self.create_notification_from_event(event.into()).await?);
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

        let (title, content) = match event.event_type.as_str() {
            "JOB_RECOMMENDATION" => {
                let job = payload
                    .get("jobPosting")
                    .and_then(|j| j.as_object())
                    .ok_or_else(|| DomainError::InvalidInput("Invalid job details".to_string()))?;

                let title = format!(
                    "LinkedIn Job: {} ({}% Match)",
                    job.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("New Job Opportunity"),
                    job.get("match_score")
                        .and_then(|s| s.as_f64())
                        .map(|s| (s * 100.0).round())
                        .unwrap_or(0.0)
                );
                let content = format!(
                    "{}\n\nCompany: {}\nLocation: {}, {}",
                    job.get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("No description available"),
                    job.get("company")
                        .and_then(|c| c.as_object())
                        .and_then(|c| c.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown Company"),
                    job.get("location")
                        .and_then(|l| l.as_object())
                        .and_then(|l| l.get("city"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("Unknown City"),
                    job.get("location")
                        .and_then(|l| l.as_object())
                        .and_then(|l| l.get("country"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("Unknown Country")
                );

                (title, content)
            }
            "MESSAGE_RECEIVED" => {
                let message = payload.get("messageDetails").ok_or_else(|| {
                    DomainError::InvalidInput("Invalid message details".to_string())
                })?;

                let sender = message.get("sender").ok_or_else(|| {
                    DomainError::InvalidInput("Invalid sender in message details".to_string())
                })?;

                let title = format!(
                    "Message from {} {}",
                    sender
                        .get("firstName")
                        .and_then(|f| f.as_str())
                        .unwrap_or(""),
                    sender
                        .get("lastName")
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                );

                let content = message
                    .get("body")
                    .and_then(|b| b.as_str())
                    .unwrap_or("No message content")
                    .to_string();

                (title, content)
            }
            "CONNECTION_REQUEST" => {
                let connection = payload.get("connectionDetails").ok_or_else(|| {
                    DomainError::InvalidInput("Invalid connection details".to_string())
                })?;

                let profile = connection.get("profile").ok_or_else(|| {
                    DomainError::InvalidInput("Invalid profile in connection details".to_string())
                })?;

                let name = format!(
                    "{} {}",
                    profile
                        .get("firstName")
                        .and_then(|f| f.as_str())
                        .unwrap_or(""),
                    profile
                        .get("lastName")
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                );

                let title = format!("Connection Request from {}", name);
                let content = format!(
                    "{}\n\nShared Connections: {}",
                    profile
                        .get("headline")
                        .and_then(|h| h.as_str())
                        .unwrap_or("Would like to connect with you"),
                    connection
                        .get("sharedConnections")
                        .and_then(|s| s.as_i64())
                        .unwrap_or(0)
                );

                (title, content)
            }
            _ => (
                "LinkedIn Update".to_string(),
                serde_json::to_string_pretty(&event.payload)
                    .map_err(|e| DomainError::InternalError(e.to_string()))?,
            ),
        };

        Ok(<dyn IntegrationService>::event_to_notification(
            self,
            &event,
            title,
            content,
            self.determine_priority(&serde_json::from_value(event.payload.clone())?),
        ))
    }

    async fn send_response(&self, notification: &Notification, response: &str) -> DomainResult<()> {
        let headers = self.get_headers().await?;
        let event_type = notification
            .metadata
            .custom_data
            .as_ref()
            .and_then(|data| data.get("event_type"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| DomainError::InvalidInput("No event type found".to_string()))?;

        match event_type {
            "CONNECTION_REQUEST" => {
                let invitation_id =
                    notification.metadata.external_id.as_ref().ok_or_else(|| {
                        DomainError::InvalidInput("No invitation ID found".to_string())
                    })?;

                let action_url = format!(
                    "https://api.linkedin.com/v2/invitations/{}/action",
                    invitation_id
                );
                let payload = serde_json::json!({
                    "action": "accept",
                    "message": response
                });

                self.client
                    .post(&action_url)
                    .headers(headers)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;
            }
            "JOB_RECOMMENDATION" => {
                let job_id = notification
                    .metadata
                    .external_id
                    .as_ref()
                    .ok_or_else(|| DomainError::InvalidInput("No job ID found".to_string()))?;

                let action_url =
                    format!("https://api.linkedin.com/v2/jobs/{}/applications", job_id);
                let payload = serde_json::json!({
                    "message": response
                });

                self.client
                    .post(&action_url)
                    .headers(headers)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| DomainError::ExternalServiceError(e.to_string()))?;
            }
            _ => {
                return Err(DomainError::InvalidInput(
                    "Unsupported event type for response".to_string(),
                ))
            }
        }

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
            .ok_or_else(|| DomainError::InvalidInput("No resource ID found".to_string()))?;

        let action_url = match action_type {
            "save_job" => format!("https://api.linkedin.com/v2/jobs/{}/saves", resource_id),
            "apply_job" => format!(
                "https://api.linkedin.com/v2/jobs/{}/applications",
                resource_id
            ),
            "connect" => format!(
                "https://api.linkedin.com/v2/invitations/{}/action",
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

impl From<LinkedInEvent> for super::IntegrationEvent {
    fn from(event: LinkedInEvent) -> Self {
        let id = event.id.clone();
        Self {
            id: event.id,
            event_type: event.event_type.clone(),
            source: NotificationSource::LinkedIn,
            created_at: chrono::DateTime::parse_from_rfc3339(&event.created_at)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .into(),
            payload: serde_json::json!({
                "eventType": event.event_type,
                "serviceUpdateUrn": event.service_update_urn,
                "id": id,
                "jobPosting": event.data.clone().job_details.map(|j| {
                    let mut posting = serde_json::to_value(j.posting).unwrap_or_default();
                    if let serde_json::Value::Object(ref mut map) = posting {
                        map.insert("match_score".to_string(), serde_json::to_value(j.match_score).unwrap_or_default());
                        map.insert("status".to_string(), serde_json::to_value(j.status).unwrap_or_default());
                    }
                    posting
                }),
                "messageDetails": event.data.message_details,
                "connectionDetails": event.data.connection_details,
                "createdAt": event.created_at,
                "data": serde_json::to_value(event.data.clone()).unwrap_or_default(),
                "profile": event.data.connection_details.as_ref().and_then(|c| serde_json::to_value(c.profile.clone()).ok()).unwrap_or_default()
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
    use wiremock;

    #[tokio::test]
    async fn test_linkedin_service_initialization() {
        tracing::debug!("Creating LinkedIn service instance");
        let service = LinkedInService::new();

        tracing::debug!("Setting up service configuration");
        let config = ServiceConfig::new(
            "LinkedIn".to_string(),
            ServiceType::LinkedIn,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test_client".to_string(),
                client_secret: "test_secret".to_string(),
                redirect_uri: "http://localhost/callback".to_string(),
                auth_url: "https://www.linkedin.com/oauth/v2/authorization".to_string(),
                token_url: "https://www.linkedin.com/oauth/v2/accessToken".to_string(),
                scope: vec![
                    "r_liteprofile".to_string(),
                    "r_emailaddress".to_string(),
                    "w_member_social".to_string(),
                ],
                access_token: Some("test_token".to_string()),
                refresh_token: None,
                token_expires_at: None,
            }),
            ServiceEndpoints {
                base_url: "https://api.linkedin.com/v2".to_string(),
                endpoints: serde_json::Map::new(),
            },
        );

        assert!(service.initialize(config).await.is_ok());
    }

    #[tokio::test]
    async fn test_linkedin_service_sync_notifications() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{self, Mock, MockServer, ResponseTemplate};

        // Initialize test logging
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .init();

        tracing::info!("Starting LinkedIn service notification sync test");
        let mock_server = MockServer::start().await;

        // Mock messages endpoint
        Mock::given(method("GET"))
            .and(path("/api/v2/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "elements": [{
                    "id": "msg123",
                    "messageDetails": {
                        "id": "msg123",
                        "subject": "Connection Request",
                        "body": "Hi, I'd like to connect with you",
                        "sender": {
                            "id": "user123",
                            "firstName": "John",
                            "lastName": "Doe",
                            "headline": "Software Developer",
                            "publicProfileUrl": "https://linkedin.com/in/johndoe",
                            "profilePicture": "https://example.com/pic.jpg"
                        },
                        "messageType": "INMAIL"
                    }
                }]
            })))
            .mount(&mock_server)
            .await;

        // Mock jobs recommendations endpoint
        Mock::given(method("GET"))
            .and(path("/api/v2/jobs/recommendations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "elements": [{
                    "id": "test_job_id_full",
                    "jobPosting": {
                        "id": "test_job_id",
                        "title": "Software Engineer",
                        "description": "Looking for a talented developer...",
                        "company": {
                            "id": "company_id",
                            "name": "Tech Corp",
                            "logoUrl": "https://example.com/logo.jpg",
                            "industry": "Software"
                        },
                        "location": {
                            "country": "US",
                            "city": "San Francisco",
                            "region": "CA",
                            "postalCode": "94105"
                        },
                        "listedAt": "2024-01-01T00:00:00Z",
                        "expiresAt": "2024-02-01T00:00:00Z"
                    },
                    "matchScore": 0.85,
                    "status": "OPEN"
                }]
            })))
            .mount(&mock_server)
            .await;

        // Mock invitations endpoint
        Mock::given(method("GET"))
            .and(path("/api/v2/invitations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "elements": [{
                    "invitation": {
                        "id": "test_invitation_id",
                        "profile": {
                            "id": "profile123",
                            "firstName": "John",
                            "lastName": "Doe",
                            "headline": "Software Developer",
                            "publicProfileUrl": "https://linkedin.com/in/johndoe",
                            "profilePicture": "https://example.com/picture.jpg"
                        },
                        "status": "PENDING",
                        "sharedConnections": 5
                    }
                }]
            })))
            .mount(&mock_server)
            .await;

        // Create service instance with mocked base URL
        let service = LinkedInService::new();
        let config = ServiceConfig::new(
            "LinkedIn".to_string(),
            ServiceType::LinkedIn,
            AuthType::OAuth2,
            AuthConfig::OAuth2(OAuth2Config {
                client_id: "test".to_string(),
                client_secret: "test".to_string(),
                redirect_uri: "test".to_string(),
                auth_url: "test".to_string(),
                token_url: "test".to_string(),
                scope: vec!["test".to_string()],
                access_token: Some("test".to_string()),
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

        tracing::debug!("Received notifications: {:?}", notifications);
        assert!(!notifications.is_empty());
        assert_eq!(notifications.len(), 3); // Should have message, job and connection notifications

        // Verify message notification
        let message_notification = notifications
            .iter()
            .find(|n| n.title.contains("Message from John Doe"))
            .expect("Message notification not found");
        assert_eq!(
            message_notification.metadata.source,
            NotificationSource::LinkedIn
        );
        assert!(message_notification
            .content
            .contains("Hi, I'd like to connect with you"));
        assert_eq!(message_notification.priority, NotificationPriority::Medium);

        // Verify job notification
        let job_notification = notifications
            .iter()
            .find(|n| n.title.contains("Software Engineer"))
            .expect("Job notification not found");
        assert_eq!(
            job_notification.metadata.source,
            NotificationSource::LinkedIn
        );
        assert!(job_notification
            .content
            .contains("Looking for a talented developer"));
        assert_eq!(job_notification.priority, NotificationPriority::Medium);

        // Verify connection notification
        let connection_notification = notifications
            .iter()
            .find(|n| n.title.contains("Connection Request from John Doe"))
            .expect("Connection notification not found");
        assert_eq!(
            connection_notification.metadata.source,
            NotificationSource::LinkedIn
        );
        assert!(connection_notification
            .content
            .contains("Software Developer"));
        assert_eq!(
            connection_notification.priority,
            NotificationPriority::Medium
        );
    }
}
