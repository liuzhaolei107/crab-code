//! MCP resource change watching and notification.
//!
//! Provides [`ResourceWatcher`] for monitoring resource changes,
//! [`ResourceChangeEvent`] for change events, and subscriber notification.

use crate::resource_subscription::{SubscriptionManager, uri_matches_pattern};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Type of resource change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

/// A resource change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChangeEvent {
    /// URI of the changed resource.
    pub uri: String,
    /// Type of change.
    pub change_type: ChangeType,
    /// When the change was detected.
    #[serde(skip)]
    pub timestamp: Option<Instant>,
}

impl ResourceChangeEvent {
    #[must_use]
    pub fn new(uri: impl Into<String>, change_type: ChangeType) -> Self {
        Self {
            uri: uri.into(),
            change_type,
            timestamp: Some(Instant::now()),
        }
    }

    #[must_use]
    pub fn created(uri: impl Into<String>) -> Self {
        Self::new(uri, ChangeType::Created)
    }

    #[must_use]
    pub fn modified(uri: impl Into<String>) -> Self {
        Self::new(uri, ChangeType::Modified)
    }

    #[must_use]
    pub fn deleted(uri: impl Into<String>) -> Self {
        Self::new(uri, ChangeType::Deleted)
    }
}

/// Configuration for the resource watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherConfig {
    /// Poll interval in milliseconds.
    pub poll_interval_ms: u64,
    /// Debounce window in milliseconds (coalesce rapid changes).
    pub debounce_ms: u64,
    /// Maximum events to buffer before dropping.
    pub max_buffer_size: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
            debounce_ms: 100,
            max_buffer_size: 1000,
        }
    }
}

/// Notification delivered to a subscriber.
#[derive(Debug, Clone)]
pub struct SubscriberNotification {
    pub subscriber_id: String,
    pub event: ResourceChangeEvent,
}

/// Trait for receiving resource change notifications.
pub trait ChangeNotifier: Send + Sync {
    fn notify(&self, subscriber_id: &str, event: &ResourceChangeEvent);
}

/// A notifier that records notifications for testing.
pub struct RecordingNotifier {
    notifications: Arc<Mutex<Vec<SubscriberNotification>>>,
}

impl RecordingNotifier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[must_use]
    pub fn notifications(&self) -> Vec<SubscriberNotification> {
        self.notifications.lock().unwrap().clone()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.notifications.lock().unwrap().len()
    }
}

impl Default for RecordingNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeNotifier for RecordingNotifier {
    fn notify(&self, subscriber_id: &str, event: &ResourceChangeEvent) {
        self.notifications
            .lock()
            .unwrap()
            .push(SubscriberNotification {
                subscriber_id: subscriber_id.to_string(),
                event: event.clone(),
            });
    }
}

/// Watches for resource changes and notifies subscribers.
#[derive(Debug)]
pub struct ResourceWatcher {
    inner: Arc<Mutex<WatcherInner>>,
}

#[derive(Debug)]
struct WatcherInner {
    config: WatcherConfig,
    /// URIs/patterns being watched.
    watched: Vec<String>,
    /// Event history (bounded by `config.max_buffer_size`).
    events: Vec<ResourceChangeEvent>,
    /// Total events processed.
    total_events: u64,
    /// Total events dropped.
    dropped_events: u64,
}

impl ResourceWatcher {
    #[must_use]
    pub fn new(config: WatcherConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(WatcherInner {
                config,
                watched: Vec::new(),
                events: Vec::new(),
                total_events: 0,
                dropped_events: 0,
            })),
        }
    }

    /// Add a URI/pattern to watch.
    pub fn watch(&self, uri: impl Into<String>) {
        let uri = uri.into();
        let mut inner = self.inner.lock().unwrap();
        if !inner.watched.contains(&uri) {
            inner.watched.push(uri);
        }
    }

    /// Stop watching a URI/pattern.
    pub fn unwatch(&self, uri: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let before = inner.watched.len();
        inner.watched.retain(|w| w != uri);
        inner.watched.len() < before
    }

    /// Check if a URI is being watched (exact or pattern match).
    #[must_use]
    pub fn is_watched(&self, uri: &str) -> bool {
        let inner = self.inner.lock().unwrap();
        inner
            .watched
            .iter()
            .any(|pattern| uri_matches_pattern(uri, pattern))
    }

    /// Report a change event. Notifies matching subscribers via the notifier.
    /// Returns the list of subscriber IDs notified.
    pub fn report_change(
        &self,
        event: ResourceChangeEvent,
        subscriptions: &SubscriptionManager,
        notifier: &dyn ChangeNotifier,
    ) -> Vec<String> {
        let subscribers = subscriptions.subscribers_for(&event.uri);
        for sub_id in &subscribers {
            notifier.notify(sub_id, &event);
        }

        let mut inner = self.inner.lock().unwrap();
        inner.total_events += 1;
        if inner.events.len() >= inner.config.max_buffer_size {
            inner.events.remove(0);
            inner.dropped_events += 1;
        }
        inner.events.push(event);

        subscribers
    }

    /// Get recent events.
    #[must_use]
    pub fn recent_events(&self) -> Vec<ResourceChangeEvent> {
        self.inner.lock().unwrap().events.clone()
    }

    /// Number of watched URIs/patterns.
    #[must_use]
    pub fn watch_count(&self) -> usize {
        self.inner.lock().unwrap().watched.len()
    }

    /// Total events processed.
    #[must_use]
    pub fn total_events(&self) -> u64 {
        self.inner.lock().unwrap().total_events
    }

    /// Total events dropped due to buffer overflow.
    #[must_use]
    pub fn dropped_events(&self) -> u64 {
        self.inner.lock().unwrap().dropped_events
    }

    /// Access the watcher config.
    #[must_use]
    pub fn config(&self) -> WatcherConfig {
        self.inner.lock().unwrap().config.clone()
    }
}

impl Clone for ResourceWatcher {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_type_display() {
        assert_eq!(ChangeType::Created.to_string(), "created");
        assert_eq!(ChangeType::Modified.to_string(), "modified");
        assert_eq!(ChangeType::Deleted.to_string(), "deleted");
    }

    #[test]
    fn change_type_serde() {
        let ct = ChangeType::Modified;
        let json = serde_json::to_string(&ct).unwrap();
        let back: ChangeType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ct);
    }

    #[test]
    fn event_factories() {
        let e1 = ResourceChangeEvent::created("file:///a.rs");
        assert_eq!(e1.change_type, ChangeType::Created);
        let e2 = ResourceChangeEvent::modified("file:///b.rs");
        assert_eq!(e2.change_type, ChangeType::Modified);
        let e3 = ResourceChangeEvent::deleted("file:///c.rs");
        assert_eq!(e3.change_type, ChangeType::Deleted);
    }

    #[test]
    fn event_serde_roundtrip() {
        let e = ResourceChangeEvent::new("file:///test.rs", ChangeType::Modified);
        let json = serde_json::to_string(&e).unwrap();
        let back: ResourceChangeEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.uri, "file:///test.rs");
        assert_eq!(back.change_type, ChangeType::Modified);
    }

    #[test]
    fn watcher_config_default() {
        let cfg = WatcherConfig::default();
        assert_eq!(cfg.poll_interval_ms, 1000);
        assert_eq!(cfg.debounce_ms, 100);
        assert_eq!(cfg.max_buffer_size, 1000);
    }

    #[test]
    fn watcher_config_serde() {
        let cfg = WatcherConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: WatcherConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.poll_interval_ms, cfg.poll_interval_ms);
    }

    #[test]
    fn watcher_watch_unwatch() {
        let w = ResourceWatcher::new(WatcherConfig::default());
        w.watch("file:///src/*");
        assert_eq!(w.watch_count(), 1);
        assert!(w.is_watched("file:///src/main.rs"));
        assert!(!w.is_watched("file:///other.rs"));

        assert!(w.unwatch("file:///src/*"));
        assert_eq!(w.watch_count(), 0);
    }

    #[test]
    fn watcher_no_duplicate_watch() {
        let w = ResourceWatcher::new(WatcherConfig::default());
        w.watch("file:///a.rs");
        w.watch("file:///a.rs");
        assert_eq!(w.watch_count(), 1);
    }

    #[test]
    fn watcher_report_change_notifies_subscribers() {
        let w = ResourceWatcher::new(WatcherConfig::default());
        let subs = SubscriptionManager::new();
        let notifier = RecordingNotifier::new();

        subs.subscribe("file:///src/*", "c1");
        subs.subscribe("file:///src/main.rs", "c2");

        let event = ResourceChangeEvent::modified("file:///src/main.rs");
        let notified = w.report_change(event, &subs, &notifier);

        assert_eq!(notified.len(), 2);
        assert_eq!(notifier.count(), 2);
        assert_eq!(w.total_events(), 1);
    }

    #[test]
    fn watcher_event_buffer_overflow() {
        let cfg = WatcherConfig {
            max_buffer_size: 3,
            ..Default::default()
        };
        let w = ResourceWatcher::new(cfg);
        let subs = SubscriptionManager::new();
        let notifier = RecordingNotifier::new();

        for i in 0..5 {
            w.report_change(
                ResourceChangeEvent::modified(format!("file:///{i}.rs")),
                &subs,
                &notifier,
            );
        }

        assert_eq!(w.total_events(), 5);
        assert_eq!(w.dropped_events(), 2);
        let events = w.recent_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].uri, "file:///2.rs");
    }

    #[test]
    fn watcher_no_subscribers() {
        let w = ResourceWatcher::new(WatcherConfig::default());
        let subs = SubscriptionManager::new();
        let notifier = RecordingNotifier::new();

        let notified = w.report_change(
            ResourceChangeEvent::created("file:///new.rs"),
            &subs,
            &notifier,
        );
        assert!(notified.is_empty());
        assert_eq!(notifier.count(), 0);
    }

    #[test]
    fn recording_notifier_default() {
        let n = RecordingNotifier::default();
        assert_eq!(n.count(), 0);
    }

    #[test]
    fn watcher_thread_safe() {
        let w = ResourceWatcher::new(WatcherConfig::default());
        let subs = SubscriptionManager::new();
        let notifier = Arc::new(RecordingNotifier::new());

        let handles: Vec<_> = (0..4)
            .map(|i| {
                let w = w.clone();
                let subs = subs.clone();
                let notifier = Arc::clone(&notifier);
                std::thread::spawn(move || {
                    for j in 0..25 {
                        w.report_change(
                            ResourceChangeEvent::modified(format!("file:///{i}_{j}.rs")),
                            &subs,
                            notifier.as_ref(),
                        );
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(w.total_events(), 100);
    }
}
