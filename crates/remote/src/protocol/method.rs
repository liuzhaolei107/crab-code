//! Canonical method-name constants for crab-proto JSON-RPC messages.
//!
//! Grouped by domain; `/` separates namespace from action per JSON-RPC
//! community convention (also what MCP uses). Constants live here so
//! adding a new method is one place to add the string, one place for
//! consumers to match on.

// ─── Lifecycle ───

pub const INITIALIZE: &str = "initialize";
/// Notification; client sends once after `initialize` succeeds, signalling
/// it is ready to receive server-initiated notifications.
pub const INITIALIZED: &str = "initialized";

// ─── Session ───

/// Attach to an existing session by id.
pub const SESSION_ATTACH: &str = "session/attach";
/// Create a new session.
pub const SESSION_CREATE: &str = "session/create";
/// Send a user input / prompt to the currently attached session.
pub const SESSION_SEND_INPUT: &str = "session/sendInput";
/// Cancel any in-flight work on the currently attached session.
pub const SESSION_CANCEL: &str = "session/cancel";
/// Server → client: a wrapped `core::Event` from the attached session.
/// Always a notification, never requires a response.
pub const SESSION_EVENT: &str = "session/event";

// ─── Trigger ───
// `trigger.fire` / `trigger.list` etc. land in a follow-up phase.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_constants_are_distinct() {
        let all = [
            INITIALIZE,
            INITIALIZED,
            SESSION_ATTACH,
            SESSION_CREATE,
            SESSION_SEND_INPUT,
            SESSION_CANCEL,
            SESSION_EVENT,
        ];
        let mut sorted: Vec<_> = all.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "method names must be distinct");
    }
}
