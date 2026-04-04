//! Shared rate limiting and exponential backoff.

use std::time::{Duration, Instant};

/// Tracks rate limit state from API response headers.
pub struct RateLimiter {
    pub remaining_requests: u32,
    pub remaining_tokens: u32,
    pub reset_at: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            remaining_requests: u32::MAX,
            remaining_tokens: u32::MAX,
            reset_at: Instant::now(),
        }
    }

    /// Update state from API response headers.
    pub const fn update(
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
    pub const fn should_wait(&self) -> bool {
        self.remaining_requests == 0 || self.remaining_tokens == 0
    }

    /// Duration to wait before the next request.
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
pub fn backoff_delay(attempt: u32) -> Duration {
    let base = Duration::from_millis(500);
    let max = Duration::from_secs(30);
    let delay = base * 2u32.pow(attempt.min(6));
    delay.min(max)
}

/// Maximum number of retry attempts.
pub const MAX_RETRIES: u32 = 3;
