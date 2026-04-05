//! Prompt caching: avoids rebuilding system prompts when the context
//! has not meaningfully changed. Uses a fingerprint of the context to
//! decide cache validity.

use std::collections::HashMap;

// ── Cache key / fingerprint ────────────────────────────────────────────

/// Inputs that determine whether a cached prompt is still valid.
#[derive(Debug, Clone, Default)]
pub struct PromptCacheKey {
    /// Scenario or template name.
    pub template: String,
    /// Sorted list of context variable key=value pairs.
    pub variables: Vec<(String, String)>,
    /// Turn count bucket (rounded to reduce churn).
    pub turn_bucket: u32,
}

impl PromptCacheKey {
    /// Produce a deterministic string fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        use std::fmt::Write;
        let mut buf = String::new();
        let _ = write!(buf, "t={};b={};", self.template, self.turn_bucket);
        for (k, v) in &self.variables {
            let _ = write!(buf, "{}={};", k, v);
        }
        buf
    }
}

/// Generate a cache key from high-level context.
///
/// `turn_count` is bucketed into groups of `bucket_size` to avoid
/// invalidating the cache on every single turn.
#[must_use]
pub fn cache_key(
    template: &str,
    variables: &[(String, String)],
    turn_count: usize,
    bucket_size: usize,
) -> PromptCacheKey {
    let bucket_size = bucket_size.max(1);
    let mut vars = variables.to_vec();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    PromptCacheKey {
        template: template.to_string(),
        variables: vars,
        turn_bucket: (turn_count / bucket_size) as u32,
    }
}

// ── Cache statistics ───────────────────────────────────────────────────

/// Hit/miss statistics for the prompt cache.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    /// Hit rate in the range `[0.0, 1.0]`.
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }

    /// Total lookups.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.hits + self.misses
    }
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PromptCache: {} hits, {} misses ({:.1}% hit rate)",
            self.hits,
            self.misses,
            self.hit_rate() * 100.0
        )
    }
}

// ── Cache implementation ───────────────────────────────────────────────

/// A simple in-memory cache for rendered system prompts, keyed by
/// context fingerprint.
#[derive(Debug, Clone)]
pub struct PromptCache {
    entries: HashMap<String, String>,
    max_entries: usize,
    stats: CacheStats,
}

impl PromptCache {
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            stats: CacheStats::default(),
        }
    }

    /// Look up a cached prompt by its fingerprint.
    pub fn get(&mut self, key: &PromptCacheKey) -> Option<&str> {
        let fp = key.fingerprint();
        if self.entries.contains_key(&fp) {
            self.stats.hits += 1;
            self.entries.get(&fp).map(String::as_str)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Store a rendered prompt.
    pub fn put(&mut self, key: &PromptCacheKey, prompt: String) {
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key.fingerprint())
        {
            // Evict the first entry (simple eviction).
            if let Some(oldest) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(key.fingerprint(), prompt);
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

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get cache statistics.
    #[must_use]
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = CacheStats::default();
    }
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new(32)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_deterministic() {
        let k1 = cache_key("coding", &[("a".into(), "1".into())], 5, 5);
        let k2 = cache_key("coding", &[("a".into(), "1".into())], 5, 5);
        assert_eq!(k1.fingerprint(), k2.fingerprint());
    }

    #[test]
    fn fingerprint_differs_on_template() {
        let k1 = cache_key("coding", &[], 0, 5);
        let k2 = cache_key("debug", &[], 0, 5);
        assert_ne!(k1.fingerprint(), k2.fingerprint());
    }

    #[test]
    fn fingerprint_differs_on_variable() {
        let k1 = cache_key("t", &[("a".into(), "1".into())], 0, 5);
        let k2 = cache_key("t", &[("a".into(), "2".into())], 0, 5);
        assert_ne!(k1.fingerprint(), k2.fingerprint());
    }

    #[test]
    fn turn_bucketing() {
        let k1 = cache_key("t", &[], 3, 5);
        let k2 = cache_key("t", &[], 4, 5);
        assert_eq!(k1.fingerprint(), k2.fingerprint()); // Both in bucket 0

        let k3 = cache_key("t", &[], 5, 5);
        assert_ne!(k1.fingerprint(), k3.fingerprint()); // Bucket 1
    }

    #[test]
    fn variables_sorted() {
        let k1 = cache_key(
            "t",
            &[("b".into(), "2".into()), ("a".into(), "1".into())],
            0,
            5,
        );
        let k2 = cache_key(
            "t",
            &[("a".into(), "1".into()), ("b".into(), "2".into())],
            0,
            5,
        );
        assert_eq!(k1.fingerprint(), k2.fingerprint());
    }

    #[test]
    fn bucket_size_zero_defaults_to_one() {
        let k = cache_key("t", &[], 10, 0);
        assert_eq!(k.turn_bucket, 10);
    }

    #[test]
    fn cache_miss_then_hit() {
        let mut cache = PromptCache::new(10);
        let key = cache_key("t", &[], 0, 5);
        assert!(cache.get(&key).is_none());
        cache.put(&key, "prompt text".into());
        assert_eq!(cache.get(&key), Some("prompt text"));
    }

    #[test]
    fn cache_stats_tracking() {
        let mut cache = PromptCache::new(10);
        let key = cache_key("t", &[], 0, 5);

        cache.get(&key); // miss
        cache.put(&key, "p".into());
        cache.get(&key); // hit
        cache.get(&key); // hit

        assert_eq!(cache.stats().hits, 2);
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().total(), 3);
        assert!((cache.stats().hit_rate() - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn cache_stats_display() {
        let stats = CacheStats { hits: 3, misses: 1 };
        let text = stats.to_string();
        assert!(text.contains("3 hits"));
        assert!(text.contains("1 misses"));
        assert!(text.contains("75.0%"));
    }

    #[test]
    fn cache_stats_empty() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn cache_eviction() {
        let mut cache = PromptCache::new(2);
        let k1 = cache_key("a", &[], 0, 5);
        let k2 = cache_key("b", &[], 0, 5);
        let k3 = cache_key("c", &[], 0, 5);

        cache.put(&k1, "A".into());
        cache.put(&k2, "B".into());
        assert_eq!(cache.len(), 2);

        cache.put(&k3, "C".into());
        assert_eq!(cache.len(), 2); // Evicted one
        // k3 should be present
        assert_eq!(cache.get(&k3), Some("C"));
    }

    #[test]
    fn cache_put_same_key_no_eviction() {
        let mut cache = PromptCache::new(1);
        let key = cache_key("t", &[], 0, 5);
        cache.put(&key, "v1".into());
        cache.put(&key, "v2".into());
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key), Some("v2"));
    }

    #[test]
    fn cache_clear() {
        let mut cache = PromptCache::new(10);
        let key = cache_key("t", &[], 0, 5);
        cache.put(&key, "p".into());
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_reset_stats() {
        let mut cache = PromptCache::new(10);
        let key = cache_key("t", &[], 0, 5);
        cache.get(&key); // miss
        assert_eq!(cache.stats().misses, 1);
        cache.reset_stats();
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn default_cache() {
        let cache = PromptCache::default();
        assert_eq!(cache.max_entries, 32);
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_key_default() {
        let key = PromptCacheKey::default();
        assert!(key.template.is_empty());
        assert!(key.variables.is_empty());
        assert_eq!(key.turn_bucket, 0);
    }
}
