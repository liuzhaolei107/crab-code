//! File-based memory system for Crab Code.
//!
//! Provides persistent, cross-session memory storage organized as
//! markdown files with YAML frontmatter in `~/.crab/memory/`.

pub mod age;
pub mod agents_md;
pub mod index;
pub mod paths;
pub mod prompt;
#[cfg(feature = "mem-ranker")]
pub mod ranker;
pub mod relevance;
pub mod security;
pub mod store;
pub mod team;
pub mod types;

pub use index::{IndexEntry, MemoryIndex, Truncation};
pub use prompt::MemoryPromptBuilder;
#[cfg(feature = "mem-ranker")]
pub use ranker::LlmMemoryRanker;
pub use relevance::{MemoryRanker, MemorySelector, ScoredMemory};
pub use store::{MemoryFile, MemoryStore};
pub use team::TeamMemoryStore;
pub use types::{
    MemoryMetadata, MemoryType, extract_body, format_frontmatter, format_memory_for_prompt,
    parse_frontmatter,
};
