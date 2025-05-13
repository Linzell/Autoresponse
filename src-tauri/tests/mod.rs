pub mod common;
pub mod e2e;
pub mod integration;

// Re-export common utilities
pub use common::{init, mock::*, test_utils::*};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup() {
        init();
        // Verify test environment
        assert!(
            std::env::var("DATABASE_URL").is_ok(),
            "Database URL not set"
        );
    }
}

// Test macros
#[macro_export]
macro_rules! async_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> anyhow::Result<()> {
            crate::common::init();
            $body.await
        }
    };
}

#[macro_export]
macro_rules! test_case {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            crate::common::init();
            $body
        }
    };
}

// Test utilities
pub mod test_utils {
    pub use autoresponse_lib::domain::{entities::*, error::*, services::*};
    pub use mockall::automock;
    pub use tokio;
    pub use uuid::Uuid;
}

pub use test_utils::*;
