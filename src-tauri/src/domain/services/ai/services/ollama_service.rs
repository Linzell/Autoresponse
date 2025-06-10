use crate::domain::{
    error::DomainResult,
    services::ai::{FormalityLevel, ResponseLength, ResponseTone, UserPreferences},
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::RwLock, time::timeout};

use super::super::{AIAnalysis, AIConfig, AIService};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub content: String,
}

#[derive(Debug)]
pub struct ConversationMemory {
    pub entries: VecDeque<ConversationEntry>,
    pub max_entries: usize,
}

impl ConversationMemory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn add_entry(&mut self, role: String, content: String) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(ConversationEntry {
            timestamp: Utc::now(),
            role,
            content,
        });
    }

    pub fn get_context(&self) -> String {
        self.entries
            .iter()
            .map(|entry| format!("{}({}): {}", entry.role, entry.timestamp, entry.content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug)]
pub struct OllamaService {
    pub config: Arc<RwLock<AIConfig>>,
    pub client: Client,
    pub memory: Arc<Mutex<ConversationMemory>>,
}

impl OllamaService {
    pub fn new(config: AIConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            client: Client::new(),
            memory: Arc::new(Mutex::new(ConversationMemory::new(10))),
        }
    }

    async fn send_prompt(&self, prompt: String) -> DomainResult<String> {
        let config = self.config.read().await;
        let request = OllamaRequest {
            model: config.model.clone(),
            prompt,
            stream: false,
        };

        let response = timeout(
            Duration::from_secs(config.timeout_seconds),
            self.client
                .post(format!("{}/api/generate", config.base_url))
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

#[async_trait::async_trait]
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

        let response = self.send_prompt(prompt).await?;

        serde_json::from_str(&response).map_err(|e| {
            DomainError::ValidationError(format!("Failed to parse AI analysis: {}", e))
        })
    }

    async fn generate_response(&self, context: &str) -> DomainResult<String> {
        let config = self.config.read().await;
        let prefs = &config.user_preferences;
        let tone = format!("{:?}", prefs.tone).to_lowercase();
        let length = format!("{:?}", prefs.length).to_lowercase();
        let formality = format!("{:?}", prefs.formality_level).to_lowercase();

        let custom_instructions = prefs.custom_instructions.join("\n");

        let conversation_history = self
            .memory
            .lock()
            .expect("Failed to acquire lock on memory for conversation history")
            .get_context();

        let prompt = format!(
            "Generate a response for the following context, adhering to these specifications:\n\
            Tone: {}\n\
            Length: {}\n\
            Language: {}\n\
            Formality: {}\n\
            Custom Instructions:\n{}\n\n\
            Previous Conversation:\n{}\n\n\
            Current Context:\n{}\n\n\
            The response should be clear and actionable while maintaining consistency with previous interactions.",
            tone, length, prefs.language, formality, custom_instructions, conversation_history, context
        );

        // Add user's message to memory first
        self.memory
            .lock()
            .unwrap()
            .add_entry("User".to_string(), context.to_string());

        let response = self.send_prompt(prompt).await?;
        self.memory
            .lock()
            .unwrap()
            .add_entry("Assistant".to_string(), response.clone());
        Ok(response)
    }

    async fn configure(
        &self,
        config: crate::presentation::dtos::AIConfigRequest,
    ) -> DomainResult<()> {
        let mut ai_config = self.config.write().await;
        *ai_config = AIConfig {
            model: config.model,
            base_url: "http://localhost:11434".to_string(), // Could be made configurable
            timeout_seconds: 30,
            user_preferences: UserPreferences {
                // Map AIConfigRequest preferences to UserPreferences
                tone: ResponseTone::Professional, // Map based on config.response_style
                length: ResponseLength::Medium,
                language: "en".to_string(),
                formality_level: FormalityLevel::Standard,
                custom_instructions: config.custom_prompt.map(|p| vec![p]).unwrap_or_default(),
                response_templates: HashMap::new(),
            },
        };
        Ok(())
    }

    async fn test_connection(
        &self,
        config: &crate::presentation::dtos::AIConfigRequest,
    ) -> DomainResult<()> {
        let cur_config = self.config.read().await;
        let request = OllamaRequest {
            model: config.model.clone(),
            prompt: "Test connection".to_string(),
            stream: false,
        };

        // Try to send a request to verify connection
        timeout(
            Duration::from_secs(5), // Short timeout for test
            self.client
                .post(format!("{}/api/generate", cur_config.base_url))
                .json(&request)
                .send(),
        )
        .await
        .map_err(|e| {
            DomainError::ExternalServiceError(format!("AI service connection test failed: {}", e))
        })??;

        Ok(())
    }
}

use crate::domain::error::DomainError;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::domain::services::ai::{
        FormalityLevel, ResponseLength, ResponseTone, UserPreferences,
    };
    use crate::domain::services::PriorityLevel;

    use super::*;
    use tokio;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_user_preferences() {
        // Test default preferences
        let prefs = UserPreferences::default();
        assert_eq!(prefs.tone, ResponseTone::Professional);
        assert_eq!(prefs.length, ResponseLength::Medium);
        assert_eq!(prefs.language, "en");
        assert_eq!(prefs.formality_level, FormalityLevel::Standard);
        assert!(prefs.custom_instructions.is_empty());
        assert!(prefs.response_templates.is_empty());

        // Test custom preferences
        let mut templates = HashMap::new();
        templates.insert("greeting".to_string(), "Hello, {name}!".to_string());

        let custom_prefs = UserPreferences {
            tone: ResponseTone::Friendly,
            length: ResponseLength::Detailed,
            language: "es".to_string(),
            formality_level: FormalityLevel::Casual,
            custom_instructions: vec!["Always include emoji".to_string()],
            response_templates: templates,
        };

        assert_eq!(custom_prefs.tone, ResponseTone::Friendly);
        assert_eq!(custom_prefs.length, ResponseLength::Detailed);
        assert_eq!(custom_prefs.language, "es");
        assert_eq!(custom_prefs.formality_level, FormalityLevel::Casual);
        assert!(!custom_prefs.custom_instructions.is_empty());
        assert!(custom_prefs.response_templates.contains_key("greeting"));
    }

    #[test]
    fn test_conversation_memory() {
        let mut memory = ConversationMemory::new(2);

        memory.add_entry("User".to_string(), "Hello".to_string());
        memory.add_entry("Assistant".to_string(), "Hi there!".to_string());
        memory.add_entry("User".to_string(), "How are you?".to_string());

        let context = memory.get_context();
        assert!(!context.contains("Hello")); // First message should be evicted
        assert!(context.contains("Hi there!"));
        assert!(context.contains("How are you?"));
    }

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
            user_preferences: UserPreferences::default(),
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

    #[tokio::test]
    async fn test_generate_response_with_memory() {
        let mock_server = MockServer::start().await;

        let sample_response = r#"{
            "response": "I understand the urgency. Let's proceed with the deployment review.",
            "done": true
        }"#;

        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sample_response, "application/json"),
            )
            .mount(&mock_server)
            .await;

        let user_prefs = UserPreferences {
            tone: ResponseTone::Technical,
            length: ResponseLength::Concise,
            ..Default::default()
        };

        let config = AIConfig {
            base_url: mock_server.uri(),
            model: "test-model".to_string(),
            timeout_seconds: 5,
            user_preferences: user_prefs,
        };

        let service = OllamaService::new(config);

        // Test conversation memory with multiple interactions
        let responses = vec![
            "Need urgent deployment review",
            "When can we start?",
            "What are the risks?",
        ];

        for prompt in responses {
            let response = service.generate_response(prompt).await.unwrap();
            assert!(!response.is_empty());

            // Verify memory after each interaction
            let memory_context = service.memory.lock().unwrap().get_context();
            assert!(memory_context.contains(&response));
            assert!(memory_context.contains("Assistant"));
        }

        // Verify memory size constraint
        let memory = service.memory.lock().unwrap();
        assert!(memory.entries.len() <= 10); // Max entries check
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mock_server = MockServer::start().await;

        // Test timeout scenario
        let config = AIConfig {
            base_url: mock_server.uri(),
            model: "test-model".to_string(),
            timeout_seconds: 1,
            user_preferences: UserPreferences::default(),
        };

        let service = OllamaService::new(config);
        let result = service.generate_response("Test prompt").await;
        assert!(matches!(
            result.unwrap_err(),
            DomainError::ExternalServiceError(_)
        ));

        // Test invalid response format
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{ "response": "Invalid JSON", "done": true }"#,
                "application/json",
            ))
            .mount(&mock_server)
            .await;

        let result = service.analyze_content("Test content").await;
        assert!(matches!(
            result.unwrap_err(),
            DomainError::ValidationError(_)
        ));
    }

    #[tokio::test]
    async fn test_conversation_context_influence() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{ "response": "Context-aware response", "done": true }"#,
                "application/json",
            ))
            .mount(&mock_server)
            .await;

        let service = OllamaService::new(AIConfig {
            base_url: mock_server.uri(),
            model: "test-model".to_string(),
            timeout_seconds: 5,
            user_preferences: UserPreferences::default(),
        });

        // Simulate a conversation flow
        // Add initial context
        service.memory.lock().unwrap().add_entry(
            "User".to_string(),
            "Initial context information".to_string(),
        );

        // Generate response to follow-up question
        let response = service
            .generate_response("Follow-up question")
            .await
            .unwrap();
        assert!(!response.is_empty());

        // Verify that the memory maintains conversation order
        let memory = service.memory.lock().unwrap();
        let context = memory.get_context();

        // Check all entries are present in the context string
        assert!(
            context.contains("Initial context information"),
            "Missing initial context"
        );
        assert!(
            context.contains("Follow-up question"),
            "Missing follow-up question"
        );
        assert!(
            context.contains("Context-aware response"),
            "Missing AI response"
        );

        // Verify entries are in correct order
        let entries: Vec<_> = memory.entries.iter().collect();
        assert_eq!(
            entries.len(),
            3,
            "Expected 3 entries in conversation memory"
        );
        assert!(
            entries[0].content.contains("Initial context information"),
            "Wrong first entry"
        );
        assert!(
            entries[1].content.contains("Follow-up question"),
            "Wrong second entry"
        );
        assert!(
            entries[2].content.contains("Context-aware response"),
            "Wrong third entry"
        );
    }
}
