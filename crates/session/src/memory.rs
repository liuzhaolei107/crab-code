//! Backward-compatible memory store for `crab-session`.
//!
//! Delegates to `crab_memory` for all I/O while preserving the legacy
//! [`MemoryFile`] shape (with `memory_type: String`) that downstream crates
//! (notably `crab-agent`) depend on.

use std::path::PathBuf;

// ─── Re-export index entry under old name ──────────────────────────────

/// Re-export `crab_memory::IndexEntry` as `MemoryIndexEntry` for compat.
pub use crab_memory::IndexEntry as MemoryIndexEntry;

// ─── Legacy MemoryFile (String-typed) ──────────────────────────────────

/// A single memory file with frontmatter metadata.
///
/// This is the **legacy** shape consumed by `crab-agent`. The `memory_type`
/// field is a plain `String` (e.g. `"user"`) rather than the typed enum in
/// `crab_memory::MemoryFile`.
#[derive(Debug, Clone)]
pub struct MemoryFile {
    pub name: String,
    pub description: String,
    pub memory_type: String,
    pub body: String,
    /// Filename (without directory).
    pub filename: String,
}

// ─── Compat MemoryStore ────────────────────────────────────────────────

/// File-based memory system — reads/writes `~/.crab/memory/`.
///
/// Thin wrapper around `crab_memory::MemoryStore` that converts the typed
/// [`crab_memory::MemoryFile`] into the legacy [`MemoryFile`] on read paths.
pub struct MemoryStore {
    inner: crab_memory::MemoryStore,
}

impl MemoryStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            inner: crab_memory::MemoryStore::new(path),
        }
    }

    /// Save a memory file (overwrites if exists).
    pub fn save(&self, filename: &str, content: &str) -> crab_common::Result<()> {
        self.inner.save(filename, content)
    }

    /// Load a memory file by filename. Returns `None` if not found.
    pub fn load(&self, filename: &str) -> crab_common::Result<Option<String>> {
        self.inner.load(filename)
    }

    /// Delete a memory file.
    pub fn delete(&self, filename: &str) -> crab_common::Result<()> {
        self.inner.delete(filename)
    }

    /// Parse a memory file's frontmatter and body into a legacy [`MemoryFile`].
    pub fn parse_memory_file(content: &str) -> Option<MemoryFile> {
        let metadata = crab_memory::parse_frontmatter(content)?;
        let body = crab_memory::extract_body(content).trim().to_string();

        if metadata.name.is_empty() {
            return None;
        }

        Some(MemoryFile {
            name: metadata.name,
            description: metadata.description,
            memory_type: metadata.memory_type.to_string(),
            body,
            filename: String::new(), // caller fills this in
        })
    }

    /// Load and parse all memory files (excluding `MEMORY.md`), returning
    /// legacy [`MemoryFile`] values sorted by filename.
    pub fn load_all(&self) -> crab_common::Result<Vec<MemoryFile>> {
        let typed_files = self.inner.scan()?;

        let mut memories: Vec<MemoryFile> = typed_files
            .into_iter()
            .map(|tf| MemoryFile {
                name: tf.metadata.name,
                description: tf.metadata.description,
                memory_type: tf.metadata.memory_type.to_string(),
                body: tf.body,
                filename: tf.filename,
            })
            .collect();

        // The old implementation sorted by filename.
        memories.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(memories)
    }

    /// Parse the `MEMORY.md` index file.
    pub fn load_index(&self) -> crab_common::Result<Vec<MemoryIndexEntry>> {
        let index = crab_memory::index::load_index(self.inner.dir())?;
        Ok(index.entries)
    }

    /// Save the `MEMORY.md` index file.
    pub fn save_index(&self, entries: &[MemoryIndexEntry]) -> crab_common::Result<()> {
        crab_memory::index::save_index(self.inner.dir(), entries)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Existing tests (preserved from original) ────────────────────

    #[test]
    fn parse_memory_file_basic() {
        let content = r"---
name: Test memory
description: A test memory file
type: user
---

This is the body content.

**Why:** because testing.
";
        let mem = MemoryStore::parse_memory_file(content).unwrap();
        assert_eq!(mem.name, "Test memory");
        assert_eq!(mem.description, "A test memory file");
        assert_eq!(mem.memory_type, "user");
        assert!(mem.body.contains("This is the body content."));
        assert!(mem.body.contains("**Why:** because testing."));
    }

    #[test]
    fn parse_memory_file_no_frontmatter() {
        assert!(MemoryStore::parse_memory_file("just some text").is_none());
    }

    #[test]
    fn parse_memory_file_no_name() {
        let content = "---\ndescription: no name\ntype: user\n---\nbody";
        assert!(MemoryStore::parse_memory_file(content).is_none());
    }

    #[test]
    fn parse_index_entries() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        // Write a MEMORY.md manually so we can test load_index.
        std::fs::write(
            dir.path().join("MEMORY.md"),
            "- [No telemetry](project_no_telemetry.md) \u{2014} All data stays local\n\
             - [Config path](project_config.md) \u{2014} Use ~/.crab/ only\n",
        )
        .unwrap();

        let entries = store.load_index().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "No telemetry");
        assert_eq!(entries[0].filename, "project_no_telemetry.md");
        assert_eq!(entries[0].description, "All data stays local");
        assert_eq!(entries[1].title, "Config path");
    }

    #[test]
    fn save_and_load_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        let content = "---\nname: Test\ndescription: test\ntype: user\n---\n\nBody here.";
        store.save("test_memory.md", content).unwrap();

        let loaded = store.load("test_memory.md").unwrap().unwrap();
        assert_eq!(loaded, content);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());
        assert!(store.load("nope.md").unwrap().is_none());
    }

    #[test]
    fn load_all_parses_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        store
            .save(
                "user_role.md",
                "---\nname: User role\ndescription: Role info\ntype: user\n---\n\nSenior dev.",
            )
            .unwrap();
        store
            .save(
                "feedback_style.md",
                "---\nname: Style feedback\ndescription: Code style\ntype: feedback\n---\n\nBe terse.",
            )
            .unwrap();
        // MEMORY.md should be excluded
        store
            .save("MEMORY.md", "- [User role](user_role.md)")
            .unwrap();

        let all = store.load_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].filename, "feedback_style.md");
        assert_eq!(all[1].filename, "user_role.md");
    }

    #[test]
    fn save_and_load_index() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        let entries = vec![
            MemoryIndexEntry {
                title: "Role".into(),
                filename: "user_role.md".into(),
                description: "User's role info".into(),
            },
            MemoryIndexEntry {
                title: "Style".into(),
                filename: "feedback_style.md".into(),
                description: "Code style prefs".into(),
            },
        ];
        store.save_index(&entries).unwrap();

        let loaded = store.load_index().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].title, "Role");
        assert_eq!(loaded[1].title, "Style");
    }

    #[test]
    fn delete_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        store.save("temp.md", "content").unwrap();
        assert!(store.load("temp.md").unwrap().is_some());

        store.delete("temp.md").unwrap();
        assert!(store.load("temp.md").unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());
        store.delete("nonexistent.md").unwrap(); // should not error
    }

    #[test]
    fn parse_memory_file_empty_body() {
        let content = "---\nname: Empty body\ndescription: test\ntype: user\n---\n";
        let mem = MemoryStore::parse_memory_file(content).unwrap();
        assert_eq!(mem.name, "Empty body");
        assert!(mem.body.is_empty());
    }

    #[test]
    fn parse_memory_file_incomplete_frontmatter() {
        // Missing closing ---
        let content = "---\nname: Test\ndescription: test\ntype: user\nno closing";
        assert!(MemoryStore::parse_memory_file(content).is_none());
    }

    #[test]
    fn parse_memory_file_extra_fields_ignored() {
        let content =
            "---\nname: Test\ndescription: test\ntype: user\nunknown_field: value\n---\n\nBody.";
        let mem = MemoryStore::parse_memory_file(content).unwrap();
        assert_eq!(mem.name, "Test");
    }

    #[test]
    fn load_all_skips_non_md_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        store
            .save(
                "user_role.md",
                "---\nname: Role\ndescription: test\ntype: user\n---\n\nBody.",
            )
            .unwrap();
        // Save a non-md file
        std::fs::write(dir.path().join("notes.txt"), "not a memory file").unwrap();
        // Save a file without valid frontmatter
        store.save("invalid.md", "no frontmatter here").unwrap();

        let all = store.load_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].filename, "user_role.md");
    }

    #[test]
    fn load_all_nonexistent_dir_returns_empty() {
        let store = MemoryStore::new(PathBuf::from("/nonexistent/memory/dir"));
        let all = store.load_all().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn load_index_nonexistent_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());
        let entries = store.load_index().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn save_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        store.save("test.md", "original").unwrap();
        store.save("test.md", "updated").unwrap();
        let loaded = store.load("test.md").unwrap().unwrap();
        assert_eq!(loaded, "updated");
    }

    #[test]
    fn parse_index_with_dash_dash_separator() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        std::fs::write(
            dir.path().join("MEMORY.md"),
            "- [Title](file.md) -- description with dashes\n",
        )
        .unwrap();

        let entries = store.load_index().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Title");
        assert_eq!(entries[0].filename, "file.md");
        assert_eq!(entries[0].description, "description with dashes");
    }

    #[test]
    fn save_index_then_load_index_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        let entries = vec![MemoryIndexEntry {
            title: "First".into(),
            filename: "first.md".into(),
            description: "First memory".into(),
        }];
        store.save_index(&entries).unwrap();

        let loaded = store.load_index().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].title, "First");
        assert_eq!(loaded[0].filename, "first.md");
        assert_eq!(loaded[0].description, "First memory");
    }

    #[test]
    fn save_creates_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("deep").join("memory");
        let store = MemoryStore::new(nested.clone());

        store.save("test.md", "content").unwrap();
        assert!(nested.join("test.md").exists());
    }

    // ── Compat-specific tests ──────────────────────────────────────

    #[test]
    fn compat_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        let content = "---\nname: Compat\ndescription: compat test\ntype: project\n---\n\nBody.";
        store.save("compat.md", content).unwrap();

        let loaded = store.load("compat.md").unwrap().unwrap();
        assert_eq!(loaded, content);

        // Round-trip parse
        let mem = MemoryStore::parse_memory_file(&loaded).unwrap();
        assert_eq!(mem.name, "Compat");
        assert_eq!(mem.memory_type, "project");
    }

    #[test]
    fn compat_parse_memory_file() {
        let content =
            "---\nname: Style\ndescription: Be terse\ntype: feedback\n---\n\nShort answers.";
        let mem = MemoryStore::parse_memory_file(content).unwrap();

        // Verify String-typed memory_type (not enum).
        assert_eq!(mem.memory_type, "feedback");
        assert_eq!(mem.name, "Style");
        assert_eq!(mem.description, "Be terse");
        assert_eq!(mem.body, "Short answers.");
        // filename is not set by parse_memory_file
        assert!(mem.filename.is_empty());
    }

    #[test]
    fn compat_load_all() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path().to_path_buf());

        store
            .save(
                "alpha.md",
                "---\nname: Alpha\ndescription: first\ntype: user\n---\n\nA body.",
            )
            .unwrap();
        store
            .save(
                "beta.md",
                "---\nname: Beta\ndescription: second\ntype: reference\n---\n\nB body.",
            )
            .unwrap();
        // MEMORY.md index should be excluded
        store
            .save("MEMORY.md", "- [Alpha](alpha.md) \u{2014} first")
            .unwrap();

        let all = store.load_all().unwrap();
        assert_eq!(all.len(), 2);

        // Sorted by filename
        assert_eq!(all[0].filename, "alpha.md");
        assert_eq!(all[1].filename, "beta.md");

        // memory_type is String, not enum
        assert_eq!(all[0].memory_type, "user");
        assert_eq!(all[1].memory_type, "reference");

        // Fields are populated
        assert_eq!(all[0].name, "Alpha");
        assert_eq!(all[1].description, "second");
    }
}
