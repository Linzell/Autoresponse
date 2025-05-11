use crate::domain::entities::{OAuth2Config, ServiceType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

#[async_trait]
pub trait OAuthService: Send + Sync {
    async fn get_authorization_url(&self, service_type: ServiceType) -> String;
    async fn exchange_code_for_token(
        &self,
        code: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, String>;
    async fn refresh_token(
        &self,
        refresh_token: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, String>;
}

pub struct DefaultOAuthService {
    http_client: reqwest::Client,
}

impl DefaultOAuthService {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    fn get_oauth_config(&self, service_type: ServiceType) -> OAuth2Config {
        match service_type {
            ServiceType::Github => OAuth2Config {
                client_id: std::env::var("GITHUB_CLIENT_ID").unwrap_or_default(),
                client_secret: std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default(),
                redirect_uri: "http://localhost:1420/oauth/callback".to_string(),
                auth_url: "https://github.com/login/oauth/authorize".to_string(),
                token_url: "https://github.com/login/oauth/access_token".to_string(),
                scope: vec!["repo".to_string(), "user".to_string()],
                access_token: None,
                refresh_token: None,
                token_expires_at: None,
            },
            ServiceType::Google => OAuth2Config {
                client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
                client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
                redirect_uri: "http://localhost:1420/oauth/callback".to_string(),
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                scope: vec!["email".to_string(), "profile".to_string()],
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
    async fn get_authorization_url(&self, service_type: ServiceType) -> String {
        let config = self.get_oauth_config(service_type);
        let mut url = reqwest::Url::parse(&config.auth_url).unwrap();

        url.query_pairs_mut()
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", &config.redirect_uri)
            .append_pair("scope", &config.scope.join(" "))
            .append_pair("response_type", "code");

        url.to_string()
    }

    async fn exchange_code_for_token(
        &self,
        code: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, String> {
        let config = self.get_oauth_config(service_type);

        let response = self
            .http_client
            .post(&config.token_url)
            .form(&[
                ("client_id", config.client_id),
                ("client_secret", config.client_secret),
                ("code", code),
                ("redirect_uri", config.redirect_uri),
                ("grant_type", "authorization_code".to_string()),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| e.to_string())
    }

    async fn refresh_token(
        &self,
        refresh_token: String,
        service_type: ServiceType,
    ) -> Result<TokenResponse, String> {
        let config = self.get_oauth_config(service_type);

        let response = self
            .http_client
            .post(&config.token_url)
            .form(&[
                ("client_id", config.client_id),
                ("client_secret", config.client_secret),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token".to_string()),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| e.to_string())
    }
}

pub type DynOAuthService = Arc<dyn OAuthService>;

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        OAuthService {}

        #[async_trait]
        impl OAuthService for OAuthService {
            async fn get_authorization_url(&self, service_type: ServiceType) -> String;
            async fn exchange_code_for_token(&self, code: String, service_type: ServiceType) -> Result<TokenResponse, String>;
            async fn refresh_token(&self, refresh_token: String, service_type: ServiceType) -> Result<TokenResponse, String>;
        }
    }

    #[tokio::test]
    async fn test_get_authorization_url() {
        let service = DefaultOAuthService::new();
        let url = service.get_authorization_url(ServiceType::Github).await;
        assert!(url.contains("github.com"));
        assert!(url.contains("client_id="));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("scope="));
    }
}
