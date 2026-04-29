use serde::{Deserialize, Serialize};

/// When a hook fires relative to tool or lifecycle events.
///
/// This is the canonical definition shared by configuration parsing
/// (`crab_config`) and runtime execution (`crab_plugin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTrigger {
    /// Before a tool is invoked.
    PreToolUse,
    /// After a tool completes.
    PostToolUse,
    /// When user submits a prompt (before it reaches the LLM).
    #[serde(alias = "prompt_submit")]
    UserPromptSubmit,
    /// After the model finishes an assistant message, before it is written
    /// to the conversation or shown in the UI. The hook may return
    /// `HookAction::Modify` to rewrite the assistant text.
    ///
    /// The hook has no visibility into `tool_use` blocks — those are
    /// considered part of the tool-boundary contract and belong to
    /// [`Self::PreToolUse`] / [`Self::PostToolUse`].
    PostSampling,
    /// When the query loop is about to exit (model produced no tool calls).
    /// A hook returning `Retry` continues the loop instead of stopping.
    Stop,
    /// When a notification is sent.
    Notification,
    /// When a session starts.
    SessionStart,
    /// When a session ends.
    SessionEnd,
    /// After the user accepts the trust dialog for a project for the
    /// first time. Fires once per project lifetime; useful for one-shot
    /// project setup (install hooks, materialize config, ...).
    Setup,
    /// When a watched file changes (settings.json, skills dir).
    FileChanged,
    /// When conversation compaction completes.
    Compact,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_trigger_serde_roundtrip() {
        let trigger = HookTrigger::PreToolUse;
        let json = serde_json::to_string(&trigger).unwrap();
        assert_eq!(json, r#""pre_tool_use""#);
        let back: HookTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(back, trigger);
    }

    #[test]
    fn hook_trigger_all_variants() {
        let triggers = [
            (HookTrigger::PreToolUse, "pre_tool_use"),
            (HookTrigger::PostToolUse, "post_tool_use"),
            (HookTrigger::UserPromptSubmit, "user_prompt_submit"),
            (HookTrigger::PostSampling, "post_sampling"),
            (HookTrigger::Stop, "stop"),
            (HookTrigger::Notification, "notification"),
            (HookTrigger::SessionStart, "session_start"),
            (HookTrigger::SessionEnd, "session_end"),
            (HookTrigger::Setup, "setup"),
            (HookTrigger::FileChanged, "file_changed"),
            (HookTrigger::Compact, "compact"),
        ];
        for (trigger, expected) in triggers {
            let json = serde_json::to_string(&trigger).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
        }
    }

    #[test]
    fn hook_trigger_prompt_submit_alias() {
        let parsed: HookTrigger = serde_json::from_str("\"prompt_submit\"").unwrap();
        assert_eq!(parsed, HookTrigger::UserPromptSubmit);
        let parsed2: HookTrigger = serde_json::from_str("\"user_prompt_submit\"").unwrap();
        assert_eq!(parsed2, HookTrigger::UserPromptSubmit);
    }
}
