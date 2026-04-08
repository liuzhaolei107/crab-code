//! Per-section memoization for system prompt components.
//!
//! Sections are cached individually and invalidated selectively:
//! - `/clear` or `/compact` invalidates all sections
//! - Tool registry mutation invalidates the "tools" section
//! - CRAB.md file change invalidates the `crab_md` section
//! - Memory file change invalidates the "memory" section
//!
//! Maps to CCB's section-level memoization in `systemPromptSections.ts`.

use std::collections::HashMap;
use std::sync::RwLock;

/// Per-section cache for system prompt components.
///
/// Thread-safe via `RwLock`; designed for single-writer (the agent loop)
/// with occasional reads from diagnostics/logging.
#[derive(Debug, Default)]
pub struct SectionCache {
    entries: RwLock<HashMap<&'static str, CacheEntry>>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    content: String,
    generation: u64,
}

impl SectionCache {
    /// Create a new, empty section cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cached section, or `None` if not cached or stale.
    ///
    /// A cached entry is considered fresh only if its stored generation
    /// matches `current_generation`.
    pub fn get(&self, name: &str, current_generation: u64) -> Option<String> {
        let entries = self.entries.read().expect("SectionCache lock poisoned");
        entries.get(name).and_then(|entry| {
            if entry.generation == current_generation {
                Some(entry.content.clone())
            } else {
                None
            }
        })
    }

    /// Store a section in the cache with the given generation.
    pub fn put(&self, name: &'static str, content: String, generation: u64) {
        let mut entries = self.entries.write().expect("SectionCache lock poisoned");
        entries.insert(
            name,
            CacheEntry {
                content,
                generation,
            },
        );
    }

    /// Invalidate a specific section by name.
    pub fn invalidate(&self, name: &str) {
        let mut entries = self.entries.write().expect("SectionCache lock poisoned");
        entries.remove(name);
    }

    /// Invalidate all cached sections (used by `/clear`, `/compact`).
    pub fn invalidate_all(&self) {
        let mut entries = self.entries.write().expect("SectionCache lock poisoned");
        entries.clear();
    }

    /// Return the number of currently cached sections.
    pub fn len(&self) -> usize {
        let entries = self.entries.read().expect("SectionCache lock poisoned");
        entries.len()
    }

    /// Return whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cache_is_empty() {
        let cache = SectionCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn put_and_get_fresh() {
        let cache = SectionCache::new();
        cache.put("env", "environment info".into(), 1);
        assert_eq!(cache.get("env", 1), Some("environment info".into()));
    }

    #[test]
    fn get_stale_returns_none() {
        let cache = SectionCache::new();
        cache.put("env", "old info".into(), 1);
        assert_eq!(cache.get("env", 2), None);
    }

    #[test]
    fn get_missing_returns_none() {
        let cache = SectionCache::new();
        assert_eq!(cache.get("nonexistent", 1), None);
    }

    #[test]
    fn invalidate_single_section() {
        let cache = SectionCache::new();
        cache.put("env", "e".into(), 1);
        cache.put("tools", "t".into(), 1);
        assert_eq!(cache.len(), 2);

        cache.invalidate("env");
        assert_eq!(cache.get("env", 1), None);
        assert_eq!(cache.get("tools", 1), Some("t".into()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn invalidate_all_clears_everything() {
        let cache = SectionCache::new();
        cache.put("env", "e".into(), 1);
        cache.put("tools", "t".into(), 1);
        cache.put("git", "g".into(), 1);

        cache.invalidate_all();
        assert!(cache.is_empty());
        assert_eq!(cache.get("env", 1), None);
        assert_eq!(cache.get("tools", 1), None);
        assert_eq!(cache.get("git", 1), None);
    }

    #[test]
    fn put_overwrites_existing() {
        let cache = SectionCache::new();
        cache.put("env", "old".into(), 1);
        cache.put("env", "new".into(), 2);
        assert_eq!(cache.get("env", 2), Some("new".into()));
        assert_eq!(cache.get("env", 1), None);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn invalidate_nonexistent_is_noop() {
        let cache = SectionCache::new();
        cache.put("env", "e".into(), 1);
        cache.invalidate("nonexistent");
        assert_eq!(cache.len(), 1);
    }
}
