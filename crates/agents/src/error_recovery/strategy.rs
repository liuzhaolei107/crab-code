use std::time::Duration;

use super::category::ErrorCategory;

// ── Recovery strategy ─────────────────────────────────────────────────

/// Action to take when recovering from an error. Mirrors CCB's minimal set
/// — CCB does not ship a `Fallback` path (retries exhausted → surface error
/// to the user), so crab drops that variant too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry the operation after a delay.
    Retry { delay: Duration, max_attempts: u32 },
    /// Ask the user for guidance (e.g. re-auth, decide whether to keep waiting).
    AskUser { message: String },
    /// Abort the operation — error is not recoverable, or retries exhausted.
    Abort { reason: String },
}

impl std::fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retry {
                delay,
                max_attempts,
            } => {
                write!(
                    f,
                    "retry (delay: {}ms, max: {})",
                    delay.as_millis(),
                    max_attempts
                )
            }
            Self::AskUser { message } => write!(f, "ask user: {message}"),
            Self::Abort { reason } => write!(f, "abort: {reason}"),
        }
    }
}

/// Selects recovery strategies based on error category and context.
#[derive(Debug, Clone)]
pub struct RecoveryStrategy {
    /// Maximum retry attempts for transient errors.
    pub transient_max_retries: u32,
    /// Base delay for transient retries.
    pub transient_base_delay: Duration,
    /// Delay for rate limit retries.
    pub rate_limit_delay: Duration,
    /// Maximum retry attempts for rate limits.
    pub rate_limit_max_retries: u32,
    /// Maximum retry attempts for timeouts.
    pub timeout_max_retries: u32,
    /// Delay for timeout retries.
    pub timeout_delay: Duration,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            transient_max_retries: 3,
            transient_base_delay: Duration::from_secs(1),
            rate_limit_delay: Duration::from_secs(10),
            rate_limit_max_retries: 5,
            timeout_max_retries: 2,
            timeout_delay: Duration::from_secs(5),
        }
    }
}

impl RecoveryStrategy {
    /// Determine the recovery action for a given error category.
    #[must_use]
    pub fn recommend(&self, category: ErrorCategory) -> RecoveryAction {
        match category {
            ErrorCategory::Transient => RecoveryAction::Retry {
                delay: self.transient_base_delay,
                max_attempts: self.transient_max_retries,
            },
            ErrorCategory::RateLimit => RecoveryAction::Retry {
                delay: self.rate_limit_delay,
                max_attempts: self.rate_limit_max_retries,
            },
            ErrorCategory::Timeout => RecoveryAction::Retry {
                delay: self.timeout_delay,
                max_attempts: self.timeout_max_retries,
            },
            ErrorCategory::Auth => RecoveryAction::AskUser {
                message: "Authentication failed. Please check your API key or credentials.".into(),
            },
            ErrorCategory::Permanent | ErrorCategory::Unknown => RecoveryAction::Abort {
                reason: match category {
                    ErrorCategory::Permanent => {
                        "The request is invalid and cannot be retried.".into()
                    }
                    _ => "Unknown error — aborting to surface to the user.".into(),
                },
            },
        }
    }

    /// Determine recovery action with attempt context.
    ///
    /// If the maximum retries for a category are exhausted, escalates to
    /// `AskUser` (rate limit) or `Abort` (transient/timeout).
    #[must_use]
    pub fn recommend_with_attempts(
        &self,
        category: ErrorCategory,
        attempts_so_far: u32,
    ) -> RecoveryAction {
        let base = self.recommend(category);
        match &base {
            RecoveryAction::Retry { max_attempts, .. } if attempts_so_far >= *max_attempts => {
                match category {
                    ErrorCategory::RateLimit => RecoveryAction::AskUser {
                        message: "Rate limit persists after retries. Wait or check quota.".into(),
                    },
                    _ => RecoveryAction::Abort {
                        reason: format!("Retries exhausted after {attempts_so_far} attempts."),
                    },
                }
            }
            _ => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::category::ErrorCategory;
    use super::*;

    #[test]
    fn recovery_action_display() {
        let retry = RecoveryAction::Retry {
            delay: Duration::from_secs(1),
            max_attempts: 3,
        };
        assert!(retry.to_string().contains("retry"));

        let abort = RecoveryAction::Abort {
            reason: "fatal".into(),
        };
        assert!(abort.to_string().contains("abort"));

        let ask = RecoveryAction::AskUser {
            message: "help".into(),
        };
        assert!(ask.to_string().contains("ask user"));
    }

    // ── RecoveryStrategy ───────────────────────────────────────────

    #[test]
    fn strategy_defaults() {
        let s = RecoveryStrategy::default();
        assert_eq!(s.transient_max_retries, 3);
        assert_eq!(s.rate_limit_max_retries, 5);
        assert_eq!(s.timeout_max_retries, 2);
    }

    #[test]
    fn strategy_transient_recommends_retry() {
        let s = RecoveryStrategy::default();
        let action = s.recommend(ErrorCategory::Transient);
        assert!(matches!(action, RecoveryAction::Retry { .. }));
    }

    #[test]
    fn strategy_rate_limit_recommends_retry() {
        let s = RecoveryStrategy::default();
        let action = s.recommend(ErrorCategory::RateLimit);
        if let RecoveryAction::Retry { delay, .. } = action {
            assert_eq!(delay, Duration::from_secs(10));
        } else {
            panic!("Expected Retry");
        }
    }

    #[test]
    fn strategy_auth_recommends_ask_user() {
        let s = RecoveryStrategy::default();
        let action = s.recommend(ErrorCategory::Auth);
        assert!(matches!(action, RecoveryAction::AskUser { .. }));
    }

    #[test]
    fn strategy_permanent_recommends_abort() {
        let s = RecoveryStrategy::default();
        let action = s.recommend(ErrorCategory::Permanent);
        assert!(matches!(action, RecoveryAction::Abort { .. }));
    }

    #[test]
    fn strategy_unknown_recommends_abort() {
        let s = RecoveryStrategy::default();
        let action = s.recommend(ErrorCategory::Unknown);
        assert!(matches!(action, RecoveryAction::Abort { .. }));
    }

    #[test]
    fn strategy_timeout_recommends_retry() {
        let s = RecoveryStrategy::default();
        let action = s.recommend(ErrorCategory::Timeout);
        assert!(matches!(action, RecoveryAction::Retry { .. }));
    }

    #[test]
    fn strategy_with_attempts_escalates_transient_to_abort() {
        let s = RecoveryStrategy::default();
        let action = s.recommend_with_attempts(ErrorCategory::Transient, 1);
        assert!(matches!(action, RecoveryAction::Retry { .. }));
        let action = s.recommend_with_attempts(ErrorCategory::Transient, 3);
        assert!(matches!(action, RecoveryAction::Abort { .. }));
    }

    #[test]
    fn strategy_with_attempts_escalates_rate_limit_to_ask_user() {
        let s = RecoveryStrategy::default();
        let action = s.recommend_with_attempts(ErrorCategory::RateLimit, 5);
        assert!(matches!(action, RecoveryAction::AskUser { .. }));
    }

    #[test]
    fn strategy_with_attempts_escalates_timeout_to_abort() {
        let s = RecoveryStrategy::default();
        let action = s.recommend_with_attempts(ErrorCategory::Timeout, 2);
        assert!(matches!(action, RecoveryAction::Abort { .. }));
    }

    #[test]
    fn strategy_with_attempts_no_escalation_for_permanent() {
        let s = RecoveryStrategy::default();
        let action = s.recommend_with_attempts(ErrorCategory::Permanent, 0);
        assert!(matches!(action, RecoveryAction::Abort { .. }));
    }
}
