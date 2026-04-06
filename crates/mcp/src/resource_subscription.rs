//! MCP resource subscription management.
//!
//! Provides [`ResourceSubscription`] for tracking subscriptions,
//! [`SubscriptionManager`] for managing subscriber-resource relationships,
//! with support for wildcard URI patterns.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A single resource subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSubscription {
    /// The resource URI or pattern being subscribed to.
    pub uri: String,
    /// ID of the subscriber.
    pub subscriber_id: String,
    /// When the subscription was created (serialized as epoch millis).
    #[serde(skip)]
    pub created_at: Option<Instant>,
}

impl ResourceSubscription {
    #[must_use]
    pub fn new(uri: impl Into<String>, subscriber_id: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            subscriber_id: subscriber_id.into(),
            created_at: Some(Instant::now()),
        }
    }
}

/// Check if a URI matches a pattern (supports `*` wildcard at end of path segments).
#[must_use]
pub fn uri_matches_pattern(uri: &str, pattern: &str) -> bool {
    if pattern == uri {
        return true;
    }
    // Support glob-style wildcards: file://project/src/*.rs
    if let Some(prefix) = pattern.strip_suffix("*") {
        return uri.starts_with(prefix);
    }
    // Support single segment wildcard: file://project/*/config.json
    if pattern.contains('*') {
        let pattern_parts: Vec<&str> = pattern.split('*').collect();
        if pattern_parts.len() == 2 {
            return uri.starts_with(pattern_parts[0]) && uri.ends_with(pattern_parts[1]);
        }
    }
    false
}

/// Manages resource subscriptions.
#[derive(Debug)]
pub struct SubscriptionManager {
    inner: Arc<Mutex<SubscriptionInner>>,
}

#[derive(Debug)]
struct SubscriptionInner {
    /// `subscriber_id` → list of subscribed URIs/patterns
    subscriptions: HashMap<String, Vec<ResourceSubscription>>,
}

impl SubscriptionManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SubscriptionInner {
                subscriptions: HashMap::new(),
            })),
        }
    }

    /// Subscribe to a resource URI or pattern.
    #[allow(clippy::significant_drop_tightening)]
    pub fn subscribe(&self, uri: impl Into<String>, subscriber_id: impl Into<String>) {
        let uri = uri.into();
        let subscriber_id = subscriber_id.into();
        let mut inner = self.inner.lock().unwrap();
        let subs = inner
            .subscriptions
            .entry(subscriber_id.clone())
            .or_default();
        // Avoid duplicate subscriptions
        if !subs.iter().any(|s| s.uri == uri) {
            subs.push(ResourceSubscription::new(uri, subscriber_id));
        }
    }

    /// Unsubscribe from a specific URI/pattern.
    pub fn unsubscribe(&self, uri: &str, subscriber_id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let Some(subs) = inner.subscriptions.get_mut(subscriber_id) else {
            return false;
        };
        let before = subs.len();
        subs.retain(|s| s.uri != uri);
        let removed = subs.len() < before;
        if subs.is_empty() {
            inner.subscriptions.remove(subscriber_id);
        }
        removed
    }

    /// Unsubscribe all subscriptions for a subscriber.
    pub fn unsubscribe_all(&self, subscriber_id: &str) -> usize {
        let mut inner = self.inner.lock().unwrap();
        inner
            .subscriptions
            .remove(subscriber_id)
            .map_or(0, |subs| subs.len())
    }

    /// List all subscriptions for a subscriber.
    #[must_use]
    pub fn list_subscriptions(&self, subscriber_id: &str) -> Vec<ResourceSubscription> {
        let inner = self.inner.lock().unwrap();
        inner
            .subscriptions
            .get(subscriber_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Find all subscriber IDs that are subscribed to a given resource URI.
    /// Checks both exact matches and wildcard patterns.
    #[must_use]
    #[allow(clippy::significant_drop_tightening)]
    pub fn subscribers_for(&self, uri: &str) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        let mut result = Vec::new();
        for (subscriber_id, subs) in &inner.subscriptions {
            for sub in subs {
                if uri_matches_pattern(uri, &sub.uri) {
                    result.push(subscriber_id.clone());
                    break;
                }
            }
        }
        result
    }

    /// Total number of subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().unwrap().subscriptions.len()
    }

    /// Total number of subscriptions across all subscribers.
    #[must_use]
    pub fn subscription_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .subscriptions
            .values()
            .map(Vec::len)
            .sum()
    }

    /// Whether there are any subscriptions at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().subscriptions.is_empty()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SubscriptionManager {
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
    fn subscription_new() {
        let sub = ResourceSubscription::new("file:///tmp/test.rs", "client-1");
        assert_eq!(sub.uri, "file:///tmp/test.rs");
        assert_eq!(sub.subscriber_id, "client-1");
        assert!(sub.created_at.is_some());
    }

    #[test]
    fn subscription_serde_roundtrip() {
        let sub = ResourceSubscription::new("file:///test", "c1");
        let json = serde_json::to_string(&sub).unwrap();
        let back: ResourceSubscription = serde_json::from_str(&json).unwrap();
        assert_eq!(back.uri, "file:///test");
        assert_eq!(back.subscriber_id, "c1");
    }

    #[test]
    fn uri_matches_exact() {
        assert!(uri_matches_pattern("file:///a.rs", "file:///a.rs"));
        assert!(!uri_matches_pattern("file:///a.rs", "file:///b.rs"));
    }

    #[test]
    fn uri_matches_trailing_wildcard() {
        assert!(uri_matches_pattern("file:///src/main.rs", "file:///src/*"));
        assert!(uri_matches_pattern("file:///src/lib.rs", "file:///src/*"));
        assert!(!uri_matches_pattern("file:///other/a.rs", "file:///src/*"));
    }

    #[test]
    fn uri_matches_middle_wildcard() {
        assert!(uri_matches_pattern(
            "file:///project/src/config.json",
            "file:///project/*/config.json"
        ));
        assert!(!uri_matches_pattern(
            "file:///project/src/other.json",
            "file:///project/*/config.json"
        ));
    }

    #[test]
    fn manager_subscribe_and_list() {
        let mgr = SubscriptionManager::new();
        mgr.subscribe("file:///a.rs", "c1");
        mgr.subscribe("file:///b.rs", "c1");

        let subs = mgr.list_subscriptions("c1");
        assert_eq!(subs.len(), 2);
        assert_eq!(mgr.subscription_count(), 2);
        assert_eq!(mgr.subscriber_count(), 1);
    }

    #[test]
    fn manager_no_duplicate_subscribe() {
        let mgr = SubscriptionManager::new();
        mgr.subscribe("file:///a.rs", "c1");
        mgr.subscribe("file:///a.rs", "c1");
        assert_eq!(mgr.subscription_count(), 1);
    }

    #[test]
    fn manager_unsubscribe() {
        let mgr = SubscriptionManager::new();
        mgr.subscribe("file:///a.rs", "c1");
        mgr.subscribe("file:///b.rs", "c1");
        assert!(mgr.unsubscribe("file:///a.rs", "c1"));
        assert_eq!(mgr.subscription_count(), 1);
    }

    #[test]
    fn manager_unsubscribe_all() {
        let mgr = SubscriptionManager::new();
        mgr.subscribe("file:///a.rs", "c1");
        mgr.subscribe("file:///b.rs", "c1");
        let removed = mgr.unsubscribe_all("c1");
        assert_eq!(removed, 2);
        assert!(mgr.is_empty());
    }

    #[test]
    fn manager_subscribers_for_exact() {
        let mgr = SubscriptionManager::new();
        mgr.subscribe("file:///a.rs", "c1");
        mgr.subscribe("file:///b.rs", "c2");

        let subs = mgr.subscribers_for("file:///a.rs");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0], "c1");
    }

    #[test]
    fn manager_subscribers_for_wildcard() {
        let mgr = SubscriptionManager::new();
        mgr.subscribe("file:///src/*", "c1");
        mgr.subscribe("file:///src/main.rs", "c2");

        let subs = mgr.subscribers_for("file:///src/main.rs");
        assert_eq!(subs.len(), 2);
    }

    #[test]
    fn manager_empty() {
        let mgr = SubscriptionManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.subscriber_count(), 0);
        assert_eq!(mgr.subscription_count(), 0);
    }

    #[test]
    fn manager_unsubscribe_nonexistent() {
        let mgr = SubscriptionManager::new();
        assert!(!mgr.unsubscribe("file:///nope", "nobody"));
    }

    #[test]
    fn manager_list_empty_subscriber() {
        let mgr = SubscriptionManager::new();
        let subs = mgr.list_subscriptions("nobody");
        assert!(subs.is_empty());
    }

    #[test]
    fn manager_thread_safe() {
        let mgr = SubscriptionManager::new();
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let mgr = mgr.clone();
                std::thread::spawn(move || {
                    for j in 0..25 {
                        mgr.subscribe(format!("file:///t{i}_{j}.rs"), format!("c{i}"));
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(mgr.subscription_count(), 100);
        assert_eq!(mgr.subscriber_count(), 4);
    }
}
