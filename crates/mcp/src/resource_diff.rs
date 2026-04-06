//! MCP resource incremental diff computation and application.
//!
//! Provides [`compute_diff`] for generating diffs between resource versions,
//! [`apply_diff`] for applying diffs to content, and [`DiffChunk`] as the
//! unit of change.

use serde::{Deserialize, Serialize};

/// A single chunk of a diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffChunk {
    /// Byte offset in the original content where the change starts.
    pub offset: usize,
    /// The original text that was replaced (empty for insertions).
    pub old_text: String,
    /// The new text that replaces `old_text` (empty for deletions).
    pub new_text: String,
}

impl DiffChunk {
    #[must_use]
    pub fn new(offset: usize, old_text: impl Into<String>, new_text: impl Into<String>) -> Self {
        Self {
            offset,
            old_text: old_text.into(),
            new_text: new_text.into(),
        }
    }

    /// Create an insertion chunk (no old text removed).
    #[must_use]
    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self::new(offset, "", text)
    }

    /// Create a deletion chunk (no new text added).
    #[must_use]
    pub fn delete(offset: usize, text: impl Into<String>) -> Self {
        Self::new(offset, text, "")
    }

    /// Whether this chunk is an insertion.
    #[must_use]
    pub fn is_insert(&self) -> bool {
        self.old_text.is_empty() && !self.new_text.is_empty()
    }

    /// Whether this chunk is a deletion.
    #[must_use]
    pub fn is_delete(&self) -> bool {
        !self.old_text.is_empty() && self.new_text.is_empty()
    }

    /// Whether this chunk is a replacement.
    #[must_use]
    pub fn is_replace(&self) -> bool {
        !self.old_text.is_empty() && !self.new_text.is_empty()
    }

    /// Size delta: positive means content grew, negative means it shrank.
    #[must_use]
    pub fn size_delta(&self) -> isize {
        self.new_text.len().cast_signed() - self.old_text.len().cast_signed()
    }
}

/// Format for diff output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffFormat {
    /// Send the full new content.
    Full,
    /// Send only the diff chunks.
    Incremental,
}

/// Result of a diff computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDiff {
    /// The diff chunks.
    pub chunks: Vec<DiffChunk>,
    /// Whether the content changed at all.
    pub has_changes: bool,
    /// Total bytes in old content.
    pub old_size: usize,
    /// Total bytes in new content.
    pub new_size: usize,
}

/// Compute a line-based diff between old and new content.
/// Returns a list of diff chunks representing the changes.
#[must_use]
pub fn compute_diff(old_content: &str, new_content: &str) -> ResourceDiff {
    if old_content == new_content {
        return ResourceDiff {
            chunks: Vec::new(),
            has_changes: false,
            old_size: old_content.len(),
            new_size: new_content.len(),
        };
    }

    // Split preserving line terminators for accurate byte offsets
    let old_lines = split_lines_with_terminators(old_content);
    let new_lines = split_lines_with_terminators(new_content);

    let mut chunks = Vec::new();
    let mut old_idx = 0;
    let mut new_idx = 0;
    let mut byte_offset = 0;

    while old_idx < old_lines.len() && new_idx < new_lines.len() {
        if old_lines[old_idx] == new_lines[new_idx] {
            byte_offset += old_lines[old_idx].len();
            old_idx += 1;
            new_idx += 1;
        } else {
            let old_start = old_idx;
            let new_start = new_idx;

            let (old_end, new_end) = find_resync_raw(&old_lines, &new_lines, old_idx, new_idx);

            let old_text: String = old_lines[old_start..old_end].concat();
            let new_text: String = new_lines[new_start..new_end].concat();

            chunks.push(DiffChunk::new(byte_offset, old_text, new_text));

            for line in &old_lines[old_start..old_end] {
                byte_offset += line.len();
            }
            old_idx = old_end;
            new_idx = new_end;
        }
    }

    // Remaining old lines (deletions)
    if old_idx < old_lines.len() {
        let old_text: String = old_lines[old_idx..].concat();
        chunks.push(DiffChunk::delete(byte_offset, old_text));
    }

    // Remaining new lines (insertions)
    if new_idx < new_lines.len() {
        let new_text: String = new_lines[new_idx..].concat();
        chunks.push(DiffChunk::insert(byte_offset, new_text));
    }

    ResourceDiff {
        chunks,
        has_changes: true,
        old_size: old_content.len(),
        new_size: new_content.len(),
    }
}

/// Split a string into lines, preserving the line terminator on each line.
fn split_lines_with_terminators(s: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        if ch == '\n' {
            lines.push(&s[start..=i]);
            start = i + 1;
        }
    }
    if start < s.len() {
        lines.push(&s[start..]);
    }
    lines
}

/// Find a resync point where old and new lines match again.
fn find_resync_raw(
    old_lines: &[&str],
    new_lines: &[&str],
    old_start: usize,
    new_start: usize,
) -> (usize, usize) {
    let max_look = 10;
    for ahead in 1..=max_look {
        if old_start + ahead < old_lines.len()
            && new_start < new_lines.len()
            && old_lines[old_start + ahead] == new_lines[new_start]
        {
            return (old_start + ahead, new_start);
        }
        if new_start + ahead < new_lines.len()
            && old_start < old_lines.len()
            && new_lines[new_start + ahead] == old_lines[old_start]
        {
            return (old_start, new_start + ahead);
        }
        if old_start + ahead < old_lines.len()
            && new_start + ahead < new_lines.len()
            && old_lines[old_start + ahead] == new_lines[new_start + ahead]
        {
            return (old_start + ahead, new_start + ahead);
        }
    }
    (
        (old_start + 1).min(old_lines.len()),
        (new_start + 1).min(new_lines.len()),
    )
}

/// Apply a diff to the original content, producing the new content.
/// Chunks must be sorted by offset and non-overlapping.
#[must_use]
pub fn apply_diff(content: &str, diff: &ResourceDiff) -> String {
    if !diff.has_changes {
        return content.to_string();
    }

    let mut result = String::with_capacity(diff.new_size);
    let mut pos = 0;
    let bytes = content.as_bytes();

    for chunk in &diff.chunks {
        // Copy unchanged content before this chunk
        if chunk.offset > pos {
            let end = chunk.offset.min(bytes.len());
            result.push_str(&content[pos..end]);
        }
        // Apply the chunk: skip old_text, insert new_text
        result.push_str(&chunk.new_text);
        pos = chunk.offset + chunk.old_text.len();
    }

    // Copy remaining content after last chunk
    if pos < bytes.len() {
        result.push_str(&content[pos..]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_chunk_insert() {
        let c = DiffChunk::insert(10, "hello");
        assert!(c.is_insert());
        assert!(!c.is_delete());
        assert!(!c.is_replace());
        assert_eq!(c.size_delta(), 5);
    }

    #[test]
    fn diff_chunk_delete() {
        let c = DiffChunk::delete(5, "world");
        assert!(c.is_delete());
        assert!(!c.is_insert());
        assert_eq!(c.size_delta(), -5);
    }

    #[test]
    fn diff_chunk_replace() {
        let c = DiffChunk::new(0, "old", "new_longer");
        assert!(c.is_replace());
        assert_eq!(c.size_delta(), 7);
    }

    #[test]
    fn diff_chunk_serde_roundtrip() {
        let c = DiffChunk::new(10, "old", "new");
        let json = serde_json::to_string(&c).unwrap();
        let back: DiffChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn compute_diff_identical() {
        let diff = compute_diff("hello\nworld\n", "hello\nworld\n");
        assert!(!diff.has_changes);
        assert!(diff.chunks.is_empty());
    }

    #[test]
    fn compute_diff_single_line_change() {
        let old = "line1\nline2\nline3";
        let new = "line1\nmodified\nline3";
        let diff = compute_diff(old, new);
        assert!(diff.has_changes);
        assert!(!diff.chunks.is_empty());
        // Apply and verify roundtrip
        let result = apply_diff(old, &diff);
        assert_eq!(result, new);
    }

    #[test]
    fn compute_diff_insertion() {
        let old = "line1\nline3";
        let new = "line1\nline2\nline3";
        let diff = compute_diff(old, new);
        assert!(diff.has_changes);
        let result = apply_diff(old, &diff);
        assert_eq!(result, new);
    }

    #[test]
    fn compute_diff_deletion() {
        let old = "line1\nline2\nline3";
        let new = "line1\nline3";
        let diff = compute_diff(old, new);
        assert!(diff.has_changes);
        let result = apply_diff(old, &diff);
        assert_eq!(result, new);
    }

    #[test]
    fn compute_diff_complete_replacement() {
        let old = "aaa\nbbb\nccc";
        let new = "xxx\nyyy\nzzz";
        let diff = compute_diff(old, new);
        assert!(diff.has_changes);
        let result = apply_diff(old, &diff);
        assert_eq!(result, new);
    }

    #[test]
    fn apply_diff_no_changes() {
        let content = "unchanged";
        let diff = ResourceDiff {
            chunks: Vec::new(),
            has_changes: false,
            old_size: content.len(),
            new_size: content.len(),
        };
        assert_eq!(apply_diff(content, &diff), content);
    }

    #[test]
    fn apply_diff_insert_at_start() {
        let diff = ResourceDiff {
            chunks: vec![DiffChunk::insert(0, "prefix ")],
            has_changes: true,
            old_size: 5,
            new_size: 12,
        };
        assert_eq!(apply_diff("hello", &diff), "prefix hello");
    }

    #[test]
    fn apply_diff_delete_from_middle() {
        let diff = ResourceDiff {
            chunks: vec![DiffChunk::delete(5, " cruel")],
            has_changes: true,
            old_size: 17,
            new_size: 11,
        };
        assert_eq!(apply_diff("hello cruel world", &diff), "hello world");
    }

    #[test]
    fn diff_format_serde() {
        let f = DiffFormat::Incremental;
        let json = serde_json::to_string(&f).unwrap();
        let back: DiffFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(back, f);
    }

    #[test]
    fn resource_diff_serde_roundtrip() {
        let diff = ResourceDiff {
            chunks: vec![DiffChunk::new(0, "a", "b")],
            has_changes: true,
            old_size: 1,
            new_size: 1,
        };
        let json = serde_json::to_string(&diff).unwrap();
        let back: ResourceDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(back.chunks.len(), 1);
        assert!(back.has_changes);
    }

    #[test]
    fn compute_diff_empty_to_content() {
        let diff = compute_diff("", "new content");
        assert!(diff.has_changes);
        let result = apply_diff("", &diff);
        assert_eq!(result, "new content");
    }

    #[test]
    fn compute_diff_content_to_empty() {
        let diff = compute_diff("old content", "");
        assert!(diff.has_changes);
        let result = apply_diff("old content", &diff);
        assert_eq!(result, "");
    }

    #[test]
    fn diff_sizes() {
        let old = "hello";
        let new = "hello world";
        let diff = compute_diff(old, new);
        assert_eq!(diff.old_size, 5);
        assert_eq!(diff.new_size, 11);
    }
}
