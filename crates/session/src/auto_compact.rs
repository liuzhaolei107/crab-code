//! Reactive auto-compaction based on context usage.
//!
//! Monitors token usage relative to the model's context window and triggers
//! compaction when a configurable threshold is exceeded. After compaction,
//! performs cleanup to re-attach file/skill attachments and reset caches.
//!
//! Maps to CCB `compact/autoCompact.ts` + `reactiveCompact.ts`.

// ─── Configuration ─────────────────────────────────────────────────────

/// Configuration for automatic compaction triggers.
#[derive(Debug, Clone)]
pub struct AutoCompactConfig {
    /// Whether auto-compaction is enabled.
    pub enabled: bool,
    /// Token usage fraction at which to trigger compaction (e.g. 0.85 = 85%).
    pub threshold_percent: f64,
    /// Minimum number of messages in the conversation before compaction is
    /// allowed. Prevents compacting very short conversations.
    pub min_messages_before_compact: usize,
}

impl Default for AutoCompactConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_percent: 0.85,
            min_messages_before_compact: 6,
        }
    }
}

// ─── Trigger ───────────────────────────────────────────────────────────

/// Describes why compaction was triggered.
#[derive(Debug, Clone, PartialEq)]
pub enum CompactTrigger {
    /// Token usage exceeded the configured threshold.
    ThresholdExceeded {
        /// Actual usage percentage at the time of trigger.
        usage_percent: f64,
    },
    /// The user explicitly requested compaction.
    ManualRequest,
    /// Compaction is needed as part of session restoration.
    SessionRestore,
}

// ─── Decision function ─────────────────────────────────────────────────

/// Determine whether auto-compaction should be triggered.
///
/// Returns `Some(CompactTrigger)` if compaction should happen, `None` otherwise.
///
/// # Arguments
///
/// * `total_tokens` — Current total token count of the conversation.
/// * `context_window` — Model's context window size.
/// * `config` — Auto-compaction configuration.
/// * `message_count` — Number of messages in the conversation.
///
/// # Example
///
/// ```
/// use crab_session::auto_compact::{AutoCompactConfig, should_auto_compact, CompactTrigger};
///
/// let config = AutoCompactConfig {
///     enabled: true,
///     threshold_percent: 0.80,
///     min_messages_before_compact: 4,
/// };
/// // 90k out of 100k context window = 90%, should trigger
/// // (but implementation is todo!())
/// ```
pub fn should_auto_compact(
    _total_tokens: usize,
    _context_window: usize,
    _config: &AutoCompactConfig,
    _message_count: usize,
) -> Option<CompactTrigger> {
    todo!("should_auto_compact: check threshold_percent and min_messages_before_compact")
}

/// Post-compact cleanup: re-attach file/skill attachments, reset caches.
///
/// After a compaction pass removes or summarizes old messages, certain
/// context attachments (open files, active skills) need to be re-injected
/// into the conversation so the model retains awareness of them.
pub fn post_compact_cleanup(/* TODO: conversation, context_manager */) {
    todo!("post_compact_cleanup: re-attach file/skill attachments and reset caches")
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = AutoCompactConfig::default();
        assert!(config.enabled);
        assert!((config.threshold_percent - 0.85).abs() < f64::EPSILON);
        assert_eq!(config.min_messages_before_compact, 6);
    }

    #[test]
    fn compact_trigger_equality() {
        let a = CompactTrigger::ManualRequest;
        let b = CompactTrigger::ManualRequest;
        assert_eq!(a, b);

        let c = CompactTrigger::ThresholdExceeded {
            usage_percent: 0.90,
        };
        let d = CompactTrigger::ThresholdExceeded {
            usage_percent: 0.90,
        };
        assert_eq!(c, d);
    }

    #[test]
    fn compact_trigger_debug() {
        let trigger = CompactTrigger::SessionRestore;
        let debug = format!("{trigger:?}");
        assert!(debug.contains("SessionRestore"));
    }
}
