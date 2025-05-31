use crate::domain::error::{DomainError, DomainResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

use super::{AIAnalysis, AIService};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfig {
    pub endpoint: String,
    pub api_key: String,
    pub timeout_seconds: u64,
}

impl Default for MCPConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:5000".to_string(),
            api_key: String::new(),
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Serialize)]
struct MCPRequest {
    content: String,
    api_key: String,
    service_type: String,
}

#[derive(Debug, Deserialize)]
struct MCPResponse {
    success: bool,
    response: Option<String>,
    error: Option<String>,
}

#[derive(Debug)]
pub struct MCPConnector {
    config: MCPConfig,
    client: Client,
}

impl MCPConnector {
    pub fn new(config: MCPConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn send_request(&self, content: &str, service_type: &str) -> DomainResult<String> {
        let request = MCPRequest {
            content: content.to_string(),
            api_key: self.config.api_key.clone(),
            service_type: service_type.to_string(),
        };

        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.client
                .post(&self.config.endpoint)
                .json(&request)
                .send(),
        )
        .await
        .map_err(|e| {
            DomainError::ExternalServiceError(format!("MCP service request timed out: {}", e))
        })??;

        let mcp_response: MCPResponse = response.json().await.map_err(|e| {
            DomainError::ExternalServiceError(format!("Failed to parse MCP response: {}", e))
        })?;

        match (
            mcp_response.success,
            mcp_response.response,
            mcp_response.error,
        ) {
            (true, Some(response), _) => Ok(response),
            (false, _, Some(error)) => Err(DomainError::ExternalServiceError(error)),
            _ => Err(DomainError::ExternalServiceError(
                "Invalid MCP response format".to_string(),
            )),
        }
    }
}

#[async_trait]
impl AIService for MCPConnector {
    async fn analyze_content(&self, content: &str) -> DomainResult<AIAnalysis> {
        let response = self.send_request(content, "analyze").await?;

        serde_json::from_str(&response).map_err(|e| {
            DomainError::ValidationError(format!("Failed to parse MCP analysis: {}", e))
        })
    }

    async fn generate_response(&self, context: &str) -> DomainResult<String> {
        self.send_request(context, "generate").await
    }
}

pub type DynMCPConnector = Arc<MCPConnector>;

#[cfg(test)]
mod tests {
    use crate::domain::services::PriorityLevel;

    use super::*;
    use tokio;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_mcp_analyze_content() {
        let mock_server = MockServer::start().await;

        let sample_response = r#"{
            "success": true,
            "response": "{\"requires_action\": true, \"priority_level\": \"High\", \"summary\": \"Test summary\", \"suggested_actions\": [\"Action 1\", \"Action 2\"]}",
            "error": null
        }"#;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sample_response, "application/json"),
            )
            .mount(&mock_server)
            .await;

        let config = MCPConfig {
            endpoint: mock_server.uri(),
            api_key: "test-key".to_string(),
            timeout_seconds: 5,
        };

        let connector = MCPConnector::new(config);
        let analysis = connector
            .analyze_content("Test content")
            .await
            .expect("Analysis should succeed");

        assert!(analysis.requires_action);
        assert_eq!(analysis.priority_level, PriorityLevel::High);
        assert_eq!(analysis.summary, "Test summary");
        assert_eq!(analysis.suggested_actions.len(), 2);
    }

    #[tokio::test]
    async fn test_mcp_generate_response() {
        let mock_server = MockServer::start().await;

        let sample_response = r#"{
            "success": true,
            "response": "Generated response",
            "error": null
        }"#;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sample_response, "application/json"),
            )
            .mount(&mock_server)
            .await;

        let config = MCPConfig {
            endpoint: mock_server.uri(),
            api_key: "test-key".to_string(),
            timeout_seconds: 5,
        };

        let connector = MCPConnector::new(config);
        let response = connector
            .generate_response("Test context")
            .await
            .expect("Response generation should succeed");

        assert_eq!(response, "Generated response");
    }

    #[tokio::test]
    async fn test_mcp_error_handling() {
        let mock_server = MockServer::start().await;

        let error_response = r#"{
            "success": false,
            "response": null,
            "error": "API Error"
        }"#;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(error_response, "application/json"),
            )
            .mount(&mock_server)
            .await;

        let config = MCPConfig {
            endpoint: mock_server.uri(),
            api_key: "test-key".to_string(),
            timeout_seconds: 5,
        };

        let connector = MCPConnector::new(config);
        let result = connector.generate_response("Test context").await;

        assert!(matches!(
            result,
            Err(DomainError::ExternalServiceError(e)) if e == "API Error"
        ));
    }
}
