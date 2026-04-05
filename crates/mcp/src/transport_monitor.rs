//! Transport health monitoring with alert thresholds.
//!
//! Provides [`TransportMonitor`] for real-time monitoring of transport
//! connections, [`HealthSnapshot`] for point-in-time health data,
//! and [`MonitorAlert`] for threshold-based alerting.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// A monitor alert triggered when thresholds are exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorAlert {
    pub level: AlertLevel,
    pub message: String,
    pub transport_id: String,
    #[serde(skip)]
    pub timestamp: Option<Instant>,
}

impl MonitorAlert {
    #[must_use]
    pub fn new(
        level: AlertLevel,
        message: impl Into<String>,
        transport_id: impl Into<String>,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            transport_id: transport_id.into(),
            timestamp: Some(Instant::now()),
        }
    }
}

/// Configuration for monitor alert thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorThresholds {
    /// Latency above this triggers a warning (default: 5s).
    pub latency_warning_ms: u64,
    /// Latency above this triggers a critical alert (default: 10s).
    pub latency_critical_ms: u64,
    /// Error rate above this triggers a warning (default: 0.10 = 10%).
    pub error_rate_warning: f64,
    /// Error rate above this triggers a critical alert (default: 0.25 = 25%).
    pub error_rate_critical: f64,
    /// Seconds without activity before warning.
    pub inactivity_warning_secs: u64,
}

impl Default for MonitorThresholds {
    fn default() -> Self {
        Self {
            latency_warning_ms: 5_000,
            latency_critical_ms: 10_000,
            error_rate_warning: 0.10,
            error_rate_critical: 0.25,
            inactivity_warning_secs: 60,
        }
    }
}

/// Point-in-time health snapshot for a transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSnapshot {
    pub transport_id: String,
    /// Average latency in milliseconds.
    pub latency_avg_ms: f64,
    /// Error rate (0.0 - 1.0).
    pub error_rate: f64,
    /// Total uptime in seconds.
    pub uptime_secs: f64,
    /// Seconds since last activity.
    pub last_activity_secs: f64,
    /// Whether the transport is connected.
    pub connected: bool,
    /// Total messages processed.
    pub total_messages: u64,
    /// Total errors.
    pub total_errors: u64,
}

/// Internal state tracked per transport.
#[derive(Debug)]
struct MonitoredTransport {
    transport_id: String,
    total_messages: u64,
    total_errors: u64,
    latency_sum_ms: f64,
    latency_count: u64,
    connected: bool,
    started_at: Instant,
    last_activity: Instant,
}

impl MonitoredTransport {
    fn new(id: String) -> Self {
        let now = Instant::now();
        Self {
            transport_id: id,
            total_messages: 0,
            total_errors: 0,
            latency_sum_ms: 0.0,
            latency_count: 0,
            connected: false,
            started_at: now,
            last_activity: now,
        }
    }

    fn error_rate(&self) -> f64 {
        if self.total_messages == 0 {
            return 0.0;
        }
        self.total_errors as f64 / self.total_messages as f64
    }

    fn avg_latency_ms(&self) -> f64 {
        if self.latency_count == 0 {
            return 0.0;
        }
        self.latency_sum_ms / self.latency_count as f64
    }

    fn snapshot(&self) -> HealthSnapshot {
        HealthSnapshot {
            transport_id: self.transport_id.clone(),
            latency_avg_ms: self.avg_latency_ms(),
            error_rate: self.error_rate(),
            uptime_secs: self.started_at.elapsed().as_secs_f64(),
            last_activity_secs: self.last_activity.elapsed().as_secs_f64(),
            connected: self.connected,
            total_messages: self.total_messages,
            total_errors: self.total_errors,
        }
    }
}

/// Real-time transport health monitor with alerting.
#[derive(Debug)]
pub struct TransportMonitor {
    inner: Arc<Mutex<MonitorInner>>,
}

#[derive(Debug)]
struct MonitorInner {
    transports: Vec<MonitoredTransport>,
    thresholds: MonitorThresholds,
    alerts: Vec<MonitorAlert>,
    alert_capacity: usize,
}

impl TransportMonitor {
    /// Create a new monitor with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self::with_thresholds(MonitorThresholds::default())
    }

    /// Create a monitor with custom thresholds.
    #[must_use]
    pub fn with_thresholds(thresholds: MonitorThresholds) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MonitorInner {
                transports: Vec::new(),
                thresholds,
                alerts: Vec::new(),
                alert_capacity: 100,
            })),
        }
    }

    /// Register a transport for monitoring.
    pub fn register(&self, transport_id: impl Into<String>) {
        let id = transport_id.into();
        let mut inner = self.inner.lock().unwrap();
        if !inner.transports.iter().any(|t| t.transport_id == id) {
            inner.transports.push(MonitoredTransport::new(id));
        }
    }

    /// Remove a transport from monitoring.
    pub fn unregister(&self, transport_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.transports.retain(|t| t.transport_id != transport_id);
    }

    /// Record a successful message for a transport.
    pub fn record_message(&self, transport_id: &str, latency: Duration) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(t) = inner
            .transports
            .iter_mut()
            .find(|t| t.transport_id == transport_id)
        {
            t.total_messages += 1;
            t.latency_sum_ms += latency.as_secs_f64() * 1000.0;
            t.latency_count += 1;
            t.last_activity = Instant::now();

            // Check latency thresholds
            let latency_ms = latency.as_millis() as u64;
            let thresholds = &inner.thresholds;
            if latency_ms > thresholds.latency_critical_ms {
                let alert = MonitorAlert::new(
                    AlertLevel::Critical,
                    format!(
                        "Latency {latency_ms}ms exceeds critical threshold {}ms",
                        thresholds.latency_critical_ms
                    ),
                    transport_id,
                );
                let cap = inner.alert_capacity;
                push_alert(&mut inner.alerts, alert, cap);
            } else if latency_ms > thresholds.latency_warning_ms {
                let alert = MonitorAlert::new(
                    AlertLevel::Warning,
                    format!(
                        "Latency {latency_ms}ms exceeds warning threshold {}ms",
                        thresholds.latency_warning_ms
                    ),
                    transport_id,
                );
                let cap = inner.alert_capacity;
                push_alert(&mut inner.alerts, alert, cap);
            }
        }
    }

    /// Record an error for a transport.
    pub fn record_error(&self, transport_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(t) = inner
            .transports
            .iter_mut()
            .find(|t| t.transport_id == transport_id)
        {
            t.total_messages += 1;
            t.total_errors += 1;
            t.last_activity = Instant::now();

            // Check error rate threshold
            let rate = t.error_rate();
            let thresholds = &inner.thresholds;
            if rate > thresholds.error_rate_critical {
                let alert = MonitorAlert::new(
                    AlertLevel::Critical,
                    format!(
                        "Error rate {:.1}% exceeds critical threshold {:.1}%",
                        rate * 100.0,
                        thresholds.error_rate_critical * 100.0
                    ),
                    transport_id,
                );
                let cap = inner.alert_capacity;
                push_alert(&mut inner.alerts, alert, cap);
            } else if rate > thresholds.error_rate_warning {
                let alert = MonitorAlert::new(
                    AlertLevel::Warning,
                    format!(
                        "Error rate {:.1}% exceeds warning threshold {:.1}%",
                        rate * 100.0,
                        thresholds.error_rate_warning * 100.0
                    ),
                    transport_id,
                );
                let cap = inner.alert_capacity;
                push_alert(&mut inner.alerts, alert, cap);
            }
        }
    }

    /// Set connection state for a transport.
    pub fn set_connected(&self, transport_id: &str, connected: bool) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(t) = inner
            .transports
            .iter_mut()
            .find(|t| t.transport_id == transport_id)
        {
            t.connected = connected;
            if !connected {
                let alert =
                    MonitorAlert::new(AlertLevel::Critical, "Transport disconnected", transport_id);
                let cap = inner.alert_capacity;
                push_alert(&mut inner.alerts, alert, cap);
            }
        }
    }

    /// Get a health snapshot for a specific transport.
    #[must_use]
    pub fn snapshot(&self, transport_id: &str) -> Option<HealthSnapshot> {
        let inner = self.inner.lock().unwrap();
        inner
            .transports
            .iter()
            .find(|t| t.transport_id == transport_id)
            .map(MonitoredTransport::snapshot)
    }

    /// Get health snapshots for all monitored transports.
    #[must_use]
    pub fn all_snapshots(&self) -> Vec<HealthSnapshot> {
        let inner = self.inner.lock().unwrap();
        inner
            .transports
            .iter()
            .map(MonitoredTransport::snapshot)
            .collect()
    }

    /// Check all transports for inactivity and generate alerts.
    pub fn check_inactivity(&self) -> Vec<MonitorAlert> {
        let mut inner = self.inner.lock().unwrap();
        let threshold_secs = inner.thresholds.inactivity_warning_secs;
        let new_alerts: Vec<MonitorAlert> = inner
            .transports
            .iter()
            .filter(|t| t.connected && t.last_activity.elapsed().as_secs() > threshold_secs)
            .map(|t| {
                MonitorAlert::new(
                    AlertLevel::Warning,
                    format!(
                        "No activity for {}s (threshold: {threshold_secs}s)",
                        t.last_activity.elapsed().as_secs()
                    ),
                    &t.transport_id,
                )
            })
            .collect();
        let cap = inner.alert_capacity;
        for alert in &new_alerts {
            push_alert(&mut inner.alerts, alert.clone(), cap);
        }
        new_alerts
    }

    /// Get all recorded alerts.
    #[must_use]
    pub fn alerts(&self) -> Vec<MonitorAlert> {
        self.inner.lock().unwrap().alerts.clone()
    }

    /// Get alerts of a specific level or higher.
    #[must_use]
    pub fn alerts_at_level(&self, min_level: AlertLevel) -> Vec<MonitorAlert> {
        self.inner
            .lock()
            .unwrap()
            .alerts
            .iter()
            .filter(|a| a.level >= min_level)
            .cloned()
            .collect()
    }

    /// Clear all alerts.
    pub fn clear_alerts(&self) {
        self.inner.lock().unwrap().alerts.clear();
    }

    /// Number of monitored transports.
    #[must_use]
    pub fn transport_count(&self) -> usize {
        self.inner.lock().unwrap().transports.len()
    }
}

impl Default for TransportMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TransportMonitor {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

fn push_alert(alerts: &mut Vec<MonitorAlert>, alert: MonitorAlert, capacity: usize) {
    if alerts.len() >= capacity {
        alerts.remove(0);
    }
    alerts.push(alert);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_level_ordering() {
        assert!(AlertLevel::Info < AlertLevel::Warning);
        assert!(AlertLevel::Warning < AlertLevel::Critical);
    }

    #[test]
    fn alert_level_display() {
        assert_eq!(AlertLevel::Info.to_string(), "info");
        assert_eq!(AlertLevel::Warning.to_string(), "warning");
        assert_eq!(AlertLevel::Critical.to_string(), "critical");
    }

    #[test]
    fn monitor_register_unregister() {
        let m = TransportMonitor::new();
        m.register("t1");
        m.register("t2");
        assert_eq!(m.transport_count(), 2);
        m.unregister("t1");
        assert_eq!(m.transport_count(), 1);
    }

    #[test]
    fn monitor_duplicate_register() {
        let m = TransportMonitor::new();
        m.register("t1");
        m.register("t1");
        assert_eq!(m.transport_count(), 1);
    }

    #[test]
    fn monitor_record_message() {
        let m = TransportMonitor::new();
        m.register("t1");
        m.record_message("t1", Duration::from_millis(50));
        m.record_message("t1", Duration::from_millis(100));

        let snap = m.snapshot("t1").unwrap();
        assert_eq!(snap.total_messages, 2);
        assert!((snap.latency_avg_ms - 75.0).abs() < 1.0);
    }

    #[test]
    fn monitor_record_error() {
        let m = TransportMonitor::new();
        m.register("t1");
        m.record_message("t1", Duration::from_millis(10));
        m.record_error("t1");
        // 2 total messages, 1 error
        let snap = m.snapshot("t1").unwrap();
        assert_eq!(snap.total_errors, 1);
        assert!((snap.error_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn monitor_latency_warning_alert() {
        let thresholds = MonitorThresholds {
            latency_warning_ms: 100,
            latency_critical_ms: 500,
            ..Default::default()
        };
        let m = TransportMonitor::with_thresholds(thresholds);
        m.register("t1");
        m.record_message("t1", Duration::from_millis(200));

        let alerts = m.alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, AlertLevel::Warning);
        assert!(alerts[0].message.contains("200ms"));
    }

    #[test]
    fn monitor_latency_critical_alert() {
        let thresholds = MonitorThresholds {
            latency_warning_ms: 100,
            latency_critical_ms: 500,
            ..Default::default()
        };
        let m = TransportMonitor::with_thresholds(thresholds);
        m.register("t1");
        m.record_message("t1", Duration::from_millis(600));

        let alerts = m.alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, AlertLevel::Critical);
    }

    #[test]
    fn monitor_error_rate_warning() {
        let thresholds = MonitorThresholds {
            error_rate_warning: 0.10,
            error_rate_critical: 0.50,
            ..Default::default()
        };
        let m = TransportMonitor::with_thresholds(thresholds);
        m.register("t1");
        // 4 ok messages, then enough errors to cross 10%
        for _ in 0..4 {
            m.record_message("t1", Duration::from_millis(1));
        }
        m.record_error("t1"); // 1/5 = 20% > 10%

        let warnings = m.alerts_at_level(AlertLevel::Warning);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn monitor_disconnect_alert() {
        let m = TransportMonitor::new();
        m.register("t1");
        m.set_connected("t1", true);
        m.set_connected("t1", false);

        let alerts = m.alerts();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, AlertLevel::Critical);
        assert!(alerts[0].message.contains("disconnected"));
    }

    #[test]
    fn monitor_all_snapshots() {
        let m = TransportMonitor::new();
        m.register("t1");
        m.register("t2");
        m.set_connected("t1", true);

        let snaps = m.all_snapshots();
        assert_eq!(snaps.len(), 2);
    }

    #[test]
    fn monitor_snapshot_missing() {
        let m = TransportMonitor::new();
        assert!(m.snapshot("nonexistent").is_none());
    }

    #[test]
    fn monitor_clear_alerts() {
        let m = TransportMonitor::new();
        m.register("t1");
        m.set_connected("t1", true);
        m.set_connected("t1", false);
        assert!(!m.alerts().is_empty());
        m.clear_alerts();
        assert!(m.alerts().is_empty());
    }

    #[test]
    fn health_snapshot_serde() {
        let snap = HealthSnapshot {
            transport_id: "t1".into(),
            latency_avg_ms: 42.5,
            error_rate: 0.05,
            uptime_secs: 120.0,
            last_activity_secs: 1.5,
            connected: true,
            total_messages: 100,
            total_errors: 5,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: HealthSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.transport_id, "t1");
        assert!((back.latency_avg_ms - 42.5).abs() < 0.1);
    }

    #[test]
    fn monitor_alert_serde() {
        let alert = MonitorAlert {
            level: AlertLevel::Warning,
            message: "test alert".into(),
            transport_id: "t1".into(),
            timestamp: None,
        };
        let json = serde_json::to_string(&alert).unwrap();
        let back: MonitorAlert = serde_json::from_str(&json).unwrap();
        assert_eq!(back.level, AlertLevel::Warning);
        assert_eq!(back.message, "test alert");
    }

    #[test]
    fn thresholds_serde_roundtrip() {
        let t = MonitorThresholds::default();
        let json = serde_json::to_string(&t).unwrap();
        let back: MonitorThresholds = serde_json::from_str(&json).unwrap();
        assert_eq!(back.latency_warning_ms, 5_000);
        assert!((back.error_rate_warning - 0.10).abs() < 0.001);
    }

    #[test]
    fn monitor_thread_safe() {
        let m = TransportMonitor::new();
        m.register("t1");
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let m = m.clone();
                std::thread::spawn(move || {
                    for _ in 0..25 {
                        m.record_message("t1", Duration::from_millis(10));
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let snap = m.snapshot("t1").unwrap();
        assert_eq!(snap.total_messages, 100);
    }
}
