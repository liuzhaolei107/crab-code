//! Session-related request / result / notification params.
//!
//! Every type derives [`JsonSchema`] so TS / Swift / Kotlin
//! client stubs can be generated from the same source.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Params for [`method::SESSION_ATTACH`](super::method::SESSION_ATTACH).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionAttachParams {
    /// Opaque id previously handed back by [`SessionCreateResult::session_id`]
    /// or stored by the client across reconnects.
    pub session_id: String,
}

/// Result for a successful attach.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionAttachResult {
    pub session_id: String,
    /// Whether the session has an in-flight turn right now. Clients use this
    /// to decide whether to show a "cancel" button vs an "input" box.
    pub busy: bool,
}

/// Params for [`method::SESSION_CREATE`](super::method::SESSION_CREATE).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateParams {
    /// Absolute working directory for the new session.
    pub working_dir: String,
    /// Optional initial prompt — if set, the server kicks off the first turn
    /// immediately after creation; equivalent to `create + sendInput` in one
    /// round-trip.
    #[serde(default)]
    pub initial_prompt: Option<String>,
}

/// Result for a successful create.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateResult {
    pub session_id: String,
}

/// Params for [`method::SESSION_SEND_INPUT`](super::method::SESSION_SEND_INPUT).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionSendInputParams {
    /// User-facing prompt text.
    pub text: String,
}

/// Params for [`method::SESSION_CANCEL`](super::method::SESSION_CANCEL).
///
/// Empty for now, but kept as a named struct so future fields (e.g.
/// `reason` / `force`) can be added without breaking wire shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SessionCancelParams {}

/// Params for the [`method::SESSION_EVENT`](super::method::SESSION_EVENT) notification.
///
/// The `event` field carries a `core::Event` serialised as a [`Value`].
/// It is untyped at the envelope layer because `core::Event` spans many
/// crate-specific payload types; clients that care reconstruct the shape
/// from the event's own tag.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionEventParams {
    pub session_id: String,
    /// A `core::Event` JSON payload. See `crab-core::event` for the full enum.
    pub event: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_params_roundtrip_camel_case() {
        let p = SessionAttachParams {
            session_id: "sess_1".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(
            json.contains("\"sessionId\""),
            "wire must be camelCase: {json}"
        );
        let back: SessionAttachParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, "sess_1");
    }

    #[test]
    fn create_params_optional_prompt() {
        let p: SessionCreateParams = serde_json::from_str(r#"{"workingDir":"/tmp"}"#).unwrap();
        assert_eq!(p.working_dir, "/tmp");
        assert!(p.initial_prompt.is_none());

        let p: SessionCreateParams =
            serde_json::from_str(r#"{"workingDir":"/tmp","initialPrompt":"hi"}"#).unwrap();
        assert_eq!(p.initial_prompt.as_deref(), Some("hi"));
    }

    #[test]
    fn cancel_params_is_empty_object() {
        let p = SessionCancelParams::default();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "{}");
        let _back: SessionCancelParams = serde_json::from_str("{}").unwrap();
    }

    #[test]
    fn event_params_carries_opaque_value() {
        let p = SessionEventParams {
            session_id: "sess_1".into(),
            event: serde_json::json!({ "type": "ContentDelta", "text": "hi" }),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: SessionEventParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, "sess_1");
        assert_eq!(back.event["text"], "hi");
    }
}
