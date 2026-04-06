//! Provider health tracking.
//!
//! `ProviderHealth` monitors per-provider availability by recording
//! successes and failures. `ProviderMetrics` stores latency percentiles
//! and error rates. `HealthProbe` manages periodic liveness checks.

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// ProviderStatus
// ---------------------------------------------------------------------------

/// Health status of a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderStatus {
    /// Provider is operating normally.
    Healthy,
    /// Provider is responding but with elevated errors or latency.
    Degraded,
    /// Provider is unreachable or returning persistent errors.
    Down,
    /// No data yet.
    Unknown,
}

impl fmt::Display for ProviderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Down => write!(f, "down"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// ProviderMetrics
// ---------------------------------------------------------------------------

/// Aggregated metrics for a single provider.
#[derive(Debug, Clone)]
pub struct ProviderMetrics {
    /// Approximate 50th percentile latency.
    pub latency_p50: Duration,
    /// Approximate 99th percentile latency.
    pub latency_p99: Duration,
    /// Error rate (0.0–1.0).
    pub error_rate: f64,
    /// Uptime ratio (0.0–1.0) over the observation window.
    pub uptime: f64,
    /// Total number of requests observed.
    pub total_requests: u32,
}

// ---------------------------------------------------------------------------
// LatencyTracker (internal)
// ---------------------------------------------------------------------------

/// Fixed-size ring buffer for latency samples.
#[derive(Debug)]
struct LatencyTracker {
    samples: Vec<Duration>,
    capacity: usize,
    next_idx: usize,
    filled: bool,
}

impl LatencyTracker {
    fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            capacity,
            next_idx: 0,
            filled: false,
        }
    }

    fn record(&mut self, latency: Duration) {
        if self.samples.len() < self.capacity {
            self.samples.push(latency);
        } else {
            self.samples[self.next_idx] = latency;
        }
        self.next_idx = (self.next_idx + 1) % self.capacity;
        if self.next_idx == 0 && self.samples.len() == self.capacity {
            self.filled = true;
        }
    }

    fn percentile(&self, pct: f64) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted: Vec<Duration> = self.samples.clone();
        sorted.sort();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let idx = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
        let idx = idx.min(sorted.len() - 1);
        sorted[idx]
    }
}

// ---------------------------------------------------------------------------
// ProviderRecord (internal)
// ---------------------------------------------------------------------------

/// Internal tracking state for a single provider.
#[derive(Debug)]
struct ProviderRecord {
    status: ProviderStatus,
    total_requests: u32,
    total_errors: u32,
    consecutive_errors: u32,
    latencies: LatencyTracker,
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
    /// Thresholds
    degraded_error_rate: f64,
    down_consecutive_errors: u32,
}

impl ProviderRecord {
    fn new() -> Self {
        Self {
            status: ProviderStatus::Unknown,
            total_requests: 0,
            total_errors: 0,
            consecutive_errors: 0,
            latencies: LatencyTracker::new(100),
            last_success: None,
            last_failure: None,
            degraded_error_rate: 0.1,
            down_consecutive_errors: 5,
        }
    }

    fn record_success(&mut self, latency: Duration) {
        self.total_requests += 1;
        self.consecutive_errors = 0;
        self.latencies.record(latency);
        self.last_success = Some(Instant::now());
        self.recompute_status();
    }

    fn record_failure(&mut self) {
        self.total_requests += 1;
        self.total_errors += 1;
        self.consecutive_errors += 1;
        self.last_failure = Some(Instant::now());
        self.recompute_status();
    }

    fn recompute_status(&mut self) {
        if self.consecutive_errors >= self.down_consecutive_errors {
            self.status = ProviderStatus::Down;
        } else if self.total_requests > 0 {
            let error_rate = f64::from(self.total_errors) / f64::from(self.total_requests);
            if error_rate >= self.degraded_error_rate {
                self.status = ProviderStatus::Degraded;
            } else {
                self.status = ProviderStatus::Healthy;
            }
        }
    }

    fn metrics(&self) -> ProviderMetrics {
        let error_rate = if self.total_requests > 0 {
            f64::from(self.total_errors) / f64::from(self.total_requests)
        } else {
            0.0
        };

        let uptime = if self.total_requests > 0 {
            f64::from(self.total_requests - self.total_errors) / f64::from(self.total_requests)
        } else {
            0.0
        };

        ProviderMetrics {
            latency_p50: self.latencies.percentile(50.0),
            latency_p99: self.latencies.percentile(99.0),
            error_rate,
            uptime,
            total_requests: self.total_requests,
        }
    }
}

// ---------------------------------------------------------------------------
// ProviderHealth
// ---------------------------------------------------------------------------

/// Tracks health state for multiple LLM providers.
#[derive(Debug)]
pub struct ProviderHealth {
    providers: HashMap<String, ProviderRecord>,
    /// Error rate threshold for Degraded status.
    degraded_threshold: f64,
    /// Consecutive errors before Down status.
    down_threshold: u32,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderHealth {
    /// Create with default thresholds (10% error rate = degraded, 5 consecutive = down).
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            degraded_threshold: 0.1,
            down_threshold: 5,
        }
    }

    /// Set the error rate threshold for degraded status.
    #[must_use]
    pub fn with_degraded_threshold(mut self, threshold: f64) -> Self {
        self.degraded_threshold = threshold;
        self
    }

    /// Set the consecutive error count threshold for down status.
    #[must_use]
    pub fn with_down_threshold(mut self, threshold: u32) -> Self {
        self.down_threshold = threshold;
        self
    }

    /// Record a successful request.
    pub fn record_success(&mut self, provider: &str, latency: Duration) {
        let record = self.get_or_create(provider);
        record.record_success(latency);
    }

    /// Record a failed request.
    pub fn record_failure(&mut self, provider: &str) {
        let record = self.get_or_create(provider);
        record.record_failure();
    }

    /// Get the current status of a provider.
    #[must_use]
    pub fn status(&self, provider: &str) -> ProviderStatus {
        self.providers
            .get(provider)
            .map_or(ProviderStatus::Unknown, |r| r.status)
    }

    /// Get metrics for a provider.
    #[must_use]
    pub fn metrics(&self, provider: &str) -> Option<ProviderMetrics> {
        self.providers.get(provider).map(ProviderRecord::metrics)
    }

    /// List all tracked providers and their status.
    #[must_use]
    pub fn all_statuses(&self) -> Vec<(&str, ProviderStatus)> {
        self.providers
            .iter()
            .map(|(name, r)| (name.as_str(), r.status))
            .collect()
    }

    /// Get providers that are healthy or degraded (available for routing).
    #[must_use]
    pub fn available_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|(_, r)| matches!(r.status, ProviderStatus::Healthy | ProviderStatus::Degraded))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Reset all tracking data.
    pub fn reset(&mut self) {
        self.providers.clear();
    }

    fn get_or_create(&mut self, provider: &str) -> &mut ProviderRecord {
        let degraded = self.degraded_threshold;
        let down = self.down_threshold;
        self.providers
            .entry(provider.to_string())
            .or_insert_with(|| {
                let mut r = ProviderRecord::new();
                r.degraded_error_rate = degraded;
                r.down_consecutive_errors = down;
                r
            })
    }
}

// ---------------------------------------------------------------------------
// HealthProbe
// ---------------------------------------------------------------------------

/// Manages periodic health probes for providers.
#[derive(Debug)]
pub struct HealthProbe {
    /// Probe interval.
    interval: Duration,
    /// Last probe time per provider.
    last_probe: HashMap<String, Instant>,
}

impl HealthProbe {
    /// Create a probe with the given interval.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_probe: HashMap::new(),
        }
    }

    /// Whether the given provider should be probed now.
    #[must_use]
    pub fn should_probe(&self, provider: &str) -> bool {
        self.last_probe
            .get(provider)
            .is_none_or(|last| last.elapsed() >= self.interval)
    }

    /// Record that a probe was sent.
    pub fn record_probe(&mut self, provider: &str) {
        self.last_probe.insert(provider.to_string(), Instant::now());
    }

    /// List providers that are due for probing.
    #[must_use]
    pub fn due_providers(&self, all_providers: &[&str]) -> Vec<String> {
        all_providers
            .iter()
            .filter(|p| self.should_probe(p))
            .map(|p| (*p).to_string())
            .collect()
    }

    /// Probe interval.
    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display() {
        assert_eq!(ProviderStatus::Healthy.to_string(), "healthy");
        assert_eq!(ProviderStatus::Down.to_string(), "down");
        assert_eq!(ProviderStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn starts_unknown() {
        let health = ProviderHealth::new();
        assert_eq!(health.status("anthropic"), ProviderStatus::Unknown);
    }

    #[test]
    fn success_makes_healthy() {
        let mut health = ProviderHealth::new();
        health.record_success("anthropic", Duration::from_millis(100));
        assert_eq!(health.status("anthropic"), ProviderStatus::Healthy);
    }

    #[test]
    fn high_error_rate_makes_degraded() {
        let mut health = ProviderHealth::new().with_degraded_threshold(0.5);
        // 1 success, 1 failure = 50% error rate
        health.record_success("openai", Duration::from_millis(100));
        health.record_failure("openai");
        assert_eq!(health.status("openai"), ProviderStatus::Degraded);
    }

    #[test]
    fn consecutive_errors_make_down() {
        let mut health = ProviderHealth::new().with_down_threshold(3);
        for _ in 0..3 {
            health.record_failure("anthropic");
        }
        assert_eq!(health.status("anthropic"), ProviderStatus::Down);
    }

    #[test]
    fn success_after_errors_recovers() {
        let mut health = ProviderHealth::new().with_down_threshold(3);
        for _ in 0..3 {
            health.record_failure("anthropic");
        }
        assert_eq!(health.status("anthropic"), ProviderStatus::Down);

        // Many successes to dilute error rate
        for _ in 0..30 {
            health.record_success("anthropic", Duration::from_millis(50));
        }
        assert_eq!(health.status("anthropic"), ProviderStatus::Healthy);
    }

    #[test]
    fn metrics_calculation() {
        let mut health = ProviderHealth::new();
        health.record_success("anthropic", Duration::from_millis(100));
        health.record_success("anthropic", Duration::from_millis(200));
        health.record_success("anthropic", Duration::from_millis(300));
        health.record_failure("anthropic");

        let metrics = health.metrics("anthropic").unwrap();
        assert_eq!(metrics.total_requests, 4);
        assert!((metrics.error_rate - 0.25).abs() < 0.01);
        assert!((metrics.uptime - 0.75).abs() < 0.01);
        assert!(metrics.latency_p50 > Duration::ZERO);
    }

    #[test]
    fn available_providers_excludes_down() {
        let mut health = ProviderHealth::new().with_down_threshold(2);
        health.record_success("anthropic", Duration::from_millis(100));
        health.record_failure("openai");
        health.record_failure("openai");

        let available = health.available_providers();
        assert!(available.contains(&"anthropic"));
        assert!(!available.contains(&"openai"));
    }

    #[test]
    fn all_statuses() {
        let mut health = ProviderHealth::new();
        health.record_success("anthropic", Duration::from_millis(50));
        health.record_success("openai", Duration::from_millis(100));

        let statuses = health.all_statuses();
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn reset_clears_all() {
        let mut health = ProviderHealth::new();
        health.record_success("anthropic", Duration::from_millis(50));
        health.reset();
        assert_eq!(health.status("anthropic"), ProviderStatus::Unknown);
        assert!(health.all_statuses().is_empty());
    }

    #[test]
    fn unknown_provider_metrics_none() {
        let health = ProviderHealth::new();
        assert!(health.metrics("unknown").is_none());
    }

    #[test]
    fn health_probe_initial_should_probe() {
        let probe = HealthProbe::new(Duration::from_secs(30));
        assert!(probe.should_probe("anthropic"));
    }

    #[test]
    fn health_probe_after_record() {
        let mut probe = HealthProbe::new(Duration::from_secs(30));
        probe.record_probe("anthropic");
        // Just probed — should not probe again yet
        assert!(!probe.should_probe("anthropic"));
        // Different provider — should probe
        assert!(probe.should_probe("openai"));
    }

    #[test]
    fn health_probe_due_providers() {
        let probe = HealthProbe::new(Duration::from_secs(30));
        let due = probe.due_providers(&["anthropic", "openai"]);
        assert_eq!(due.len(), 2);
    }

    #[test]
    fn latency_tracker_percentiles() {
        let mut tracker = LatencyTracker::new(10);
        for i in 1..=10 {
            tracker.record(Duration::from_millis(i * 10));
        }
        let p50 = tracker.percentile(50.0);
        assert!(p50 >= Duration::from_millis(50) && p50 <= Duration::from_millis(60));
        let p99 = tracker.percentile(99.0);
        assert!(p99 >= Duration::from_millis(90));
    }
}
