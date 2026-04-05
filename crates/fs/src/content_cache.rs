//! LRU content cache for recently-read file contents.
//!
//! Provides [`ContentCache`] — an in-memory cache that keeps the most recently
//! accessed file contents, evicting the least-recently-used entries when the
//! configured limits are exceeded.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Configuration ────────────────────────────────────────────────────

/// Configuration for the content cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries (files) to keep cached.
    pub max_entries: usize,
    /// Maximum total bytes of cached content. `0` means unlimited.
    pub max_total_bytes: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_total_bytes: 64 * 1024 * 1024, // 64 MiB
        }
    }
}

// ── Cache entry ──────────────────────────────────────────────────────

/// A single cached file entry.
#[derive(Debug, Clone)]
struct CacheEntry {
    content: Vec<u8>,
    /// Monotonically increasing access counter (higher = more recent).
    last_access: u64,
}

// ── ContentCache ─────────────────────────────────────────────────────

/// LRU cache for file contents.
///
/// Entries are evicted when either `max_entries` or `max_total_bytes` is
/// exceeded. The least-recently-used entry (lowest `last_access`) is evicted
/// first.
#[derive(Debug)]
pub struct ContentCache {
    entries: HashMap<PathBuf, CacheEntry>,
    config: CacheConfig,
    /// Current total bytes stored.
    total_bytes: usize,
    /// Monotonic counter bumped on every access.
    clock: u64,
}

impl ContentCache {
    /// Create a new cache with the given configuration.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
            total_bytes: 0,
            clock: 0,
        }
    }

    /// Create a cache with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Insert or update a file's cached content.
    pub fn put(&mut self, path: &Path, content: Vec<u8>) {
        let content_len = content.len();

        // If already present, remove old entry's byte count first.
        if let Some(old) = self.entries.remove(path) {
            self.total_bytes = self.total_bytes.saturating_sub(old.content.len());
        }

        self.clock += 1;
        self.entries.insert(
            path.to_path_buf(),
            CacheEntry {
                content,
                last_access: self.clock,
            },
        );
        self.total_bytes += content_len;

        self.evict();
    }

    /// Retrieve cached content for `path`, bumping its access time.
    /// Returns `None` if not cached.
    pub fn get(&mut self, path: &Path) -> Option<&[u8]> {
        // Two-phase borrow: check existence first, then mutate.
        if !self.entries.contains_key(path) {
            return None;
        }
        self.clock += 1;
        let entry = self.entries.get_mut(path).expect("just checked");
        entry.last_access = self.clock;
        Some(&entry.content)
    }

    /// Peek at cached content without updating access time.
    #[must_use]
    pub fn peek(&self, path: &Path) -> Option<&[u8]> {
        self.entries.get(path).map(|e| e.content.as_slice())
    }

    /// Remove a specific path from the cache.
    pub fn invalidate(&mut self, path: &Path) {
        if let Some(entry) = self.entries.remove(path) {
            self.total_bytes = self.total_bytes.saturating_sub(entry.content.len());
        }
    }

    /// Invalidate all entries whose path starts with `prefix`.
    pub fn invalidate_prefix(&mut self, prefix: &Path) {
        let to_remove: Vec<PathBuf> = self
            .entries
            .keys()
            .filter(|p| p.starts_with(prefix))
            .cloned()
            .collect();
        for p in to_remove {
            self.invalidate(&p);
        }
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }

    /// Number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total bytes of cached content.
    #[must_use]
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Current configuration.
    #[must_use]
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Evict least-recently-used entries until within limits.
    fn evict(&mut self) {
        while self.needs_eviction() {
            let lru_path = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_access)
                .map(|(p, _)| p.clone());

            if let Some(path) = lru_path {
                if let Some(entry) = self.entries.remove(&path) {
                    self.total_bytes = self.total_bytes.saturating_sub(entry.content.len());
                }
            } else {
                break;
            }
        }
    }

    /// Check if eviction is needed.
    fn needs_eviction(&self) -> bool {
        if self.entries.len() > self.config.max_entries {
            return true;
        }
        self.config.max_total_bytes > 0 && self.total_bytes > self.config.max_total_bytes
    }
}

impl Default for ContentCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get() {
        let mut cache = ContentCache::with_defaults();
        cache.put(Path::new("/a.txt"), b"hello".to_vec());
        assert_eq!(cache.get(Path::new("/a.txt")), Some(b"hello".as_slice()));
    }

    #[test]
    fn get_miss_returns_none() {
        let mut cache = ContentCache::with_defaults();
        assert!(cache.get(Path::new("/missing")).is_none());
    }

    #[test]
    fn put_overwrites_existing() {
        let mut cache = ContentCache::with_defaults();
        cache.put(Path::new("/a.txt"), b"old".to_vec());
        cache.put(Path::new("/a.txt"), b"new".to_vec());
        assert_eq!(cache.get(Path::new("/a.txt")), Some(b"new".as_slice()));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.total_bytes(), 3);
    }

    #[test]
    fn evicts_lru_when_max_entries_exceeded() {
        let config = CacheConfig {
            max_entries: 2,
            max_total_bytes: 0,
        };
        let mut cache = ContentCache::new(config);
        cache.put(Path::new("/a"), b"1".to_vec());
        cache.put(Path::new("/b"), b"2".to_vec());
        cache.put(Path::new("/c"), b"3".to_vec()); // should evict /a
        assert_eq!(cache.len(), 2);
        assert!(cache.peek(Path::new("/a")).is_none());
        assert!(cache.peek(Path::new("/b")).is_some());
        assert!(cache.peek(Path::new("/c")).is_some());
    }

    #[test]
    fn evicts_lru_when_max_bytes_exceeded() {
        let config = CacheConfig {
            max_entries: 100,
            max_total_bytes: 10,
        };
        let mut cache = ContentCache::new(config);
        cache.put(Path::new("/a"), vec![0; 5]);
        cache.put(Path::new("/b"), vec![0; 5]);
        // total = 10, ok
        assert_eq!(cache.len(), 2);

        cache.put(Path::new("/c"), vec![0; 3]);
        // total would be 13, evicts /a (5 bytes) → total = 8
        assert!(cache.peek(Path::new("/a")).is_none());
        assert_eq!(cache.total_bytes(), 8);
    }

    #[test]
    fn get_bumps_access_time() {
        let config = CacheConfig {
            max_entries: 2,
            max_total_bytes: 0,
        };
        let mut cache = ContentCache::new(config);
        cache.put(Path::new("/a"), b"1".to_vec());
        cache.put(Path::new("/b"), b"2".to_vec());
        // Access /a to make it more recent
        let _ = cache.get(Path::new("/a"));
        cache.put(Path::new("/c"), b"3".to_vec()); // should evict /b (LRU)
        assert!(cache.peek(Path::new("/a")).is_some());
        assert!(cache.peek(Path::new("/b")).is_none());
        assert!(cache.peek(Path::new("/c")).is_some());
    }

    #[test]
    fn invalidate_single() {
        let mut cache = ContentCache::with_defaults();
        cache.put(Path::new("/a"), b"data".to_vec());
        cache.invalidate(Path::new("/a"));
        assert!(cache.is_empty());
        assert_eq!(cache.total_bytes(), 0);
    }

    #[test]
    fn invalidate_prefix() {
        let mut cache = ContentCache::with_defaults();
        cache.put(Path::new("/src/a.rs"), b"a".to_vec());
        cache.put(Path::new("/src/b.rs"), b"b".to_vec());
        cache.put(Path::new("/doc/c.md"), b"c".to_vec());
        cache.invalidate_prefix(Path::new("/src"));
        assert_eq!(cache.len(), 1);
        assert!(cache.peek(Path::new("/doc/c.md")).is_some());
    }

    #[test]
    fn clear_empties_cache() {
        let mut cache = ContentCache::with_defaults();
        cache.put(Path::new("/a"), b"x".to_vec());
        cache.put(Path::new("/b"), b"y".to_vec());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.total_bytes(), 0);
    }

    #[test]
    fn peek_does_not_bump_access() {
        let config = CacheConfig {
            max_entries: 2,
            max_total_bytes: 0,
        };
        let mut cache = ContentCache::new(config);
        cache.put(Path::new("/a"), b"1".to_vec());
        cache.put(Path::new("/b"), b"2".to_vec());
        // Peek /a — should NOT bump its access time
        let _ = cache.peek(Path::new("/a"));
        cache.put(Path::new("/c"), b"3".to_vec()); // should evict /a (still LRU)
        assert!(cache.peek(Path::new("/a")).is_none());
    }

    #[test]
    fn default_config_values() {
        let config = CacheConfig::default();
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.max_total_bytes, 64 * 1024 * 1024);
    }

    #[test]
    fn invalidate_nonexistent_is_noop() {
        let mut cache = ContentCache::with_defaults();
        cache.invalidate(Path::new("/nope"));
        assert!(cache.is_empty());
    }
}
