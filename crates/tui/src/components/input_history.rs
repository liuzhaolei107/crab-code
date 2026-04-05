//! Persistent input history with file-based storage.
//!
//! Stores input history to `~/.crab/input_history` so previous commands
//! survive across sessions. Supports max-size trimming and deduplication.

use std::collections::VecDeque;
use std::path::PathBuf;

/// Maximum number of history entries to retain.
const DEFAULT_MAX_ENTRIES: usize = 500;

/// Persistent input history backed by a file.
pub struct InputHistory {
    /// Ordered entries (oldest first, newest last).
    entries: VecDeque<String>,
    /// Maximum number of entries to keep.
    max_entries: usize,
    /// File path for persistence (None = in-memory only).
    file_path: Option<PathBuf>,
    /// Current browsing index (None = not browsing).
    browse_index: Option<usize>,
    /// Saved current input when entering browse mode.
    saved_input: Option<String>,
}

impl InputHistory {
    /// Create a new in-memory history with default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
            file_path: None,
            browse_index: None,
            saved_input: None,
        }
    }

    /// Create a history backed by a file at the given path.
    /// Loads existing entries from the file if it exists.
    #[must_use]
    pub fn with_file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut history = Self {
            entries: VecDeque::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
            file_path: Some(path),
            browse_index: None,
            saved_input: None,
        };
        history.load();
        history
    }

    /// Create with a custom max entry count.
    #[must_use]
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max.max(1);
        self.trim();
        self
    }

    /// Return the default file path: `~/.crab/input_history`.
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        dirs_path().map(|p| p.join("input_history"))
    }

    /// Number of entries in history.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the history is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Push a new entry. Deduplicates (removes previous occurrence).
    /// Trims to max size. Persists if a file path is set.
    pub fn push(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        if entry.trim().is_empty() {
            return;
        }

        // Remove duplicate if exists
        if let Some(pos) = self.entries.iter().position(|e| e == &entry) {
            self.entries.remove(pos);
        }

        self.entries.push_back(entry);
        self.trim();
        self.save();
    }

    /// Get all entries (oldest first).
    #[must_use]
    pub fn entries(&self) -> Vec<&str> {
        self.entries.iter().map(String::as_str).collect()
    }

    /// Search entries matching a prefix (newest first).
    #[must_use]
    pub fn search_prefix(&self, prefix: &str) -> Vec<&str> {
        self.entries
            .iter()
            .rev()
            .filter(|e| e.starts_with(prefix))
            .map(String::as_str)
            .collect()
    }

    /// Search entries containing a substring (newest first).
    #[must_use]
    pub fn search_contains(&self, query: &str) -> Vec<&str> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .rev()
            .filter(|e| e.to_lowercase().contains(&query_lower))
            .map(String::as_str)
            .collect()
    }

    /// Start or continue browsing up (older). Returns the entry to display.
    /// `current_input` is saved on first call for later restoration.
    pub fn browse_up(&mut self, current_input: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        let idx = match self.browse_index {
            None => {
                self.saved_input = Some(current_input.to_string());
                self.entries.len() - 1
            }
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => return self.entries.front().map(String::as_str),
        };

        self.browse_index = Some(idx);
        self.entries.get(idx).map(String::as_str)
    }

    /// Browse down (newer). Returns the entry or the saved input.
    pub fn browse_down(&mut self) -> Option<&str> {
        match self.browse_index {
            Some(idx) if idx + 1 < self.entries.len() => {
                let new_idx = idx + 1;
                self.browse_index = Some(new_idx);
                self.entries.get(new_idx).map(String::as_str)
            }
            Some(_) => {
                self.browse_index = None;
                self.saved_input.as_deref()
            }
            None => None,
        }
    }

    /// Reset browse state without changing entries.
    pub fn reset_browse(&mut self) {
        self.browse_index = None;
        self.saved_input = None;
    }

    /// Whether currently in browse mode.
    #[must_use]
    pub fn is_browsing(&self) -> bool {
        self.browse_index.is_some()
    }

    /// Clear all history entries and the backing file.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.browse_index = None;
        self.saved_input = None;
        self.save();
    }

    // ── Internal ──

    fn trim(&mut self) {
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    fn load(&mut self) {
        let Some(path) = &self.file_path else {
            return;
        };
        let Ok(content) = std::fs::read_to_string(path) else {
            return;
        };
        for line in content.lines() {
            let decoded = line.replace("\\n", "\n").replace("\\\\", "\\");
            if !decoded.trim().is_empty() {
                self.entries.push_back(decoded);
            }
        }
        self.trim();
    }

    fn save(&self) {
        let Some(path) = &self.file_path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content: String = self
            .entries
            .iter()
            .map(|e| {
                let encoded = e.replace('\\', "\\\\").replace('\n', "\\n");
                encoded + "\n"
            })
            .collect();
        let _ = std::fs::write(path, content);
    }
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve the `~/.crab` directory path.
fn dirs_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| PathBuf::from(p).join(".crab"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".crab"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_history_is_empty() {
        let history = InputHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn push_and_retrieve() {
        let mut history = InputHistory::new();
        history.push("hello");
        history.push("world");
        assert_eq!(history.len(), 2);
        assert_eq!(history.entries(), vec!["hello", "world"]);
    }

    #[test]
    fn push_deduplicates() {
        let mut history = InputHistory::new();
        history.push("hello");
        history.push("world");
        history.push("hello");
        assert_eq!(history.len(), 2);
        assert_eq!(history.entries(), vec!["world", "hello"]);
    }

    #[test]
    fn push_ignores_empty() {
        let mut history = InputHistory::new();
        history.push("");
        history.push("   ");
        assert!(history.is_empty());
    }

    #[test]
    fn max_entries_trims() {
        let mut history = InputHistory::new().with_max_entries(3);
        history.push("a");
        history.push("b");
        history.push("c");
        history.push("d");
        assert_eq!(history.len(), 3);
        assert_eq!(history.entries(), vec!["b", "c", "d"]);
    }

    #[test]
    fn search_prefix() {
        let mut history = InputHistory::new();
        history.push("cargo build");
        history.push("cargo test");
        history.push("git status");
        let results = history.search_prefix("cargo");
        assert_eq!(results, vec!["cargo test", "cargo build"]);
    }

    #[test]
    fn search_contains() {
        let mut history = InputHistory::new();
        history.push("run tests");
        history.push("build project");
        history.push("run benchmarks");
        let results = history.search_contains("run");
        assert_eq!(results, vec!["run benchmarks", "run tests"]);
    }

    #[test]
    fn search_contains_case_insensitive() {
        let mut history = InputHistory::new();
        history.push("Hello World");
        let results = history.search_contains("hello");
        assert_eq!(results, vec!["Hello World"]);
    }

    #[test]
    fn browse_up_down() {
        let mut history = InputHistory::new();
        history.push("first");
        history.push("second");
        history.push("third");

        // Browse up from current input
        let entry = history.browse_up("current");
        assert_eq!(entry, Some("third"));

        let entry = history.browse_up("current");
        assert_eq!(entry, Some("second"));

        let entry = history.browse_up("current");
        assert_eq!(entry, Some("first"));

        // At the top, stays at first
        let entry = history.browse_up("current");
        assert_eq!(entry, Some("first"));

        // Browse back down
        let entry = history.browse_down();
        assert_eq!(entry, Some("second"));

        let entry = history.browse_down();
        assert_eq!(entry, Some("third"));

        // Past the end restores saved input
        let entry = history.browse_down();
        assert_eq!(entry, Some("current"));

        assert!(!history.is_browsing());
    }

    #[test]
    fn browse_empty_history() {
        let mut history = InputHistory::new();
        assert_eq!(history.browse_up("test"), None);
        assert!(!history.is_browsing());
    }

    #[test]
    fn reset_browse() {
        let mut history = InputHistory::new();
        history.push("entry");
        history.browse_up("test");
        assert!(history.is_browsing());
        history.reset_browse();
        assert!(!history.is_browsing());
    }

    #[test]
    fn clear_empties_everything() {
        let mut history = InputHistory::new();
        history.push("entry1");
        history.push("entry2");
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn file_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("crab_test_history");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_history");

        // Write some history
        {
            let mut history = InputHistory::with_file(&path);
            history.push("line one");
            history.push("line two");
            history.push("multi\nline\nentry");
        }

        // Load it back
        {
            let history = InputHistory::with_file(&path);
            assert_eq!(history.len(), 3);
            let entries = history.entries();
            assert_eq!(entries[0], "line one");
            assert_eq!(entries[1], "line two");
            assert_eq!(entries[2], "multi\nline\nentry");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_persistence_handles_missing_file() {
        let history = InputHistory::with_file("/nonexistent/path/history");
        assert!(history.is_empty());
    }

    #[test]
    fn default_path_returns_some() {
        // Should work on any system with HOME or USERPROFILE set
        let path = InputHistory::default_path();
        if std::env::var("HOME").is_ok() || std::env::var("USERPROFILE").is_ok() {
            assert!(path.is_some());
            let p = path.unwrap();
            assert!(p.to_string_lossy().contains(".crab"));
            assert!(p.to_string_lossy().contains("input_history"));
        }
    }

    #[test]
    fn encoding_handles_backslashes() {
        let dir = std::env::temp_dir().join("crab_test_history_bs");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_bs");

        {
            let mut history = InputHistory::with_file(&path);
            history.push("path\\to\\file");
        }
        {
            let history = InputHistory::with_file(&path);
            assert_eq!(history.entries()[0], "path\\to\\file");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
