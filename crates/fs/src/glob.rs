//! Glob-based file pattern matching.
//!
//! Uses [`globset`] for pattern compilation and [`ignore`] for directory
//! traversal that automatically respects `.gitignore` rules.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Result of a glob search.
#[derive(Debug, Clone)]
pub struct GlobResult {
    /// Matched file paths, sorted by modification time (most recent first).
    pub matches: Vec<PathBuf>,
    /// `true` when the result set was truncated at `limit`.
    pub truncated: bool,
}

/// Options controlling a glob search.
///
/// Use [`GlobOptions::new`] for defaults, then override fields as needed.
pub struct GlobOptions<'a> {
    /// Root directory to search from.
    pub root: &'a Path,
    /// Glob pattern in gitglob syntax (e.g. `**/*.rs`, `src/{a,b}/*.toml`).
    pub pattern: &'a str,
    /// Maximum number of results. `0` means unlimited.
    pub limit: usize,
    /// Whether to respect `.gitignore` rules. Default: `true`.
    pub respect_gitignore: bool,
    /// Whether to include hidden files/directories. Default: `false`.
    pub include_hidden: bool,
    /// Optional maximum directory depth (`None` = unlimited).
    pub max_depth: Option<usize>,
}

impl<'a> GlobOptions<'a> {
    /// Create options with sensible defaults.
    #[must_use]
    pub fn new(root: &'a Path, pattern: &'a str) -> Self {
        Self {
            root,
            pattern,
            limit: 0,
            respect_gitignore: true,
            include_hidden: false,
            max_depth: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Find files matching a glob pattern under a directory.
///
/// # Algorithm
///
/// 1. Compile `pattern` with [`globset::GlobBuilder`].
/// 2. Walk `root` with [`ignore::WalkBuilder`], configured per the options
///    (gitignore, hidden files, max depth).
/// 3. Test each entry against the compiled glob.
/// 4. Collect file metadata, sort by modification time (most recent first).
/// 5. Truncate at `limit` and set [`GlobResult::truncated`] accordingly.
///
/// # Errors
///
/// - Invalid glob pattern.
/// - `root` does not exist or is inaccessible.
pub fn find_files(opts: &GlobOptions<'_>) -> crab_common::Result<GlobResult> {
    let _ = opts;
    todo!()
}
