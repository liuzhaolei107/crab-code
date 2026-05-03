//! Cached markdown renderer with background syntax highlighting.
//!
//! The design layers two primitives over the base renderer:
//!
//! - [`cache::MarkdownCache`] — LRU of `Arc<Vec<Line<'static>>>` keyed by
//!   `(content, theme, width)` hash. Lookups are O(1); on hit, the exact
//!   same Lines are returned to be painted.
//! - [`highlight::HighlightWorker`] — a background thread that performs
//!   expensive syntect-based highlighting. While a highlight is in
//!   flight, the renderer emits a placeholder region; the worker's
//!   result overwrites the cache entry and a redraw signal fires.
//!
//! The base renderer remains [`crate::components::markdown::MarkdownRenderer`]
//! (pulldown-cmark + synchronous syntect). The cache wraps any renderer
//! that implements the `Render` trait.

pub mod cache;
mod cached;
pub mod highlight;
pub mod table;

pub use cache::{MarkdownCache, MarkdownCacheKey};
pub use cached::{CachedMarkdownRenderer, DEFAULT_CACHE_CAPACITY};
pub use highlight::{HighlightJob, HighlightRequest, HighlightWorker};
pub use table::{TableRow, compute_min_table_width, render_gfm_table, render_vertical_table};
