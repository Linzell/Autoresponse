// Integration tests module

#[cfg(test)]
mod action_test;

#[cfg(test)]
mod event_test;

#[cfg(test)]
mod job_test;

#[cfg(test)]
mod notification_test;

#[cfg(test)]
mod processor_test;

#[cfg(test)]
mod service_config_test;

#[cfg(test)]
mod cached_repository_test;

#[cfg(test)]
mod mcp_server_test;

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_loading() {
        // This test ensures the test modules are correctly loaded
        // It will fail if there are any module loading issues
    }
}
