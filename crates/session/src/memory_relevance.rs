//! Score and select memories for injection into the system prompt.
//!
//! Scans memory directories, scores each memory entry against the current
//! conversation context, and selects the most relevant entries up to a
//! configurable budget. This keeps the system prompt focused and avoids
//! wasting context window on irrelevant memories.
//!
//! Maps to CCB `memdir/findRelevantMemories.ts` + `memoryScan.ts`.

use std::path::{Path, PathBuf};

use super::memory_types::MemoryMetadata;

// ─── Memory entry ──────────────────────────────────────────────────────

/// A loaded memory file with its metadata, content, and computed relevance.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// Filesystem path to the memory file.
    pub path: PathBuf,
    /// Parsed frontmatter metadata.
    pub metadata: MemoryMetadata,
    /// The markdown body content (without frontmatter).
    pub content: String,
    /// Computed relevance score (0.0 = irrelevant, 1.0 = highly relevant).
    pub relevance_score: f64,
}

// ─── Memory selector ───────────────────────────────────────────────────

/// Configuration and logic for selecting relevant memories.
///
/// Controls how many memories are injected and the total character budget
/// to avoid overloading the system prompt.
///
/// # Example
///
/// ```
/// use crab_session::memory_relevance::MemorySelector;
///
/// let selector = MemorySelector {
///     max_memories: 10,
///     max_total_chars: 5_000,
/// };
/// // selector.select_memories(memory_dir, context_hint) -> Vec<MemoryEntry>
/// ```
pub struct MemorySelector {
    /// Maximum number of memory entries to inject.
    pub max_memories: usize,
    /// Maximum total characters across all selected memory bodies.
    pub max_total_chars: usize,
}

impl Default for MemorySelector {
    fn default() -> Self {
        Self {
            max_memories: 20,
            max_total_chars: 8_000,
        }
    }
}

impl MemorySelector {
    /// Scan a memory directory and select the most relevant entries.
    ///
    /// Loads all `.md` files from `memory_dir`, parses their frontmatter,
    /// scores them against the optional `context_hint` (e.g. current working
    /// directory, recent user messages), sorts by relevance, and returns up
    /// to `max_memories` entries within `max_total_chars`.
    ///
    /// # Arguments
    ///
    /// * `memory_dir` — Path to the memory directory (e.g. `~/.crab/memory/`).
    /// * `context_hint` — Optional text used to bias relevance scoring
    ///   (e.g. the project name, recent user input, or CWD).
    pub fn select_memories(
        &self,
        _memory_dir: &Path,
        _context_hint: Option<&str>,
    ) -> Vec<MemoryEntry> {
        todo!("select_memories: scan directory, parse frontmatter, score, sort, and select")
    }

    /// Score a single memory entry against the current context.
    ///
    /// Uses keyword overlap, memory type priority, and recency to compute
    /// a relevance score in the range `[0.0, 1.0]`.
    #[must_use]
    pub fn score_memory(_entry: &MemoryEntry, _context: &str) -> f64 {
        todo!("score_memory: keyword overlap + type priority + recency heuristic")
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_selector_values() {
        let sel = MemorySelector::default();
        assert_eq!(sel.max_memories, 20);
        assert_eq!(sel.max_total_chars, 8_000);
    }

    #[test]
    fn memory_entry_debug() {
        let entry = MemoryEntry {
            path: PathBuf::from("/tmp/test.md"),
            metadata: MemoryMetadata {
                name: "Test".into(),
                description: "test".into(),
                memory_type: super::super::memory_types::MemoryType::User,
                created_at: None,
                updated_at: None,
            },
            content: "body".into(),
            relevance_score: 0.75,
        };
        let debug = format!("{entry:?}");
        assert!(debug.contains("Test"));
        assert!(debug.contains("0.75"));
    }
}
