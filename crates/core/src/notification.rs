//! Cross-crate notification payload used by the status bar, the bridge
//! server (to forward to remote clients), and the telemetry recorder.
//!
//! Kept deliberately minimal — consumers decide how to render / route
//! based on [`NotificationKind`]. The `id` is for deduping and for
//! "mark as read" style acknowledgement.

use serde::{Deserialize, Serialize};

/// Severity / intent of a notification. Determines icon + colour +
/// audible cue in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationKind {
    /// Informational — low-priority status update.
    Info,
    /// Positive outcome — a task completed.
    Success,
    /// Warning — something needs attention but isn't fatal.
    Warning,
    /// Error — a failure the user should know about.
    Error,
}

/// A user-facing notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    /// Stable identifier for dedup / acknowledgement.
    pub id: String,
    /// Short title (≤ ~60 chars, fits a status-bar slot).
    pub title: String,
    /// Optional longer body for a modal or expanded tooltip.
    #[serde(default)]
    pub body: Option<String>,
    /// Severity / intent.
    pub kind: NotificationKind,
    /// Creation timestamp (Unix epoch millis).
    pub created_at: i64,
}

impl Notification {
    /// Convenience constructor. `created_at` is stamped from
    /// `SystemTime::now()` — callers needing deterministic timestamps
    /// should construct the struct directly.
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>, kind: NotificationKind) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            kind,
            created_at,
        }
    }

    /// Attach a body to an existing notification (builder style).
    #[must_use]
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stamps_recent_timestamp() {
        let n = Notification::new("id1", "hello", NotificationKind::Info);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        assert!((now - n.created_at).abs() < 5000);
        assert!(n.body.is_none());
    }

    #[test]
    fn with_body_chains() {
        let n =
            Notification::new("id", "t", NotificationKind::Warning).with_body("more details here");
        assert_eq!(n.body.as_deref(), Some("more details here"));
    }

    #[test]
    fn serde_roundtrip_full() {
        let n = Notification {
            id: "id".into(),
            title: "t".into(),
            body: Some("b".into()),
            kind: NotificationKind::Error,
            created_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&n).unwrap();
        let back: Notification = serde_json::from_str(&json).unwrap();
        assert_eq!(n, back);
    }

    #[test]
    fn kind_variants_distinguishable() {
        assert_ne!(NotificationKind::Info, NotificationKind::Success);
        assert_ne!(NotificationKind::Warning, NotificationKind::Error);
    }
}
