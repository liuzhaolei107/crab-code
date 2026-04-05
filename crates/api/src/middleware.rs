//! Request/response middleware pipeline for the API layer.
//!
//! Middleware can inspect and modify requests before they are sent and
//! responses after they are received. Middleware is executed in order
//! for requests and in reverse order for responses (onion model).

use std::sync::Arc;
use std::time::Instant;

use crate::types::{MessageRequest, MessageResponse};

/// Context passed through the middleware chain.
///
/// Middleware can attach metadata to the context for downstream middleware
/// or for the caller to inspect after the chain completes.
#[derive(Debug, Clone, Default)]
pub struct MiddlewareContext {
    /// Key-value metadata that middleware can read and write.
    pub metadata: std::collections::HashMap<String, String>,
    /// Whether the chain should be short-circuited (skip remaining middleware).
    pub short_circuit: bool,
    /// A cached/synthetic response to return instead of calling the backend.
    pub cached_response: Option<MessageResponse>,
    /// Timing information set by the pipeline.
    pub request_start: Option<Instant>,
}

impl MiddlewareContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a metadata value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get a metadata value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(String::as_str)
    }

    /// Short-circuit the chain with a cached response.
    pub fn short_circuit_with(&mut self, response: MessageResponse) {
        self.short_circuit = true;
        self.cached_response = Some(response);
    }
}

/// Result type for middleware operations.
pub type MiddlewareResult<T> = std::result::Result<T, MiddlewareError>;

/// Errors that can occur in middleware.
#[derive(Debug, Clone)]
pub struct MiddlewareError {
    /// Which middleware caused the error.
    pub middleware_name: String,
    /// Error message.
    pub message: String,
}

impl std::fmt::Display for MiddlewareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.middleware_name, self.message)
    }
}

impl std::error::Error for MiddlewareError {}

/// Trait for request/response middleware.
///
/// Middleware is called in order for `on_request` and in reverse order
/// for `on_response` (onion model, like tower middleware).
pub trait Middleware: Send + Sync {
    /// Middleware name for logging and error attribution.
    fn name(&self) -> &str;

    /// Called before the request is sent to the backend.
    ///
    /// Can modify the context (e.g., add metadata, short-circuit).
    /// The request itself is passed by reference for inspection.
    fn on_request(
        &self,
        _request: &MessageRequest<'_>,
        _ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        Ok(())
    }

    /// Called after the response is received from the backend.
    ///
    /// Can inspect or modify the context. The response is passed by reference.
    fn on_response(
        &self,
        _response: &MessageResponse,
        _ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        Ok(())
    }

    /// Called when an error occurs during the request.
    fn on_error(&self, _error: &str, _ctx: &mut MiddlewareContext) -> MiddlewareResult<()> {
        Ok(())
    }
}

/// An ordered pipeline of middleware.
pub struct MiddlewarePipeline {
    middleware: Vec<Arc<dyn Middleware>>,
}

impl MiddlewarePipeline {
    /// Create an empty pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    /// Add middleware to the end of the pipeline.
    pub fn add(&mut self, m: Arc<dyn Middleware>) {
        self.middleware.push(m);
    }

    /// Number of middleware in the pipeline.
    #[must_use]
    pub fn len(&self) -> usize {
        self.middleware.len()
    }

    /// Whether the pipeline is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }

    /// Execute the request phase of the pipeline.
    ///
    /// Calls `on_request` on each middleware in order. Stops early if
    /// any middleware sets `short_circuit` on the context.
    pub fn process_request(
        &self,
        request: &MessageRequest<'_>,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        ctx.request_start = Some(Instant::now());

        for m in &self.middleware {
            m.on_request(request, ctx)?;
            if ctx.short_circuit {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Execute the response phase of the pipeline.
    ///
    /// Calls `on_response` on each middleware in reverse order.
    pub fn process_response(
        &self,
        response: &MessageResponse,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        for m in self.middleware.iter().rev() {
            m.on_response(response, ctx)?;
        }
        Ok(())
    }

    /// Execute the error phase of the pipeline.
    pub fn process_error(&self, error: &str, ctx: &mut MiddlewareContext) -> MiddlewareResult<()> {
        for m in self.middleware.iter().rev() {
            m.on_error(error, ctx)?;
        }
        Ok(())
    }

    /// Get middleware names in order.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.middleware.iter().map(|m| m.name()).collect()
    }
}

impl Default for MiddlewarePipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Built-in middleware ───

/// Logs request and response metadata to the context.
pub struct LoggingMiddleware {
    /// Log level label (informational only, no real logging dependency).
    pub level: String,
}

impl LoggingMiddleware {
    #[must_use]
    pub fn new(level: impl Into<String>) -> Self {
        Self {
            level: level.into(),
        }
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new("info")
    }
}

impl Middleware for LoggingMiddleware {
    fn name(&self) -> &'static str {
        "logging"
    }

    fn on_request(
        &self,
        request: &MessageRequest<'_>,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        ctx.set("log_level", &self.level);
        ctx.set("log_model", request.model.as_str());
        ctx.set("log_message_count", request.messages.len().to_string());
        Ok(())
    }

    fn on_response(
        &self,
        response: &MessageResponse,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        ctx.set("log_response_id", &response.id);
        ctx.set(
            "log_output_tokens",
            response.usage.output_tokens.to_string(),
        );
        Ok(())
    }

    fn on_error(&self, error: &str, ctx: &mut MiddlewareContext) -> MiddlewareResult<()> {
        ctx.set("log_error", error);
        Ok(())
    }
}

/// Records request timing and token metrics.
pub struct MetricsMiddleware {
    /// Whether to record detailed per-request metrics.
    pub detailed: bool,
}

impl MetricsMiddleware {
    #[must_use]
    pub fn new(detailed: bool) -> Self {
        Self { detailed }
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Middleware for MetricsMiddleware {
    fn name(&self) -> &'static str {
        "metrics"
    }

    fn on_request(
        &self,
        request: &MessageRequest<'_>,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        ctx.set("metrics_model", request.model.as_str());
        ctx.set("metrics_tool_count", request.tools.len().to_string());
        if self.detailed {
            ctx.set("metrics_message_count", request.messages.len().to_string());
        }
        Ok(())
    }

    fn on_response(
        &self,
        response: &MessageResponse,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        if let Some(start) = ctx.request_start {
            let elapsed = start.elapsed();
            ctx.set("metrics_latency_ms", elapsed.as_millis().to_string());
        }
        ctx.set(
            "metrics_input_tokens",
            response.usage.input_tokens.to_string(),
        );
        ctx.set(
            "metrics_output_tokens",
            response.usage.output_tokens.to_string(),
        );
        ctx.set("metrics_total_tokens", response.usage.total().to_string());
        Ok(())
    }
}

/// Middleware that short-circuits with a cached response when the context
/// has a `cache_hit` metadata key set to `"true"`.
///
/// This is a building block — the actual cache lookup should be done by
/// a prior middleware or by the caller before entering the pipeline.
pub struct CacheMiddleware;

impl Middleware for CacheMiddleware {
    fn name(&self) -> &'static str {
        "cache"
    }

    fn on_request(
        &self,
        _request: &MessageRequest<'_>,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        // If the caller pre-populated a cached response, short-circuit
        if ctx.cached_response.is_some() {
            ctx.short_circuit = true;
            ctx.set("cache_status", "hit");
        } else {
            ctx.set("cache_status", "miss");
        }
        Ok(())
    }
}

/// Middleware that enforces a maximum request rate by checking context metadata.
///
/// This is a policy check — the actual rate tracking is external.
/// If `rate_limit_remaining` metadata is "0", it produces an error.
pub struct RateLimitMiddleware;

impl Middleware for RateLimitMiddleware {
    fn name(&self) -> &'static str {
        "rate_limit"
    }

    fn on_request(
        &self,
        _request: &MessageRequest<'_>,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        if ctx.get("rate_limit_remaining") == Some("0") {
            return Err(MiddlewareError {
                middleware_name: "rate_limit".into(),
                message: "rate limit exceeded".into(),
            });
        }
        Ok(())
    }
}

/// Conditional middleware wrapper — only runs the inner middleware if the
/// predicate returns true.
pub struct ConditionalMiddleware<F: Fn(&MiddlewareContext) -> bool + Send + Sync> {
    inner: Arc<dyn Middleware>,
    predicate: F,
}

impl<F: Fn(&MiddlewareContext) -> bool + Send + Sync> ConditionalMiddleware<F> {
    pub fn new(inner: Arc<dyn Middleware>, predicate: F) -> Self {
        Self { inner, predicate }
    }
}

impl<F: Fn(&MiddlewareContext) -> bool + Send + Sync> Middleware for ConditionalMiddleware<F> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn on_request(
        &self,
        request: &MessageRequest<'_>,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        if (self.predicate)(ctx) {
            self.inner.on_request(request, ctx)
        } else {
            Ok(())
        }
    }

    fn on_response(
        &self,
        response: &MessageResponse,
        ctx: &mut MiddlewareContext,
    ) -> MiddlewareResult<()> {
        if (self.predicate)(ctx) {
            self.inner.on_response(response, ctx)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::message::Message;
    use crab_core::model::{ModelId, TokenUsage};
    use std::borrow::Cow;

    fn test_request() -> MessageRequest<'static> {
        MessageRequest {
            model: ModelId::from("test-model"),
            messages: Cow::Owned(vec![Message::user("hello")]),
            system: Some("sys".into()),
            max_tokens: 1024,
            tools: vec![serde_json::json!({"name": "read"})],
            temperature: None,
            cache_breakpoints: vec![],
        }
    }

    fn test_response() -> MessageResponse {
        MessageResponse {
            id: "msg_01".into(),
            message: Message::assistant("world"),
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
            },
        }
    }

    // ─── MiddlewareContext tests ───

    #[test]
    fn context_new_is_empty() {
        let ctx = MiddlewareContext::new();
        assert!(ctx.metadata.is_empty());
        assert!(!ctx.short_circuit);
        assert!(ctx.cached_response.is_none());
    }

    #[test]
    fn context_set_and_get() {
        let mut ctx = MiddlewareContext::new();
        ctx.set("key", "value");
        assert_eq!(ctx.get("key"), Some("value"));
        assert!(ctx.get("missing").is_none());
    }

    #[test]
    fn context_short_circuit() {
        let mut ctx = MiddlewareContext::new();
        ctx.short_circuit_with(test_response());
        assert!(ctx.short_circuit);
        assert!(ctx.cached_response.is_some());
    }

    // ─── MiddlewarePipeline tests ───

    #[test]
    fn pipeline_new_is_empty() {
        let pipeline = MiddlewarePipeline::new();
        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    #[test]
    fn pipeline_add_middleware() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(LoggingMiddleware::default()));
        pipeline.add(Arc::new(MetricsMiddleware::default()));
        assert_eq!(pipeline.len(), 2);
        assert_eq!(pipeline.names(), vec!["logging", "metrics"]);
    }

    #[test]
    fn pipeline_process_request() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(LoggingMiddleware::default()));
        pipeline.add(Arc::new(MetricsMiddleware::default()));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        pipeline.process_request(&req, &mut ctx).unwrap();

        assert_eq!(ctx.get("log_model"), Some("test-model"));
        assert_eq!(ctx.get("log_message_count"), Some("1"));
        assert_eq!(ctx.get("metrics_model"), Some("test-model"));
        assert_eq!(ctx.get("metrics_tool_count"), Some("1"));
    }

    #[test]
    fn pipeline_process_response() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(LoggingMiddleware::default()));
        pipeline.add(Arc::new(MetricsMiddleware::default()));

        let resp = test_response();
        let mut ctx = MiddlewareContext::new();
        ctx.request_start = Some(Instant::now());
        pipeline.process_response(&resp, &mut ctx).unwrap();

        assert_eq!(ctx.get("log_response_id"), Some("msg_01"));
        assert_eq!(ctx.get("metrics_output_tokens"), Some("50"));
        assert_eq!(ctx.get("metrics_total_tokens"), Some("150"));
        assert!(ctx.get("metrics_latency_ms").is_some());
    }

    #[test]
    fn pipeline_process_error() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(LoggingMiddleware::default()));

        let mut ctx = MiddlewareContext::new();
        pipeline.process_error("timeout", &mut ctx).unwrap();

        assert_eq!(ctx.get("log_error"), Some("timeout"));
    }

    #[test]
    fn pipeline_short_circuit() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(CacheMiddleware));
        pipeline.add(Arc::new(MetricsMiddleware::default()));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        // Pre-populate cached response
        ctx.cached_response = Some(test_response());

        pipeline.process_request(&req, &mut ctx).unwrap();

        assert!(ctx.short_circuit);
        assert_eq!(ctx.get("cache_status"), Some("hit"));
        // MetricsMiddleware was skipped due to short-circuit
        assert!(ctx.get("metrics_model").is_none());
    }

    #[test]
    fn pipeline_cache_miss() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(CacheMiddleware));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();

        pipeline.process_request(&req, &mut ctx).unwrap();

        assert!(!ctx.short_circuit);
        assert_eq!(ctx.get("cache_status"), Some("miss"));
    }

    // ─── RateLimitMiddleware tests ───

    #[test]
    fn rate_limit_allows_when_remaining() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(RateLimitMiddleware));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        ctx.set("rate_limit_remaining", "10");

        pipeline.process_request(&req, &mut ctx).unwrap();
    }

    #[test]
    fn rate_limit_blocks_when_zero() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(RateLimitMiddleware));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        ctx.set("rate_limit_remaining", "0");

        let err = pipeline.process_request(&req, &mut ctx).unwrap_err();
        assert_eq!(err.middleware_name, "rate_limit");
        assert!(err.message.contains("rate limit exceeded"));
    }

    #[test]
    fn rate_limit_allows_when_no_metadata() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(RateLimitMiddleware));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();

        // No rate_limit_remaining set — should pass
        pipeline.process_request(&req, &mut ctx).unwrap();
    }

    // ─── ConditionalMiddleware tests ───

    #[test]
    fn conditional_runs_when_true() {
        let logging = Arc::new(LoggingMiddleware::default());
        let conditional =
            ConditionalMiddleware::new(logging, |ctx| ctx.get("enable_logging") == Some("true"));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        ctx.set("enable_logging", "true");

        conditional.on_request(&req, &mut ctx).unwrap();
        assert_eq!(ctx.get("log_model"), Some("test-model"));
    }

    #[test]
    fn conditional_skips_when_false() {
        let logging = Arc::new(LoggingMiddleware::default());
        let conditional =
            ConditionalMiddleware::new(logging, |ctx| ctx.get("enable_logging") == Some("true"));

        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        // enable_logging not set — predicate returns false

        conditional.on_request(&req, &mut ctx).unwrap();
        assert!(ctx.get("log_model").is_none()); // skipped
    }

    // ─── LoggingMiddleware tests ───

    #[test]
    fn logging_middleware_name() {
        let m = LoggingMiddleware::default();
        assert_eq!(m.name(), "logging");
    }

    #[test]
    fn logging_middleware_custom_level() {
        let m = LoggingMiddleware::new("debug");
        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        m.on_request(&req, &mut ctx).unwrap();
        assert_eq!(ctx.get("log_level"), Some("debug"));
    }

    // ─── MetricsMiddleware tests ───

    #[test]
    fn metrics_middleware_name() {
        let m = MetricsMiddleware::default();
        assert_eq!(m.name(), "metrics");
    }

    #[test]
    fn metrics_detailed_mode() {
        let m = MetricsMiddleware::new(true);
        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        m.on_request(&req, &mut ctx).unwrap();
        assert_eq!(ctx.get("metrics_message_count"), Some("1"));
    }

    #[test]
    fn metrics_non_detailed_mode() {
        let m = MetricsMiddleware::new(false);
        let req = test_request();
        let mut ctx = MiddlewareContext::new();
        m.on_request(&req, &mut ctx).unwrap();
        assert!(ctx.get("metrics_message_count").is_none());
    }

    // ─── MiddlewareError tests ───

    #[test]
    fn middleware_error_display() {
        let err = MiddlewareError {
            middleware_name: "test".into(),
            message: "something went wrong".into(),
        };
        assert_eq!(err.to_string(), "[test] something went wrong");
    }

    // ─── Full pipeline integration ───

    #[test]
    fn full_pipeline_request_response_cycle() {
        let mut pipeline = MiddlewarePipeline::new();
        pipeline.add(Arc::new(LoggingMiddleware::default()));
        pipeline.add(Arc::new(MetricsMiddleware::new(true)));

        let req = test_request();
        let resp = test_response();
        let mut ctx = MiddlewareContext::new();

        // Request phase
        pipeline.process_request(&req, &mut ctx).unwrap();
        assert_eq!(ctx.get("log_model"), Some("test-model"));

        // Response phase (reverse order)
        pipeline.process_response(&resp, &mut ctx).unwrap();
        assert_eq!(ctx.get("metrics_total_tokens"), Some("150"));
        assert_eq!(ctx.get("log_response_id"), Some("msg_01"));
    }

    #[test]
    fn pipeline_default_trait() {
        let pipeline = MiddlewarePipeline::default();
        assert!(pipeline.is_empty());
    }
}
