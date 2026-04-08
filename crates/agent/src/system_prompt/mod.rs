//! System prompt construction and caching.
//!
//! The system prompt is assembled from modular sections, each independently
//! cacheable. A dynamic/static boundary marker controls API prompt cache scope.

mod builder;
pub mod cache;
pub mod sections;

// Re-export the main builder functions for backward compatibility
pub use builder::{build_system_prompt, build_system_prompt_with_memories};
