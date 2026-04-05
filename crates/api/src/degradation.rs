//! Degradation modes for API resilience.
//!
//! `DegradationMode` represents the current operating mode when the API
//! is experiencing issues. `DegradationPolicy` automatically transitions
//! between modes based on error frequency.

use std::fmt;
use std::time::{Duration, Instant};

use crate::error_classifier::ErrorCategory;

// ---------------------------------------------------------------------------
// DegradationMode
// ---------------------------------------------------------------------------

/// Operating mode for the API layer under degraded conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DegradationMode {
    /// Normal operation — full context, primary model.
    Normal,
    /// Reduced context window to avoid timeouts.
    ReducedContext,
    /// Switch to a cheaper/faster fallback model.
    FallbackModel,
    /// Only serve from cache, no live API calls.
    CachedOnly,
    /// Completely offline — return error immediately.
    Offline,
}

impl fmt::Display for DegradationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::ReducedContext => write!(f, "reduced_context"),
            Self::FallbackModel => write!(f, "fallback_model"),
            Self::CachedOnly => write!(f, "cached_only"),
            Self::Offline => write!(f, "offline"),
        }
    }
}

impl DegradationMode {
    /// Whether the mode allows live API calls.
    #[must_use]
    pub fn allows_api_calls(&self) -> bool {
        matches!(
            self,
            Self::Normal | Self::ReducedContext | Self::FallbackModel
        )
    }

    /// Severity level (higher = more degraded). 0=Normal, 4=Offline.
    #[must_use]
    pub fn severity(&self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::ReducedContext => 1,
            Self::FallbackModel => 2,
            Self::CachedOnly => 3,
            Self::Offline => 4,
        }
    }
}

// ---------------------------------------------------------------------------
// DegradationPolicy
// ---------------------------------------------------------------------------

/// Policy that automatically transitions degradation mode based on error patterns.
#[derive(Debug)]
pub struct DegradationPolicy {
    /// Current mode.
    current_mode: DegradationMode,
    /// Consecutive timeout count.
    consecutive_timeouts: u32,
    /// Consecutive error count (any type).
    consecutive_errors: u32,
    /// Threshold: consecutive timeouts before reducing context.
    timeout_threshold: u32,
    /// Threshold: consecutive errors before fallback model.
    error_threshold: u32,
    /// Threshold: consecutive errors before cached-only mode.
    cached_only_threshold: u32,
    /// Threshold: consecutive errors before offline.
    offline_threshold: u32,
    /// Time of last successful request (for recovery).
    last_success: Option<Instant>,
    /// How long to stay in degraded mode before attempting recovery.
    recovery_window: Duration,
}

impl Default for DegradationPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl DegradationPolicy {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_mode: DegradationMode::Normal,
            consecutive_timeouts: 0,
            consecutive_errors: 0,
            timeout_threshold: 3,
            error_threshold: 5,
            cached_only_threshold: 10,
            offline_threshold: 20,
            last_success: None,
            recovery_window: Duration::from_secs(60),
        }
    }

    /// Set timeout threshold.
    #[must_use]
    pub fn with_timeout_threshold(mut self, threshold: u32) -> Self {
        self.timeout_threshold = threshold;
        self
    }

    /// Set error threshold.
    #[must_use]
    pub fn with_error_threshold(mut self, threshold: u32) -> Self {
        self.error_threshold = threshold;
        self
    }

    /// Set recovery window.
    #[must_use]
    pub fn with_recovery_window(mut self, window: Duration) -> Self {
        self.recovery_window = window;
        self
    }

    /// Current degradation mode.
    #[must_use]
    pub fn mode(&self) -> DegradationMode {
        self.current_mode
    }

    /// Record a successful request — resets error counters and may recover.
    pub fn record_success(&mut self) {
        self.consecutive_timeouts = 0;
        self.consecutive_errors = 0;
        self.last_success = Some(Instant::now());

        // Recover one step at a time.
        self.current_mode = match self.current_mode {
            DegradationMode::Offline => DegradationMode::CachedOnly,
            DegradationMode::CachedOnly => DegradationMode::FallbackModel,
            DegradationMode::FallbackModel => DegradationMode::ReducedContext,
            DegradationMode::ReducedContext | DegradationMode::Normal => DegradationMode::Normal,
        };
    }

    /// Record a failed request and update the degradation mode.
    pub fn record_failure(&mut self, category: ErrorCategory) {
        self.consecutive_errors += 1;

        if category == ErrorCategory::Timeout {
            self.consecutive_timeouts += 1;
        }

        self.current_mode = self.compute_mode();
    }

    /// Attempt recovery if enough time has passed since last success.
    pub fn attempt_recovery(&mut self) {
        if self.current_mode == DegradationMode::Normal {
            return;
        }

        // Recover if: no last success recorded, or last success was recent
        // (within recovery window). A zero recovery window means always recover.
        let should_recover = self
            .last_success
            .is_none_or(|t| self.recovery_window.is_zero() || t.elapsed() < self.recovery_window);

        if should_recover {
            // Step down one severity level.
            self.current_mode = match self.current_mode {
                DegradationMode::Offline => DegradationMode::CachedOnly,
                DegradationMode::CachedOnly => DegradationMode::FallbackModel,
                DegradationMode::FallbackModel => DegradationMode::ReducedContext,
                DegradationMode::ReducedContext | DegradationMode::Normal => {
                    DegradationMode::Normal
                }
            };
            // Reset counters on recovery attempt.
            self.consecutive_errors = 0;
            self.consecutive_timeouts = 0;
        }
    }

    /// Reset to normal mode.
    pub fn reset(&mut self) {
        self.current_mode = DegradationMode::Normal;
        self.consecutive_timeouts = 0;
        self.consecutive_errors = 0;
    }

    fn compute_mode(&self) -> DegradationMode {
        if self.consecutive_errors >= self.offline_threshold {
            DegradationMode::Offline
        } else if self.consecutive_errors >= self.cached_only_threshold {
            DegradationMode::CachedOnly
        } else if self.consecutive_errors >= self.error_threshold {
            DegradationMode::FallbackModel
        } else if self.consecutive_timeouts >= self.timeout_threshold {
            DegradationMode::ReducedContext
        } else {
            // Don't downgrade — keep current if already degraded.
            self.current_mode
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- DegradationMode --

    #[test]
    fn mode_display() {
        assert_eq!(DegradationMode::Normal.to_string(), "normal");
        assert_eq!(DegradationMode::CachedOnly.to_string(), "cached_only");
        assert_eq!(DegradationMode::Offline.to_string(), "offline");
    }

    #[test]
    fn mode_allows_api_calls() {
        assert!(DegradationMode::Normal.allows_api_calls());
        assert!(DegradationMode::ReducedContext.allows_api_calls());
        assert!(DegradationMode::FallbackModel.allows_api_calls());
        assert!(!DegradationMode::CachedOnly.allows_api_calls());
        assert!(!DegradationMode::Offline.allows_api_calls());
    }

    #[test]
    fn mode_severity_ordering() {
        assert!(DegradationMode::Normal.severity() < DegradationMode::ReducedContext.severity());
        assert!(
            DegradationMode::ReducedContext.severity() < DegradationMode::FallbackModel.severity()
        );
        assert!(DegradationMode::FallbackModel.severity() < DegradationMode::CachedOnly.severity());
        assert!(DegradationMode::CachedOnly.severity() < DegradationMode::Offline.severity());
    }

    // -- DegradationPolicy --

    #[test]
    fn policy_starts_normal() {
        let policy = DegradationPolicy::new();
        assert_eq!(policy.mode(), DegradationMode::Normal);
    }

    #[test]
    fn policy_timeout_triggers_reduced_context() {
        let mut policy = DegradationPolicy::new().with_timeout_threshold(3);
        for _ in 0..3 {
            policy.record_failure(ErrorCategory::Timeout);
        }
        assert_eq!(policy.mode(), DegradationMode::ReducedContext);
    }

    #[test]
    fn policy_errors_trigger_fallback_model() {
        let mut policy = DegradationPolicy::new().with_error_threshold(5);
        for _ in 0..5 {
            policy.record_failure(ErrorCategory::Transient);
        }
        assert_eq!(policy.mode(), DegradationMode::FallbackModel);
    }

    #[test]
    fn policy_many_errors_go_cached_only() {
        let mut policy = DegradationPolicy::new();
        for _ in 0..10 {
            policy.record_failure(ErrorCategory::Transient);
        }
        assert_eq!(policy.mode(), DegradationMode::CachedOnly);
    }

    #[test]
    fn policy_extreme_errors_go_offline() {
        let mut policy = DegradationPolicy::new();
        for _ in 0..20 {
            policy.record_failure(ErrorCategory::Transient);
        }
        assert_eq!(policy.mode(), DegradationMode::Offline);
    }

    #[test]
    fn policy_success_recovers_one_step() {
        let mut policy = DegradationPolicy::new();
        for _ in 0..20 {
            policy.record_failure(ErrorCategory::Transient);
        }
        assert_eq!(policy.mode(), DegradationMode::Offline);

        policy.record_success();
        assert_eq!(policy.mode(), DegradationMode::CachedOnly);

        policy.record_success();
        assert_eq!(policy.mode(), DegradationMode::FallbackModel);

        policy.record_success();
        assert_eq!(policy.mode(), DegradationMode::ReducedContext);

        policy.record_success();
        assert_eq!(policy.mode(), DegradationMode::Normal);
    }

    #[test]
    fn policy_reset() {
        let mut policy = DegradationPolicy::new();
        for _ in 0..20 {
            policy.record_failure(ErrorCategory::Transient);
        }
        policy.reset();
        assert_eq!(policy.mode(), DegradationMode::Normal);
    }

    #[test]
    fn policy_attempt_recovery() {
        let mut policy = DegradationPolicy::new().with_recovery_window(Duration::from_secs(0));
        for _ in 0..10 {
            policy.record_failure(ErrorCategory::Transient);
        }
        assert_eq!(policy.mode(), DegradationMode::CachedOnly);

        // Record a recent success so recovery window check passes.
        policy.last_success = Some(Instant::now());
        policy.attempt_recovery();
        assert_eq!(policy.mode(), DegradationMode::FallbackModel);
    }
}
