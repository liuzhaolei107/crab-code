//! Team-scoped memory store.
//!
//! Team memories live under `~/.crab/teams/<team_name>/memory/` so that
//! discoveries made by one agent in a multi-agent session are visible to
//! its teammates.

use std::fmt::Write;
use std::path::PathBuf;

use crate::paths;
use crate::store::{MemoryFile, MemoryStore};
use crate::types::{MemoryMetadata, format_frontmatter};

/// Team-scoped memory store.
///
/// Wraps a [`MemoryStore`] rooted at the team's memory directory and adds
/// a convenience [`save`](Self::save) method that writes YAML frontmatter
/// plus a markdown body to a slugified filename.
pub struct TeamMemoryStore {
    store: MemoryStore,
    team_name: String,
}

impl TeamMemoryStore {
    /// Create a store using the default team memory directory
    /// (`~/.crab/teams/<team_name>/memory/`).
    #[must_use]
    pub fn new(team_name: &str) -> Self {
        let dir = paths::team_memory_dir(team_name);
        Self {
            store: MemoryStore::new(dir),
            team_name: team_name.to_owned(),
        }
    }

    /// Create a store rooted at an explicit directory. Primarily used for
    /// testing.
    #[must_use]
    pub fn new_with_dir(team_name: &str, dir: PathBuf) -> Self {
        Self {
            store: MemoryStore::new(dir),
            team_name: team_name.to_owned(),
        }
    }

    /// Save a memory entry as `<slug(name)>.md` with YAML frontmatter.
    ///
    /// The filename is derived from [`MemoryMetadata::name`] via [`slugify`].
    /// The final content is `format_frontmatter(metadata) + "\n" + body`,
    /// with a trailing newline appended if `body` does not already end in
    /// one.
    pub fn save(&self, metadata: &MemoryMetadata, body: &str) -> crab_core::Result<()> {
        let filename = format!("{}.md", slugify(&metadata.name));
        let mut content = format_frontmatter(metadata);
        content.push('\n');
        content.push_str(body);
        if !body.ends_with('\n') {
            content.push('\n');
        }
        self.store.save(&filename, &content)
    }

    /// Load every team memory via [`MemoryStore::scan`].
    pub fn load_all(&self) -> crab_core::Result<Vec<MemoryFile>> {
        self.store.scan()
    }

    /// Name of the team this store belongs to — useful for rendering a
    /// heading when the memories are injected into a system prompt.
    #[must_use]
    pub fn team_name(&self) -> &str {
        &self.team_name
    }

    /// Render all team memories as a markdown block prefixed with a
    /// team-name heading. Returns an empty string when the team has no
    /// memories yet so callers can splice it into a larger prompt
    /// without an empty section.
    pub fn render_prompt_block(&self) -> crab_core::Result<String> {
        let memories = self.load_all()?;
        if memories.is_empty() {
            return Ok(String::new());
        }
        let mut out = String::new();
        let _ = writeln!(out, "# Team Memory — {}\n", self.team_name);
        for mem in &memories {
            let _ = writeln!(
                out,
                "## {} (type: {})",
                mem.metadata.name, mem.metadata.memory_type
            );
            if !mem.metadata.description.is_empty() {
                let _ = writeln!(out, "> {}", mem.metadata.description);
            }
            let _ = writeln!(out, "\n{}\n", mem.body.trim_start());
        }
        Ok(out)
    }
}

/// Slugify a string for use as a filename.
///
/// - ASCII alphanumerics and `-` are preserved (lowercased for alphabetic).
/// - Everything else becomes `_`.
/// - Leading and trailing `_` are trimmed.
fn slugify(s: &str) -> String {
    let raw: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    raw.trim_matches('_').to_string()
}

// ─── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemoryType;

    fn sample_metadata(name: &str) -> MemoryMetadata {
        MemoryMetadata {
            name: name.to_owned(),
            description: "a test memory".into(),
            memory_type: MemoryType::Project,
            created_at: Some("2025-06-15".into()),
            updated_at: None,
        }
    }

    #[test]
    fn save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TeamMemoryStore::new_with_dir("team-a", tmp.path().to_path_buf());

        let meta = sample_metadata("test-entry");
        store.save(&meta, "Test body content.").unwrap();

        let memories = store.load_all().unwrap();
        assert_eq!(memories.len(), 1);
        let loaded = &memories[0];
        assert_eq!(loaded.metadata.name, "test-entry");
        assert_eq!(loaded.metadata.memory_type, MemoryType::Project);
        assert_eq!(loaded.body, "\nTest body content.\n");
        assert_eq!(loaded.filename, "test-entry.md");
    }

    #[test]
    fn load_nonexistent_team() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does_not_exist");
        let store = TeamMemoryStore::new_with_dir("ghost", missing);
        let memories = store.load_all().unwrap();
        assert!(memories.is_empty());
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello_world");
        assert_eq!(slugify("my-memory"), "my-memory");
        assert_eq!(slugify("test 123!"), "test_123");
    }

    #[test]
    fn team_name_accessor_returns_original() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TeamMemoryStore::new_with_dir("crab-dev", tmp.path().to_path_buf());
        assert_eq!(store.team_name(), "crab-dev");
    }

    #[test]
    fn render_prompt_block_is_empty_for_empty_store() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TeamMemoryStore::new_with_dir("alpha", tmp.path().to_path_buf());
        let rendered = store.render_prompt_block().unwrap();
        assert!(rendered.is_empty());
    }

    #[test]
    fn render_prompt_block_includes_team_name_and_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let store = TeamMemoryStore::new_with_dir("quality", tmp.path().to_path_buf());
        let meta = sample_metadata("lint-policy");
        store.save(&meta, "Always run clippy before push.").unwrap();

        let rendered = store.render_prompt_block().unwrap();
        assert!(rendered.contains("# Team Memory — quality"));
        assert!(rendered.contains("## lint-policy (type: project)"));
        assert!(rendered.contains("Always run clippy before push."));
    }
}
