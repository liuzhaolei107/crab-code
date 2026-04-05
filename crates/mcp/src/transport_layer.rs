//! Unified transport layer abstraction with configuration and metrics.
//!
//! Extends the base [`Transport`](crate::Transport) trait with connection
//! state tracking, transport-level configuration, and send/receive metrics.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ─── Transport configuration ──────────────────────────────────────────

/// Generic transport configuration applicable to all transport types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportConfig {
    /// Connection timeout.
    #[serde(with = "duration_secs")]
    pub timeout: Duration,
    /// Read/write buffer size in bytes.
    pub buffer_size: usize,
    /// Keep-alive interval (0 = disabled).
    #[serde(with = "duration_secs")]
    pub keep_alive_interval: Duration,
    /// Maximum message size in bytes (0 = unlimited).
    pub max_message_size: usize,
    /// Transport type identifier.
    pub transport_type: TransportType,
}

impl TransportConfig {
    #[must_use]
    pub fn stdio() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            buffer_size: 65_536,
            keep_alive_interval: Duration::ZERO,
            max_message_size: 0,
            transport_type: TransportType::Stdio,
        }
    }

    #[must_use]
    pub fn sse(url: impl Into<String>) -> Self {
        Self {
            timeout: Duration::from_secs(30),
            buffer_size: 65_536,
            keep_alive_interval: Duration::from_secs(15),
            max_message_size: 0,
            transport_type: TransportType::Sse { url: url.into() },
        }
    }

    #[must_use]
    pub fn websocket(url: impl Into<String>) -> Self {
        Self {
            timeout: Duration::from_secs(30),
            buffer_size: 65_536,
            keep_alive_interval: Duration::from_secs(30),
            max_message_size: 0,
            transport_type: TransportType::WebSocket { url: url.into() },
        }
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self::stdio()
    }
}

/// Transport type with type-specific parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransportType {
    Stdio,
    Sse { url: String },
    WebSocket { url: String },
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio => write!(f, "stdio"),
            Self::Sse { url } => write!(f, "sse({url})"),
            Self::WebSocket { url } => write!(f, "ws({url})"),
        }
    }
}

// ─── Transport metrics ────────────────────────────────────────────────

/// Real-time metrics for a transport connection.
#[derive(Debug)]
pub struct TransportMetrics {
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    errors: AtomicU64,
    connected: AtomicBool,
    latencies: Arc<Mutex<LatencyTracker>>,
    created_at: Instant,
}

/// Rolling window latency tracker.
#[derive(Debug)]
struct LatencyTracker {
    samples: Vec<Duration>,
    capacity: usize,
    pos: usize,
}

impl LatencyTracker {
    fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            capacity,
            pos: 0,
        }
    }

    fn record(&mut self, latency: Duration) {
        if self.samples.len() < self.capacity {
            self.samples.push(latency);
        } else {
            self.samples[self.pos] = latency;
        }
        self.pos = (self.pos + 1) % self.capacity;
    }

    fn average(&self) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }
        let total: Duration = self.samples.iter().sum();
        Some(total / self.samples.len() as u32)
    }

    fn max(&self) -> Option<Duration> {
        self.samples.iter().max().copied()
    }

    fn min(&self) -> Option<Duration> {
        self.samples.iter().min().copied()
    }

    fn count(&self) -> usize {
        self.samples.len()
    }
}

impl TransportMetrics {
    /// Create new metrics with the given latency sample capacity.
    #[must_use]
    pub fn new(latency_capacity: usize) -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            connected: AtomicBool::new(false),
            latencies: Arc::new(Mutex::new(LatencyTracker::new(latency_capacity))),
            created_at: Instant::now(),
        }
    }

    /// Record a sent message.
    pub fn record_send(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a received message.
    pub fn record_receive(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a latency sample.
    pub fn record_latency(&self, latency: Duration) {
        self.latencies.lock().unwrap().record(latency);
    }

    /// Record an error.
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Set connection state.
    pub fn set_connected(&self, connected: bool) {
        self.connected.store(connected, Ordering::Relaxed);
    }

    /// Total messages sent.
    #[must_use]
    pub fn messages_sent(&self) -> u64 {
        self.messages_sent.load(Ordering::Relaxed)
    }

    /// Total messages received.
    #[must_use]
    pub fn messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    /// Total bytes sent.
    #[must_use]
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Total bytes received.
    #[must_use]
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Total errors.
    #[must_use]
    pub fn error_count(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Whether the transport is currently connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Average latency, if any samples recorded.
    #[must_use]
    pub fn average_latency(&self) -> Option<Duration> {
        self.latencies.lock().unwrap().average()
    }

    /// Max latency.
    #[must_use]
    pub fn max_latency(&self) -> Option<Duration> {
        self.latencies.lock().unwrap().max()
    }

    /// Min latency.
    #[must_use]
    pub fn min_latency(&self) -> Option<Duration> {
        self.latencies.lock().unwrap().min()
    }

    /// Number of latency samples.
    #[must_use]
    pub fn latency_sample_count(&self) -> usize {
        self.latencies.lock().unwrap().count()
    }

    /// Error rate (errors / total messages). Returns 0 if no messages.
    #[must_use]
    pub fn error_rate(&self) -> f64 {
        let total = self.messages_sent() + self.messages_received();
        if total == 0 {
            return 0.0;
        }
        self.error_count() as f64 / total as f64
    }

    /// Uptime since creation.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Take a snapshot of current metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            messages_sent: self.messages_sent(),
            messages_received: self.messages_received(),
            bytes_sent: self.bytes_sent(),
            bytes_received: self.bytes_received(),
            error_count: self.error_count(),
            error_rate: self.error_rate(),
            average_latency: self.average_latency(),
            connected: self.is_connected(),
            uptime: self.uptime(),
        }
    }
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Point-in-time snapshot of transport metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSnapshot {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub error_count: u64,
    pub error_rate: f64,
    #[serde(
        serialize_with = "serialize_opt_duration",
        deserialize_with = "deserialize_opt_duration"
    )]
    pub average_latency: Option<Duration>,
    pub connected: bool,
    #[serde(with = "duration_secs")]
    pub uptime: Duration,
}

// ─── Serde helpers for Duration ───────────────────────────────────────

mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(d.as_secs_f64())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = f64::deserialize(d)?;
        Ok(Duration::from_secs_f64(secs))
    }
}

fn serialize_opt_duration<S: serde::Serializer>(
    d: &Option<Duration>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match d {
        Some(d) => s.serialize_some(&d.as_secs_f64()),
        None => s.serialize_none(),
    }
}

fn deserialize_opt_duration<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Duration>, D::Error> {
    let opt = Option::<f64>::deserialize(d)?;
    Ok(opt.map(Duration::from_secs_f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_config_stdio() {
        let cfg = TransportConfig::stdio();
        assert_eq!(cfg.transport_type, TransportType::Stdio);
        assert_eq!(cfg.timeout, Duration::from_secs(30));
    }

    #[test]
    fn transport_config_sse() {
        let cfg = TransportConfig::sse("http://localhost:8080");
        assert!(matches!(cfg.transport_type, TransportType::Sse { .. }));
    }

    #[test]
    fn transport_config_websocket() {
        let cfg = TransportConfig::websocket("ws://localhost:9090");
        assert!(matches!(
            cfg.transport_type,
            TransportType::WebSocket { .. }
        ));
    }

    #[test]
    fn transport_config_serde_roundtrip() {
        let cfg = TransportConfig::sse("http://example.com/sse");
        let json = serde_json::to_string(&cfg).unwrap();
        let back: TransportConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.transport_type, cfg.transport_type);
        assert_eq!(back.buffer_size, cfg.buffer_size);
    }

    #[test]
    fn transport_type_display() {
        assert_eq!(TransportType::Stdio.to_string(), "stdio");
        assert_eq!(
            TransportType::Sse {
                url: "http://x".into()
            }
            .to_string(),
            "sse(http://x)"
        );
        assert_eq!(
            TransportType::WebSocket {
                url: "ws://y".into()
            }
            .to_string(),
            "ws(ws://y)"
        );
    }

    #[test]
    fn metrics_record_send_receive() {
        let m = TransportMetrics::new(10);
        m.record_send(100);
        m.record_send(200);
        m.record_receive(150);
        assert_eq!(m.messages_sent(), 2);
        assert_eq!(m.messages_received(), 1);
        assert_eq!(m.bytes_sent(), 300);
        assert_eq!(m.bytes_received(), 150);
    }

    #[test]
    fn metrics_error_rate() {
        let m = TransportMetrics::new(10);
        assert_eq!(m.error_rate(), 0.0);
        m.record_send(10);
        m.record_send(10);
        m.record_error();
        // error_rate = 1 / (2 sent + 0 recv) = 0.5
        assert!((m.error_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn metrics_latency_tracking() {
        let m = TransportMetrics::new(10);
        assert!(m.average_latency().is_none());

        m.record_latency(Duration::from_millis(100));
        m.record_latency(Duration::from_millis(200));
        m.record_latency(Duration::from_millis(300));

        assert_eq!(m.latency_sample_count(), 3);
        let avg = m.average_latency().unwrap();
        assert!((avg.as_millis() as i64 - 200).abs() < 2);
        assert_eq!(m.max_latency(), Some(Duration::from_millis(300)));
        assert_eq!(m.min_latency(), Some(Duration::from_millis(100)));
    }

    #[test]
    fn metrics_latency_ring_buffer() {
        let m = TransportMetrics::new(3);
        m.record_latency(Duration::from_millis(10));
        m.record_latency(Duration::from_millis(20));
        m.record_latency(Duration::from_millis(30));
        // Buffer full, next overwrites oldest
        m.record_latency(Duration::from_millis(100));
        assert_eq!(m.latency_sample_count(), 3);
        // Should now have 20, 30, 100
        assert_eq!(m.min_latency(), Some(Duration::from_millis(20)));
        assert_eq!(m.max_latency(), Some(Duration::from_millis(100)));
    }

    #[test]
    fn metrics_connected_state() {
        let m = TransportMetrics::new(10);
        assert!(!m.is_connected());
        m.set_connected(true);
        assert!(m.is_connected());
    }

    #[test]
    fn metrics_snapshot() {
        let m = TransportMetrics::new(10);
        m.set_connected(true);
        m.record_send(50);
        m.record_receive(30);
        let snap = m.snapshot();
        assert_eq!(snap.messages_sent, 1);
        assert_eq!(snap.messages_received, 1);
        assert_eq!(snap.bytes_sent, 50);
        assert_eq!(snap.bytes_received, 30);
        assert!(snap.connected);
    }

    #[test]
    fn metrics_snapshot_serde() {
        let m = TransportMetrics::new(10);
        m.record_latency(Duration::from_millis(50));
        let snap = m.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: MetricsSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.messages_sent, snap.messages_sent);
        assert!(back.average_latency.is_some());
    }

    #[test]
    fn metrics_uptime() {
        let m = TransportMetrics::new(10);
        std::thread::sleep(Duration::from_millis(5));
        assert!(m.uptime() >= Duration::from_millis(4));
    }

    #[test]
    fn metrics_thread_safe() {
        let m = Arc::new(TransportMetrics::new(1000));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let m = Arc::clone(&m);
                std::thread::spawn(move || {
                    for _ in 0..25 {
                        m.record_send(10);
                        m.record_receive(10);
                        m.record_latency(Duration::from_millis(1));
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(m.messages_sent(), 100);
        assert_eq!(m.messages_received(), 100);
        assert_eq!(m.latency_sample_count(), 100);
    }

    #[test]
    fn default_config() {
        let cfg = TransportConfig::default();
        assert_eq!(cfg.transport_type, TransportType::Stdio);
    }

    #[test]
    fn default_metrics() {
        let m = TransportMetrics::default();
        assert_eq!(m.messages_sent(), 0);
        assert!(!m.is_connected());
    }
}
