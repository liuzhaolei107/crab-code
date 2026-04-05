//! LRU response cache for LLM API calls.
//!
//! Caches non-streaming responses keyed by a hash of the request parameters
//! (model + messages + system prompt + tools + temperature). Entries expire
//! after a configurable TTL. Thread-safe via `Mutex`.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::types::{MessageRequest, MessageResponse};

/// Configuration for the response cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of cached entries.
    pub max_entries: usize,
    /// Time-to-live for each entry.
    pub ttl: Duration,
    /// Whether the cache is enabled.
    pub enabled: bool,
}

impl CacheConfig {
    /// Create a default config: 128 entries, 5 minute TTL, enabled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_entries: 128,
            ttl: Duration::from_secs(300),
            enabled: true,
        }
    }

    /// Disabled cache config.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// Set max entries.
    #[must_use]
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Set TTL.
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// A cached response entry with access tracking for LRU eviction.
#[derive(Debug, Clone)]
struct CacheEntry {
    response: MessageResponse,
    inserted_at: Instant,
    last_accessed: Instant,
}

/// Thread-safe LRU response cache.
pub struct ResponseCache {
    config: CacheConfig,
    entries: Mutex<HashMap<u64, CacheEntry>>,
}

impl ResponseCache {
    /// Create a new cache with the given configuration.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Look up a cached response for the given request.
    ///
    /// Returns `None` if cache is disabled, no entry exists, or the entry has expired.
    pub fn get(&self, req: &MessageRequest<'_>) -> Option<MessageResponse> {
        if !self.config.enabled {
            return None;
        }

        let key = request_hash(req);
        let mut entries = self.entries.lock().ok()?;

        let entry = entries.get_mut(&key)?;

        // Check TTL
        if entry.inserted_at.elapsed() > self.config.ttl {
            entries.remove(&key);
            return None;
        }

        entry.last_accessed = Instant::now();
        Some(entry.response.clone())
    }

    /// Insert a response into the cache.
    ///
    /// If the cache is full, evicts the least-recently-accessed entry.
    pub fn put(&self, req: &MessageRequest<'_>, response: MessageResponse) {
        if !self.config.enabled {
            return;
        }

        let key = request_hash(req);
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

        // Evict expired entries first
        let now = Instant::now();
        let ttl = self.config.ttl;
        entries.retain(|_, entry| entry.inserted_at.elapsed() <= ttl);

        // If still full, evict LRU
        if entries.len() >= self.config.max_entries
            && let Some((&lru_key, _)) = entries.iter().min_by_key(|(_, entry)| entry.last_accessed)
            {
                entries.remove(&lru_key);
            }

        entries.insert(
            key,
            CacheEntry {
                response,
                inserted_at: now,
                last_accessed: now,
            },
        );
    }

    /// Remove all entries from the cache.
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }

    /// Number of entries currently in the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.lock().map_or(0, |e| e.len())
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Cache hit statistics.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let total = entries.len();
        let now = Instant::now();
        let expired = entries
            .values()
            .filter(|e| e.inserted_at.elapsed() > self.config.ttl)
            .count();
        CacheStats {
            total_entries: total,
            expired_entries: expired,
            active_entries: total.saturating_sub(expired),
            max_entries: self.config.max_entries,
            oldest_age: entries
                .values()
                .map(|e| now.duration_since(e.inserted_at))
                .max()
                .unwrap_or(Duration::ZERO),
        }
    }
}

/// Snapshot of cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub max_entries: usize,
    pub oldest_age: Duration,
}

/// Compute a hash key from the significant parts of a request.
///
/// The hash includes model, messages text, system prompt, tool schemas,
/// and temperature. It intentionally excludes `max_tokens` and `cache_breakpoints`
/// since those don't affect the semantic content of the response.
fn request_hash(req: &MessageRequest<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();

    req.model.as_str().hash(&mut hasher);

    for msg in req.messages.as_ref() {
        msg.text().hash(&mut hasher);
    }

    if let Some(sys) = &req.system {
        sys.hash(&mut hasher);
    }

    // Hash tool schemas as JSON strings for deterministic ordering
    for tool in &req.tools {
        tool.to_string().hash(&mut hasher);
    }

    // Hash temperature (f32 bits for deterministic hashing)
    if let Some(temp) = req.temperature {
        temp.to_bits().hash(&mut hasher);
    }

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::message::Message;
    use crab_core::model::{ModelId, TokenUsage};
    use std::borrow::Cow;

    fn test_request(msg: &str) -> MessageRequest<'_> {
        MessageRequest {
            model: ModelId::from("test-model"),
            messages: Cow::Owned(vec![Message::user(msg)]),
            system: Some("sys".into()),
            max_tokens: 1024,
            tools: vec![],
            temperature: None,
            cache_breakpoints: vec![],
        }
    }

    fn test_response(text: &str) -> MessageResponse {
        MessageResponse {
            id: "msg_01".into(),
            message: Message::assistant(text),
            usage: TokenUsage::default(),
        }
    }

    #[test]
    fn cache_config_default() {
        let config = CacheConfig::new();
        assert_eq!(config.max_entries, 128);
        assert_eq!(config.ttl, Duration::from_secs(300));
        assert!(config.enabled);
    }

    #[test]
    fn cache_config_disabled() {
        let config = CacheConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn cache_config_builder() {
        let config = CacheConfig::new()
            .with_max_entries(64)
            .with_ttl(Duration::from_secs(60));
        assert_eq!(config.max_entries, 64);
        assert_eq!(config.ttl, Duration::from_secs(60));
    }

    #[test]
    fn cache_put_and_get() {
        let cache = ResponseCache::new(CacheConfig::new());
        let req = test_request("hello");
        let resp = test_response("world");

        cache.put(&req, resp.clone());
        let cached = cache.get(&req).unwrap();
        assert_eq!(cached.message.text(), "world");
    }

    #[test]
    fn cache_miss_returns_none() {
        let cache = ResponseCache::new(CacheConfig::new());
        let req = test_request("hello");
        assert!(cache.get(&req).is_none());
    }

    #[test]
    fn cache_disabled_returns_none() {
        let cache = ResponseCache::new(CacheConfig::disabled());
        let req = test_request("hello");
        let resp = test_response("world");

        cache.put(&req, resp);
        assert!(cache.get(&req).is_none());
    }

    #[test]
    fn cache_different_messages_different_keys() {
        let cache = ResponseCache::new(CacheConfig::new());
        let req1 = test_request("hello");
        let req2 = test_request("goodbye");

        cache.put(&req1, test_response("resp1"));
        cache.put(&req2, test_response("resp2"));

        assert_eq!(cache.get(&req1).unwrap().message.text(), "resp1");
        assert_eq!(cache.get(&req2).unwrap().message.text(), "resp2");
    }

    #[test]
    fn cache_evicts_lru_when_full() {
        let cache = ResponseCache::new(CacheConfig::new().with_max_entries(2));

        let req1 = test_request("first");
        let req2 = test_request("second");
        let req3 = test_request("third");

        cache.put(&req1, test_response("r1"));
        cache.put(&req2, test_response("r2"));

        // Access req1 to make req2 the LRU
        let _ = cache.get(&req1);

        // Insert req3, should evict req2 (LRU)
        cache.put(&req3, test_response("r3"));

        assert!(cache.get(&req1).is_some());
        assert!(cache.get(&req2).is_none()); // evicted
        assert!(cache.get(&req3).is_some());
    }

    #[test]
    fn cache_ttl_expiration() {
        let cache = ResponseCache::new(CacheConfig::new().with_ttl(Duration::from_millis(1)));
        let req = test_request("hello");
        cache.put(&req, test_response("world"));

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(10));

        assert!(cache.get(&req).is_none());
    }

    #[test]
    fn cache_clear() {
        let cache = ResponseCache::new(CacheConfig::new());
        cache.put(&test_request("a"), test_response("1"));
        cache.put(&test_request("b"), test_response("2"));
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_stats_empty() {
        let cache = ResponseCache::new(CacheConfig::new());
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.active_entries, 0);
        assert_eq!(stats.expired_entries, 0);
    }

    #[test]
    fn cache_stats_with_entries() {
        let cache = ResponseCache::new(CacheConfig::new().with_max_entries(10));
        cache.put(&test_request("a"), test_response("1"));
        cache.put(&test_request("b"), test_response("2"));

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
        assert_eq!(stats.max_entries, 10);
    }

    #[test]
    fn cache_overwrite_same_key() {
        let cache = ResponseCache::new(CacheConfig::new());
        let req = test_request("hello");

        cache.put(&req, test_response("first"));
        cache.put(&req, test_response("second"));

        assert_eq!(cache.get(&req).unwrap().message.text(), "second");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn request_hash_deterministic() {
        let req = test_request("hello");
        let h1 = request_hash(&req);
        let h2 = request_hash(&req);
        assert_eq!(h1, h2);
    }

    #[test]
    fn request_hash_different_models() {
        let req1 = MessageRequest {
            model: ModelId::from("model-a"),
            messages: Cow::Owned(vec![Message::user("hi")]),
            system: None,
            max_tokens: 1024,
            tools: vec![],
            temperature: None,
            cache_breakpoints: vec![],
        };
        let req2 = MessageRequest {
            model: ModelId::from("model-b"),
            messages: Cow::Owned(vec![Message::user("hi")]),
            system: None,
            max_tokens: 1024,
            tools: vec![],
            temperature: None,
            cache_breakpoints: vec![],
        };
        assert_ne!(request_hash(&req1), request_hash(&req2));
    }

    #[test]
    fn request_hash_different_temperature() {
        let req1 = MessageRequest {
            model: ModelId::from("m"),
            messages: Cow::Owned(vec![Message::user("hi")]),
            system: None,
            max_tokens: 1024,
            tools: vec![],
            temperature: Some(0.5),
            cache_breakpoints: vec![],
        };
        let req2 = MessageRequest {
            model: ModelId::from("m"),
            messages: Cow::Owned(vec![Message::user("hi")]),
            system: None,
            max_tokens: 1024,
            tools: vec![],
            temperature: Some(0.9),
            cache_breakpoints: vec![],
        };
        assert_ne!(request_hash(&req1), request_hash(&req2));
    }

    #[test]
    fn request_hash_ignores_max_tokens() {
        let req1 = MessageRequest {
            model: ModelId::from("m"),
            messages: Cow::Owned(vec![Message::user("hi")]),
            system: None,
            max_tokens: 1024,
            tools: vec![],
            temperature: None,
            cache_breakpoints: vec![],
        };
        let req2 = MessageRequest {
            model: ModelId::from("m"),
            messages: Cow::Owned(vec![Message::user("hi")]),
            system: None,
            max_tokens: 4096,
            tools: vec![],
            temperature: None,
            cache_breakpoints: vec![],
        };
        assert_eq!(request_hash(&req1), request_hash(&req2));
    }
}
