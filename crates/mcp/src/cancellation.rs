//! MCP request cancellation.
//!
//! Provides [`CancellationToken`] for cooperative cancellation checking,
//! [`CancellationRegistry`] for managing cancellations by request ID,
//! and [`CancellationReason`] for describing why a request was cancelled.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Reason a request was cancelled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationReason {
    /// The user explicitly cancelled the request.
    UserRequested,
    /// The request timed out.
    Timeout,
    /// The client disconnected.
    Disconnected,
    /// The request was superseded by a newer request.
    Superseded,
    /// Custom reason with description.
    Other(String),
}

impl std::fmt::Display for CancellationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserRequested => write!(f, "user requested"),
            Self::Timeout => write!(f, "timeout"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Superseded => write!(f, "superseded"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// MCP cancellation notification params (notifications/cancelled).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancellationParams {
    /// The ID of the request to cancel.
    pub request_id: String,
    /// Optional reason for cancellation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A cooperative cancellation token that can be checked by long-running operations.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<Mutex<CancellationInner>>,
}

#[derive(Debug)]
struct CancellationInner {
    cancelled: bool,
    reason: Option<CancellationReason>,
    cancelled_at: Option<Instant>,
}

impl CancellationToken {
    /// Create a new uncancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CancellationInner {
                cancelled: false,
                reason: None,
                cancelled_at: None,
            })),
        }
    }

    /// Cancel this token with a reason.
    pub fn cancel(&self, reason: CancellationReason) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.cancelled {
            inner.cancelled = true;
            inner.reason = Some(reason);
            inner.cancelled_at = Some(Instant::now());
        }
    }

    /// Check if this token has been cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.lock().unwrap().cancelled
    }

    /// Get the cancellation reason, if cancelled.
    #[must_use]
    pub fn reason(&self) -> Option<CancellationReason> {
        self.inner.lock().unwrap().reason.clone()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry managing cancellation tokens by request ID.
#[derive(Debug)]
pub struct CancellationRegistry {
    tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl CancellationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new request and return its cancellation token.
    pub fn register(&self, request_id: impl Into<String>) -> CancellationToken {
        let token = CancellationToken::new();
        self.tokens
            .lock()
            .unwrap()
            .insert(request_id.into(), token.clone());
        token
    }

    /// Cancel a request by ID with a reason. Returns true if the request was found.
    pub fn cancel(&self, request_id: &str, reason: CancellationReason) -> bool {
        let tokens = self.tokens.lock().unwrap();
        if let Some(token) = tokens.get(request_id) {
            token.cancel(reason);
            true
        } else {
            false
        }
    }

    /// Check if a request has been cancelled.
    #[must_use]
    pub fn is_cancelled(&self, request_id: &str) -> bool {
        let tokens = self.tokens.lock().unwrap();
        tokens
            .get(request_id)
            .is_some_and(|t| t.is_cancelled())
    }

    /// Remove a request from the registry (e.g., after completion).
    pub fn remove(&self, request_id: &str) -> Option<CancellationToken> {
        self.tokens.lock().unwrap().remove(request_id)
    }

    /// Number of tracked requests.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.lock().unwrap().len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.lock().unwrap().is_empty()
    }

    /// Remove all cancelled tokens, returning their request IDs.
    pub fn cleanup_cancelled(&self) -> Vec<String> {
        let mut tokens = self.tokens.lock().unwrap();
        let cancelled: Vec<String> = tokens
            .iter()
            .filter(|(_, t)| t.is_cancelled())
            .map(|(id, _)| id.clone())
            .collect();
        for id in &cancelled {
            tokens.remove(id);
        }
        cancelled
    }
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CancellationRegistry {
    fn clone(&self) -> Self {
        Self {
            tokens: Arc::clone(&self.tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_reason_display() {
        assert_eq!(CancellationReason::UserRequested.to_string(), "user requested");
        assert_eq!(CancellationReason::Timeout.to_string(), "timeout");
        assert_eq!(CancellationReason::Disconnected.to_string(), "disconnected");
        assert_eq!(CancellationReason::Superseded.to_string(), "superseded");
        assert_eq!(
            CancellationReason::Other("custom".into()).to_string(),
            "custom"
        );
    }

    #[test]
    fn cancellation_reason_serde() {
        let r = CancellationReason::UserRequested;
        let json = serde_json::to_string(&r).unwrap();
        let back: CancellationReason = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);

        let r2 = CancellationReason::Other("test".into());
        let json2 = serde_json::to_string(&r2).unwrap();
        let back2: CancellationReason = serde_json::from_str(&json2).unwrap();
        assert_eq!(back2, r2);
    }

    #[test]
    fn token_initially_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        assert!(token.reason().is_none());
    }

    #[test]
    fn token_cancel() {
        let token = CancellationToken::new();
        token.cancel(CancellationReason::Timeout);
        assert!(token.is_cancelled());
        assert_eq!(token.reason(), Some(CancellationReason::Timeout));
    }

    #[test]
    fn token_cancel_idempotent() {
        let token = CancellationToken::new();
        token.cancel(CancellationReason::UserRequested);
        token.cancel(CancellationReason::Timeout); // second cancel ignored
        assert_eq!(token.reason(), Some(CancellationReason::UserRequested));
    }

    #[test]
    fn token_clone_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        token.cancel(CancellationReason::Disconnected);
        assert!(clone.is_cancelled());
    }

    #[test]
    fn registry_register_and_cancel() {
        let reg = CancellationRegistry::new();
        let token = reg.register("req-1");
        assert!(!token.is_cancelled());
        assert_eq!(reg.len(), 1);

        assert!(reg.cancel("req-1", CancellationReason::UserRequested));
        assert!(token.is_cancelled());
        assert!(reg.is_cancelled("req-1"));
    }

    #[test]
    fn registry_cancel_unknown_returns_false() {
        let reg = CancellationRegistry::new();
        assert!(!reg.cancel("nonexistent", CancellationReason::Timeout));
    }

    #[test]
    fn registry_remove() {
        let reg = CancellationRegistry::new();
        reg.register("req-1");
        assert_eq!(reg.len(), 1);
        let removed = reg.remove("req-1");
        assert!(removed.is_some());
        assert!(reg.is_empty());
    }

    #[test]
    fn registry_cleanup_cancelled() {
        let reg = CancellationRegistry::new();
        reg.register("req-1");
        reg.register("req-2");
        reg.register("req-3");

        reg.cancel("req-1", CancellationReason::Timeout);
        reg.cancel("req-3", CancellationReason::Superseded);

        let cleaned = reg.cleanup_cancelled();
        assert_eq!(cleaned.len(), 2);
        assert!(cleaned.contains(&"req-1".to_string()));
        assert!(cleaned.contains(&"req-3".to_string()));
        assert_eq!(reg.len(), 1); // only req-2 remains
        assert!(!reg.is_cancelled("req-2"));
    }

    #[test]
    fn registry_is_cancelled_missing() {
        let reg = CancellationRegistry::new();
        assert!(!reg.is_cancelled("nope"));
    }

    #[test]
    fn cancellation_params_serde() {
        let params = CancellationParams {
            request_id: "req-42".into(),
            reason: Some("user cancelled".into()),
        };
        let json = serde_json::to_string(&params).unwrap();
        let back: CancellationParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.request_id, "req-42");
        assert_eq!(back.reason.as_deref(), Some("user cancelled"));
    }

    #[test]
    fn token_default() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn registry_thread_safe() {
        let reg = CancellationRegistry::new();
        // Register tokens from main thread
        for i in 0..20 {
            reg.register(format!("req-{i}"));
        }

        let handles: Vec<_> = (0..4)
            .map(|t| {
                let reg = reg.clone();
                std::thread::spawn(move || {
                    for i in 0..5 {
                        let id = format!("req-{}", t * 5 + i);
                        reg.cancel(&id, CancellationReason::UserRequested);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // All 20 should be cancelled
        for i in 0..20 {
            assert!(reg.is_cancelled(&format!("req-{i}")));
        }
    }
}
