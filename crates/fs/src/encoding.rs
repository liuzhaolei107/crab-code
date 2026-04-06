//! File encoding detection and BOM (Byte Order Mark) handling.
//!
//! Detects text file encoding from byte content using BOM signatures and
//! statistical byte-distribution analysis. Provides utilities to strip or
//! add BOMs.

use serde::{Deserialize, Serialize};

// ── Encoding enum ──────────────────────────────────────────────────

/// Recognized text encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    /// UTF-8 without BOM.
    Utf8,
    /// UTF-8 with BOM (EF BB BF).
    Utf8Bom,
    /// UTF-16 Little Endian (FF FE).
    Utf16Le,
    /// UTF-16 Big Endian (FE FF).
    Utf16Be,
    /// Pure ASCII (all bytes < 128).
    Ascii,
    /// Latin-1 / ISO 8859-1.
    Latin1,
    /// Could not determine encoding.
    Unknown,
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Utf8 => write!(f, "UTF-8"),
            Self::Utf8Bom => write!(f, "UTF-8 with BOM"),
            Self::Utf16Le => write!(f, "UTF-16 LE"),
            Self::Utf16Be => write!(f, "UTF-16 BE"),
            Self::Ascii => write!(f, "ASCII"),
            Self::Latin1 => write!(f, "Latin-1"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// ── BOM policy ─────────────────────────────────────────────────────

/// Policy for handling BOMs when reading/writing files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BomPolicy {
    /// Remove the BOM if present.
    Strip,
    /// Keep the BOM as-is.
    #[default]
    Preserve,
    /// Add a BOM if not already present.
    Add,
}

// ── Detection result ───────────────────────────────────────────────

/// Result of encoding detection.
#[derive(Debug, Clone, Serialize)]
pub struct DetectedEncoding {
    /// The detected encoding.
    pub encoding: Encoding,
    /// Confidence score (0.0 – 1.0).
    pub confidence: f64,
}

// ── BOM constants ──────────────────────────────────────────────────

/// UTF-8 BOM bytes.
pub const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// UTF-16 LE BOM bytes.
pub const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
/// UTF-16 BE BOM bytes.
pub const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];

// ── Public API ─────────────────────────────────────────────────────

/// Detect encoding from raw bytes.
///
/// Checks BOM signatures first, then falls back to statistical analysis.
#[must_use]
pub fn detect_encoding(bytes: &[u8]) -> DetectedEncoding {
    if bytes.is_empty() {
        return DetectedEncoding {
            encoding: Encoding::Utf8,
            confidence: 1.0,
        };
    }

    // Check BOM signatures (most specific first)
    if let Some(enc) = detect_bom(bytes) {
        return enc;
    }

    // Statistical detection
    detect_by_statistics(bytes)
}

/// Detect encoding from BOM bytes only.
///
/// Returns `None` if no BOM is found.
#[must_use]
pub fn detect_bom(bytes: &[u8]) -> Option<DetectedEncoding> {
    if bytes.len() >= 3 && bytes[..3] == *UTF8_BOM {
        return Some(DetectedEncoding {
            encoding: Encoding::Utf8Bom,
            confidence: 1.0,
        });
    }
    if bytes.len() >= 2 && bytes[..2] == *UTF16_BE_BOM {
        return Some(DetectedEncoding {
            encoding: Encoding::Utf16Be,
            confidence: 1.0,
        });
    }
    if bytes.len() >= 2 && bytes[..2] == *UTF16_LE_BOM {
        return Some(DetectedEncoding {
            encoding: Encoding::Utf16Le,
            confidence: 1.0,
        });
    }
    None
}

/// Strip BOM from the beginning of a byte slice.
///
/// Returns the remaining bytes and the encoding indicated by the BOM (if any).
#[must_use]
pub fn strip_bom(bytes: &[u8]) -> (&[u8], Option<Encoding>) {
    if bytes.len() >= 3 && bytes[..3] == *UTF8_BOM {
        return (&bytes[3..], Some(Encoding::Utf8Bom));
    }
    if bytes.len() >= 2 && bytes[..2] == *UTF16_BE_BOM {
        return (&bytes[2..], Some(Encoding::Utf16Be));
    }
    if bytes.len() >= 2 && bytes[..2] == *UTF16_LE_BOM {
        return (&bytes[2..], Some(Encoding::Utf16Le));
    }
    (bytes, None)
}

/// Prepend a BOM for the given encoding.
///
/// Only UTF-8 BOM, UTF-16 LE, and UTF-16 BE have defined BOMs. For other
/// encodings the content is returned unchanged.
#[must_use]
pub fn add_bom(content: &[u8], encoding: Encoding) -> Vec<u8> {
    let bom: &[u8] = match encoding {
        Encoding::Utf8Bom | Encoding::Utf8 => UTF8_BOM,
        Encoding::Utf16Le => UTF16_LE_BOM,
        Encoding::Utf16Be => UTF16_BE_BOM,
        _ => return content.to_vec(),
    };
    let mut out = Vec::with_capacity(bom.len() + content.len());
    out.extend_from_slice(bom);
    out.extend_from_slice(content);
    out
}

/// Apply a [`BomPolicy`] to raw bytes.
#[must_use]
pub fn apply_bom_policy(bytes: &[u8], policy: BomPolicy, encoding: Encoding) -> Vec<u8> {
    match policy {
        BomPolicy::Strip => {
            let (content, _) = strip_bom(bytes);
            content.to_vec()
        }
        BomPolicy::Preserve => bytes.to_vec(),
        BomPolicy::Add => {
            let (content, existing) = strip_bom(bytes);
            if existing.is_some() {
                // Already has a BOM — keep as-is
                bytes.to_vec()
            } else {
                add_bom(content, encoding)
            }
        }
    }
}

// ── Statistical detection ──────────────────────────────────────────

/// Byte-distribution heuristic for encoding detection.
#[allow(clippy::cast_precision_loss)]
fn detect_by_statistics(bytes: &[u8]) -> DetectedEncoding {
    let len = bytes.len();
    let mut null_count: usize = 0;
    let mut high_count: usize = 0; // bytes >= 128
    let mut control_count: usize = 0; // non-text control chars (0x01–0x08, 0x0E–0x1F excl tab/nl/cr)

    for &b in bytes {
        match b {
            0x00 => null_count += 1,
            0x01..=0x08 | 0x0E..=0x1F | 0x7F => control_count += 1,
            0x09 | 0x0A | 0x0D | 0x20..=0x7E => {} // ASCII text
            _ => high_count += 1,                  // 0x80..=0xFF
        }
    }

    // Lots of nulls suggest UTF-16
    if null_count > 0 {
        let null_ratio = null_count as f64 / len as f64;
        if null_ratio > 0.1 {
            // Check alternating null pattern
            return detect_utf16_pattern(bytes);
        }
    }

    // Pure ASCII
    if high_count == 0 && control_count == 0 && null_count == 0 {
        return DetectedEncoding {
            encoding: Encoding::Ascii,
            confidence: 1.0,
        };
    }

    // Try valid UTF-8
    if std::str::from_utf8(bytes).is_ok() {
        let confidence = if high_count > 0 { 0.9 } else { 1.0 };
        return DetectedEncoding {
            encoding: Encoding::Utf8,
            confidence,
        };
    }

    // High bytes present but not valid UTF-8 → likely Latin-1
    if high_count > 0 && control_count as f64 / (len as f64) < 0.05 {
        return DetectedEncoding {
            encoding: Encoding::Latin1,
            confidence: 0.6,
        };
    }

    DetectedEncoding {
        encoding: Encoding::Unknown,
        confidence: 0.0,
    }
}

/// Detect UTF-16 LE vs BE from alternating null byte patterns.
#[allow(clippy::cast_precision_loss)]
fn detect_utf16_pattern(bytes: &[u8]) -> DetectedEncoding {
    if bytes.len() < 2 {
        return DetectedEncoding {
            encoding: Encoding::Unknown,
            confidence: 0.0,
        };
    }

    // Count nulls at even vs odd positions
    let mut even_nulls: usize = 0;
    let mut odd_nulls: usize = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == 0x00 {
            if i % 2 == 0 {
                even_nulls += 1;
            } else {
                odd_nulls += 1;
            }
        }
    }

    let total_pairs = bytes.len() / 2;
    if total_pairs == 0 {
        return DetectedEncoding {
            encoding: Encoding::Unknown,
            confidence: 0.0,
        };
    }

    // UTF-16 LE: high byte (odd positions) often null for ASCII-range chars
    let odd_ratio = odd_nulls as f64 / total_pairs as f64;
    // UTF-16 BE: low byte (even positions) often null
    let even_ratio = even_nulls as f64 / total_pairs as f64;

    if odd_ratio > 0.3 && odd_ratio > even_ratio {
        DetectedEncoding {
            encoding: Encoding::Utf16Le,
            confidence: (odd_ratio * 0.8).min(0.9),
        }
    } else if even_ratio > 0.3 && even_ratio > odd_ratio {
        DetectedEncoding {
            encoding: Encoding::Utf16Be,
            confidence: (even_ratio * 0.8).min(0.9),
        }
    } else {
        DetectedEncoding {
            encoding: Encoding::Unknown,
            confidence: 0.2,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BOM detection ──────────────────────────────────────

    #[test]
    fn detect_utf8_bom() {
        let data = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        let result = detect_encoding(&data);
        assert_eq!(result.encoding, Encoding::Utf8Bom);
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn detect_utf16_le_bom() {
        let data = [0xFF, 0xFE, b'h', 0x00];
        let result = detect_encoding(&data);
        assert_eq!(result.encoding, Encoding::Utf16Le);
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn detect_utf16_be_bom() {
        let data = [0xFE, 0xFF, 0x00, b'h'];
        let result = detect_encoding(&data);
        assert_eq!(result.encoding, Encoding::Utf16Be);
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn detect_bom_none() {
        assert!(detect_bom(b"hello").is_none());
    }

    #[test]
    fn detect_empty() {
        let result = detect_encoding(b"");
        assert_eq!(result.encoding, Encoding::Utf8);
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    // ── Statistical detection ──────────────────────────────

    #[test]
    fn detect_ascii() {
        let result = detect_encoding(b"Hello, world!\n");
        assert_eq!(result.encoding, Encoding::Ascii);
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn detect_utf8_no_bom() {
        // "café" in UTF-8
        let data = "caf\u{00E9}".as_bytes();
        let result = detect_encoding(data);
        assert_eq!(result.encoding, Encoding::Utf8);
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn detect_latin1() {
        // Bytes 0xE9 alone is not valid UTF-8 multi-byte — it's Latin-1 'é'
        let data: &[u8] = &[b'c', b'a', b'f', 0xE9];
        let result = detect_encoding(data);
        assert_eq!(result.encoding, Encoding::Latin1);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn detect_utf16_le_no_bom() {
        // "Hi" in UTF-16 LE without BOM: H=0x48,0x00  i=0x69,0x00
        let data: &[u8] = &[0x48, 0x00, 0x69, 0x00, 0x0A, 0x00];
        let result = detect_encoding(data);
        assert_eq!(result.encoding, Encoding::Utf16Le);
        assert!(result.confidence > 0.2);
    }

    #[test]
    fn detect_utf16_be_no_bom() {
        // "Hi" in UTF-16 BE without BOM: H=0x00,0x48  i=0x00,0x69
        let data: &[u8] = &[0x00, 0x48, 0x00, 0x69, 0x00, 0x0A];
        let result = detect_encoding(data);
        assert_eq!(result.encoding, Encoding::Utf16Be);
        assert!(result.confidence > 0.2);
    }

    // ── strip_bom ──────────────────────────────────────────

    #[test]
    fn strip_utf8_bom() {
        let data = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        let (content, enc) = strip_bom(&data);
        assert_eq!(content, b"hi");
        assert_eq!(enc, Some(Encoding::Utf8Bom));
    }

    #[test]
    fn strip_utf16_le_bom() {
        let data = [0xFF, 0xFE, 0x48, 0x00];
        let (content, enc) = strip_bom(&data);
        assert_eq!(content, &[0x48, 0x00]);
        assert_eq!(enc, Some(Encoding::Utf16Le));
    }

    #[test]
    fn strip_utf16_be_bom() {
        let data = [0xFE, 0xFF, 0x00, 0x48];
        let (content, enc) = strip_bom(&data);
        assert_eq!(content, &[0x00, 0x48]);
        assert_eq!(enc, Some(Encoding::Utf16Be));
    }

    #[test]
    fn strip_no_bom() {
        let data = b"hello";
        let (content, enc) = strip_bom(data);
        assert_eq!(content, b"hello");
        assert_eq!(enc, None);
    }

    // ── add_bom ────────────────────────────────────────────

    #[test]
    fn add_utf8_bom() {
        let result = add_bom(b"hi", Encoding::Utf8Bom);
        assert_eq!(&result[..3], UTF8_BOM);
        assert_eq!(&result[3..], b"hi");
    }

    #[test]
    fn add_utf16_le_bom() {
        let result = add_bom(b"hi", Encoding::Utf16Le);
        assert_eq!(&result[..2], UTF16_LE_BOM);
        assert_eq!(&result[2..], b"hi");
    }

    #[test]
    fn add_utf16_be_bom() {
        let result = add_bom(b"hi", Encoding::Utf16Be);
        assert_eq!(&result[..2], UTF16_BE_BOM);
        assert_eq!(&result[2..], b"hi");
    }

    #[test]
    fn add_bom_ascii_noop() {
        let result = add_bom(b"hi", Encoding::Ascii);
        assert_eq!(result, b"hi");
    }

    #[test]
    fn add_bom_unknown_noop() {
        let result = add_bom(b"hi", Encoding::Unknown);
        assert_eq!(result, b"hi");
    }

    // ── apply_bom_policy ───────────────────────────────────

    #[test]
    fn policy_strip_removes_bom() {
        let data = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        let result = apply_bom_policy(&data, BomPolicy::Strip, Encoding::Utf8);
        assert_eq!(result, b"hi");
    }

    #[test]
    fn policy_strip_no_bom() {
        let result = apply_bom_policy(b"hi", BomPolicy::Strip, Encoding::Utf8);
        assert_eq!(result, b"hi");
    }

    #[test]
    fn policy_preserve_keeps_bom() {
        let data = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        let result = apply_bom_policy(&data, BomPolicy::Preserve, Encoding::Utf8);
        assert_eq!(result, data);
    }

    #[test]
    fn policy_add_inserts_bom() {
        let result = apply_bom_policy(b"hi", BomPolicy::Add, Encoding::Utf8Bom);
        assert_eq!(&result[..3], UTF8_BOM);
        assert_eq!(&result[3..], b"hi");
    }

    #[test]
    fn policy_add_existing_bom_noop() {
        let data = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        let result = apply_bom_policy(&data, BomPolicy::Add, Encoding::Utf8Bom);
        assert_eq!(result, data);
    }

    // ── Encoding display ───────────────────────────────────

    #[test]
    fn encoding_display() {
        assert_eq!(Encoding::Utf8.to_string(), "UTF-8");
        assert_eq!(Encoding::Utf8Bom.to_string(), "UTF-8 with BOM");
        assert_eq!(Encoding::Utf16Le.to_string(), "UTF-16 LE");
        assert_eq!(Encoding::Utf16Be.to_string(), "UTF-16 BE");
        assert_eq!(Encoding::Ascii.to_string(), "ASCII");
        assert_eq!(Encoding::Latin1.to_string(), "Latin-1");
        assert_eq!(Encoding::Unknown.to_string(), "Unknown");
    }

    // ── Serde ──────────────────────────────────────────────

    #[test]
    fn encoding_serde_roundtrip() {
        let json = serde_json::to_string(&Encoding::Utf8Bom).unwrap();
        assert_eq!(json, "\"utf8_bom\"");
        let back: Encoding = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Encoding::Utf8Bom);
    }

    #[test]
    fn bom_policy_serde_roundtrip() {
        let json = serde_json::to_string(&BomPolicy::Strip).unwrap();
        assert_eq!(json, "\"strip\"");
        let back: BomPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back, BomPolicy::Strip);
    }

    #[test]
    fn detected_encoding_serializes() {
        let det = DetectedEncoding {
            encoding: Encoding::Ascii,
            confidence: 0.95,
        };
        let json = serde_json::to_string(&det).unwrap();
        assert!(json.contains("ascii"));
        assert!(json.contains("confidence"));
    }

    #[test]
    fn bom_policy_default() {
        assert_eq!(BomPolicy::default(), BomPolicy::Preserve);
    }
}
