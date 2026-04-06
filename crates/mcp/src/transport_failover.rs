//! Automatic transport failover with primary/fallback switching.
//!
//! Provides [`TransportFailover`] for managing primary/backup transport
//! switching, [`FailoverConfig`] for configuration, and [`FailoverState`]
//! for tracking the current active transport.

use crate::transport_layer::TransportConfig;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// State of the failover system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FailoverState {
    /// Using the primary transport.
    Primary,
    /// Using a fallback transport at the given index.
    Failover { index: usize },
    /// All transports have failed.
    AllFailed,
}

impl std::fmt::Display for FailoverState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::Failover { index } => write!(f, "failover({index})"),
            Self::AllFailed => write!(f, "all_failed"),
        }
    }
}

/// Configuration for transport failover.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailoverConfig {
    /// Primary transport configuration.
    pub primary: TransportConfig,
    /// Fallback transport configurations (tried in order).
    pub fallbacks: Vec<TransportConfig>,
    /// Number of consecutive failures before switching.
    pub switch_threshold: u32,
    /// How often to check if the primary has recovered.
    pub recovery_check_interval_secs: u64,
    /// Maximum number of recovery attempts before giving up.
    pub max_recovery_attempts: u32,
}

impl FailoverConfig {
    #[must_use]
    pub fn new(primary: TransportConfig) -> Self {
        Self {
            primary,
            fallbacks: Vec::new(),
            switch_threshold: 3,
            recovery_check_interval_secs: 30,
            max_recovery_attempts: 10,
        }
    }

    /// Add a fallback transport.
    #[must_use]
    pub fn with_fallback(mut self, config: TransportConfig) -> Self {
        self.fallbacks.push(config);
        self
    }

    /// Set the failure threshold for switching.
    #[must_use]
    pub fn with_switch_threshold(mut self, threshold: u32) -> Self {
        self.switch_threshold = threshold;
        self
    }

    /// Set the recovery check interval.
    #[must_use]
    pub fn with_recovery_interval(mut self, secs: u64) -> Self {
        self.recovery_check_interval_secs = secs;
        self
    }

    /// Total number of transports (primary + fallbacks).
    #[must_use]
    pub fn total_transports(&self) -> usize {
        1 + self.fallbacks.len()
    }
}

/// Manages automatic transport failover and recovery.
#[derive(Debug)]
pub struct TransportFailover {
    config: FailoverConfig,
    state: FailoverState,
    consecutive_failures: u32,
    last_recovery_check: Option<Instant>,
    recovery_attempts: u32,
    switch_count: u64,
}

impl TransportFailover {
    /// Create a new failover manager.
    #[must_use]
    pub fn new(config: FailoverConfig) -> Self {
        Self {
            config,
            state: FailoverState::Primary,
            consecutive_failures: 0,
            last_recovery_check: None,
            recovery_attempts: 0,
            switch_count: 0,
        }
    }

    /// Current failover state.
    #[must_use]
    pub fn state(&self) -> &FailoverState {
        &self.state
    }

    /// Get the active transport config based on current state.
    #[must_use]
    pub fn active_config(&self) -> Option<&TransportConfig> {
        match &self.state {
            FailoverState::Primary => Some(&self.config.primary),
            FailoverState::Failover { index } => self.config.fallbacks.get(*index),
            FailoverState::AllFailed => None,
        }
    }

    /// Record a successful operation (resets failure counter).
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Record a failure. Returns the new state if a switch occurred.
    pub fn record_failure(&mut self) -> Option<FailoverState> {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.config.switch_threshold {
            return self.switch_to_next();
        }
        None
    }

    /// Attempt to switch to the next available transport.
    fn switch_to_next(&mut self) -> Option<FailoverState> {
        self.consecutive_failures = 0;
        let new_state = match &self.state {
            FailoverState::Primary => {
                if self.config.fallbacks.is_empty() {
                    FailoverState::AllFailed
                } else {
                    FailoverState::Failover { index: 0 }
                }
            }
            FailoverState::Failover { index } => {
                let next = index + 1;
                if next < self.config.fallbacks.len() {
                    FailoverState::Failover { index: next }
                } else {
                    FailoverState::AllFailed
                }
            }
            FailoverState::AllFailed => return None,
        };
        self.state = new_state.clone();
        self.switch_count += 1;
        Some(new_state)
    }

    /// Check if it's time to attempt recovery to the primary transport.
    /// Returns true if a recovery check should be performed.
    #[must_use]
    pub fn should_check_recovery(&self) -> bool {
        // Only check recovery when not on primary and not all failed
        if self.state == FailoverState::Primary || self.state == FailoverState::AllFailed {
            return false;
        }
        if self.recovery_attempts >= self.config.max_recovery_attempts {
            return false;
        }
        let interval = Duration::from_secs(self.config.recovery_check_interval_secs);
        self.last_recovery_check
            .is_none_or(|last| last.elapsed() >= interval)
    }

    /// Record a recovery check attempt. Call after checking primary health.
    pub fn record_recovery_check(&mut self) {
        self.last_recovery_check = Some(Instant::now());
        self.recovery_attempts += 1;
    }

    /// Switch back to the primary transport (after successful recovery check).
    pub fn recover_to_primary(&mut self) {
        self.state = FailoverState::Primary;
        self.consecutive_failures = 0;
        self.recovery_attempts = 0;
        self.last_recovery_check = None;
        self.switch_count += 1;
    }

    /// Force switch to all-failed state.
    pub fn mark_all_failed(&mut self) {
        self.state = FailoverState::AllFailed;
    }

    /// Force reset to primary.
    pub fn reset(&mut self) {
        self.state = FailoverState::Primary;
        self.consecutive_failures = 0;
        self.recovery_attempts = 0;
        self.last_recovery_check = None;
    }

    /// Number of consecutive failures on the current transport.
    #[must_use]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Total number of transport switches performed.
    #[must_use]
    pub fn switch_count(&self) -> u64 {
        self.switch_count
    }

    /// Number of recovery attempts made.
    #[must_use]
    pub fn recovery_attempts(&self) -> u32 {
        self.recovery_attempts
    }

    /// Whether on the primary transport.
    #[must_use]
    pub fn is_primary(&self) -> bool {
        self.state == FailoverState::Primary
    }

    /// Whether all transports have failed.
    #[must_use]
    pub fn is_all_failed(&self) -> bool {
        self.state == FailoverState::AllFailed
    }

    /// Access the failover configuration.
    #[must_use]
    pub fn config(&self) -> &FailoverConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport_layer::TransportConfig;

    fn make_config() -> FailoverConfig {
        FailoverConfig::new(TransportConfig::stdio())
            .with_fallback(TransportConfig::sse("http://fallback1"))
            .with_fallback(TransportConfig::websocket("ws://fallback2"))
            .with_switch_threshold(3)
    }

    #[test]
    fn initial_state_is_primary() {
        let fo = TransportFailover::new(make_config());
        assert!(fo.is_primary());
        assert!(fo.active_config().is_some());
    }

    #[test]
    fn success_resets_failures() {
        let mut fo = TransportFailover::new(make_config());
        fo.record_failure();
        fo.record_failure();
        assert_eq!(fo.consecutive_failures(), 2);
        fo.record_success();
        assert_eq!(fo.consecutive_failures(), 0);
    }

    #[test]
    fn switch_on_threshold() {
        let mut fo = TransportFailover::new(make_config());
        assert!(fo.record_failure().is_none());
        assert!(fo.record_failure().is_none());
        let new_state = fo.record_failure(); // 3rd failure triggers switch
        assert_eq!(new_state, Some(FailoverState::Failover { index: 0 }));
        assert_eq!(fo.switch_count(), 1);
    }

    #[test]
    fn cascade_through_fallbacks() {
        let mut fo = TransportFailover::new(make_config());
        // Fail past primary
        for _ in 0..3 {
            fo.record_failure();
        }
        assert_eq!(*fo.state(), FailoverState::Failover { index: 0 });

        // Fail past first fallback
        for _ in 0..3 {
            fo.record_failure();
        }
        assert_eq!(*fo.state(), FailoverState::Failover { index: 1 });

        // Fail past second fallback → all failed
        for _ in 0..3 {
            fo.record_failure();
        }
        assert!(fo.is_all_failed());
        assert!(fo.active_config().is_none());
    }

    #[test]
    fn no_fallbacks_goes_directly_to_all_failed() {
        let config = FailoverConfig::new(TransportConfig::stdio()).with_switch_threshold(2);
        let mut fo = TransportFailover::new(config);
        fo.record_failure();
        fo.record_failure();
        assert!(fo.is_all_failed());
    }

    #[test]
    fn recovery_to_primary() {
        let mut fo = TransportFailover::new(make_config());
        // Switch to fallback
        for _ in 0..3 {
            fo.record_failure();
        }
        assert!(!fo.is_primary());

        fo.recover_to_primary();
        assert!(fo.is_primary());
        assert_eq!(fo.consecutive_failures(), 0);
        assert_eq!(fo.recovery_attempts(), 0);
    }

    #[test]
    fn should_check_recovery() {
        let config = FailoverConfig::new(TransportConfig::stdio())
            .with_fallback(TransportConfig::sse("http://fb"))
            .with_switch_threshold(1)
            .with_recovery_interval(0); // immediate recovery checks for testing

        let mut fo = TransportFailover::new(config);
        // On primary — no recovery needed
        assert!(!fo.should_check_recovery());

        fo.record_failure(); // switch to fallback
        assert!(fo.should_check_recovery());

        fo.record_recovery_check();
        assert_eq!(fo.recovery_attempts(), 1);
    }

    #[test]
    fn max_recovery_attempts() {
        let mut config = FailoverConfig::new(TransportConfig::stdio())
            .with_fallback(TransportConfig::sse("http://fb"))
            .with_switch_threshold(1)
            .with_recovery_interval(0);
        config.max_recovery_attempts = 2;

        let mut fo = TransportFailover::new(config);
        fo.record_failure(); // switch to fallback

        fo.record_recovery_check();
        fo.record_recovery_check();
        // At max attempts, no more recovery checks
        assert!(!fo.should_check_recovery());
    }

    #[test]
    fn reset() {
        let mut fo = TransportFailover::new(make_config());
        for _ in 0..3 {
            fo.record_failure();
        }
        assert!(!fo.is_primary());
        fo.reset();
        assert!(fo.is_primary());
        assert_eq!(fo.consecutive_failures(), 0);
    }

    #[test]
    fn mark_all_failed() {
        let mut fo = TransportFailover::new(make_config());
        fo.mark_all_failed();
        assert!(fo.is_all_failed());
    }

    #[test]
    fn all_failed_record_failure_noop() {
        let mut fo = TransportFailover::new(make_config());
        fo.mark_all_failed();
        assert!(fo.record_failure().is_none());
    }

    #[test]
    fn failover_state_display() {
        assert_eq!(FailoverState::Primary.to_string(), "primary");
        assert_eq!(
            FailoverState::Failover { index: 2 }.to_string(),
            "failover(2)"
        );
        assert_eq!(FailoverState::AllFailed.to_string(), "all_failed");
    }

    #[test]
    fn failover_state_serde() {
        let s = FailoverState::Failover { index: 1 };
        let json = serde_json::to_string(&s).unwrap();
        let back: FailoverState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn failover_config_builder() {
        let config = FailoverConfig::new(TransportConfig::stdio())
            .with_fallback(TransportConfig::sse("http://a"))
            .with_fallback(TransportConfig::websocket("ws://b"))
            .with_switch_threshold(5)
            .with_recovery_interval(60);
        assert_eq!(config.total_transports(), 3);
        assert_eq!(config.switch_threshold, 5);
        assert_eq!(config.recovery_check_interval_secs, 60);
    }

    #[test]
    fn failover_config_serde() {
        let config = make_config();
        let json = serde_json::to_string(&config).unwrap();
        let back: FailoverConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.switch_threshold, 3);
        assert_eq!(back.fallbacks.len(), 2);
    }

    #[test]
    fn active_config_changes_with_state() {
        let mut fo = TransportFailover::new(make_config());
        // Primary should be stdio
        let cfg = fo.active_config().unwrap();
        assert!(matches!(
            cfg.transport_type,
            crate::transport_layer::TransportType::Stdio
        ));

        // Switch to first fallback (SSE)
        for _ in 0..3 {
            fo.record_failure();
        }
        let cfg = fo.active_config().unwrap();
        assert!(matches!(
            cfg.transport_type,
            crate::transport_layer::TransportType::Sse { .. }
        ));
    }

    #[test]
    fn switch_count_tracks_all_switches() {
        let mut fo = TransportFailover::new(make_config());
        assert_eq!(fo.switch_count(), 0);
        // Primary → fallback 0
        for _ in 0..3 {
            fo.record_failure();
        }
        assert_eq!(fo.switch_count(), 1);
        // Recover to primary
        fo.recover_to_primary();
        assert_eq!(fo.switch_count(), 2);
    }
}
