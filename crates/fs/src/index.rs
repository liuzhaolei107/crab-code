//! Incremental file index for fast path lookup and glob matching.
//!
//! Maintains a sorted `Vec<PathBuf>` of all known file paths in a project.
//! Supports incremental updates via [`WatchEvent`] from the [`watch`](crate::watch)
//! module, avoiding full directory rescans on every change.

use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};

use crate::watch::WatchEvent;

// ── FileIndex ────────────────────────────────────────────────────────

/// A sorted-vec file path index supporting fast glob matching and
/// incremental updates from file-watcher events.
#[derive(Debug, Clone)]
pub struct FileIndex {
    /// Sorted list of all indexed file paths (absolute or relative,
    /// depending on how they were inserted).
    paths: Vec<PathBuf>,
    /// Root directory of the index (used for `build_from_dir`).
    root: PathBuf,
}

impl FileIndex {
    /// Create an empty index rooted at `root`.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            paths: Vec::new(),
            root,
        }
    }

    /// Build an index by recursively walking `root`, respecting `.gitignore`.
    ///
    /// # Errors
    ///
    /// Returns an error if the root directory cannot be read.
    pub fn build_from_dir(root: &Path) -> crab_common::Result<Self> {
        let mut paths = Vec::new();
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry = entry.map_err(|e| crab_common::Error::Other(format!("walk error: {e}")))?;
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                paths.push(entry.into_path());
            }
        }

        paths.sort();
        Ok(Self {
            paths,
            root: root.to_path_buf(),
        })
    }

    /// Add a path to the index (maintains sorted order, no duplicates).
    pub fn add(&mut self, path: PathBuf) {
        match self.paths.binary_search(&path) {
            Ok(_) => {} // already present
            Err(pos) => self.paths.insert(pos, path),
        }
    }

    /// Remove a path from the index.
    /// Returns `true` if the path was present and removed.
    pub fn remove(&mut self, path: &Path) -> bool {
        match self.paths.binary_search(&path.to_path_buf()) {
            Ok(pos) => {
                self.paths.remove(pos);
                true
            }
            Err(_) => false,
        }
    }

    /// Rename a path in the index (remove old, add new).
    pub fn rename(&mut self, from: &Path, to: PathBuf) {
        self.remove(from);
        self.add(to);
    }

    /// Apply a [`WatchEvent`] to incrementally update the index.
    pub fn apply_watch_event(&mut self, event: &WatchEvent) {
        match event {
            WatchEvent::Created(path) => self.add(path.clone()),
            WatchEvent::Modified(_) => {
                // Content change doesn't affect the path index.
            }
            WatchEvent::Removed(path) => {
                self.remove(path);
            }
            WatchEvent::Renamed { from, to } => self.rename(from, to.clone()),
        }
    }

    /// Return all paths matching a glob pattern.
    ///
    /// The pattern is matched against the full path (not just the filename).
    ///
    /// # Errors
    ///
    /// Returns an error if the glob pattern is invalid.
    pub fn glob_match(&self, pattern: &str) -> crab_common::Result<Vec<&Path>> {
        let glob = Glob::new(pattern)
            .map_err(|e| crab_common::Error::Other(format!("invalid glob: {e}")))?;
        let matcher = glob.compile_matcher();
        Ok(self.glob_match_compiled(&matcher))
    }

    /// Return all paths matching a pre-compiled glob matcher.
    #[must_use]
    pub fn glob_match_compiled<'a>(&'a self, matcher: &GlobMatcher) -> Vec<&'a Path> {
        self.paths
            .iter()
            .filter(|p| matcher.is_match(p))
            .map(PathBuf::as_path)
            .collect()
    }

    /// Return all paths under a given directory prefix.
    #[must_use]
    pub fn paths_under(&self, prefix: &Path) -> Vec<&Path> {
        // Binary search to find the first path >= prefix, then scan forward.
        let prefix_buf = prefix.to_path_buf();
        let start = match self.paths.binary_search(&prefix_buf) {
            Ok(pos) | Err(pos) => pos,
        };
        self.paths[start..]
            .iter()
            .take_while(|p| p.starts_with(prefix))
            .map(PathBuf::as_path)
            .collect()
    }

    /// Check if a path is in the index.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        self.paths.binary_search(&path.to_path_buf()).is_ok()
    }

    /// Total number of indexed paths.
    #[must_use]
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// The root directory of this index.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// All indexed paths as a slice.
    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn empty_index() {
        let idx = FileIndex::new(PathBuf::from("/tmp"));
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn add_and_contains() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.add(PathBuf::from("/tmp/a.rs"));
        idx.add(PathBuf::from("/tmp/b.rs"));
        assert!(idx.contains(Path::new("/tmp/a.rs")));
        assert!(idx.contains(Path::new("/tmp/b.rs")));
        assert!(!idx.contains(Path::new("/tmp/c.rs")));
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn add_duplicate_is_noop() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.add(PathBuf::from("/tmp/a.rs"));
        idx.add(PathBuf::from("/tmp/a.rs"));
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn remove_existing() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.add(PathBuf::from("/tmp/a.rs"));
        assert!(idx.remove(Path::new("/tmp/a.rs")));
        assert!(!idx.contains(Path::new("/tmp/a.rs")));
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        assert!(!idx.remove(Path::new("/tmp/nope")));
    }

    #[test]
    fn rename_updates_index() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.add(PathBuf::from("/tmp/old.rs"));
        idx.rename(Path::new("/tmp/old.rs"), PathBuf::from("/tmp/new.rs"));
        assert!(!idx.contains(Path::new("/tmp/old.rs")));
        assert!(idx.contains(Path::new("/tmp/new.rs")));
    }

    #[test]
    fn paths_are_sorted() {
        let mut idx = FileIndex::new(PathBuf::from("/"));
        idx.add(PathBuf::from("/c"));
        idx.add(PathBuf::from("/a"));
        idx.add(PathBuf::from("/b"));
        let paths: Vec<&str> = idx.paths().iter().filter_map(|p| p.to_str()).collect();
        assert_eq!(paths, vec!["/a", "/b", "/c"]);
    }

    #[test]
    fn apply_created_event() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.apply_watch_event(&WatchEvent::Created(PathBuf::from("/tmp/new.rs")));
        assert!(idx.contains(Path::new("/tmp/new.rs")));
    }

    #[test]
    fn apply_removed_event() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.add(PathBuf::from("/tmp/gone.rs"));
        idx.apply_watch_event(&WatchEvent::Removed(PathBuf::from("/tmp/gone.rs")));
        assert!(!idx.contains(Path::new("/tmp/gone.rs")));
    }

    #[test]
    fn apply_modified_event_is_noop() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.add(PathBuf::from("/tmp/a.rs"));
        idx.apply_watch_event(&WatchEvent::Modified(PathBuf::from("/tmp/a.rs")));
        assert_eq!(idx.len(), 1);
        assert!(idx.contains(Path::new("/tmp/a.rs")));
    }

    #[test]
    fn apply_renamed_event() {
        let mut idx = FileIndex::new(PathBuf::from("/tmp"));
        idx.add(PathBuf::from("/tmp/old.rs"));
        idx.apply_watch_event(&WatchEvent::Renamed {
            from: PathBuf::from("/tmp/old.rs"),
            to: PathBuf::from("/tmp/new.rs"),
        });
        assert!(!idx.contains(Path::new("/tmp/old.rs")));
        assert!(idx.contains(Path::new("/tmp/new.rs")));
    }

    #[test]
    fn glob_match_basic() {
        let mut idx = FileIndex::new(PathBuf::from("/project"));
        idx.add(PathBuf::from("/project/src/main.rs"));
        idx.add(PathBuf::from("/project/src/lib.rs"));
        idx.add(PathBuf::from("/project/README.md"));

        let matches = idx.glob_match("**/*.rs").unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn glob_match_invalid_pattern() {
        let idx = FileIndex::new(PathBuf::from("/tmp"));
        assert!(idx.glob_match("[invalid").is_err());
    }

    #[test]
    fn paths_under_prefix() {
        let mut idx = FileIndex::new(PathBuf::from("/project"));
        idx.add(PathBuf::from("/project/src/a.rs"));
        idx.add(PathBuf::from("/project/src/b.rs"));
        idx.add(PathBuf::from("/project/doc/c.md"));

        let under_src = idx.paths_under(Path::new("/project/src"));
        assert_eq!(under_src.len(), 2);
    }

    #[test]
    fn build_from_dir_walks_files() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src.join("lib.rs"), "// lib").unwrap();
        fs::write(dir.path().join("README.md"), "# Hi").unwrap();

        let idx = FileIndex::build_from_dir(dir.path()).unwrap();
        assert_eq!(idx.len(), 3);
        assert!(idx.contains(&src.join("main.rs")));
        assert!(idx.contains(&src.join("lib.rs")));
        assert!(idx.contains(&dir.path().join("README.md")));
    }

    #[test]
    fn root_returns_correct_path() {
        let idx = FileIndex::new(PathBuf::from("/my/project"));
        assert_eq!(idx.root(), Path::new("/my/project"));
    }
}
