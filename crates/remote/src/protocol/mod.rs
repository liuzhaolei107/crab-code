//! Wire types for `crab-proto`.
//!
//! Messages are JSON-RPC 2.0 envelopes carried over WebSocket text frames.
//! All public types derive [`schemars::JsonSchema`] so the wire schema can be
//! dumped (see [`dump_schema`]) and used to generate TS / Swift / Kotlin
//! client stubs — supporting the web / mobile / desktop entry points without
//! Rust bindings on those clients.
//!
//! ## Protocol versioning
//!
//! The protocol follows semver at the message-envelope level. Breaking changes
//! (removed fields, changed semantics) bump the major; additive changes (new
//! optional fields, new message kinds) bump the minor. Clients and servers
//! negotiate on `initialize` and reject mismatched majors.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Current protocol version as a semver-compatible string.
///
/// Advertised by both sides during the `initialize` handshake; peers MUST
/// reject connections with a different major version.
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// JSON-RPC request id, restricted to `u64`.
///
/// JSON-RPC permits `number | string | null`; we narrow to `u64` because every
/// known client assigns numeric ids, and keeping a native integer key lets the
/// server hash-key its pending-request map without allocating.
pub type MessageId = u64;

/// `initialize` request params — first message sent by the client after the
/// WebSocket handshake.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Protocol version the client speaks. Server rejects on major mismatch.
    pub protocol_version: String,
    /// Free-form client identification — useful for server-side logging and
    /// for the TUI to display "connected: vscode-extension 1.2.3".
    pub client_info: ClientInfo,
}

/// Client identification carried in [`InitializeParams`]. Mirrors the MCP
/// equivalent so a future merge with `crab-mcp::ClientInfo` stays painless.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// `initialize` response — server echoes its own identity + negotiated version.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub server_info: ServerInfo,
}

/// Server identification carried in [`InitializeResult`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Dump the JSON Schema for a type — used by `xtask` / CLI to export the
/// schema file that drives client-stub generation.
///
/// Example:
/// ```
/// use crab_remote::protocol::{dump_schema, InitializeParams};
/// let schema = dump_schema::<InitializeParams>();
/// assert!(schema.contains("InitializeParams"));
/// ```
pub fn dump_schema<T: JsonSchema>() -> String {
    let schema = schemars::schema_for!(T);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_semver_shaped() {
        let parts: Vec<&str> = PROTOCOL_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "version must be major.minor.patch");
        for p in parts {
            p.parse::<u32>().expect("each version part must be numeric");
        }
    }

    #[test]
    fn initialize_params_roundtrip() {
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            client_info: ClientInfo {
                name: "test-client".into(),
                version: "1.0".into(),
            },
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"protocolVersion\""));
        assert!(json.contains("\"clientInfo\""));
        let back: InitializeParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.protocol_version, params.protocol_version);
    }

    #[test]
    fn schema_dump_is_valid_json() {
        let schema = dump_schema::<InitializeParams>();
        let v: serde_json::Value = serde_json::from_str(&schema).unwrap();
        assert!(v.is_object());
    }
}
