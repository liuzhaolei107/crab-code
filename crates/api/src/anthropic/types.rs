//! Anthropic Messages API native request/response types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Anthropic Messages API request body.
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Value>,
    pub stream: bool,
}

/// System prompt block (supports `cache_control`).
#[derive(Debug, Clone, Serialize)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControlDirective>,
}

/// Cache control directive for Anthropic prompt caching.
#[derive(Debug, Clone, Serialize)]
pub struct CacheControlDirective {
    #[serde(rename = "type")]
    pub directive_type: String,
}

/// A message in Anthropic format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
}

/// Content block in Anthropic format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    Image {
        source: AnthropicImageSource,
    },
}

/// Image source in Anthropic format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Anthropic Messages API response (non-streaming).
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: AnthropicUsage,
}

/// Token usage in Anthropic format.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
}

// ─── SSE event types ───

/// Anthropic SSE event wrapper.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicSseEvent {
    MessageStart {
        message: AnthropicSseMessageStart,
    },
    ContentBlockStart {
        index: usize,
        content_block: AnthropicContentBlockInfo,
    },
    ContentBlockDelta {
        index: usize,
        delta: AnthropicDelta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: AnthropicMessageDeltaBody,
        usage: AnthropicDeltaUsage,
    },
    MessageStop,
    Ping,
    Error {
        error: AnthropicApiError,
    },
}

/// Message start payload.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicSseMessageStart {
    pub id: String,
    pub model: String,
    pub usage: AnthropicUsage,
}

/// Content block info at start.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicContentBlockInfo {
    #[serde(rename = "type")]
    pub block_type: String,
}

/// Delta payload for content block.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

/// Message delta body.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessageDeltaBody {
    pub stop_reason: Option<String>,
}

/// Usage in `message_delta` event.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicDeltaUsage {
    pub output_tokens: u64,
}

/// Anthropic API error payload.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicApiError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}
