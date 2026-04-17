//! Remote-control status and session identifiers — shared with TUI and
//! `core::Event`.
//!
//! The concrete WebSocket protocol types, auth, and session attach logic
//! live in `crab-remote`; this module only carries the user-visible shapes
//! that the TUI and event stream need, so consumers can render them without
//! depending on the remote crate.

use serde::{Deserialize, Serialize};

/// How the remote-control surface is configured to operate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteMode {
    /// No remote surface — local CLI/TUI only.
    Local,
    /// Local crab connects outbound to an upstream crab-proto server.
    Client,
    /// Local crab runs an inbound server that remote clients attach to.
    Server,
    /// Both roles active simultaneously.
    Hybrid,
}

/// Origin of a remote client connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientSource {
    VsCode,
    JetBrains,
    Web,
    Desktop,
    Mobile,
    Cli,
    Unknown,
}

/// Current remote-server state surfaced to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteStatus {
    /// Feature disabled in config.
    Disabled,
    /// WebSocket server bound and listening.
    Listening { port: u16 },
    /// N clients currently attached.
    Connected(u32),
    /// Fatal error; server is stopped.
    Error(String),
}

/// Stable identifier for a remotely-attached session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RemoteSessionId(pub String);

/// Stable identifier for a scheduled trigger / job.
///
/// Migrates to `crab-job` once that crate lands its `JobId` type; for now
/// lives here because `core::Event` references it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerId(pub String);

/// Lifecycle state of a remotely-attached session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteSessionStatus {
    Pending,
    Running,
    Succeeded,
    Failed(String),
    Cancelled,
}

/// Snapshot of a remote session for UI display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteSessionInfo {
    pub id: RemoteSessionId,
    pub prompt_preview: String,
    /// Unix epoch millis.
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_hashable() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(RemoteSessionId("abc".into()));
        assert!(set.contains(&RemoteSessionId("abc".into())));
    }

    #[test]
    fn session_status_serde_roundtrip() {
        let s = RemoteSessionStatus::Failed("timeout".into());
        let json = serde_json::to_string(&s).unwrap();
        let back: RemoteSessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn info_serde_roundtrip() {
        let info = RemoteSessionInfo {
            id: RemoteSessionId("sess_1".into()),
            prompt_preview: "fix the bug".into(),
            created_at: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: RemoteSessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, back);
    }

    #[test]
    fn server_status_serde_roundtrip() {
        let s = RemoteStatus::Listening { port: 4180 };
        let json = serde_json::to_string(&s).unwrap();
        let back: RemoteStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn client_source_equality() {
        assert_eq!(ClientSource::VsCode, ClientSource::VsCode);
        assert_ne!(ClientSource::VsCode, ClientSource::Web);
    }
}
