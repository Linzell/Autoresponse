use std::sync::Once;
use tracing_subscriber::{self, fmt::format::FmtSpan};

// E2E test configuration
mod notification_actions;
pub mod setup;

use std::{env, fs};

static INIT: Once = Once::new();

pub fn setup_test_env() {
    INIT.call_once(|| {
        // Create test .env file if it doesn't exist
        if !fs::metadata(".env").is_ok() {
            fs::write(
                ".env",
                "DATABASE_URL=postgres://test:test@localhost:5432/test",
            )
            .ok();
        }

        // Initialize logging for e2e tests
        if env::var("RUST_LOG").is_err() {
            env::set_var("RUST_LOG", "debug");
        }

        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_span_events(FmtSpan::CLOSE)
            .with_test_writer()
            .compact()
            .try_init();

        // Initialize any other global test configurations
        dotenv::dotenv().ok();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_setup() {
        setup_test_env();
        // Validate test environment is properly configured
        assert!(
            std::env::var("DATABASE_URL").is_ok(),
            "Database URL not set"
        );
        assert_eq!(
            std::env::var("DATABASE_URL").unwrap(),
            "postgres://test:test@localhost:5432/test",
            "Database URL does not match expected value"
        );
    }
}
