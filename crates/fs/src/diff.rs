//! Diff generation and edit application using [`similar`].
//!
//! Provides exact-string replacement with ambiguity checking and unified
//! diff output — the building block for the `EditTool`.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Result of applying an edit operation.
#[derive(Debug, Clone)]
pub struct EditResult {
    /// File content *before* the edit.
    pub old_content: String,
    /// File content *after* the edit.
    pub new_content: String,
    /// Unified diff between old and new content.
    pub unified_diff: String,
    /// Number of replacements that were made.
    pub replacements: usize,
}

/// Options controlling an edit operation.
///
/// Use [`EditOptions::new`] for the common single-replacement case.
pub struct EditOptions<'a> {
    /// Full file content to edit.
    pub file_content: &'a str,
    /// Exact string to find.
    pub old_string: &'a str,
    /// Replacement string.
    pub new_string: &'a str,
    /// If `true`, replace **all** occurrences. If `false` (default), return an
    /// error when `old_string` matches more than once.
    pub replace_all: bool,
    /// Optional file-path label for the diff header (display only).
    pub file_label: Option<&'a str>,
}

impl<'a> EditOptions<'a> {
    /// Create options for a single-occurrence replacement.
    #[must_use]
    pub fn new(file_content: &'a str, old_string: &'a str, new_string: &'a str) -> Self {
        Self {
            file_content,
            old_string,
            new_string,
            replace_all: false,
            file_label: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Apply an exact string replacement within file content.
///
/// # Algorithm
///
/// 1. Count occurrences of `old_string` in `file_content`.
/// 2. If **0** — return error *"`old_string` not found in file content"*.
/// 3. If **> 1** and `replace_all` is `false` — return error
///    *"`old_string` matches N times; use `replace_all` or provide more context"*.
/// 4. Perform the replacement(s).
/// 5. Generate a unified diff via [`similar::TextDiff::from_lines`].
/// 6. Return [`EditResult`] with old content, new content, diff, and count.
///
/// # Errors
///
/// - `old_string` not found in `file_content`.
/// - `old_string` matches multiple times and `replace_all` is `false`.
pub fn apply_edit(opts: &EditOptions<'_>) -> crab_common::Result<EditResult> {
    let _ = opts;
    todo!()
}

/// Convenience wrapper matching the original skeleton signature.
///
/// Equivalent to `apply_edit(&EditOptions::new(file_content, old_string, new_string))`.
///
/// # Errors
///
/// Same as [`apply_edit`].
pub fn apply_edit_simple(
    file_content: &str,
    old_string: &str,
    new_string: &str,
) -> crab_common::Result<EditResult> {
    apply_edit(&EditOptions::new(file_content, old_string, new_string))
}

/// Generate a unified diff between two strings without applying any edit.
///
/// Useful for dry-run previews (e.g. `WriteTool` showing what will change).
#[must_use]
pub fn unified_diff(_old: &str, _new: &str, _old_label: &str, _new_label: &str) -> String {
    todo!()
}
