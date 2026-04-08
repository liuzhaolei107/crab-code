//! Team memory paths and loading for multi-agent collaboration.
//!
//! When running in team mode, agents share a team-scoped memory directory
//! (`~/.crab/teams/<name>/memory/`) so that discoveries by one agent are
//! visible to its teammates. This module handles reading and writing those
//! shared memories.

use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use super::memory_types::{MemoryMetadata, parse_memory_frontmatter};

// ── Paths ─────────────────────────────────────────────────────────────

/// Return the directory where team memories are stored.
///
/// The directory is `~/.crab/teams/<team_name>/memory/` and may not
/// exist yet; callers should create it on first write.
#[must_use]
pub fn team_memory_dir(team_name: &str) -> PathBuf {
    crab_common::utils::path::home_dir()
        .join(".crab")
        .join("teams")
        .join(team_name)
        .join("memory")
}

// ── Loading ───────────────────────────────────────────────────────────

/// Load all memory entries for the given team.
///
/// Scans the team memory directory, parses each `.md` file's YAML
/// frontmatter, and returns the metadata. Files that fail to parse are
/// silently skipped.
#[must_use]
pub fn load_team_memories(team_name: &str) -> Vec<MemoryMetadata> {
    let dir = team_memory_dir(team_name);

    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut memories = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process .md files, skip MEMORY.md index
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if path.file_name().and_then(|f| f.to_str()) == Some("MEMORY.md") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path)
            && let Some(meta) = parse_memory_frontmatter(&content)
        {
            memories.push(meta);
        }
    }

    memories
}

// ── Writing ───────────────────────────────────────────────────────────

/// Save a new memory entry for the given team.
///
/// Creates or overwrites a file in the team memory directory with YAML
/// frontmatter from `entry` and the markdown `content` as the body.
///
/// # Errors
///
/// Returns `Err` if the directory cannot be created or the file cannot
/// be written.
pub fn save_team_memory(
    team_name: &str,
    entry: &MemoryMetadata,
    content: &str,
) -> std::io::Result<()> {
    let dir = team_memory_dir(team_name);
    fs::create_dir_all(&dir)?;

    // Build the filename from the memory name (slugified)
    let filename = slugify(&entry.name);
    let filepath = dir.join(format!("{filename}.md"));

    // Build file content: YAML frontmatter + body
    let mut file_content = String::with_capacity(256 + content.len());
    file_content.push_str("---\n");
    let _ = writeln!(file_content, "name: {}", entry.name);
    let _ = writeln!(file_content, "description: {}", entry.description);
    let _ = writeln!(file_content, "type: {}", entry.memory_type);
    if let Some(ref created) = entry.created_at {
        let _ = writeln!(file_content, "created_at: {created}");
    }
    if let Some(ref updated) = entry.updated_at {
        let _ = writeln!(file_content, "updated_at: {updated}");
    }
    file_content.push_str("---\n\n");
    file_content.push_str(content);
    if !content.ends_with('\n') {
        file_content.push('\n');
    }

    fs::write(filepath, file_content)
}

/// Slugify a string for use as a filename.
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::memory_types::MemoryType;
    use super::*;

    #[test]
    fn team_memory_dir_path() {
        let dir = team_memory_dir("my-team");
        let path_str = dir.to_string_lossy();
        assert!(path_str.contains("teams"));
        assert!(path_str.contains("my-team"));
        assert!(path_str.contains("memory"));
    }

    #[test]
    fn load_nonexistent_team_returns_empty() {
        let result = load_team_memories("nonexistent_team_12345");
        assert!(result.is_empty());
    }

    #[test]
    fn save_and_load_team_memory() {
        let team_name = "test_team_save_load";
        let entry = MemoryMetadata {
            name: "test-entry".into(),
            description: "A test memory".into(),
            memory_type: MemoryType::Project,
            created_at: Some("2025-06-15".into()),
            updated_at: None,
        };

        // Save
        save_team_memory(team_name, &entry, "Test body content.").unwrap();

        // Load
        let memories = load_team_memories(team_name);
        assert!(!memories.is_empty());
        assert!(memories.iter().any(|m| m.name == "test-entry"));

        // Cleanup
        let dir = team_memory_dir(team_name);
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello_world");
        assert_eq!(slugify("my-memory"), "my-memory");
        assert_eq!(slugify("test 123!"), "test_123");
    }
}
