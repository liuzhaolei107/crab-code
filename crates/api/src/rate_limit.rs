//! Shared rate limiting, exponential backoff, and retry logic.

use std::time::{Duration, Instant};

use crate::error::ApiError;

/// Tracks rate limit state from API response headers.
pub struct RateLimiter {
    pub remaining_requests: u32,
    pub remaining_tokens: u32,
    pub reset_at: Instant,
}

impl RateLimiter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            remaining_requests: u32::MAX,
            remaining_tokens: u32::MAX,
            reset_at: Instant::now(),
        }
    }

    /// Update state from API response headers.
    pub fn update(
        &mut self,
        remaining_requests: u32,
        remaining_tokens: u32,
        reset_at: Instant,
    ) {
        self.remaining_requests = remaining_requests;
        self.remaining_tokens = remaining_tokens;
        self.reset_at = reset_at;
    }

    /// Whether we should wait before sending the next request.
    #[must_use]
    pub fn should_wait(&self) -> bool {
        self.remaining_requests == 0 || self.remaining_tokens == 0
    }

    /// Duration to wait before the next request.
    #[must_use]
    pub fn wait_duration(&self) -> Duration {
        if self.should_wait() {
            self.reset_at.saturating_duration_since(Instant::now())
        } else {
            Duration::ZERO
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Exponential backoff delay for retries.
///
/// Base 500ms, multiplied by 2^attempt, capped at 30 seconds.
#[must_use]
pub fn backoff_delay(attempt: u32) -> Duration {
    let base = Duration::from_millis(500);
    let max = Duration::from_secs(30);
    let delay = base.saturating_mul(2u32.saturating_pow(attempt.min(6)));
    delay.min(max)
}

/// Maximum number of retry attempts.
pub const MAX_RETRIES: u32 = 3;

/// Check whether an API error is retryable.
///
/// Retryable conditions:
/// - 429 Too Many Requests (rate limited)
/// - 529 Overloaded (server overloaded)
/// - Connection/timeout errors
#[must_use]
pub fn is_retryable(err: &ApiError) -> bool {
    match err {
        ApiError::RateLimited { .. } | ApiError::Timeout => true,
        ApiError::Api { status, .. } => *status == 429 || *status == 529,
        ApiError::Http(e) => e.is_timeout() || e.is_connect(),
        ApiError::Json(_) | ApiError::Sse(_) | ApiError::Common(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_delay_increases_exponentially() {
        let d0 = backoff_delay(0);
        let d1 = backoff_delay(1);
        let d2 = backoff_delay(2);
        assert_eq!(d0, Duration::from_millis(500));
        assert_eq!(d1, Duration::from_millis(1000));
        assert_eq!(d2, Duration::from_millis(2000));
    }

    #[test]
    fn backoff_delay_capped_at_30s() {
        let d10 = backoff_delay(10);
        assert_eq!(d10, Duration::from_secs(30));
    }

    #[test]
    fn rate_limited_is_retryable() {
        let err = ApiError::RateLimited { retry_after_ms: 1000 };
        assert!(is_retryable(&err));
    }

    #[test]
    fn status_429_is_retryable() {
        let err = ApiError::Api { status: 429, message: "rate limited".into() };
        assert!(is_retryable(&err));
    }

    #[test]
    fn status_529_is_retryable() {
        let err = ApiError::Api { status: 529, message: "overloaded".into() };
        assert!(is_retryable(&err));
    }

    #[test]
    fn status_400_is_not_retryable() {
        let err = ApiError::Api { status: 400, message: "bad request".into() };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn json_error_is_not_retryable() {
        let err = ApiError::Sse("parse error".into());
        assert!(!is_retryable(&err));
    }

    #[test]
    fn rate_limiter_defaults() {
        let rl = RateLimiter::new();
        assert!(!rl.should_wait());
        assert_eq!(rl.wait_duration(), Duration::ZERO);
    }
}
