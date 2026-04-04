//! Glob-based file pattern matching (globset + ignore).

use std::path::{Path, PathBuf};

/// Result of a glob search.
pub struct GlobResult {
    pub matches: Vec<PathBuf>,
    pub truncated: bool,
}

/// Find files matching a glob pattern under `root`.
///
/// Uses the `ignore` crate so `.gitignore` rules are respected automatically.
/// Results are sorted by modification time (most recent first).
///
/// # Errors
///
/// Returns an error if the glob pattern is invalid or the root path is inaccessible.
pub fn find_files(_root: &Path, _pattern: &str, _limit: usize) -> crab_common::Result<GlobResult> {
    todo!()
}
