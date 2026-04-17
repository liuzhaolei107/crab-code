//! JSON-RPC 2.0 envelope types for crab-proto.
//!
//! The envelope carries `method` as a string (see [`super::method`] for the
//! full set of valid values) and `params` as an untyped [`serde_json::Value`].
//! Concrete typed param structs (e.g. [`super::InitializeParams`]) live in
//! sibling modules and are deserialized from `params` by handlers.
//!
//! Rationale for not using a fully-typed enum dispatch: it complicates
//! JSON Schema generation for third-party clients (TS / Swift / Kotlin)
//! and means every new method requires touching the same enum. The
//! string + `Value` pattern matches `crab-mcp`, keeping the protocols
//! structurally familiar.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

use super::MessageId;

/// Process-wide counter for outgoing request ids.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a fresh request id.
pub fn next_request_id() -> MessageId {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcRequest {
    /// Must be `"2.0"`.
    pub jsonrpc: String,
    pub id: MessageId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Build a new request with a freshly allocated id.
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: next_request_id(),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response envelope. Exactly one of `result` / `error` is set.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: MessageId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Construct a success response echoing the incoming request's id.
    pub fn ok(id: MessageId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Construct an error response echoing the incoming request's id.
    pub fn err(id: MessageId, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// JSON-RPC 2.0 notification — no id, no response expected.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Build a new notification.
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 error object.
///
/// See [`super::error::ErrorCode`] for the code values this protocol uses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Build a simple error object with `code` and `message`, no structured data.
    pub fn simple(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids_are_monotonic() {
        let a = next_request_id();
        let b = next_request_id();
        assert!(b > a, "ids must increment: {a} < {b}");
    }

    #[test]
    fn request_roundtrip() {
        let req = JsonRpcRequest::new("initialize", Some(serde_json::json!({"foo": 1})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
        let back: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.method, "initialize");
    }

    #[test]
    fn response_ok_and_err_are_distinct() {
        let ok = JsonRpcResponse::ok(1, serde_json::json!({}));
        let err = JsonRpcResponse::err(1, JsonRpcError::simple(-1, "boom"));
        assert!(!ok.is_error());
        assert!(err.is_error());
    }

    #[test]
    fn notification_has_no_id_field() {
        let n = JsonRpcNotification::new("session/event", None);
        let json = serde_json::to_string(&n).unwrap();
        assert!(!json.contains("\"id\""));
    }
}
