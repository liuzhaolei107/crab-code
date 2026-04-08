//! Selectively replace large tool outputs with "[snipped]" markers.
//!
//! Unlike full compaction (which summarizes or truncates the entire conversation),
//! snip compaction targets individual tool results that exceed a character threshold
//! and replaces them with a short marker. This is a fast, non-LLM operation that
//! reduces context usage without altering conversation structure.
//!
//! # Relationship to `micro_compact.rs`
//!
//! `micro_compact` replaces large tool results with LLM-generated summaries.
//! `snip_compact` is a cheaper alternative that simply truncates the output
//! and inserts a "[snipped]" marker — no LLM call required.

use crab_core::message::Message;

// ─── Configuration ─────────────────────────────────────────────────────

/// Configuration for snip compaction.
#[derive(Debug, Clone)]
pub struct SnipConfig {
    /// Maximum character count for a single tool result before it is snipped.
    pub max_result_chars: usize,
    /// Marker string inserted in place of the snipped content.
    /// `{n}` is replaced with the original character count.
    pub snip_marker: String,
}

impl Default for SnipConfig {
    fn default() -> Self {
        Self {
            max_result_chars: 10_000,
            snip_marker: "[output snipped — was {n} chars]".into(),
        }
    }
}

impl SnipConfig {
    /// Format the snip marker, replacing `{n}` with the actual character count.
    #[must_use]
    pub fn format_marker(&self, original_len: usize) -> String {
        self.snip_marker.replace("{n}", &original_len.to_string())
    }
}

// ─── Snip function ─────────────────────────────────────────────────────

/// Scan messages and replace tool results exceeding `config.max_result_chars`
/// with a snip marker.
///
/// Returns the number of tool results that were snipped.
///
/// # Arguments
///
/// * `messages` — Mutable slice of conversation messages to scan.
/// * `config` — Snip configuration (threshold and marker).
pub fn snip_large_outputs(_messages: &mut [Message], _config: &SnipConfig) -> usize {
    todo!(
        "snip_large_outputs: iterate messages, find ToolResult blocks exceeding max_result_chars, replace with snip_marker"
    )
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = SnipConfig::default();
        assert_eq!(config.max_result_chars, 10_000);
        assert!(config.snip_marker.contains("{n}"));
    }

    #[test]
    fn format_marker_replaces_n() {
        let config = SnipConfig::default();
        let marker = config.format_marker(25_000);
        assert!(marker.contains("25000"));
        assert!(!marker.contains("{n}"));
    }

    #[test]
    fn format_marker_custom() {
        let config = SnipConfig {
            max_result_chars: 5_000,
            snip_marker: "SNIPPED({n})".into(),
        };
        assert_eq!(config.format_marker(12_345), "SNIPPED(12345)");
    }
}
