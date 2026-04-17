//! Status-bar tip state shared between `crab-agent` (producer) and
//! `crab-tui` (consumer).
//!
//! Tips are short hints surfaced in the status bar when a context
//! trigger fires (e.g. "Pro tip: use `/compact` to shorten the
//! conversation"). Each tip is shown at most once per cooldown window
//! so users aren't spammed with the same suggestion every turn.
//!
//! The tip registry + trigger logic lives in `crab-agent::tips`; this
//! module carries only the shared data shape so the TUI can render and
//! acknowledge without depending on agent.

use serde::{Deserialize, Serialize};

/// Category of a tip — determines icon/colour in the status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TipKind {
    /// Informational hint; lowest priority.
    Info,
    /// Actionable suggestion the user should probably try.
    Action,
    /// Warning about a potential issue (quota, deprecated flag, etc.).
    Warning,
}

/// A single tip definition + its lifecycle state.
///
/// `id` is the stable identifier used for acknowledgement and cooldown
/// tracking. `text` is the message shown to the user. `shown_at` and
/// `cooldown_until` are Unix epoch millis; `None` means "never shown"
/// / "no cooldown".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TipState {
    /// Stable identifier, e.g. `"compact-reminder"`.
    pub id: String,
    /// Human-readable message.
    pub text: String,
    /// Tip category.
    pub kind: TipKind,
    /// When this tip was last shown (Unix epoch millis).
    pub shown_at: Option<i64>,
    /// Earliest future time this tip may be shown again (Unix epoch millis).
    pub cooldown_until: Option<i64>,
    /// Context keys that triggered this tip — used for analytics +
    /// letting the TUI highlight the triggering element if relevant.
    #[serde(default)]
    pub context_keys: Vec<String>,
}

impl TipState {
    /// Create a fresh tip with no prior show history.
    #[must_use]
    pub fn new(id: impl Into<String>, text: impl Into<String>, kind: TipKind) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            kind,
            shown_at: None,
            cooldown_until: None,
            context_keys: Vec::new(),
        }
    }

    /// Is this tip eligible to show given the current wall-clock time?
    ///
    /// Returns `true` if never shown, or if `cooldown_until` is in the past.
    #[must_use]
    pub fn is_eligible(&self, now_millis: i64) -> bool {
        self.cooldown_until.is_none_or(|u| now_millis >= u)
    }

    /// Mark the tip as shown at `now_millis` with the given cooldown.
    ///
    /// `cooldown_millis` is the gap before the tip can be shown again;
    /// pass `0` for "one-shot" tips that should never repeat in this
    /// session.
    pub fn mark_shown(&mut self, now_millis: i64, cooldown_millis: i64) {
        self.shown_at = Some(now_millis);
        self.cooldown_until = Some(now_millis.saturating_add(cooldown_millis));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tip_has_no_history() {
        let t = TipState::new("x", "hello", TipKind::Info);
        assert!(t.shown_at.is_none());
        assert!(t.cooldown_until.is_none());
        assert!(t.context_keys.is_empty());
    }

    #[test]
    fn eligible_when_never_shown() {
        let t = TipState::new("x", "hello", TipKind::Info);
        assert!(t.is_eligible(0));
        assert!(t.is_eligible(i64::MAX));
    }

    #[test]
    fn eligible_after_cooldown() {
        let mut t = TipState::new("x", "hello", TipKind::Info);
        t.mark_shown(1000, 500);
        assert!(!t.is_eligible(1000));
        assert!(!t.is_eligible(1499));
        assert!(t.is_eligible(1500));
        assert!(t.is_eligible(2000));
    }

    #[test]
    fn mark_shown_saturates_on_overflow() {
        let mut t = TipState::new("x", "hello", TipKind::Info);
        t.mark_shown(i64::MAX, 1);
        // saturating_add prevents panic; cooldown clamps at MAX.
        assert_eq!(t.cooldown_until, Some(i64::MAX));
    }

    #[test]
    fn serde_roundtrip() {
        let t = TipState {
            id: "compact".into(),
            text: "Try /compact".into(),
            kind: TipKind::Action,
            shown_at: Some(1_700_000_000_000),
            cooldown_until: Some(1_700_086_400_000),
            context_keys: vec!["high-token-usage".into()],
        };
        let json = serde_json::to_string(&t).unwrap();
        let back: TipState = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn tip_kind_variants() {
        assert_ne!(TipKind::Info, TipKind::Action);
        assert_ne!(TipKind::Action, TipKind::Warning);
    }
}
