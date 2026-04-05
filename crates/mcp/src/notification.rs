//! MCP notification system.
//!
//! Provides [`McpNotification`] for generic MCP notifications,
//! [`NotificationRouter`] for method-based dispatch, and
//! [`NotificationQueue`] for async buffering with backpressure.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Well-known MCP notification methods.
pub mod methods {
    pub const TOOLS_LIST_CHANGED: &str = "notifications/tools/list_changed";
    pub const RESOURCES_LIST_CHANGED: &str = "notifications/resources/list_changed";
    pub const PROMPTS_LIST_CHANGED: &str = "notifications/prompts/list_changed";
    pub const ROOTS_LIST_CHANGED: &str = "notifications/roots/list_changed";
    pub const CANCELLED: &str = "notifications/cancelled";
    pub const PROGRESS: &str = "notifications/progress";
    pub const MESSAGE: &str = "notifications/message";
}

/// A generic MCP notification (no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNotification {
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl McpNotification {
    /// Create a notification with no params.
    #[must_use]
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            params: None,
        }
    }

    /// Create a notification with params.
    #[must_use]
    pub fn with_params(method: impl Into<String>, params: Value) -> Self {
        Self {
            method: method.into(),
            params: Some(params),
        }
    }

    /// Create a `tools/list_changed` notification.
    #[must_use]
    pub fn tools_list_changed() -> Self {
        Self::new(methods::TOOLS_LIST_CHANGED)
    }

    /// Create a `resources/list_changed` notification.
    #[must_use]
    pub fn resources_list_changed() -> Self {
        Self::new(methods::RESOURCES_LIST_CHANGED)
    }

    /// Create a `prompts/list_changed` notification.
    #[must_use]
    pub fn prompts_list_changed() -> Self {
        Self::new(methods::PROMPTS_LIST_CHANGED)
    }

    /// Create a `roots/list_changed` notification.
    #[must_use]
    pub fn roots_list_changed() -> Self {
        Self::new(methods::ROOTS_LIST_CHANGED)
    }
}

/// Trait for handling a specific notification method.
pub trait NotificationHandler: Send + Sync {
    /// Handle a notification, returning an error message on failure.
    fn handle(&self, params: Option<&Value>) -> Result<(), String>;
}

/// A simple handler that records notifications for testing.
pub struct RecordingHandler {
    recorded: Arc<Mutex<Vec<Option<Value>>>>,
}

impl RecordingHandler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            recorded: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[must_use]
    pub fn recorded(&self) -> Vec<Option<Value>> {
        self.recorded.lock().unwrap().clone()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.recorded.lock().unwrap().len()
    }
}

impl Default for RecordingHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationHandler for RecordingHandler {
    fn handle(&self, params: Option<&Value>) -> Result<(), String> {
        self.recorded.lock().unwrap().push(params.cloned());
        Ok(())
    }
}

/// Routes notifications to registered handlers by method name.
pub struct NotificationRouter {
    handlers: HashMap<String, Arc<dyn NotificationHandler>>,
}

impl NotificationRouter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a specific method.
    pub fn register(&mut self, method: impl Into<String>, handler: Arc<dyn NotificationHandler>) {
        self.handlers.insert(method.into(), handler);
    }

    /// Route a notification to its handler.
    pub fn route(&self, notification: &McpNotification) -> Result<(), String> {
        self.handlers.get(&notification.method).map_or(
            Ok(()), // Unhandled notifications are silently ignored per MCP spec
            |handler| handler.handle(notification.params.as_ref()),
        )
    }

    /// Check if a handler is registered for a method.
    #[must_use]
    pub fn has_handler(&self, method: &str) -> bool {
        self.handlers.contains_key(method)
    }

    /// Number of registered handlers.
    #[must_use]
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for NotificationRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Bounded notification queue with backpressure.
#[derive(Debug)]
pub struct NotificationQueue {
    inner: Arc<Mutex<QueueInner>>,
}

#[derive(Debug)]
struct QueueInner {
    queue: Vec<McpNotification>,
    capacity: usize,
    dropped: u64,
}

impl NotificationQueue {
    /// Create a queue with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(QueueInner {
                queue: Vec::new(),
                capacity,
                dropped: 0,
            })),
        }
    }

    /// Enqueue a notification. Returns false if the queue is full (backpressure).
    pub fn push(&self, notification: McpNotification) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.queue.len() >= inner.capacity {
            inner.dropped += 1;
            return false;
        }
        inner.queue.push(notification);
        true
    }

    /// Dequeue the next notification.
    #[must_use]
    pub fn pop(&self) -> Option<McpNotification> {
        let mut inner = self.inner.lock().unwrap();
        if inner.queue.is_empty() {
            None
        } else {
            Some(inner.queue.remove(0))
        }
    }

    /// Drain all pending notifications.
    pub fn drain(&self) -> Vec<McpNotification> {
        let mut inner = self.inner.lock().unwrap();
        std::mem::take(&mut inner.queue)
    }

    /// Number of pending notifications.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().queue.len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().queue.is_empty()
    }

    /// Number of notifications dropped due to backpressure.
    #[must_use]
    pub fn dropped_count(&self) -> u64 {
        self.inner.lock().unwrap().dropped
    }
}

impl Clone for NotificationQueue {
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
    fn notification_new() {
        let n = McpNotification::new("test/method");
        assert_eq!(n.method, "test/method");
        assert!(n.params.is_none());
    }

    #[test]
    fn notification_with_params() {
        let n = McpNotification::with_params("test", serde_json::json!({"key": "val"}));
        assert!(n.params.is_some());
    }

    #[test]
    fn notification_factories() {
        assert_eq!(
            McpNotification::tools_list_changed().method,
            methods::TOOLS_LIST_CHANGED
        );
        assert_eq!(
            McpNotification::resources_list_changed().method,
            methods::RESOURCES_LIST_CHANGED
        );
        assert_eq!(
            McpNotification::prompts_list_changed().method,
            methods::PROMPTS_LIST_CHANGED
        );
        assert_eq!(
            McpNotification::roots_list_changed().method,
            methods::ROOTS_LIST_CHANGED
        );
    }

    #[test]
    fn notification_serde_roundtrip() {
        let n = McpNotification::with_params("test", serde_json::json!(42));
        let json = serde_json::to_string(&n).unwrap();
        let back: McpNotification = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, "test");
        assert_eq!(back.params, Some(serde_json::json!(42)));
    }

    #[test]
    fn recording_handler() {
        let h = RecordingHandler::new();
        h.handle(Some(&serde_json::json!("a"))).unwrap();
        h.handle(None).unwrap();
        assert_eq!(h.count(), 2);
        let recorded = h.recorded();
        assert_eq!(recorded[0], Some(serde_json::json!("a")));
        assert_eq!(recorded[1], None);
    }

    #[test]
    fn router_dispatch() {
        let mut router = NotificationRouter::new();
        let handler = Arc::new(RecordingHandler::new());
        router.register(
            "test/method",
            Arc::clone(&handler) as Arc<dyn NotificationHandler>,
        );

        let n = McpNotification::new("test/method");
        router.route(&n).unwrap();
        assert_eq!(handler.count(), 1);
        assert!(router.has_handler("test/method"));
        assert!(!router.has_handler("other"));
    }

    #[test]
    fn router_unknown_method_ok() {
        let router = NotificationRouter::new();
        let n = McpNotification::new("unknown/method");
        // Should not error — unhandled notifications are silently ignored
        assert!(router.route(&n).is_ok());
    }

    #[test]
    fn router_handler_count() {
        let mut router = NotificationRouter::new();
        assert_eq!(router.handler_count(), 0);
        router.register("a", Arc::new(RecordingHandler::new()));
        router.register("b", Arc::new(RecordingHandler::new()));
        assert_eq!(router.handler_count(), 2);
    }

    #[test]
    fn queue_push_pop() {
        let q = NotificationQueue::new(10);
        assert!(q.is_empty());
        q.push(McpNotification::new("a"));
        q.push(McpNotification::new("b"));
        assert_eq!(q.len(), 2);
        let n = q.pop().unwrap();
        assert_eq!(n.method, "a");
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn queue_backpressure() {
        let q = NotificationQueue::new(2);
        assert!(q.push(McpNotification::new("a")));
        assert!(q.push(McpNotification::new("b")));
        assert!(!q.push(McpNotification::new("c"))); // dropped
        assert_eq!(q.dropped_count(), 1);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn queue_drain() {
        let q = NotificationQueue::new(10);
        q.push(McpNotification::new("a"));
        q.push(McpNotification::new("b"));
        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        assert!(q.is_empty());
    }

    #[test]
    fn queue_thread_safe() {
        let q = NotificationQueue::new(1000);
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let q = q.clone();
                std::thread::spawn(move || {
                    for j in 0..25 {
                        q.push(McpNotification::new(format!("{i}-{j}")));
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(q.len(), 100);
    }
}
