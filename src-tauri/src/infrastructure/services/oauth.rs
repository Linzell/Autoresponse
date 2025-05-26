use crate::domain::entities::{AuthConfig, OAuth2Config, ServiceType};
use crate::domain::error::{DomainError, DomainResult};
use crate::domain::repositories::ServiceConfigRepository;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const DEFAULT_REDIRECT_URI: &str = "http://localhost:1420/oauth/callback";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

#[async_trait]
pub trait OAuthService: Send + Sync {
    async fn get_authorization_url(&self, service_type: ServiceType)
        -> Result<String, DomainError>;
    async fn exchange_code_for_token(
        &self,
        code: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, DomainError>;
    async fn refresh_token(
        &self,
        refresh_token: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, DomainError>;
}

pub struct DefaultOAuthService {
    http_client: reqwest::Client,
    config_repository: Arc<dyn ServiceConfigRepository>,
}

impl DefaultOAuthService {
    pub fn new(config_repository: Arc<dyn ServiceConfigRepository>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            config_repository,
        }
    }

    async fn get_oauth_config(&self, service_type: ServiceType) -> DomainResult<OAuth2Config> {
        let configs = self
            .config_repository
            .find_by_service_type(service_type.clone())
            .await?;
        let config = configs
            .first()
            .ok_or_else(|| DomainError::NotFound("OAuth configuration not found".to_string()))?;

        match &config.auth_config {
            AuthConfig::OAuth2(oauth_config) => Ok(oauth_config.clone()),
            _ => Err(DomainError::InvalidOperation(
                "Invalid auth configuration type".to_string(),
            )),
        }
    }

    fn get_default_oauth_config(&self, service_type: ServiceType) -> OAuth2Config {
        match service_type {
            ServiceType::Github => OAuth2Config {
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: DEFAULT_REDIRECT_URI.to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                scope: vec![
                    "repo".to_string(),
                    "user".to_string(),
                    "notifications".to_string(),
                ],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            },
            ServiceType::Google => OAuth2Config {
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: DEFAULT_REDIRECT_URI.to_string(),
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scope: vec![
                    "email".to_string(),
                    "profile".to_string(),
                    "https://www.googleapis.com/auth/gmail.modify".to_string(),
                ],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            },
            ServiceType::Microsoft => OAuth2Config {
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: DEFAULT_REDIRECT_URI.to_string(),
                auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
                    .to_string(),
                token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                scope: vec![
                    "offline_access".to_string(),
                    "User.Read".to_string(),
                    "Mail.ReadWrite".to_string(),
                ],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            },
            ServiceType::Gitlab => OAuth2Config {
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: DEFAULT_REDIRECT_URI.to_string(),
                auth_url: "https://gitlab.com/oauth/authorize".to_string(),
                token_url: "https://gitlab.com/oauth/token".to_string(),
                scope: vec!["api".to_string(), "read_user".to_string()],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            },
            ServiceType::LinkedIn => OAuth2Config {
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: DEFAULT_REDIRECT_URI.to_string(),
                auth_url: "https://www.linkedin.com/oauth/v2/authorization".to_string(),
                token_url: "https://www.linkedin.com/oauth/v2/accessToken".to_string(),
                scope: vec![
                    "r_liteprofile".to_string(),
                    "r_emailaddress".to_string(),
                    "w_member_social".to_string(),
                ],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            },
            _ => OAuth2Config {
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: String::new(),
                auth_url: String::new(),
                token_url: String::new(),
                scope: vec![],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            },
        }
    }
}

#[async_trait]
impl OAuthService for DefaultOAuthService {
    async fn get_authorization_url(
        &self,
        service_type: ServiceType,
    ) -> Result<String, DomainError> {
        let config = self.get_oauth_config(service_type.clone()).await?;
        let mut url = reqwest::Url::parse(&config.auth_url)
            .map_err(|e| DomainError::InvalidOperation(e.to_string()))?;
        let mut query_pairs = url.query_pairs_mut();

        query_pairs
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", &config.redirect_uri)
            .append_pair("scope", &config.scope.join(" "))
            .append_pair("response_type", "code")
            .append_pair("state", &uuid::Uuid::new_v4().to_string());

        // Add service-specific parameters
        match service_type {
            ServiceType::Microsoft => {
                query_pairs.append_pair("prompt", "consent");
            }
            ServiceType::LinkedIn => {
                query_pairs.append_pair("response_type", "code");
            }
            ServiceType::Google => {
                query_pairs
                    .append_pair("access_type", "offline")
                    .append_pair("prompt", "consent");
            }
            _ => {}
        }

        drop(query_pairs);
        Ok(url.to_string())
    }

    async fn exchange_code_for_token(
        &self,
        code: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, DomainError> {
        let config = self.get_oauth_config(service_type.clone()).await?;

        let mut form_data = vec![
            ("client_id", config.client_id),
            ("client_secret", config.client_secret),
            ("code", code),
            ("redirect_uri", config.redirect_uri),
            ("grant_type", "authorization_code".to_string()),
        ];

        // Add service-specific parameters
        match service_type {
            ServiceType::Microsoft => {
                form_data.push(("scope", config.scope.join(" ")));
            }
            ServiceType::LinkedIn => {
                form_data.push(("scope", config.scope.join(" ")));
            }
            _ => {}
        }

        let mut request = self.http_client.post(&config.token_url);

        // Add service-specific headers
        match service_type {
            ServiceType::Github => {
                request = request.header("Accept", "application/json");
            }
            ServiceType::LinkedIn => {
                request = request.header("Content-Type", "application/x-www-form-urlencoded");
            }
            _ => {}
        }

        let response = request
            .form(&form_data)
            .send()
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DomainError::InternalError(format!(
                "Token exchange failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))
    }

    async fn refresh_token(
        &self,
        refresh_token: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, DomainError> {
        let config = self.get_oauth_config(service_type.clone()).await?;

        let mut form_data = vec![
            ("client_id", config.client_id),
            ("client_secret", config.client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token".to_string()),
        ];

        // Add service-specific parameters
        match service_type {
            ServiceType::Google | ServiceType::Microsoft => {
                form_data.push(("scope", config.scope.join(" ")));
            }
            _ => {}
        }

        let mut request = self.http_client.post(&config.token_url);

        // Add service-specific headers
        match service_type {
            ServiceType::Github => {
                request = request.header("Accept", "application/json");
            }
            ServiceType::LinkedIn => {
                request = request.header("Content-Type", "application/x-www-form-urlencoded");
            }
            _ => {}
        }

        let response = request
            .form(&form_data)
            .send()
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DomainError::InternalError(format!(
                "Token refresh failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| DomainError::InternalError(e.to_string()))
    }
}

pub type DynOAuthService = Arc<dyn OAuthService>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            entities::{AuthType, ServiceConfig},
            error::DomainResult,
            repositories::ServiceConfigRepository,
        },
        test_utils::{create_test_oauth2_config, create_test_service_config},
    };
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};
    use serde_json::json;
    use wiremock::{
        matchers::{body_string_contains, header, method},
        Mock, MockServer, ResponseTemplate,
    };

    mock! {
        ServiceConfigRepository {}
        #[async_trait]
        impl ServiceConfigRepository for ServiceConfigRepository {
            async fn save(&self, config: &mut ServiceConfig) -> DomainResult<()>;
            async fn find_by_id(&self, id: uuid::Uuid) -> DomainResult<Option<ServiceConfig>>;
            async fn find_all(&self) -> DomainResult<Vec<ServiceConfig>>;
            async fn find_by_service_type(&self, service_type: ServiceType) -> DomainResult<Vec<ServiceConfig>>;
            async fn find_enabled(&self) -> DomainResult<Vec<ServiceConfig>>;
            async fn delete(&self, id: uuid::Uuid) -> DomainResult<()>;
            async fn update_auth_config(&self, id: uuid::Uuid, auth_config: AuthConfig) -> DomainResult<()>;
            async fn update_enabled_status(&self, id: uuid::Uuid, enabled: bool) -> DomainResult<()>;
            async fn update_last_sync(&self, id: uuid::Uuid) -> DomainResult<()>;
        }
    }

    #[tokio::test]
    async fn test_get_authorization_url() {
        let oauth_config = create_test_oauth2_config();
        let test_config = create_test_service_config(
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth_config),
        );

        let mut mock_repo = MockServiceConfigRepository::new();
        mock_repo
            .expect_find_by_service_type()
            .with(eq(ServiceType::Github))
            .returning(move |_| Ok(vec![test_config.clone()]));

        let service = DefaultOAuthService::new(Arc::new(mock_repo));
        let url = service
            .get_authorization_url(ServiceType::Github)
            .await
            .unwrap();

        assert!(url.contains("github.com"));
        assert!(url.contains("client_id=test_client"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1420%2Foauth%2Fcallback"));
        assert!(url.contains("scope=repo+user"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state="));
    }

    #[tokio::test]
    async fn test_exchange_code_for_token() {
        let mock_server = MockServer::start().await;
        let mut oauth_config = create_test_oauth2_config();
        oauth_config.token_url = mock_server.uri();

        let test_config = create_test_service_config(
            ServiceType::Github,
            AuthType::OAuth2,
            AuthConfig::OAuth2(oauth_config.clone()),
        );

        Mock::given(method("POST"))
            .and(header("Accept", "application/json"))
            .and(body_string_contains("client_id"))
            .and(body_string_contains("client_secret"))
            .and(body_string_contains("test_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "access_token": "test_access_token",
                    "token_type": "bearer",
                    "expires_in": 3600,
                "refresh_token": "test_refresh_token",
                "scope": "test"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let mut mock_repo = MockServiceConfigRepository::new();
        mock_repo
            .expect_find_by_service_type()
            .with(eq(ServiceType::Github))
            .returning(move |_| Ok(vec![test_config.clone()]));

        let service = DefaultOAuthService::new(Arc::new(mock_repo));
        let response = service
            .exchange_code_for_token("test_code".to_string(), ServiceType::Github)
            .await
            .expect("Failed to exchange code for token");

        assert_eq!(response.access_token, "test_access_token");
        assert_eq!(response.token_type, "bearer");
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(
            response.refresh_token,
            Some("test_refresh_token".to_string())
        );
        assert_eq!(response.scope, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_oauth_error_handling() {
        let mut mock_repo = MockServiceConfigRepository::new();
        mock_repo
            .expect_find_by_service_type()
            .returning(|_| Ok(vec![]));

        let service = DefaultOAuthService::new(Arc::new(mock_repo));
        let result = service.get_authorization_url(ServiceType::Github).await;

        assert!(result.is_err());
        match result {
            Err(DomainError::NotFound(msg)) => {
                assert_eq!(msg, "OAuth configuration not found");
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
