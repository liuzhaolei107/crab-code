//! System-prompt builder for the memory subsystem.
//!
//! Composes a header, the `MEMORY.md` index listing, any loaded memory
//! entries, and optional usage guidelines into a single markdown block
//! that can be injected into the LLM system prompt.

use std::fmt::Write;

use crate::index::MemoryIndex;
use crate::store::MemoryFile;
use crate::types::format_memory_for_prompt;

/// Builder for the memory-section of the system prompt.
#[derive(Debug, Clone)]
pub struct MemoryPromptBuilder {
    /// Human-readable label for the memory scope (e.g. project name, team
    /// name). When `Some`, included in the header sentence.
    pub display_name: Option<String>,
    /// Whether to append the "when to access memory" usage guidelines.
    pub include_guidelines: bool,
}

impl Default for MemoryPromptBuilder {
    fn default() -> Self {
        Self {
            display_name: None,
            include_guidelines: true,
        }
    }
}

impl MemoryPromptBuilder {
    /// Build the full memory prompt block.
    ///
    /// Sections (in order): header, optional context line, index listing,
    /// loaded memory entries, optional guidelines.
    #[must_use]
    pub fn build(&self, index: &MemoryIndex, selected: &[MemoryFile]) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# auto memory");
        out.push('\n');

        if let Some(name) = &self.display_name {
            let _ = writeln!(
                out,
                "You have a persistent, file-based memory system for {name}."
            );
            out.push('\n');
        }

        let _ = writeln!(out, "## Memory index");
        if index.entries.is_empty() {
            let _ = writeln!(out, "(empty)");
        } else {
            for entry in &index.entries {
                let _ = writeln!(
                    out,
                    "- [{}]({}) — {}",
                    entry.title, entry.filename, entry.description
                );
            }
        }
        out.push('\n');

        let _ = writeln!(out, "## Loaded memories");
        if selected.is_empty() {
            let _ = writeln!(out, "(none loaded)");
        } else {
            for mem in selected {
                out.push_str(&format_memory_for_prompt(&mem.metadata, &mem.body));
                out.push_str("\n\n");
            }
        }

        if self.include_guidelines {
            out.push('\n');
            out.push_str(GUIDELINES);
        }

        out
    }

    /// Build a guidelines-only block (for sub-agents that share the memory
    /// concept but load no specific content).
    #[must_use]
    pub fn build_guidelines_only(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# auto memory");
        out.push('\n');
        out.push_str(GUIDELINES);
        out
    }
}

const GUIDELINES: &str = "\
## When to access memory
- When memories seem relevant, or the user references prior-conversation work.
- You MUST access memory when the user explicitly asks you to check, recall, or remember.
- Memory records can become stale. Verify against current state before acting on them.
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{IndexEntry, MemoryIndex};
    use crate::store::MemoryFile;
    use crate::types::{MemoryMetadata, MemoryType};
    use std::path::PathBuf;

    fn sample_index() -> MemoryIndex {
        MemoryIndex {
            entries: vec![
                IndexEntry {
                    title: "user role".to_string(),
                    filename: "user_role.md".to_string(),
                    description: "user is a rust dev".to_string(),
                },
                IndexEntry {
                    title: "feedback testing".to_string(),
                    filename: "feedback-testing.md".to_string(),
                    description: "prefers TDD".to_string(),
                },
            ],
            truncation: None,
        }
    }

    fn sample_memory() -> MemoryFile {
        MemoryFile {
            filename: "user_role.md".to_string(),
            path: PathBuf::from("user_role.md"),
            metadata: MemoryMetadata {
                name: "user_role".to_string(),
                description: "user is a rust dev".to_string(),
                memory_type: MemoryType::User,
                created_at: None,
                updated_at: None,
            },
            body: "The user works primarily in Rust.".to_string(),
            mtime: None,
        }
    }

    #[test]
    fn build_includes_index_content() {
        let b = MemoryPromptBuilder::default();
        let out = b.build(&sample_index(), &[]);
        assert!(out.contains("user role"), "missing index title: {out}");
        assert!(
            out.contains("user is a rust dev"),
            "missing index desc: {out}"
        );
        assert!(out.contains("feedback-testing.md"));
    }

    #[test]
    fn build_includes_selected_memories() {
        let b = MemoryPromptBuilder::default();
        let mem = sample_memory();
        let out = b.build(&sample_index(), std::slice::from_ref(&mem));
        assert!(
            out.contains("The user works primarily in Rust."),
            "missing memory body: {out}"
        );
    }

    #[test]
    fn build_empty_index_and_memories() {
        let empty = MemoryIndex {
            entries: vec![],
            truncation: None,
        };
        let b = MemoryPromptBuilder::default();
        let out = b.build(&empty, &[]);
        assert!(out.to_lowercase().contains("memory"));
    }

    #[test]
    fn build_with_display_name_mentions_it() {
        let b = MemoryPromptBuilder {
            display_name: Some("crab-code".to_string()),
            include_guidelines: false,
        };
        let out = b.build(&sample_index(), &[]);
        assert!(out.contains("crab-code"), "display_name missing: {out}");
    }

    #[test]
    fn build_without_guidelines_omits_them() {
        let b = MemoryPromptBuilder {
            display_name: None,
            include_guidelines: false,
        };
        let out = b.build(&sample_index(), &[]);
        assert!(!out.contains("When to access memory"));
    }

    #[test]
    fn build_guidelines_only_no_content() {
        let b = MemoryPromptBuilder::default();
        let out = b.build_guidelines_only();
        assert!(out.to_lowercase().contains("memory"));
        // No specific memory names leak in.
        assert!(!out.contains("user_role"));
        assert!(!out.contains("feedback-testing.md"));
        assert!(out.contains("When to access memory"));
    }
}
