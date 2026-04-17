//! Conversation message grouping hint for the TUI renderer.
//!
//! A single user turn in crab often produces a "user message → assistant
//! text → tool use → tool result → assistant text" sequence that the
//! TUI wants to collapse into one visual block (think: a Git commit's
//! "Files changed" dropdown). This module defines a small enum tagging
//! each message with its role in such a group so `crab-tui` can fold /
//! unfold consistently without re-deriving the grouping every render.

use serde::{Deserialize, Serialize};

/// Where this message sits within a logical display group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageGroupRole {
    /// Standalone message — no surrounding group.
    Solo,
    /// Opens a new tool-use group (typically an assistant text
    /// introducing a tool call).
    GroupStart,
    /// Inside a tool-use group (tool use, tool result, continuation).
    GroupMiddle,
    /// Closes a tool-use group (usually the assistant's final text
    /// after the tool result).
    GroupEnd,
}

impl MessageGroupRole {
    /// Is this message part of any group (not `Solo`)?
    #[must_use]
    pub fn is_grouped(self) -> bool {
        !matches!(self, Self::Solo)
    }

    /// Is this the first message in its group?
    #[must_use]
    pub fn is_start(self) -> bool {
        matches!(self, Self::GroupStart)
    }

    /// Is this the last message in its group?
    #[must_use]
    pub fn is_end(self) -> bool {
        matches!(self, Self::GroupEnd)
    }
}

/// A group tag attached to a message. `group_id` ties several
/// `GroupStart/Middle/End` messages into one visual unit; `role`
/// indicates the message's position within that unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageGroup {
    /// Opaque identifier shared by every message in the same group.
    /// Usually the assistant message ID that opened the group.
    pub group_id: String,
    /// Role within the group.
    pub role: MessageGroupRole,
}

impl MessageGroup {
    /// Shorthand for a solo (ungrouped) tag. The `group_id` is
    /// intentionally empty — consumers should check
    /// [`MessageGroupRole::is_grouped`] first.
    #[must_use]
    pub fn solo() -> Self {
        Self {
            group_id: String::new(),
            role: MessageGroupRole::Solo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_helpers() {
        let g = MessageGroup::solo();
        assert!(!g.role.is_grouped());
        assert!(!g.role.is_start());
        assert!(!g.role.is_end());
        assert!(g.group_id.is_empty());
    }

    #[test]
    fn start_middle_end_predicates() {
        assert!(MessageGroupRole::GroupStart.is_grouped());
        assert!(MessageGroupRole::GroupStart.is_start());
        assert!(!MessageGroupRole::GroupStart.is_end());

        assert!(MessageGroupRole::GroupMiddle.is_grouped());
        assert!(!MessageGroupRole::GroupMiddle.is_start());
        assert!(!MessageGroupRole::GroupMiddle.is_end());

        assert!(MessageGroupRole::GroupEnd.is_grouped());
        assert!(!MessageGroupRole::GroupEnd.is_start());
        assert!(MessageGroupRole::GroupEnd.is_end());
    }

    #[test]
    fn serde_roundtrip() {
        let g = MessageGroup {
            group_id: "msg_abc".into(),
            role: MessageGroupRole::GroupStart,
        };
        let json = serde_json::to_string(&g).unwrap();
        let back: MessageGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
