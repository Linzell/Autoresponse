use std::time::Duration;

pub struct CacheConfig {
    pub notification_cache_capacity: u64,
    pub notification_cache_ttl: Duration,
    pub service_config_cache_capacity: u64,
    pub service_config_cache_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            notification_cache_capacity: 1000,
            notification_cache_ttl: Duration::from_secs(300), // 5 minutes
            service_config_cache_capacity: 100,
            service_config_cache_ttl: Duration::from_secs(600), // 10 minutes
        }
    }
}

impl CacheConfig {
    pub fn new(
        notification_cache_capacity: u64,
        notification_cache_ttl: Duration,
        service_config_cache_capacity: u64,
        service_config_cache_ttl: Duration,
    ) -> Self {
        Self {
            notification_cache_capacity,
            notification_cache_ttl,
            service_config_cache_capacity,
            service_config_cache_ttl,
        }
    }

    pub fn with_notification_cache(capacity: u64, ttl: Duration) -> Self {
        let mut config = Self::default();
        config.notification_cache_capacity = capacity;
        config.notification_cache_ttl = ttl;
        config
    }

    pub fn with_service_config_cache(capacity: u64, ttl: Duration) -> Self {
        let mut config = Self::default();
        config.service_config_cache_capacity = capacity;
        config.service_config_cache_ttl = ttl;
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.notification_cache_capacity, 1000);
        assert_eq!(config.notification_cache_ttl.as_secs(), 300);
        assert_eq!(config.service_config_cache_capacity, 100);
        assert_eq!(config.service_config_cache_ttl.as_secs(), 600);
    }

    #[test]
    fn test_custom_config() {
        let config = CacheConfig::new(
            500,
            Duration::from_secs(60),
            50,
            Duration::from_secs(120),
        );
        assert_eq!(config.notification_cache_capacity, 500);
        assert_eq!(config.notification_cache_ttl.as_secs(), 60);
        assert_eq!(config.service_config_cache_capacity, 50);
        assert_eq!(config.service_config_cache_ttl.as_secs(), 120);
    }

    #[test]
    fn test_with_notification_cache() {
        let config = CacheConfig::with_notification_cache(200, Duration::from_secs(30));
        assert_eq!(config.notification_cache_capacity, 200);
        assert_eq!(config.notification_cache_ttl.as_secs(), 30);
        // Other values should be default
        assert_eq!(config.service_config_cache_capacity, 100);
        assert_eq!(config.service_config_cache_ttl.as_secs(), 600);
    }

    #[test]
    fn test_with_service_config_cache() {
        let config = CacheConfig::with_service_config_cache(300, Duration::from_secs(90));
        assert_eq!(config.service_config_cache_capacity, 300);
        assert_eq!(config.service_config_cache_ttl.as_secs(), 90);
        // Other values should be default
        assert_eq!(config.notification_cache_capacity, 1000);
        assert_eq!(config.notification_cache_ttl.as_secs(), 300);
    }
}