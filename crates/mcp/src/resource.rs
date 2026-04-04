use crate::protocol::McpResource;
use std::collections::HashMap;

/// Cache for MCP resources fetched from servers.
pub struct ResourceCache {
    entries: HashMap<String, CachedResource>,
}

/// A cached resource with its content and metadata.
pub struct CachedResource {
    pub resource: McpResource,
    pub content: String,
}

impl ResourceCache {
    /// Create an empty resource cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Look up a cached resource by URI.
    pub fn get(&self, uri: &str) -> Option<&CachedResource> {
        self.entries.get(uri)
    }

    /// Insert or update a cached resource.
    pub fn insert(&mut self, uri: String, resource: McpResource, content: String) {
        self.entries
            .insert(uri, CachedResource { resource, content });
    }

    /// Remove a cached resource by URI.
    pub fn remove(&mut self, uri: &str) -> Option<CachedResource> {
        self.entries.remove(uri)
    }

    /// Clear all cached resources.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new()
    }
}
