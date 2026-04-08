//! Team memory paths and loading for multi-agent collaboration.
//!
//! When running in team mode, agents share a team-scoped memory directory
//! (`~/.crab/teams/<name>/memory/`) so that discoveries by one agent are
//! visible to its teammates. This module handles reading and writing those
//! shared memories.

use std::path::PathBuf;

use super::memory_types::MemoryMetadata;

// ── Paths ─────────────────────────────────────────────────────────────

/// Return the directory where team memories are stored.
///
/// The directory is `~/.crab/teams/<team_name>/memory/` and may not
/// exist yet; callers should create it on first write.
#[must_use]
pub fn team_memory_dir(_team_name: &str) -> PathBuf {
    todo!("team_memory_dir: construct ~/.crab/teams/<team_name>/memory/ path")
}

// ── Loading ───────────────────────────────────────────────────────────

/// Load all memory entries for the given team.
///
/// Scans the team memory directory, parses each `.md` file's YAML
/// frontmatter, and returns the metadata. Files that fail to parse are
/// silently skipped.
#[must_use]
pub fn load_team_memories(_team_name: &str) -> Vec<MemoryMetadata> {
    todo!("load_team_memories: scan team_memory_dir, parse frontmatter for each .md file")
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
    _team_name: &str,
    _entry: &MemoryMetadata,
    _content: &str,
) -> std::io::Result<()> {
    todo!("save_team_memory: write YAML frontmatter + content to team memory file")
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn module_compiles() {
        // Verifies the module is syntactically valid.
    }
}
