//! `CachedMarkdownRenderer` — LRU-fronted facade over the base parser.

use std::sync::Arc;

use ratatui::text::Line;

use super::cache::{self, MarkdownCache, MarkdownCacheKey};
use crate::components::markdown::MarkdownRenderer;
use crate::components::syntax::SyntaxHighlighter;
use crate::theme::Theme;

/// LRU capacity for the cached renderer.
pub const DEFAULT_CACHE_CAPACITY: usize = 500;

/// A renderer that owns both the base markdown parser and the cache in
/// front of it.
pub struct CachedMarkdownRenderer {
    cache: MarkdownCache,
}

impl CachedMarkdownRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: MarkdownCache::with_capacity(DEFAULT_CACHE_CAPACITY),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: MarkdownCache::with_capacity(capacity),
        }
    }

    /// Look up or parse + highlight a markdown string.
    ///
    /// If the same `(content, theme_name, width)` has been seen before
    /// and is still in the LRU, the cached `Arc<Vec<Line>>` is returned.
    /// Otherwise the base renderer runs synchronously and the result is
    /// memoized before being returned.
    pub fn render(
        &mut self,
        content: &str,
        theme: &Theme,
        highlighter: &SyntaxHighlighter,
        width: u16,
    ) -> Arc<Vec<Line<'static>>> {
        let key = MarkdownCacheKey::compute(content, theme, width);
        if let Some(cached) = self.cache.get(&key) {
            return cached;
        }
        let renderer = MarkdownRenderer::new(theme, highlighter);
        let lines = renderer.render(content, width);
        let arc = Arc::new(lines);
        self.cache.put(key, Arc::clone(&arc));
        arc
    }

    /// Return cache statistics. Useful for tests and diagnostics.
    pub fn stats(&self) -> cache::CacheStats {
        self.cache.stats()
    }

    /// Drop all cache entries. Use on theme change or hard refresh.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for CachedMarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}
