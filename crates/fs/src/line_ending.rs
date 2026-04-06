//! Line ending detection and normalization.
//!
//! Detects the dominant line ending style in text content, provides
//! statistics, and can normalize to a target style.

use serde::{Deserialize, Serialize};

// ── Line ending enum ───────────────────────────────────────────────

/// Line ending style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEnding {
    /// Unix-style: `\n`
    Lf,
    /// Windows-style: `\r\n`
    CrLf,
    /// Old Mac-style: `\r` (without following `\n`)
    Cr,
    /// File contains a mix of styles.
    Mixed,
}

impl LineEnding {
    /// The byte sequence for this line ending.
    ///
    /// Returns `\n` for [`Mixed`](LineEnding::Mixed) as a sensible default.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf | Self::Mixed => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lf => write!(f, "LF"),
            Self::CrLf => write!(f, "CRLF"),
            Self::Cr => write!(f, "CR"),
            Self::Mixed => write!(f, "Mixed"),
        }
    }
}

// ── Statistics ─────────────────────────────────────────────────────

/// Counts of each line ending type found in content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LineEndingStats {
    /// Number of `\n` (not preceded by `\r`).
    pub lf_count: usize,
    /// Number of `\r\n`.
    pub crlf_count: usize,
    /// Number of `\r` (not followed by `\n`).
    pub cr_count: usize,
}

impl LineEndingStats {
    /// Total line endings.
    #[must_use]
    pub fn total(&self) -> usize {
        self.lf_count + self.crlf_count + self.cr_count
    }

    /// Whether no line endings were found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

// ── Public API ─────────────────────────────────────────────────────

/// Count occurrences of each line ending type.
#[must_use]
pub fn count_line_endings(content: &str) -> LineEndingStats {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut lf_count: usize = 0;
    let mut crlf_count: usize = 0;
    let mut cr_count: usize = 0;

    let mut i = 0;
    while i < len {
        if bytes[i] == b'\r' {
            if i + 1 < len && bytes[i + 1] == b'\n' {
                crlf_count += 1;
                i += 2;
            } else {
                cr_count += 1;
                i += 1;
            }
        } else if bytes[i] == b'\n' {
            lf_count += 1;
            i += 1;
        } else {
            i += 1;
        }
    }

    LineEndingStats {
        lf_count,
        crlf_count,
        cr_count,
    }
}

/// Detect the dominant line ending style in content.
///
/// Returns [`LineEnding::Lf`] for content with no line endings.
#[must_use]
pub fn detect_line_ending(content: &str) -> LineEnding {
    let stats = count_line_endings(content);

    if stats.is_empty() {
        return LineEnding::Lf; // sensible default
    }

    let types_present = usize::from(stats.lf_count > 0)
        + usize::from(stats.crlf_count > 0)
        + usize::from(stats.cr_count > 0);

    if types_present > 1 {
        return LineEnding::Mixed;
    }

    if stats.crlf_count > 0 {
        LineEnding::CrLf
    } else if stats.cr_count > 0 {
        LineEnding::Cr
    } else {
        LineEnding::Lf
    }
}

/// Normalize all line endings in `content` to the target style.
#[must_use]
pub fn normalize_line_ending(content: &str, target: LineEnding) -> String {
    let target_str = target.as_str();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len);

    let mut i = 0;
    while i < len {
        if bytes[i] == b'\r' {
            result.push_str(target_str);
            if i + 1 < len && bytes[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
        } else if bytes[i] == b'\n' {
            result.push_str(target_str);
            i += 1;
        } else {
            // SAFETY: we're iterating byte-by-byte through a valid &str,
            // but non-line-ending chars may be multi-byte UTF-8. Copy them
            // faithfully.
            let ch_len = utf8_char_len(bytes[i]);
            let end = (i + ch_len).min(len);
            if let Ok(s) = std::str::from_utf8(&bytes[i..end]) {
                result.push_str(s);
            }
            i = end;
        }
    }

    result
}

/// Detect line ending style from a `.gitattributes` `eol` setting.
///
/// Recognises `lf`, `crlf`, and `native` (maps to platform default).
#[must_use]
pub fn line_ending_from_gitattributes(eol_value: &str) -> Option<LineEnding> {
    match eol_value.trim().to_lowercase().as_str() {
        "lf" => Some(LineEnding::Lf),
        "crlf" => Some(LineEnding::CrLf),
        "native" => Some(platform_line_ending()),
        _ => None,
    }
}

/// Detect line ending style from an `.editorconfig` `end_of_line` setting.
#[must_use]
pub fn line_ending_from_editorconfig(value: &str) -> Option<LineEnding> {
    match value.trim().to_lowercase().as_str() {
        "lf" => Some(LineEnding::Lf),
        "crlf" => Some(LineEnding::CrLf),
        "cr" => Some(LineEnding::Cr),
        _ => None,
    }
}

/// The platform's native line ending.
#[must_use]
pub fn platform_line_ending() -> LineEnding {
    if cfg!(windows) {
        LineEnding::CrLf
    } else {
        LineEnding::Lf
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Length of a UTF-8 character from its first byte.
const fn utf8_char_len(first: u8) -> usize {
    match first {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1, // ASCII, continuation byte, or invalid — treat as single byte
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── count_line_endings ─────────────────────────────────

    #[test]
    fn count_lf_only() {
        let stats = count_line_endings("a\nb\nc\n");
        assert_eq!(stats.lf_count, 3);
        assert_eq!(stats.crlf_count, 0);
        assert_eq!(stats.cr_count, 0);
        assert_eq!(stats.total(), 3);
    }

    #[test]
    fn count_crlf_only() {
        let stats = count_line_endings("a\r\nb\r\n");
        assert_eq!(stats.lf_count, 0);
        assert_eq!(stats.crlf_count, 2);
        assert_eq!(stats.cr_count, 0);
    }

    #[test]
    fn count_cr_only() {
        let stats = count_line_endings("a\rb\r");
        assert_eq!(stats.lf_count, 0);
        assert_eq!(stats.crlf_count, 0);
        assert_eq!(stats.cr_count, 2);
    }

    #[test]
    fn count_mixed() {
        let stats = count_line_endings("a\nb\r\nc\r");
        assert_eq!(stats.lf_count, 1);
        assert_eq!(stats.crlf_count, 1);
        assert_eq!(stats.cr_count, 1);
        assert_eq!(stats.total(), 3);
    }

    #[test]
    fn count_empty() {
        let stats = count_line_endings("");
        assert!(stats.is_empty());
    }

    #[test]
    fn count_no_endings() {
        let stats = count_line_endings("hello");
        assert!(stats.is_empty());
    }

    // ── detect_line_ending ─────────────────────────────────

    #[test]
    fn detect_lf() {
        assert_eq!(detect_line_ending("a\nb\n"), LineEnding::Lf);
    }

    #[test]
    fn detect_crlf() {
        assert_eq!(detect_line_ending("a\r\nb\r\n"), LineEnding::CrLf);
    }

    #[test]
    fn detect_cr() {
        assert_eq!(detect_line_ending("a\rb\r"), LineEnding::Cr);
    }

    #[test]
    fn detect_mixed() {
        assert_eq!(detect_line_ending("a\nb\r\n"), LineEnding::Mixed);
    }

    #[test]
    fn detect_empty_defaults_lf() {
        assert_eq!(detect_line_ending(""), LineEnding::Lf);
    }

    #[test]
    fn detect_no_endings_defaults_lf() {
        assert_eq!(detect_line_ending("hello"), LineEnding::Lf);
    }

    // ── normalize_line_ending ──────────────────────────────

    #[test]
    fn normalize_to_lf() {
        let result = normalize_line_ending("a\r\nb\rc\n", LineEnding::Lf);
        assert_eq!(result, "a\nb\nc\n");
    }

    #[test]
    fn normalize_to_crlf() {
        let result = normalize_line_ending("a\nb\rc\r\n", LineEnding::CrLf);
        assert_eq!(result, "a\r\nb\r\nc\r\n");
    }

    #[test]
    fn normalize_to_cr() {
        let result = normalize_line_ending("a\nb\r\nc\r", LineEnding::Cr);
        assert_eq!(result, "a\rb\rc\r");
    }

    #[test]
    fn normalize_no_change() {
        let result = normalize_line_ending("a\nb\n", LineEnding::Lf);
        assert_eq!(result, "a\nb\n");
    }

    #[test]
    fn normalize_empty() {
        let result = normalize_line_ending("", LineEnding::CrLf);
        assert_eq!(result, "");
    }

    #[test]
    fn normalize_no_endings() {
        let result = normalize_line_ending("hello", LineEnding::CrLf);
        assert_eq!(result, "hello");
    }

    #[test]
    fn normalize_preserves_utf8() {
        let result = normalize_line_ending("caf\u{00E9}\nworld\r\n", LineEnding::Lf);
        assert_eq!(result, "caf\u{00E9}\nworld\n");
    }

    // ── gitattributes / editorconfig ───────────────────────

    #[test]
    fn gitattributes_lf() {
        assert_eq!(line_ending_from_gitattributes("lf"), Some(LineEnding::Lf));
    }

    #[test]
    fn gitattributes_crlf() {
        assert_eq!(
            line_ending_from_gitattributes("crlf"),
            Some(LineEnding::CrLf)
        );
    }

    #[test]
    fn gitattributes_native() {
        let result = line_ending_from_gitattributes("native");
        assert!(result.is_some());
    }

    #[test]
    fn gitattributes_unknown() {
        assert_eq!(line_ending_from_gitattributes("auto"), None);
    }

    #[test]
    fn editorconfig_lf() {
        assert_eq!(line_ending_from_editorconfig("lf"), Some(LineEnding::Lf));
    }

    #[test]
    fn editorconfig_crlf() {
        assert_eq!(
            line_ending_from_editorconfig("crlf"),
            Some(LineEnding::CrLf)
        );
    }

    #[test]
    fn editorconfig_cr() {
        assert_eq!(line_ending_from_editorconfig("cr"), Some(LineEnding::Cr));
    }

    #[test]
    fn editorconfig_unknown() {
        assert_eq!(line_ending_from_editorconfig("auto"), None);
    }

    // ── LineEnding display + as_str ────────────────────────

    #[test]
    fn line_ending_display() {
        assert_eq!(LineEnding::Lf.to_string(), "LF");
        assert_eq!(LineEnding::CrLf.to_string(), "CRLF");
        assert_eq!(LineEnding::Cr.to_string(), "CR");
        assert_eq!(LineEnding::Mixed.to_string(), "Mixed");
    }

    #[test]
    fn line_ending_as_str() {
        assert_eq!(LineEnding::Lf.as_str(), "\n");
        assert_eq!(LineEnding::CrLf.as_str(), "\r\n");
        assert_eq!(LineEnding::Cr.as_str(), "\r");
        assert_eq!(LineEnding::Mixed.as_str(), "\n");
    }

    // ── Serde ──────────────────────────────────────────────

    #[test]
    fn line_ending_serde_roundtrip() {
        let json = serde_json::to_string(&LineEnding::CrLf).unwrap();
        assert_eq!(json, "\"cr_lf\"");
        let back: LineEnding = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LineEnding::CrLf);
    }

    #[test]
    fn stats_serializes() {
        let stats = LineEndingStats {
            lf_count: 5,
            crlf_count: 3,
            cr_count: 1,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("lf_count"));
        assert!(json.contains("crlf_count"));
    }

    // ── platform_line_ending ───────────────────────────────

    #[test]
    fn platform_default_is_valid() {
        let ending = platform_line_ending();
        assert!(ending == LineEnding::Lf || ending == LineEnding::CrLf);
    }
}
