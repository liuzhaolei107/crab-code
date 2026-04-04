//! Prompt cache management (Anthropic path only).
//!
//! Anthropic's prompt caching allows reusing previously computed prefixes
//! to reduce latency and cost for repeated system prompts and tool definitions.

/// Cache control directive for message content blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheControl {
    /// Mark this block as a cache breakpoint (ephemeral lifetime).
    Ephemeral,
}

/// Tracks prompt cache hit/miss statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

impl CacheStats {
    pub const fn record_read(&mut self, tokens: u64) {
        self.cache_read_tokens += tokens;
    }

    pub const fn record_creation(&mut self, tokens: u64) {
        self.cache_creation_tokens += tokens;
    }
}
