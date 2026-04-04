//! Content search using regex pattern matching.
//!
//! Uses [`regex`] for pattern compilation and [`ignore`] for directory
//! traversal that respects `.gitignore` rules. Binary files are silently
//! skipped.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single match from a content search.
#[derive(Debug, Clone)]
pub struct GrepMatch {
    /// File containing the match.
    pub path: PathBuf,
    /// 1-based line number of the match.
    pub line_number: usize,
    /// The matched line content (trailing newline stripped).
    pub line_content: String,
    /// Context lines *before* the match (when `context_lines > 0`).
    pub context_before: Vec<String>,
    /// Context lines *after* the match (when `context_lines > 0`).
    pub context_after: Vec<String>,
}

/// Options controlling a content search.
pub struct GrepOptions {
    /// Regex pattern to search for.
    pub pattern: String,
    /// Root path — may be a single file or a directory.
    pub path: PathBuf,
    /// Enable case-insensitive matching.
    pub case_insensitive: bool,
    /// Optional file-name glob filter (e.g. `"*.rs"`). Only files whose name
    /// matches are searched.
    pub file_glob: Option<String>,
    /// Maximum number of matches to return. `0` means unlimited.
    pub max_results: usize,
    /// Number of context lines to capture before and after each match.
    pub context_lines: usize,
    /// Whether to respect `.gitignore` rules. Default: `true`.
    pub respect_gitignore: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Search file contents by regex pattern.
///
/// # Algorithm
///
/// 1. Compile `pattern` with [`regex::RegexBuilder`] (honouring
///    `case_insensitive`).
/// 2. If `path` is a single file, search it directly.
/// 3. If `path` is a directory, walk it with [`ignore::WalkBuilder`],
///    applying `file_glob` as an additional filter.
/// 4. For each text file, read its contents and match line-by-line.
/// 5. Collect surrounding context lines for every match.
/// 6. Stop collecting once `max_results` is reached.
///
/// Binary files (those containing NUL bytes or failing UTF-8 decode) are
/// silently skipped.
///
/// # Errors
///
/// - Invalid regex pattern.
/// - `path` does not exist or is inaccessible.
pub fn search(opts: &GrepOptions) -> crab_common::Result<Vec<GrepMatch>> {
    let _ = opts;
    todo!()
}

/// Search a single file and return all matches.
///
/// This is an internal helper exposed for unit-testing convenience.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
#[allow(dead_code)]
pub(crate) fn search_file(
    _path: &Path,
    _regex: &regex::Regex,
    _context_lines: usize,
) -> crab_common::Result<Vec<GrepMatch>> {
    todo!()
}
