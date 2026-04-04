use serde::{Deserialize, Serialize};

/// Agent capability declaration.
///
/// Describes a capability that an agent possesses, used for
/// capability-based tool filtering and agent matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Unique capability name (e.g., `file_edit`, `web_search`, `code_execution`).
    pub name: String,
    /// Human-readable description of what this capability provides.
    pub description: String,
}

impl Capability {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// A set of capabilities that an agent declares.
#[derive(Debug, Clone, Default)]
pub struct CapabilitySet {
    capabilities: Vec<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, capability: Capability) {
        if !self.capabilities.iter().any(|c| c.name == capability.name) {
            self.capabilities.push(capability);
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.capabilities.iter().any(|c| c.name == name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }

    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_new() {
        let cap = Capability::new("bash", "Execute shell commands");
        assert_eq!(cap.name, "bash");
        assert_eq!(cap.description, "Execute shell commands");
    }

    #[test]
    fn capability_clone_and_eq() {
        let cap1 = Capability::new("read", "Read files");
        let cap2 = cap1.clone();
        assert_eq!(cap1, cap2);
    }

    #[test]
    fn capability_serialize_roundtrip() {
        let cap = Capability::new("edit", "Edit files");
        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn capability_set_empty() {
        let set = CapabilitySet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn capability_set_add_and_has() {
        let mut set = CapabilitySet::new();
        set.add(Capability::new("bash", "Shell"));
        set.add(Capability::new("read", "Read"));
        assert!(set.has("bash"));
        assert!(set.has("read"));
        assert!(!set.has("write"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn capability_set_deduplication() {
        let mut set = CapabilitySet::new();
        set.add(Capability::new("bash", "Shell"));
        set.add(Capability::new("bash", "Shell again"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn capability_set_iter() {
        let mut set = CapabilitySet::new();
        set.add(Capability::new("a", "A"));
        set.add(Capability::new("b", "B"));
        let names: Vec<&str> = set.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }
}
