pub mod mcp_connector;
pub mod services;
pub mod types;

pub use mcp_connector::{DynMCPConnector, MCPConfig, MCPConnector};
pub use services::ollama_service::OllamaService;
pub use types::*;

#[cfg(test)]
pub use types::MockAIService;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::domain::services::ai::services::ollama_service::ConversationMemory;
    use crate::domain::DomainError;

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
