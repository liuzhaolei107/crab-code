//! MCP initialization handshake protocol.
//!
//! Manages the `initialize` → `initialized` flow with timeout handling
//! and retry logic.

use crate::negotiation::ProtocolVersion;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, Instant};

/// State of the handshake protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandshakeState {
    /// Waiting to send or receive `initialize`.
    AwaitingInit,
    /// `initialize` sent, waiting for response.
    Negotiating,
    /// Handshake completed successfully.
    Ready,
    /// Handshake failed.
    Failed,
}

impl fmt::Display for HandshakeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AwaitingInit => write!(f, "awaiting_init"),
            Self::Negotiating => write!(f, "negotiating"),
            Self::Ready => write!(f, "ready"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Information returned after a successful handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResult {
    pub protocol_version: String,
    pub server_name: String,
    pub server_version: String,
    pub capabilities: Vec<String>,
}

/// Configuration for the handshake protocol.
#[derive(Debug, Clone)]
pub struct HandshakeConfig {
    /// Maximum time to wait for handshake to complete.
    pub timeout: Duration,
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial retry delay.
    pub initial_retry_delay: Duration,
    /// Retry delay multiplier.
    pub retry_multiplier: f64,
    /// Maximum retry delay.
    pub max_retry_delay: Duration,
}

impl Default for HandshakeConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            initial_retry_delay: Duration::from_secs(1),
            retry_multiplier: 2.0,
            max_retry_delay: Duration::from_secs(10),
        }
    }
}

/// Error during handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    /// Handshake timed out.
    Timeout,
    /// Server rejected the initialization.
    Rejected(String),
    /// All retry attempts exhausted.
    RetriesExhausted,
    /// Invalid state transition.
    InvalidState(String),
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "handshake timed out"),
            Self::Rejected(msg) => write!(f, "handshake rejected: {msg}"),
            Self::RetriesExhausted => write!(f, "handshake retries exhausted"),
            Self::InvalidState(msg) => write!(f, "invalid handshake state: {msg}"),
        }
    }
}

impl std::error::Error for HandshakeError {}

/// Manages the MCP initialize → initialized handshake flow.
#[derive(Debug)]
pub struct HandshakeProtocol {
    config: HandshakeConfig,
    state: HandshakeState,
    attempt: u32,
    started_at: Option<Instant>,
    next_retry_delay: Duration,
    result: Option<HandshakeResult>,
    last_error: Option<String>,
    protocol_version: ProtocolVersion,
}

impl HandshakeProtocol {
    /// Create a new handshake protocol.
    #[must_use]
    pub fn new(config: HandshakeConfig) -> Self {
        let initial_delay = config.initial_retry_delay;
        Self {
            config,
            state: HandshakeState::AwaitingInit,
            attempt: 0,
            started_at: None,
            next_retry_delay: initial_delay,
            result: None,
            last_error: None,
            protocol_version: ProtocolVersion::CURRENT,
        }
    }

    /// Create with default config.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(HandshakeConfig::default())
    }

    /// Current handshake state.
    #[must_use]
    pub fn state(&self) -> HandshakeState {
        self.state
    }

    /// Current attempt number (0 = not started).
    #[must_use]
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// The protocol version being negotiated.
    #[must_use]
    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    /// Begin the handshake (send initialize). Returns error if already in progress.
    pub fn begin(&mut self) -> Result<(), HandshakeError> {
        match self.state {
            HandshakeState::AwaitingInit | HandshakeState::Failed => {
                self.state = HandshakeState::Negotiating;
                self.attempt += 1;
                self.started_at = Some(Instant::now());
                Ok(())
            }
            HandshakeState::Negotiating => Err(HandshakeError::InvalidState(
                "already negotiating".into(),
            )),
            HandshakeState::Ready => Err(HandshakeError::InvalidState(
                "already completed".into(),
            )),
        }
    }

    /// Mark the handshake as successfully completed.
    pub fn complete(&mut self, result: HandshakeResult) -> Result<(), HandshakeError> {
        if self.state != HandshakeState::Negotiating {
            return Err(HandshakeError::InvalidState(format!(
                "expected Negotiating, got {}",
                self.state
            )));
        }
        self.state = HandshakeState::Ready;
        self.result = Some(result);
        Ok(())
    }

    /// Mark the handshake as failed. Returns the delay before retry,
    /// or a `RetriesExhausted` error if no retries remain.
    pub fn fail(&mut self, reason: impl Into<String>) -> Result<Duration, HandshakeError> {
        let reason = reason.into();
        self.last_error = Some(reason);
        self.state = HandshakeState::Failed;

        if self.attempt >= self.config.max_retries {
            return Err(HandshakeError::RetriesExhausted);
        }

        let delay = self.next_retry_delay;
        let next = Duration::from_secs_f64(delay.as_secs_f64() * self.config.retry_multiplier);
        self.next_retry_delay = if next > self.config.max_retry_delay {
            self.config.max_retry_delay
        } else {
            next
        };
        Ok(delay)
    }

    /// Check if the handshake has timed out.
    #[must_use]
    pub fn is_timed_out(&self) -> bool {
        self.started_at
            .is_some_and(|t| t.elapsed() >= self.config.timeout)
    }

    /// Get the handshake result (only available in Ready state).
    #[must_use]
    pub fn result(&self) -> Option<&HandshakeResult> {
        self.result.as_ref()
    }

    /// Get the last error message.
    #[must_use]
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Reset to initial state for a fresh handshake.
    pub fn reset(&mut self) {
        self.state = HandshakeState::AwaitingInit;
        self.attempt = 0;
        self.started_at = None;
        self.next_retry_delay = self.config.initial_retry_delay;
        self.result = None;
        self.last_error = None;
    }

    /// Whether the handshake is in a terminal state (Ready or exhausted).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.state == HandshakeState::Ready
            || (self.state == HandshakeState::Failed
                && self.attempt >= self.config.max_retries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_state_display() {
        assert_eq!(HandshakeState::AwaitingInit.to_string(), "awaiting_init");
        assert_eq!(HandshakeState::Negotiating.to_string(), "negotiating");
        assert_eq!(HandshakeState::Ready.to_string(), "ready");
        assert_eq!(HandshakeState::Failed.to_string(), "failed");
    }

    #[test]
    fn handshake_state_serde_roundtrip() {
        for state in [
            HandshakeState::AwaitingInit,
            HandshakeState::Negotiating,
            HandshakeState::Ready,
            HandshakeState::Failed,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: HandshakeState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn handshake_initial_state() {
        let h = HandshakeProtocol::with_defaults();
        assert_eq!(h.state(), HandshakeState::AwaitingInit);
        assert_eq!(h.attempt(), 0);
        assert!(h.result().is_none());
        assert!(!h.is_terminal());
    }

    #[test]
    fn handshake_begin_and_complete() {
        let mut h = HandshakeProtocol::with_defaults();
        h.begin().unwrap();
        assert_eq!(h.state(), HandshakeState::Negotiating);
        assert_eq!(h.attempt(), 1);

        h.complete(HandshakeResult {
            protocol_version: "2024.11.5".into(),
            server_name: "test".into(),
            server_version: "1.0.0".into(),
            capabilities: vec!["tools".into()],
        })
        .unwrap();
        assert_eq!(h.state(), HandshakeState::Ready);
        assert!(h.is_terminal());
        assert_eq!(h.result().unwrap().server_name, "test");
    }

    #[test]
    fn handshake_begin_while_negotiating_errors() {
        let mut h = HandshakeProtocol::with_defaults();
        h.begin().unwrap();
        let err = h.begin().unwrap_err();
        assert!(matches!(err, HandshakeError::InvalidState(_)));
    }

    #[test]
    fn handshake_complete_wrong_state_errors() {
        let mut h = HandshakeProtocol::with_defaults();
        let err = h
            .complete(HandshakeResult {
                protocol_version: "1.0.0".into(),
                server_name: "x".into(),
                server_version: "1.0".into(),
                capabilities: vec![],
            })
            .unwrap_err();
        assert!(matches!(err, HandshakeError::InvalidState(_)));
    }

    #[test]
    fn handshake_fail_and_retry() {
        let mut h = HandshakeProtocol::new(HandshakeConfig {
            max_retries: 3,
            initial_retry_delay: Duration::from_secs(1),
            retry_multiplier: 2.0,
            max_retry_delay: Duration::from_secs(10),
            ..Default::default()
        });

        // Attempt 1
        h.begin().unwrap();
        let d1 = h.fail("error 1").unwrap();
        assert_eq!(d1, Duration::from_secs(1));
        assert_eq!(h.state(), HandshakeState::Failed);
        assert_eq!(h.last_error(), Some("error 1"));

        // Attempt 2
        h.begin().unwrap();
        let d2 = h.fail("error 2").unwrap();
        assert_eq!(d2, Duration::from_secs(2));

        // Attempt 3 (last allowed)
        h.begin().unwrap();
        let err = h.fail("error 3").unwrap_err();
        assert_eq!(err, HandshakeError::RetriesExhausted);
        assert!(h.is_terminal());
    }

    #[test]
    fn handshake_timeout_detection() {
        let mut h = HandshakeProtocol::new(HandshakeConfig {
            timeout: Duration::from_millis(1),
            ..Default::default()
        });
        h.begin().unwrap();
        std::thread::sleep(Duration::from_millis(5));
        assert!(h.is_timed_out());
    }

    #[test]
    fn handshake_not_timed_out_before_begin() {
        let h = HandshakeProtocol::with_defaults();
        assert!(!h.is_timed_out());
    }

    #[test]
    fn handshake_reset() {
        let mut h = HandshakeProtocol::with_defaults();
        h.begin().unwrap();
        let _ = h.fail("error");
        h.reset();
        assert_eq!(h.state(), HandshakeState::AwaitingInit);
        assert_eq!(h.attempt(), 0);
        assert!(h.last_error().is_none());
        assert!(h.result().is_none());
    }

    #[test]
    fn handshake_error_display() {
        assert_eq!(HandshakeError::Timeout.to_string(), "handshake timed out");
        assert_eq!(
            HandshakeError::Rejected("bad version".into()).to_string(),
            "handshake rejected: bad version"
        );
        assert_eq!(
            HandshakeError::RetriesExhausted.to_string(),
            "handshake retries exhausted"
        );
    }

    #[test]
    fn handshake_result_serde_roundtrip() {
        let r = HandshakeResult {
            protocol_version: "2024.11.5".into(),
            server_name: "test-server".into(),
            server_version: "1.0.0".into(),
            capabilities: vec!["tools".into(), "resources".into()],
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: HandshakeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_name, "test-server");
        assert_eq!(back.capabilities.len(), 2);
    }

    #[test]
    fn handshake_protocol_version() {
        let h = HandshakeProtocol::with_defaults();
        assert_eq!(h.protocol_version(), ProtocolVersion::CURRENT);
    }

    #[test]
    fn handshake_can_retry_from_failed() {
        let mut h = HandshakeProtocol::with_defaults();
        h.begin().unwrap();
        let _ = h.fail("temp error");
        // Can begin again from Failed state
        h.begin().unwrap();
        assert_eq!(h.state(), HandshakeState::Negotiating);
        assert_eq!(h.attempt(), 2);
    }
}
