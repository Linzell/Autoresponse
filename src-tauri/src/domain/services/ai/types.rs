use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub tone: ResponseTone,
    pub length: ResponseLength,
    pub language: String,
    pub formality_level: FormalityLevel,
    pub custom_instructions: Vec<String>,
    pub response_templates: HashMap<String, String>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            tone: ResponseTone::Professional,
            length: ResponseLength::Medium,
            language: "en".to_string(),
            formality_level: FormalityLevel::Standard,
            custom_instructions: Vec::new(),
            response_templates: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseTone {
    Professional,
    Friendly,
    Technical,
    Casual,
    Formal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResponseLength {
    Concise,
    Medium,
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FormalityLevel {
    Casual,
    Standard,
    Formal,
    VeryFormal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub model: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub user_preferences: UserPreferences,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5:latest".to_string(),
            base_url: "http://localhost:11434".to_string(),
            timeout_seconds: 30,
            user_preferences: UserPreferences::default(),
        }
    }
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

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait AIService: Send + Sync + std::fmt::Debug {
    async fn analyze_content(
        &self,
        content: &str,
    ) -> crate::domain::error::DomainResult<AIAnalysis>;

    async fn generate_response(&self, context: &str) -> crate::domain::error::DomainResult<String>;

    async fn configure(
        &self,
        config: crate::presentation::dtos::AIConfigRequest,
    ) -> crate::domain::error::DomainResult<()>;

    async fn test_connection(
        &self,
        config: &crate::presentation::dtos::AIConfigRequest,
    ) -> crate::domain::error::DomainResult<()>;
}

// Change from Mutex to RwLock since we need concurrent read access
pub type DynAIService = Arc<dyn AIService>;
