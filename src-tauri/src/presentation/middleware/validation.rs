use crate::presentation::dtos::ValidationError;
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

pub trait ValidatedCommand: DeserializeOwned + Validate {}

#[derive(Debug, thiserror::Error)]
pub enum ValidationMiddlewareError {
    #[error("Validation failed: {0}")]
    ValidationFailed(ValidationErrors),
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

impl From<ValidationMiddlewareError> for ValidationError {
    fn from(error: ValidationMiddlewareError) -> Self {
        match error {
            ValidationMiddlewareError::ValidationFailed(errors) => {
                let mut validation_errors = Vec::new();

                for (field, field_errors) in errors.field_errors() {
                    for error in field_errors {
                        validation_errors.push(format!(
                            "{}: {}",
                            field,
                            error
                                .message
                                .as_ref()
                                .map_or("Invalid value".to_string(), |m| m.to_string())
                        ));
                    }
                }

                ValidationError {
                    code: "VALIDATION_ERROR".to_string(),
                    message: "Request validation failed".to_string(),
                    details: validation_errors,
                }
            }
            ValidationMiddlewareError::DeserializationFailed(message) => ValidationError {
                code: "DESERIALIZATION_ERROR".to_string(),
                message,
                details: vec![],
            },
        }
    }
}

pub fn validate_request<T: ValidatedCommand>(json: &str) -> Result<T, ValidationMiddlewareError> {
    // Deserialize the request
    let command: T = serde_json::from_str(json)
        .map_err(|e| ValidationMiddlewareError::DeserializationFailed(e.to_string()))?;

    // Validate the request
    command
        .validate()
        .map_err(ValidationMiddlewareError::ValidationFailed)?;

    Ok(command)
}

pub async fn validate_command<T, F, Fut, R, E>(json: &str, handler: F) -> Result<R, ValidationError>
where
    T: ValidatedCommand,
    F: FnOnce(T) -> Fut,
    Fut: std::future::Future<Output = Result<R, E>>,
    E: Into<ValidationError>,
{
    let command = validate_request::<T>(json)?;
    handler(command).await.map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct TestCommand {
        #[validate(length(min = 3, max = 50))]
        name: String,
        #[validate(range(min = 0, max = 100))]
        age: i32,
        #[validate(email)]
        email: String,
    }

    impl ValidatedCommand for TestCommand {}

    #[tokio::test]
    async fn test_valid_command() {
        let json = r#"{
            "name": "John Doe",
            "age": 30,
            "email": "john@example.com"
        }"#;

        let result = validate_request::<TestCommand>(json);
        assert!(result.is_ok());

        let command = result.unwrap();
        assert_eq!(command.name, "John Doe");
        assert_eq!(command.age, 30);
        assert_eq!(command.email, "john@example.com");
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let json = r#"{
            "name": "Jo",
            "age": 150,
            "email": "not-an-email"
        }"#;

        let result = validate_request::<TestCommand>(json);
        assert!(result.is_err());

        match result {
            Err(ValidationMiddlewareError::ValidationFailed(errors)) => {
                let field_errors = errors.field_errors();
                assert!(field_errors.contains_key("name"));
                assert!(field_errors.contains_key("age"));
                assert!(field_errors.contains_key("email"));
            }
            _ => panic!("Expected ValidationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let json = r#"{
            "name": "John Doe",
            "age": "not a number",
            "email": "john@example.com"
        }"#;

        let result = validate_request::<TestCommand>(json);
        assert!(matches!(
            result,
            Err(ValidationMiddlewareError::DeserializationFailed(_))
        ));
    }

    #[tokio::test]
    async fn test_validate_command() {
        let json = r#"{
            "name": "John Doe",
            "age": 30,
            "email": "john@example.com"
        }"#;

        let result =
            validate_command::<TestCommand, _, _, _, ValidationError>(json, |command| async move {
                assert_eq!(command.name, "John Doe");
                Ok(())
            })
            .await;

        assert!(result.is_ok());
    }
}
