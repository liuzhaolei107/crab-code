//! Batch API support and request queue with concurrency control.
//!
//! Provides:
//! - `BatchRequest` / `BatchResponse` — types for Anthropic/OpenAI batch APIs
//! - `RequestQueue` — concurrency-limited request queue using a semaphore

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use crate::types::MessageResponse;

// ─── Batch API types ───

/// A single request within a batch.
#[derive(Debug, Clone)]
pub struct BatchItem {
    /// Caller-assigned ID for correlation.
    pub custom_id: String,
    /// The message request to execute.
    pub model: String,
    pub messages_json: serde_json::Value,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
}

/// Status of a batch job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStatus {
    /// Batch has been created but not yet started.
    Pending,
    /// Batch is currently being processed.
    InProgress,
    /// Batch completed successfully.
    Completed,
    /// Batch failed.
    Failed,
    /// Batch was cancelled.
    Cancelled,
    /// Batch has expired.
    Expired,
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// A batch API request containing multiple items.
#[derive(Debug, Clone)]
pub struct BatchRequest {
    /// Items in the batch.
    pub items: Vec<BatchItem>,
    /// Optional metadata for tracking.
    pub metadata: Option<serde_json::Value>,
}

impl BatchRequest {
    /// Create a new empty batch request.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            metadata: None,
        }
    }

    /// Add an item to the batch.
    pub fn add(&mut self, item: BatchItem) {
        self.items.push(item);
    }

    /// Number of items in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl Default for BatchRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single item within a batch response.
#[derive(Debug, Clone)]
pub struct BatchItemResult {
    /// The `custom_id` from the request item.
    pub custom_id: String,
    /// The response, if successful.
    pub response: Option<MessageResponse>,
    /// Error message, if failed.
    pub error: Option<String>,
}

impl BatchItemResult {
    /// Whether this item succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.response.is_some() && self.error.is_none()
    }
}

/// Response from a batch API call.
#[derive(Debug, Clone)]
pub struct BatchResponse {
    /// Provider-assigned batch ID.
    pub batch_id: String,
    /// Current status.
    pub status: BatchStatus,
    /// Individual results (populated when completed).
    pub results: Vec<BatchItemResult>,
    /// Number of completed items.
    pub completed_count: usize,
    /// Number of failed items.
    pub failed_count: usize,
    /// Total items in the batch.
    pub total_count: usize,
}

impl BatchResponse {
    /// Whether the batch has finished (completed, failed, cancelled, or expired).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            BatchStatus::Completed
                | BatchStatus::Failed
                | BatchStatus::Cancelled
                | BatchStatus::Expired
        )
    }

    /// Whether all items in the batch succeeded.
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.status == BatchStatus::Completed && self.failed_count == 0
    }
}

// ─── Request Queue ───

/// Queued request awaiting execution.
#[derive(Debug)]
pub struct QueuedRequest {
    /// Unique ID for this queued request.
    pub id: u64,
    /// The model to use.
    pub model: String,
    /// Serialized request (we store JSON to avoid lifetime issues).
    pub request_json: serde_json::Value,
    /// Priority (lower = higher priority).
    pub priority: u32,
}

/// Concurrency-limited request queue.
///
/// Uses a `Semaphore` to cap the number of concurrent API requests.
/// Requests are queued in FIFO order when the concurrency limit is reached.
pub struct RequestQueue {
    /// Maximum concurrent requests.
    max_concurrent: usize,
    /// Semaphore controlling concurrency.
    semaphore: Arc<Semaphore>,
    /// Pending requests waiting for a permit.
    pending: Mutex<VecDeque<QueuedRequest>>,
    /// Counter for generating unique request IDs.
    next_id: Mutex<u64>,
}

impl RequestQueue {
    /// Create a new request queue with the given concurrency limit.
    #[must_use]
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            pending: Mutex::new(VecDeque::new()),
            next_id: Mutex::new(1),
        }
    }

    /// Enqueue a request. Returns the assigned queue ID.
    pub fn enqueue(&self, model: String, request_json: serde_json::Value, priority: u32) -> u64 {
        let id = {
            let mut next = self.next_id.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let id = *next;
            *next += 1;
            id
        };

        let mut pending = self.pending.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let req = QueuedRequest {
            id,
            model,
            request_json,
            priority,
        };

        // Insert by priority (lower priority value = higher priority)
        let pos = pending
            .iter()
            .position(|r| r.priority > priority)
            .unwrap_or(pending.len());
        pending.insert(pos, req);

        id
    }

    /// Acquire a concurrency permit. Blocks until a slot is available.
    ///
    /// The returned `QueuePermit` must be held for the duration of the request.
    /// Dropping it releases the slot for the next queued request.
    pub async fn acquire(&self) -> QueuePermit {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore closed");
        QueuePermit { _permit: permit }
    }

    /// Try to acquire a permit without waiting.
    ///
    /// Returns `None` if all slots are occupied.
    #[must_use]
    pub fn try_acquire(&self) -> Option<QueuePermit> {
        self.semaphore
            .clone()
            .try_acquire_owned()
            .ok()
            .map(|permit| QueuePermit { _permit: permit })
    }

    /// Dequeue the next pending request (highest priority first).
    pub fn dequeue(&self) -> Option<QueuedRequest> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
    }

    /// Number of requests waiting in the queue.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.lock().map_or(0, |p| p.len())
    }

    /// Number of available concurrency slots.
    #[must_use]
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Maximum concurrent requests.
    #[must_use]
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Number of currently active (in-flight) requests.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.max_concurrent - self.semaphore.available_permits()
    }

    /// Whether the queue is at capacity (all permits taken).
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.semaphore.available_permits() == 0
    }

    /// Cancel a pending request by ID. Returns true if found and removed.
    pub fn cancel(&self, id: u64) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(pos) = pending.iter().position(|r| r.id == id) {
            pending.remove(pos);
            return true;
        }
        false
    }

    /// Clear all pending requests. Returns the number removed.
    pub fn clear(&self) -> usize {
        let mut pending = self.pending.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let count = pending.len();
        pending.clear();
        count
    }
}

/// RAII permit for a concurrency slot. Dropping releases the slot.
pub struct QueuePermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::message::Message;
    use crab_core::model::TokenUsage;

    // ─── BatchRequest tests ───

    #[test]
    fn batch_request_new_is_empty() {
        let batch = BatchRequest::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn batch_request_add_items() {
        let mut batch = BatchRequest::new();
        batch.add(BatchItem {
            custom_id: "req_1".into(),
            model: "claude-sonnet-4-20250514".into(),
            messages_json: serde_json::json!([{"role": "user", "content": "hi"}]),
            system: None,
            max_tokens: 1024,
            temperature: None,
        });
        batch.add(BatchItem {
            custom_id: "req_2".into(),
            model: "claude-sonnet-4-20250514".into(),
            messages_json: serde_json::json!([{"role": "user", "content": "hello"}]),
            system: Some("Be helpful".into()),
            max_tokens: 2048,
            temperature: Some(0.7),
        });
        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
    }

    #[test]
    fn batch_request_with_metadata() {
        let batch = BatchRequest::new().with_metadata(serde_json::json!({"tag": "test"}));
        assert!(batch.metadata.is_some());
    }

    #[test]
    fn batch_request_default() {
        let batch = BatchRequest::default();
        assert!(batch.is_empty());
    }

    // ─── BatchStatus tests ───

    #[test]
    fn batch_status_display() {
        assert_eq!(BatchStatus::Pending.to_string(), "pending");
        assert_eq!(BatchStatus::InProgress.to_string(), "in_progress");
        assert_eq!(BatchStatus::Completed.to_string(), "completed");
        assert_eq!(BatchStatus::Failed.to_string(), "failed");
        assert_eq!(BatchStatus::Cancelled.to_string(), "cancelled");
        assert_eq!(BatchStatus::Expired.to_string(), "expired");
    }

    // ─── BatchResponse tests ───

    #[test]
    fn batch_response_is_terminal() {
        let resp = BatchResponse {
            batch_id: "batch_01".into(),
            status: BatchStatus::Completed,
            results: vec![],
            completed_count: 1,
            failed_count: 0,
            total_count: 1,
        };
        assert!(resp.is_terminal());
        assert!(resp.all_succeeded());
    }

    #[test]
    fn batch_response_pending_not_terminal() {
        let resp = BatchResponse {
            batch_id: "batch_01".into(),
            status: BatchStatus::Pending,
            results: vec![],
            completed_count: 0,
            failed_count: 0,
            total_count: 1,
        };
        assert!(!resp.is_terminal());
        assert!(!resp.all_succeeded());
    }

    #[test]
    fn batch_response_in_progress_not_terminal() {
        let resp = BatchResponse {
            batch_id: "batch_01".into(),
            status: BatchStatus::InProgress,
            results: vec![],
            completed_count: 0,
            failed_count: 0,
            total_count: 2,
        };
        assert!(!resp.is_terminal());
    }

    #[test]
    fn batch_response_failed_is_terminal() {
        let resp = BatchResponse {
            batch_id: "batch_01".into(),
            status: BatchStatus::Failed,
            results: vec![],
            completed_count: 0,
            failed_count: 1,
            total_count: 1,
        };
        assert!(resp.is_terminal());
        assert!(!resp.all_succeeded());
    }

    #[test]
    fn batch_response_with_failures_not_all_succeeded() {
        let resp = BatchResponse {
            batch_id: "batch_01".into(),
            status: BatchStatus::Completed,
            results: vec![],
            completed_count: 1,
            failed_count: 1,
            total_count: 2,
        };
        assert!(!resp.all_succeeded());
    }

    // ─── BatchItemResult tests ───

    #[test]
    fn batch_item_result_success() {
        let result = BatchItemResult {
            custom_id: "req_1".into(),
            response: Some(MessageResponse {
                id: "msg_01".into(),
                message: Message::assistant("Hello!"),
                usage: TokenUsage::default(),
            }),
            error: None,
        };
        assert!(result.is_success());
    }

    #[test]
    fn batch_item_result_error() {
        let result = BatchItemResult {
            custom_id: "req_1".into(),
            response: None,
            error: Some("rate limited".into()),
        };
        assert!(!result.is_success());
    }

    // ─── RequestQueue tests ───

    #[test]
    fn queue_new_state() {
        let queue = RequestQueue::new(5);
        assert_eq!(queue.max_concurrent(), 5);
        assert_eq!(queue.available_permits(), 5);
        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.active_count(), 0);
        assert!(!queue.is_full());
    }

    #[test]
    fn queue_enqueue_and_dequeue() {
        let queue = RequestQueue::new(5);
        let id = queue.enqueue("model".into(), serde_json::json!({}), 0);
        assert_eq!(id, 1);
        assert_eq!(queue.pending_count(), 1);

        let req = queue.dequeue().unwrap();
        assert_eq!(req.id, 1);
        assert_eq!(req.model, "model");
        assert_eq!(queue.pending_count(), 0);
    }

    #[test]
    fn queue_priority_ordering() {
        let queue = RequestQueue::new(5);
        queue.enqueue("low".into(), serde_json::json!({}), 10);
        queue.enqueue("high".into(), serde_json::json!({}), 1);
        queue.enqueue("medium".into(), serde_json::json!({}), 5);

        let first = queue.dequeue().unwrap();
        assert_eq!(first.model, "high");

        let second = queue.dequeue().unwrap();
        assert_eq!(second.model, "medium");

        let third = queue.dequeue().unwrap();
        assert_eq!(third.model, "low");
    }

    #[test]
    fn queue_cancel_pending() {
        let queue = RequestQueue::new(5);
        let id1 = queue.enqueue("a".into(), serde_json::json!({}), 0);
        let _id2 = queue.enqueue("b".into(), serde_json::json!({}), 0);

        assert!(queue.cancel(id1));
        assert_eq!(queue.pending_count(), 1);

        let req = queue.dequeue().unwrap();
        assert_eq!(req.model, "b");
    }

    #[test]
    fn queue_cancel_nonexistent() {
        let queue = RequestQueue::new(5);
        assert!(!queue.cancel(999));
    }

    #[test]
    fn queue_clear() {
        let queue = RequestQueue::new(5);
        queue.enqueue("a".into(), serde_json::json!({}), 0);
        queue.enqueue("b".into(), serde_json::json!({}), 0);
        queue.enqueue("c".into(), serde_json::json!({}), 0);

        let cleared = queue.clear();
        assert_eq!(cleared, 3);
        assert_eq!(queue.pending_count(), 0);
    }

    #[tokio::test]
    async fn queue_acquire_and_release() {
        let queue = RequestQueue::new(2);

        let p1 = queue.acquire().await;
        assert_eq!(queue.active_count(), 1);
        assert_eq!(queue.available_permits(), 1);

        let p2 = queue.acquire().await;
        assert_eq!(queue.active_count(), 2);
        assert!(queue.is_full());

        drop(p1);
        assert_eq!(queue.active_count(), 1);
        assert!(!queue.is_full());

        drop(p2);
        assert_eq!(queue.active_count(), 0);
    }

    #[test]
    fn queue_try_acquire() {
        let queue = RequestQueue::new(1);

        let p1 = queue.try_acquire();
        assert!(p1.is_some());

        let p2 = queue.try_acquire();
        assert!(p2.is_none()); // full

        drop(p1);
        let p3 = queue.try_acquire();
        assert!(p3.is_some());
    }

    #[test]
    fn queue_dequeue_empty() {
        let queue = RequestQueue::new(5);
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn queue_unique_ids() {
        let queue = RequestQueue::new(5);
        let id1 = queue.enqueue("a".into(), serde_json::json!({}), 0);
        let id2 = queue.enqueue("b".into(), serde_json::json!({}), 0);
        let id3 = queue.enqueue("c".into(), serde_json::json!({}), 0);
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }

    #[test]
    fn queue_same_priority_fifo() {
        let queue = RequestQueue::new(5);
        queue.enqueue("first".into(), serde_json::json!({}), 5);
        queue.enqueue("second".into(), serde_json::json!({}), 5);
        queue.enqueue("third".into(), serde_json::json!({}), 5);

        assert_eq!(queue.dequeue().unwrap().model, "first");
        assert_eq!(queue.dequeue().unwrap().model, "second");
        assert_eq!(queue.dequeue().unwrap().model, "third");
    }
}
