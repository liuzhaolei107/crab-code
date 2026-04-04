//! Diff generation and edit application using `similar`.

/// Result of applying an edit operation.
pub struct EditResult {
    pub old_content: String,
    pub new_content: String,
    pub unified_diff: String,
}

/// Apply an exact string replacement (`old_string` -> `new_string`) within `file_content`.
///
/// Returns the before/after content plus a unified diff.
///
/// # Errors
///
/// Returns an error if `old_string` is not found in `file_content` or is ambiguous.
pub fn apply_edit(
    _file_content: &str,
    _old_string: &str,
    _new_string: &str,
) -> crab_common::Result<EditResult> {
    todo!()
}
