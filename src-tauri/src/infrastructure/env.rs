use std::{collections::HashMap, env};

/// Returns a HashMap containing all environment variables.
///
/// # Examples
///
/// ```rust
/// use kiro_database::get_envv;
/// let envv = get_envv();
/// println!("Environment variables: {:?}", envv);
/// ```
pub fn get_envv() -> HashMap<String, String> {
    env::vars().collect()
}

/// Returns the value of an environment variable or a default value if not found.
///
/// # Arguments
///
/// * `key` - The environment variable name to look up
/// * `default` - The default value to return if variable is not found
///
/// # Examples
///
/// ```rust
/// let value = kiro_database::get_env_or("DATABASE_URL", "postgres://localhost/db");
/// ```
pub fn get_env_or(key: &str, default: &str) -> String {
    let envv = get_envv();
    if envv.contains_key(key) {
        // Safety: The HashMap is already checked for the key
        envv.get(key).unwrap().clone()
    } else {
        default.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_envv() {
        let envv = get_envv();
        assert!(envv.contains_key("PATH"));
    }

    #[test]
    fn test_get_env_or() {
        let envv = get_env_or("PATH", "test");
        assert_eq!(envv, env::var("PATH").unwrap());
    }
}
