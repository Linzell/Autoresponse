use std::sync::Once;
use tracing_subscriber::{self, fmt::format::FmtSpan};

// Mock implementations
pub mod mock;
pub use mock::*;

// Test utilities
pub mod test_utils;
pub use test_utils::*;

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        // Try to initialize tracing, ignore if it's already initialized
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_span_events(FmtSpan::CLOSE)
            .with_test_writer()
            .compact()
            .try_init();

        // Create test .env if it doesn't exist
        if std::env::var("DATABASE_URL").is_err() {
            std::env::set_var("DATABASE_URL", "postgres://test:test@localhost:5432/test");
        }

        dotenv::dotenv().ok();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
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
