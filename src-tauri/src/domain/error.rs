use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found error: {0}")]
    NotFoundError(String),

    #[error("Unauthorized error: {0}")]
    UnauthorizedError(String),

    #[error("Conflict error: {0}")]
    ConflictError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type DomainResult<T> = Result<T, DomainError>;

impl From<rusqlite::Error> for DomainError {
    fn from(error: rusqlite::Error) -> Self {
        DomainError::InternalError(format!("Database error: {}", error))
    }
}

impl From<serde_json::Error> for DomainError {
    fn from(error: serde_json::Error) -> Self {
        DomainError::InternalError(format!("JSON error: {}", error))
    }
}

impl From<uuid::Error> for DomainError {
    fn from(error: uuid::Error) -> Self {
        DomainError::InternalError(format!("UUID error: {}", error))
    }
}

impl From<reqwest::Error> for DomainError {
    fn from(error: reqwest::Error) -> Self {
        DomainError::InternalError(format!("HTTP error: {}", error))
    }
}

impl From<std::io::Error> for DomainError {
    fn from(error: std::io::Error) -> Self {
        DomainError::InternalError(format!("IO error: {}", error))
    }
}

impl From<String> for DomainError {
    fn from(error: String) -> Self {
        DomainError::InternalError(error)
    }
}

impl From<&str> for DomainError {
    fn from(error: &str) -> Self {
        DomainError::InternalError(error.to_string())
    }
}
