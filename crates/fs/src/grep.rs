//! Content search using ripgrep internals (grep-regex + grep-searcher).

use std::path::PathBuf;

/// A single grep match.
pub struct GrepMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

/// Options for a content search.
pub struct GrepOptions {
    pub pattern: String,
    pub path: PathBuf,
    pub case_insensitive: bool,
    pub file_glob: Option<String>,
    pub max_results: usize,
    pub context_lines: usize,
}

/// Search file contents under a directory by regex pattern.
///
/// Respects `.gitignore` rules automatically.
///
/// # Errors
///
/// Returns an error if the regex pattern is invalid or the search path is inaccessible.
pub fn search(_opts: &GrepOptions) -> crab_common::Result<Vec<GrepMatch>> {
    todo!()
}
