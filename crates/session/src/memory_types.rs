//! Structured memory types for the auto-memory system.
//!
//! Defines the schema for memory entries stored in `~/.crab/memory/` and
//! project-level `.crab/memory/` directories. Each memory file contains
//! YAML frontmatter with metadata and a markdown body with the actual content.
//!
//! Maps to CCB `memdir/memoryTypes.ts`.

use serde::{Deserialize, Serialize};

// ─── Memory type enum ──────────────────────────────────────────────────

/// Classification of a memory entry's purpose.
///
/// Used to organize memories and influence relevance scoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Information about the user (role, preferences, environment).
    User,
    /// Corrections, confirmations, and style guidance from the user.
    Feedback,
    /// Ongoing work context (current tasks, project state).
    Project,
    /// Pointers to external resources (docs, repos, APIs).
    Reference,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => f.write_str("user"),
            Self::Feedback => f.write_str("feedback"),
            Self::Project => f.write_str("project"),
            Self::Reference => f.write_str("reference"),
        }
    }
}

// ─── Memory metadata ───────────────────────────────────────────────────

/// Metadata extracted from a memory file's YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Short human-readable name for this memory.
    pub name: String,
    /// One-line description shown in the memory index.
    pub description: String,
    /// Classification of this memory's purpose.
    pub memory_type: MemoryType,
    /// ISO 8601 timestamp when this memory was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// ISO 8601 timestamp when this memory was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

// ─── Parsing and formatting ────────────────────────────────────────────

/// Parse YAML frontmatter from a memory file into [`MemoryMetadata`].
///
/// Expects the file to begin with `---` delimited YAML containing at least
/// `name`, `description`, and `type` fields. Returns `None` if the
/// frontmatter is missing or unparseable.
///
/// # Example
///
/// ```
/// use crab_session::memory_types::parse_memory_frontmatter;
///
/// let content = "---\nname: My memory\ndescription: A test\ntype: user\n---\nBody text.";
/// // (implementation is todo!())
/// ```
pub fn parse_memory_frontmatter(_content: &str) -> Option<MemoryMetadata> {
    todo!("parse_memory_frontmatter: extract YAML frontmatter and deserialize to MemoryMetadata")
}

/// Format a memory entry for injection into the system prompt.
///
/// Produces a compact representation suitable for including in the LLM's
/// context, with the memory type, name, and body content.
pub fn format_memory_for_prompt(metadata: &MemoryMetadata, _body: &str) -> String {
    todo!(
        "format_memory_for_prompt: render memory '{}' ({}) for system prompt injection",
        metadata.name,
        metadata.memory_type
    )
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_type_display() {
        assert_eq!(MemoryType::User.to_string(), "user");
        assert_eq!(MemoryType::Feedback.to_string(), "feedback");
        assert_eq!(MemoryType::Project.to_string(), "project");
        assert_eq!(MemoryType::Reference.to_string(), "reference");
    }

    #[test]
    fn memory_type_serde_roundtrip() {
        let mt = MemoryType::Feedback;
        let json = serde_json::to_string(&mt).unwrap();
        assert_eq!(json, "\"feedback\"");
        let parsed: MemoryType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mt);
    }

    #[test]
    fn memory_metadata_serde_roundtrip() {
        let meta = MemoryMetadata {
            name: "Test".into(),
            description: "A test memory".into(),
            memory_type: MemoryType::User,
            created_at: Some("2025-01-01T00:00:00Z".into()),
            updated_at: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: MemoryMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test");
        assert_eq!(parsed.memory_type, MemoryType::User);
        assert!(parsed.updated_at.is_none());
    }

    #[test]
    fn memory_metadata_skip_none_fields() {
        let meta = MemoryMetadata {
            name: "Test".into(),
            description: "test".into(),
            memory_type: MemoryType::Project,
            created_at: None,
            updated_at: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(!json.contains("created_at"));
        assert!(!json.contains("updated_at"));
    }
}
