//! Re-exports from `crab-memory` for backward compatibility.
//!
//! Previously this module defined `MemoryType`, `MemoryMetadata`, and helpers
//! inline. They now live in `crab_memory::types` and are re-exported here so
//! that existing `use super::memory_types::*` imports across `crab-session`
//! continue to compile unchanged.

pub use crab_memory::types::{
    MemoryMetadata, MemoryType, extract_body, format_memory_for_prompt,
    parse_frontmatter as parse_memory_frontmatter,
};
