use crate::domain::error::DomainResult;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub model: String,
    pub base_url: String,
    pub timeout_seconds: u64,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            model: "qwen3:32b".to_string(),
            base_url: "http://localhost:11434".to_string(),
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AIService: Send + Sync {
    async fn analyze_content(&self, content: &str) -> DomainResult<AIAnalysis>;
    async fn generate_response(&self, context: &str) -> DomainResult<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAnalysis {
    pub requires_action: bool,
    pub priority_level: PriorityLevel,
    pub summary: String,
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PriorityLevel {
    Low,
    Medium,
    High,
    Critical,
}

pub struct OllamaService {
    config: AIConfig,
    client: Client,
}

impl OllamaService {
    pub fn new(config: AIConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn send_prompt(&self, prompt: &str) -> DomainResult<String> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.client
                .post(format!("{}/api/generate", self.config.base_url))
                .json(&request)
                .send(),
        )
        .await
        .map_err(|e| {
            DomainError::ExternalServiceError(format!("AI service request timed out: {}", e))
        })??;

        let ollama_response: OllamaResponse = response.json().await.map_err(|e| {
            DomainError::ExternalServiceError(format!("Failed to parse AI response: {}", e))
        })?;

        Ok(ollama_response.response)
    }
}

#[async_trait]
impl AIService for OllamaService {
    async fn analyze_content(&self, content: &str) -> DomainResult<AIAnalysis> {
        let prompt = format!(
            "Analyze the following content and provide a structured response with the following information:\n\
            1. Does this require action? (true/false)\n\
            2. Priority level (Low, Medium, High, Critical)\n\
            3. Brief summary\n\
            4. List of suggested actions\n\n\
            Content: {}\n\n\
            Respond in JSON format.",
            content
        );

        let response = self.send_prompt(&prompt).await?;

        serde_json::from_str(&response).map_err(|e| {
            DomainError::ValidationError(format!("Failed to parse AI analysis: {}", e))
        })
    }

    async fn generate_response(&self, context: &str) -> DomainResult<String> {
        let prompt = format!(
            "Generate a professional response for the following context:\n\n{}\n\n\
            The response should be concise, clear, and actionable.",
            context
        );

        self.send_prompt(&prompt).await
    }
}

pub type DynAIService = Arc<dyn AIService>;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_analyze_content() {
        let mock_server = MockServer::start().await;

        let sample_response = r#"{
            "response": "{\"requires_action\": true, \"priority_level\": \"High\", \"summary\": \"Urgent review needed for project deployment\", \"suggested_actions\": [\"Review deployment plan\", \"Schedule team meeting\", \"Update documentation\"]}",
            "done": true
        }"#;

        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sample_response, "application/json"),
            )
            .mount(&mock_server)
            .await;

        let config = AIConfig {
            base_url: mock_server.uri(),
            model: "test-model".to_string(),
            timeout_seconds: 5,
        };

        let service = OllamaService::new(config);
        let analysis = service
            .analyze_content("Urgent: Project deployment requires immediate review")
            .await
            .unwrap();

        assert!(analysis.requires_action);
        assert_eq!(analysis.priority_level, PriorityLevel::High);
        assert_eq!(analysis.suggested_actions.len(), 3);
    }
}

use crate::domain::error::DomainError;
