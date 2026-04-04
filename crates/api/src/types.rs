//! Internal unified request/response/event types (Crab Code's own data model).
//!
//! These are NOT API abstractions — each client independently converts between
//! these types and its own API-native format.

use crab_core::message::Message;
use crab_core::model::{ModelId, TokenUsage};

/// Internal message request — each client converts this to its own API format.
#[derive(Debug, Clone)]
pub struct MessageRequest<'a> {
    pub model: ModelId,
    pub messages: std::borrow::Cow<'a, [Message]>,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub tools: Vec<serde_json::Value>,
    pub temperature: Option<f32>,
}

/// Internal unified stream event — each client maps its SSE format to this enum.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart { id: String },
    ContentBlockStart { index: usize, content_type: String },
    ContentDelta { index: usize, delta: String },
    ContentBlockStop { index: usize },
    MessageDelta { usage: TokenUsage },
    MessageStop,
    Error { message: String },
}

/// Non-streaming response wrapper.
#[derive(Debug, Clone)]
pub struct MessageResponse {
    pub id: String,
    pub message: Message,
    pub usage: TokenUsage,
}
