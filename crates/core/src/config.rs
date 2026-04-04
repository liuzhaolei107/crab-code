/// Trait for configuration sources — implemented in crab-config
pub trait ConfigSource: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
}
