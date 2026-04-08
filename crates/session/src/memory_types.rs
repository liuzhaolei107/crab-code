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

impl std::str::FromStr for MemoryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "feedback" => Ok(Self::Feedback),
            "project" => Ok(Self::Project),
            "reference" => Ok(Self::Reference),
            other => Err(format!("unknown memory type: {other}")),
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
pub fn parse_memory_frontmatter(content: &str) -> Option<MemoryMetadata> {
    // Find the opening and closing `---` delimiters
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }

    let after_open = &content[3..];
    let close_idx = after_open.find("\n---")?;
    let yaml_block = &after_open[..close_idx].trim();

    // Parse key-value pairs from the YAML block (simple flat YAML parser)
    let mut name = None;
    let mut description = None;
    let mut memory_type = None;
    let mut created_at = None;
    let mut updated_at = None;

    for line in yaml_block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" => name = Some(value.to_string()),
                "description" => description = Some(value.to_string()),
                "type" => memory_type = value.parse::<MemoryType>().ok(),
                "created_at" => created_at = Some(value.to_string()),
                "updated_at" => updated_at = Some(value.to_string()),
                _ => {} // ignore unknown keys
            }
        }
    }

    Some(MemoryMetadata {
        name: name?,
        description: description?,
        memory_type: memory_type?,
        created_at,
        updated_at,
    })
}

/// Extract the body content from a memory file (everything after the frontmatter).
pub fn extract_body(content: &str) -> &str {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return content;
    }
    let after_open = &content[3..];
    if let Some(close_idx) = after_open.find("\n---") {
        let after_close = &after_open[close_idx + 4..];
        // Skip the newline after closing ---
        after_close
            .trim_start_matches('\n')
            .trim_start_matches('\r')
    } else {
        content
    }
}

/// Format a memory entry for injection into the system prompt.
///
/// Produces a compact representation suitable for including in the LLM's
/// context, with the memory type, name, and body content.
pub fn format_memory_for_prompt(metadata: &MemoryMetadata, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        format!(
            "[{}] {}: {}",
            metadata.memory_type, metadata.name, metadata.description
        )
    } else {
        format!(
            "[{}] {}: {}\n{}",
            metadata.memory_type, metadata.name, metadata.description, body
        )
    }
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
    fn memory_type_from_str() {
        assert_eq!("user".parse::<MemoryType>().unwrap(), MemoryType::User);
        assert_eq!(
            "feedback".parse::<MemoryType>().unwrap(),
            MemoryType::Feedback
        );
        assert_eq!(
            "PROJECT".parse::<MemoryType>().unwrap(),
            MemoryType::Project
        );
        assert!("unknown".parse::<MemoryType>().is_err());
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

    #[test]
    fn parse_frontmatter_valid() {
        let content = "---\nname: My memory\ndescription: A test\ntype: user\n---\nBody text.";
        let meta = parse_memory_frontmatter(content).unwrap();
        assert_eq!(meta.name, "My memory");
        assert_eq!(meta.description, "A test");
        assert_eq!(meta.memory_type, MemoryType::User);
    }

    #[test]
    fn parse_frontmatter_with_timestamps() {
        let content = "---\nname: Test\ndescription: desc\ntype: feedback\ncreated_at: 2025-01-01\nupdated_at: 2025-06-01\n---\n";
        let meta = parse_memory_frontmatter(content).unwrap();
        assert_eq!(meta.created_at.as_deref(), Some("2025-01-01"));
        assert_eq!(meta.updated_at.as_deref(), Some("2025-06-01"));
    }

    #[test]
    fn parse_frontmatter_missing_type_returns_none() {
        let content = "---\nname: Test\ndescription: desc\n---\nBody.";
        assert!(parse_memory_frontmatter(content).is_none());
    }

    #[test]
    fn parse_frontmatter_no_delimiters_returns_none() {
        let content = "Just plain text without frontmatter";
        assert!(parse_memory_frontmatter(content).is_none());
    }

    #[test]
    fn parse_frontmatter_missing_name_returns_none() {
        let content = "---\ndescription: desc\ntype: user\n---\n";
        assert!(parse_memory_frontmatter(content).is_none());
    }

    #[test]
    fn extract_body_with_frontmatter() {
        let content = "---\nname: Test\ntype: user\ndescription: d\n---\nBody text here.";
        assert_eq!(extract_body(content), "Body text here.");
    }

    #[test]
    fn extract_body_no_frontmatter() {
        let content = "Just plain text";
        assert_eq!(extract_body(content), "Just plain text");
    }

    #[test]
    fn format_for_prompt_with_body() {
        let meta = MemoryMetadata {
            name: "role".into(),
            description: "User is a Rust developer".into(),
            memory_type: MemoryType::User,
            created_at: None,
            updated_at: None,
        };
        let result = format_memory_for_prompt(&meta, "Prefers async/await patterns.");
        assert!(result.contains("[user] role:"));
        assert!(result.contains("Prefers async/await"));
    }

    #[test]
    fn format_for_prompt_empty_body() {
        let meta = MemoryMetadata {
            name: "note".into(),
            description: "A quick note".into(),
            memory_type: MemoryType::Project,
            created_at: None,
            updated_at: None,
        };
        let result = format_memory_for_prompt(&meta, "");
        assert_eq!(result, "[project] note: A quick note");
        assert!(!result.contains('\n'));
    }
}
