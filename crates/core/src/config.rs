/// Trait for configuration sources — implemented in crab-config.
///
/// Provides a uniform interface for reading configuration values
/// regardless of the underlying source (file, environment, etc.).
pub trait ConfigSource: Send + Sync {
    /// Retrieves a configuration value by key.
    fn get(&self, key: &str) -> Option<String>;

    /// Retrieves a configuration value, returning a default if not found.
    fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_owned())
    }

    /// Checks whether a configuration key exists.
    fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestConfig {
        entries: Vec<(String, String)>,
    }

    impl ConfigSource for TestConfig {
        fn get(&self, key: &str) -> Option<String> {
            self.entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        }
    }

    fn make_config() -> TestConfig {
        TestConfig {
            entries: vec![
                ("model".into(), "claude-opus-4-6".into()),
                ("theme".into(), "dark".into()),
            ],
        }
    }

    #[test]
    fn get_existing_key() {
        let cfg = make_config();
        assert_eq!(cfg.get("model"), Some("claude-opus-4-6".into()));
    }

    #[test]
    fn get_missing_key() {
        let cfg = make_config();
        assert_eq!(cfg.get("nonexistent"), None);
    }

    #[test]
    fn get_or_with_existing_key() {
        let cfg = make_config();
        assert_eq!(cfg.get_or("model", "default"), "claude-opus-4-6");
    }

    #[test]
    fn get_or_with_missing_key() {
        let cfg = make_config();
        assert_eq!(cfg.get_or("nonexistent", "fallback"), "fallback");
    }

    #[test]
    fn contains_existing_key() {
        let cfg = make_config();
        assert!(cfg.contains("theme"));
    }

    #[test]
    fn contains_missing_key() {
        let cfg = make_config();
        assert!(!cfg.contains("nonexistent"));
    }

    #[test]
    fn config_source_is_object_safe() {
        // Verify the trait can be used as a trait object
        let cfg = make_config();
        let boxed: Box<dyn ConfigSource> = Box::new(cfg);
        assert_eq!(boxed.get("model"), Some("claude-opus-4-6".into()));
    }
}
