use crate::model::TokenUsage;

#[derive(Debug, Clone)]
pub enum Event {
    // Message lifecycle
    TurnStart {
        turn_index: usize,
    },
    MessageStart,
    ContentDelta(String),
    MessageEnd {
        usage: TokenUsage,
    },

    // Tool execution
    ToolUseStart {
        id: String,
        name: String,
    },
    ToolUseInput(String),
    ToolResult {
        id: String,
        content: String,
        is_error: bool,
    },

    // Permission interaction
    PermissionRequest {
        tool_name: String,
        input_summary: String,
        request_id: String,
    },
    PermissionResponse {
        request_id: String,
        approved: bool,
    },

    // Context compaction
    CompactStart {
        strategy: String,
        before_tokens: u64,
    },
    CompactEnd {
        after_tokens: u64,
        removed_messages: usize,
    },

    // Token warnings
    TokenWarning {
        usage_percent: u8,
        used: u64,
        limit: u64,
    },

    // Errors
    Error(String),
}
