//! Request deduplication — coalesces concurrent identical API requests.
//!
//! When multiple callers issue the same request concurrently, only the first
//! one is actually sent to the API. All callers receive a clone of the result.
//! Uses `tokio::sync::broadcast` to fan out the response.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::types::{MessageRequest, MessageResponse};

/// Deduplication tracker for in-flight API requests.
///
/// Thread-safe: the inner map is behind a `Mutex`.
pub struct RequestDedup {
    /// Map from request hash to a broadcast sender.
    /// When the first request completes, it sends the result on the channel.
    in_flight: Arc<Mutex<HashMap<u64, broadcast::Sender<DedupResult>>>>,
}

/// Result wrapper for dedup broadcast (needs Clone for broadcast).
#[derive(Debug, Clone)]
pub enum DedupResult {
    /// The request succeeded.
    Success(MessageResponse),
    /// The request failed (error message).
    Error(String),
}

/// What the caller should do after checking dedup.
pub enum DedupAction {
    /// This caller should execute the request. The `DedupGuard` must be
    /// used to publish the result when done.
    Execute(DedupGuard),
    /// Another caller is already executing this request. Wait on the receiver.
    Wait(broadcast::Receiver<DedupResult>),
}

/// Guard that must be used to publish the result of a deduplicated request.
///
/// If dropped without calling `complete` or `fail`, broadcasts an error
/// to any waiting receivers and cleans up the in-flight entry.
pub struct DedupGuard {
    key: u64,
    tx: broadcast::Sender<DedupResult>,
    in_flight: Arc<Mutex<HashMap<u64, broadcast::Sender<DedupResult>>>>,
    completed: bool,
}

impl DedupGuard {
    /// Publish a successful response to all waiters.
    pub fn complete(mut self, response: MessageResponse) {
        let _ = self.tx.send(DedupResult::Success(response));
        self.cleanup();
        self.completed = true;
    }

    /// Publish a failure to all waiters.
    pub fn fail(mut self, error: String) {
        let _ = self.tx.send(DedupResult::Error(error));
        self.cleanup();
        self.completed = true;
    }

    fn cleanup(&self) {
        if let Ok(mut map) = self.in_flight.lock() {
            map.remove(&self.key);
        }
    }
}

impl Drop for DedupGuard {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.tx.send(DedupResult::Error("request cancelled".into()));
            self.cleanup();
        }
    }
}

impl RequestDedup {
    /// Create a new dedup tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a request is already in-flight.
    ///
    /// Returns `DedupAction::Execute` if this is the first request (caller should
    /// execute and use the guard to publish results), or `DedupAction::Wait` if
    /// another caller is already executing (caller should wait on the receiver).
    pub fn check(&self, req: &MessageRequest<'_>) -> DedupAction {
        let key = request_dedup_hash(req);
        let in_flight_arc = Arc::clone(&self.in_flight);
        let result = self
            .in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .map_or_else(
                || {
                    // First request — register and return execute action
                    let (tx, _rx) = broadcast::channel(1);
                    self.in_flight
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(key, tx.clone());
                    Err((key, tx, Arc::clone(&in_flight_arc)))
                },
                |tx| Ok(tx.subscribe()),
            );
        match result {
            Ok(rx) => DedupAction::Wait(rx),
            Err((k, tx, in_flight)) => DedupAction::Execute(DedupGuard {
                key: k,
                tx,
                in_flight,
                completed: false,
            }),
        }
    }

    /// Number of currently in-flight deduplicated requests.
    #[must_use]
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.lock().map_or(0, |m| m.len())
    }

    /// Whether there are any in-flight requests.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.in_flight_count() == 0
    }
}

impl Default for RequestDedup {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a request for dedup purposes.
///
/// Same semantic hashing as `response_cache::request_hash` but kept separate
/// to allow independent evolution.
fn request_dedup_hash(req: &MessageRequest<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();

    req.model.as_str().hash(&mut hasher);

    for msg in req.messages.as_ref() {
        msg.text().hash(&mut hasher);
    }

    if let Some(sys) = &req.system {
        sys.hash(&mut hasher);
    }

    for tool in &req.tools {
        tool.to_string().hash(&mut hasher);
    }

    if let Some(temp) = req.temperature {
        temp.to_bits().hash(&mut hasher);
    }

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::message::Message;
    use crab_core::model::{ModelId, TokenUsage};
    use std::borrow::Cow;

    fn test_request(msg: &str) -> MessageRequest<'static> {
        MessageRequest {
            model: ModelId::from("test-model"),
            messages: Cow::Owned(vec![Message::user(msg)]),
            system: Some("sys".into()),
            max_tokens: 1024,
            tools: vec![],
            temperature: None,
            cache_breakpoints: vec![],
        }
    }

    fn test_response(text: &str) -> MessageResponse {
        MessageResponse {
            id: "msg_01".into(),
            message: Message::assistant(text),
            usage: TokenUsage::default(),
        }
    }

    #[test]
    fn dedup_new_is_idle() {
        let dedup = RequestDedup::new();
        assert!(dedup.is_idle());
        assert_eq!(dedup.in_flight_count(), 0);
    }

    #[test]
    fn dedup_first_request_returns_execute() {
        let dedup = RequestDedup::new();
        let req = test_request("hello");

        let action = dedup.check(&req);
        assert!(matches!(action, DedupAction::Execute(_)));
        assert_eq!(dedup.in_flight_count(), 1);

        // Drop the guard to clean up
        if let DedupAction::Execute(guard) = action {
            guard.complete(test_response("world"));
        }
        assert!(dedup.is_idle());
    }

    #[test]
    fn dedup_second_request_returns_wait() {
        let dedup = RequestDedup::new();
        let req = test_request("hello");

        let action1 = dedup.check(&req);
        assert!(matches!(action1, DedupAction::Execute(_)));

        let action2 = dedup.check(&req);
        assert!(matches!(action2, DedupAction::Wait(_)));

        assert_eq!(dedup.in_flight_count(), 1);

        // Complete the first request
        if let DedupAction::Execute(guard) = action1 {
            guard.complete(test_response("world"));
        }
    }

    #[test]
    fn dedup_different_requests_both_execute() {
        let dedup = RequestDedup::new();
        let req1 = test_request("hello");
        let req2 = test_request("goodbye");

        let action1 = dedup.check(&req1);
        let action2 = dedup.check(&req2);

        assert!(matches!(action1, DedupAction::Execute(_)));
        assert!(matches!(action2, DedupAction::Execute(_)));
        assert_eq!(dedup.in_flight_count(), 2);
    }

    #[tokio::test]
    async fn dedup_waiter_receives_result() {
        let dedup = RequestDedup::new();
        let req = test_request("hello");

        let action1 = dedup.check(&req);
        let action2 = dedup.check(&req);

        let guard = match action1 {
            DedupAction::Execute(g) => g,
            _ => panic!("expected Execute"),
        };

        let mut rx = match action2 {
            DedupAction::Wait(r) => r,
            _ => panic!("expected Wait"),
        };

        // Complete the request
        guard.complete(test_response("world"));

        // Waiter should receive the result
        let result = rx.recv().await.unwrap();
        match result {
            DedupResult::Success(resp) => assert_eq!(resp.message.text(), "world"),
            DedupResult::Error(_) => panic!("expected success"),
        }
    }

    #[tokio::test]
    async fn dedup_failure_propagates() {
        let dedup = RequestDedup::new();
        let req = test_request("hello");

        let action1 = dedup.check(&req);
        let action2 = dedup.check(&req);

        let guard = match action1 {
            DedupAction::Execute(g) => g,
            _ => panic!("expected Execute"),
        };

        let mut rx = match action2 {
            DedupAction::Wait(r) => r,
            _ => panic!("expected Wait"),
        };

        guard.fail("timeout".into());

        let result = rx.recv().await.unwrap();
        assert!(matches!(result, DedupResult::Error(msg) if msg == "timeout"));
    }

    #[test]
    fn dedup_guard_drop_cleans_up() {
        let dedup = RequestDedup::new();
        let req = test_request("hello");

        {
            let action = dedup.check(&req);
            assert!(matches!(action, DedupAction::Execute(_)));
            assert_eq!(dedup.in_flight_count(), 1);
            // Guard drops here
        }

        assert!(dedup.is_idle());
    }

    #[test]
    fn dedup_after_completion_allows_new_execute() {
        let dedup = RequestDedup::new();
        let req = test_request("hello");

        // First round
        let action = dedup.check(&req);
        if let DedupAction::Execute(guard) = action {
            guard.complete(test_response("r1"));
        }

        // Second round — should get Execute again
        let action = dedup.check(&req);
        assert!(matches!(action, DedupAction::Execute(_)));
    }

    #[test]
    fn dedup_hash_deterministic() {
        let req = test_request("hello");
        let h1 = request_dedup_hash(&req);
        let h2 = request_dedup_hash(&req);
        assert_eq!(h1, h2);
    }

    #[test]
    fn dedup_hash_different_messages() {
        let req1 = test_request("hello");
        let req2 = test_request("goodbye");
        assert_ne!(request_dedup_hash(&req1), request_dedup_hash(&req2));
    }

    #[test]
    fn dedup_default_trait() {
        let dedup = RequestDedup::default();
        assert!(dedup.is_idle());
    }
}
