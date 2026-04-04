//! Conversion between Anthropic API types and internal types.

use crab_core::message::{ContentBlock, ImageSource, Message, Role};
use crab_core::model::TokenUsage;

use super::types::{
    AnthropicContentBlock, AnthropicDelta, AnthropicImageSource, AnthropicMessage,
    AnthropicRequest, AnthropicResponse, AnthropicSseEvent, AnthropicUsage, SystemBlock,
};
use crate::error::Result;
use crate::types::{MessageRequest, StreamEvent};

/// Convert internal `MessageRequest` to Anthropic API request.
pub fn to_anthropic_request(req: &MessageRequest<'_>, stream: bool) -> AnthropicRequest {
    let messages = req
        .messages
        .iter()
        .filter(|m| m.role != Role::System)
        .map(message_to_anthropic)
        .collect();

    let system = req.system.as_ref().map(|s| {
        vec![SystemBlock {
            block_type: "text".to_string(),
            text: s.clone(),
            cache_control: None,
        }]
    });

    AnthropicRequest {
        model: req.model.0.clone(),
        messages,
        max_tokens: req.max_tokens,
        system,
        temperature: req.temperature,
        tools: req.tools.clone(),
        stream,
    }
}

/// Convert an internal `Message` to Anthropic format.
fn message_to_anthropic(msg: &Message) -> AnthropicMessage {
    let role = match msg.role {
        Role::User | Role::System => "user",
        Role::Assistant => "assistant",
    };

    let content = msg.content.iter().map(content_block_to_anthropic).collect();

    AnthropicMessage {
        role: role.to_string(),
        content,
    }
}

/// Convert an internal `ContentBlock` to Anthropic format.
fn content_block_to_anthropic(block: &ContentBlock) -> AnthropicContentBlock {
    match block {
        ContentBlock::Text { text } => AnthropicContentBlock::Text { text: text.clone() },
        ContentBlock::ToolUse { id, name, input } => AnthropicContentBlock::ToolUse {
            id: id.clone(),
            name: name.clone(),
            input: input.clone(),
        },
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => AnthropicContentBlock::ToolResult {
            tool_use_id: tool_use_id.clone(),
            content: content.clone(),
            is_error: *is_error,
        },
        ContentBlock::Image { source } => AnthropicContentBlock::Image {
            source: AnthropicImageSource {
                source_type: source.source_type.clone(),
                media_type: source.media_type.clone(),
                data: source.data.clone(),
            },
        },
    }
}

/// Convert Anthropic non-streaming response to internal types.
///
/// # Errors
///
/// Returns `ApiError` if response parsing fails.
pub fn from_anthropic_response(resp: AnthropicResponse) -> Result<(Message, TokenUsage)> {
    let content = resp
        .content
        .into_iter()
        .map(content_block_from_anthropic)
        .collect();

    let message = Message {
        role: Role::Assistant,
        content,
    };

    let usage = from_anthropic_usage(&resp.usage);

    Ok((message, usage))
}

/// Convert Anthropic content block to internal format.
fn content_block_from_anthropic(block: AnthropicContentBlock) -> ContentBlock {
    match block {
        AnthropicContentBlock::Text { text } => ContentBlock::Text { text },
        AnthropicContentBlock::ToolUse { id, name, input } => {
            ContentBlock::ToolUse { id, name, input }
        }
        AnthropicContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        },
        AnthropicContentBlock::Image { source } => ContentBlock::Image {
            source: ImageSource {
                source_type: source.source_type,
                media_type: source.media_type,
                data: source.data,
            },
        },
    }
}

/// Convert Anthropic usage to internal `TokenUsage`.
pub const fn from_anthropic_usage(usage: &AnthropicUsage) -> TokenUsage {
    TokenUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_read_tokens: usage.cache_read_input_tokens,
        cache_creation_tokens: usage.cache_creation_input_tokens,
    }
}

/// Convert Anthropic SSE event to internal `StreamEvent`.
pub fn sse_event_to_stream_event(event: AnthropicSseEvent) -> Option<StreamEvent> {
    match event {
        AnthropicSseEvent::MessageStart { message } => {
            Some(StreamEvent::MessageStart { id: message.id })
        }
        AnthropicSseEvent::ContentBlockStart {
            index,
            content_block,
        } => Some(StreamEvent::ContentBlockStart {
            index,
            content_type: content_block.block_type,
        }),
        AnthropicSseEvent::ContentBlockDelta { index, delta } => {
            let text = match delta {
                AnthropicDelta::TextDelta { text } => text,
                AnthropicDelta::InputJsonDelta { partial_json } => partial_json,
            };
            Some(StreamEvent::ContentDelta { index, delta: text })
        }
        AnthropicSseEvent::ContentBlockStop { index } => {
            Some(StreamEvent::ContentBlockStop { index })
        }
        AnthropicSseEvent::MessageDelta { usage, .. } => Some(StreamEvent::MessageDelta {
            usage: TokenUsage {
                input_tokens: 0,
                output_tokens: usage.output_tokens,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
            },
        }),
        AnthropicSseEvent::MessageStop => Some(StreamEvent::MessageStop),
        AnthropicSseEvent::Error { error } => Some(StreamEvent::Error {
            message: error.message,
        }),
        AnthropicSseEvent::Ping => None,
    }
}
