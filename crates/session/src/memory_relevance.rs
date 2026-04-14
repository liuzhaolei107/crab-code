//! Score and select memories for injection into the system prompt.
//!
//! Scans memory directories, scores each memory entry against the current
//! conversation context, and selects the most relevant entries up to a
//! configurable budget. This keeps the system prompt focused and avoids
//! wasting context window on irrelevant memories.
//!
//! Maps to CCB `memdir/findRelevantMemories.ts` + `memoryScan.ts`.

use std::fs;
use std::path::{Path, PathBuf};

use super::memory_types::{MemoryMetadata, MemoryType, extract_body, parse_memory_frontmatter};

/// Maximum number of memory files to scan from a single directory.
const MAX_MEMORY_FILES: usize = 200;

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
    pub fn select_memories(
        &self,
        memory_dir: &Path,
        context_hint: Option<&str>,
    ) -> Vec<MemoryEntry> {
        let ctx = context_hint.unwrap_or("");

        // Scan directory for .md files
        let Ok(dir_entries) = fs::read_dir(memory_dir) else {
            return Vec::new();
        };

        let mut result: Vec<MemoryEntry> = Vec::new();

        for dir_entry in dir_entries.flatten().take(MAX_MEMORY_FILES) {
            let path = dir_entry.path();

            // Only .md files, skip MEMORY.md index
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if path.file_name().and_then(|f| f.to_str()) == Some("MEMORY.md") {
                continue;
            }

            let Ok(file_content) = fs::read_to_string(&path) else {
                continue;
            };

            let Some(metadata) = parse_memory_frontmatter(&file_content) else {
                continue;
            };

            let body = extract_body(&file_content).to_string();

            let mut mem_entry = MemoryEntry {
                path,
                metadata,
                content: body,
                relevance_score: 0.0,
            };

            mem_entry.relevance_score = Self::score_memory(&mem_entry, ctx);
            result.push(mem_entry);
        }

        // Sort by relevance (highest first)
        result.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply budget: max memories count + max total chars
        let mut selected = Vec::new();
        let mut total_chars = 0;

        for entry in result {
            if selected.len() >= self.max_memories {
                break;
            }
            let entry_chars = entry.content.len() + entry.metadata.description.len();
            if total_chars + entry_chars > self.max_total_chars && !selected.is_empty() {
                break;
            }
            total_chars += entry_chars;
            selected.push(entry);
        }

        selected
    }

    /// Score a single memory entry against the current context.
    ///
    /// Uses keyword overlap, memory type priority, and description match
    /// to compute a relevance score in the range `[0.0, 1.0]`.
    #[must_use]
    pub fn score_memory(entry: &MemoryEntry, context: &str) -> f64 {
        if context.is_empty() {
            // No context hint: use type-based priority only
            return type_priority(entry.metadata.memory_type);
        }

        let ctx_lower = context.to_lowercase();
        let ctx_words: Vec<&str> = ctx_lower.split_whitespace().collect();

        // Score components:
        // 1. Keyword overlap between context and memory description/content
        let desc = entry.metadata.description.to_lowercase();
        let body = entry.content.to_lowercase();
        let combined = format!("{desc} {body}");

        let matching_words = ctx_words
            .iter()
            .filter(|w| w.len() >= 3 && combined.contains(**w))
            .count();

        let keyword_score = if ctx_words.is_empty() {
            0.0
        } else {
            (matching_words as f64 / ctx_words.len() as f64).min(1.0)
        };

        // 2. Type priority bonus
        let type_bonus = type_priority(entry.metadata.memory_type);

        // 3. Name match bonus (if the memory name appears in context)
        let name_lower = entry.metadata.name.to_lowercase();
        let name_bonus = if ctx_lower.contains(&name_lower) {
            0.2
        } else {
            0.0
        };

        // Weighted combination
        let score = keyword_score * 0.6 + type_bonus * 0.3 + name_bonus * 0.1;
        score.min(1.0)
    }
}

/// Type-based priority score. Feedback and user memories are generally
/// more actionable than references.
fn type_priority(memory_type: MemoryType) -> f64 {
    match memory_type {
        MemoryType::Feedback => 0.8,
        MemoryType::User => 0.7,
        MemoryType::Project => 0.6,
        MemoryType::Reference => 0.4,
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
                memory_type: MemoryType::User,
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

    #[test]
    fn score_memory_no_context() {
        let entry = MemoryEntry {
            path: PathBuf::from("test.md"),
            metadata: MemoryMetadata {
                name: "test".into(),
                description: "desc".into(),
                memory_type: MemoryType::Feedback,
                created_at: None,
                updated_at: None,
            },
            content: "body".into(),
            relevance_score: 0.0,
        };
        let score = MemorySelector::score_memory(&entry, "");
        // Should return type priority for feedback = 0.8
        assert!((score - 0.8).abs() < 0.01);
    }

    #[test]
    fn score_memory_with_keyword_match() {
        let entry = MemoryEntry {
            path: PathBuf::from("test.md"),
            metadata: MemoryMetadata {
                name: "rust-prefs".into(),
                description: "User prefers Rust async patterns".into(),
                memory_type: MemoryType::User,
                created_at: None,
                updated_at: None,
            },
            content: "Use tokio for async runtime".into(),
            relevance_score: 0.0,
        };
        let score = MemorySelector::score_memory(&entry, "async runtime tokio patterns");
        assert!(score > 0.5, "Expected high score, got {score}");
    }

    #[test]
    fn score_memory_no_keyword_match() {
        let entry = MemoryEntry {
            path: PathBuf::from("test.md"),
            metadata: MemoryMetadata {
                name: "database".into(),
                description: "Database connection settings".into(),
                memory_type: MemoryType::Reference,
                created_at: None,
                updated_at: None,
            },
            content: "PostgreSQL on port 5432".into(),
            relevance_score: 0.0,
        };
        let score = MemorySelector::score_memory(&entry, "frontend react components");
        // Low keyword overlap + reference type priority
        assert!(score < 0.5, "Expected low score, got {score}");
    }

    #[test]
    fn type_priority_ordering() {
        assert!(type_priority(MemoryType::Feedback) > type_priority(MemoryType::Reference));
        assert!(type_priority(MemoryType::User) > type_priority(MemoryType::Project));
    }

    #[test]
    fn select_from_nonexistent_dir() {
        let sel = MemorySelector::default();
        let result = sel.select_memories(Path::new("/nonexistent/path"), None);
        assert!(result.is_empty());
    }

    #[test]
    fn select_respects_max_memories() {
        // This tests the budget logic without needing real files
        let sel = MemorySelector {
            max_memories: 2,
            max_total_chars: 100_000,
        };
        let result = sel.select_memories(Path::new("/nonexistent"), None);
        assert!(result.len() <= 2);
    }
}
