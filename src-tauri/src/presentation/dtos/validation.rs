use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub details: Vec<String>,
}

impl ValidationError {
    pub fn new(code: &str, message: &str, details: Vec<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details,
        }
    }

    pub fn from_message(message: &str) -> Self {
        Self {
            code: "VALIDATION_ERROR".to_string(),
            message: message.to_string(),
            details: vec![],
        }
    }

    pub fn with_details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ValidationError {}
