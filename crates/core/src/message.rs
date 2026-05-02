//! Message types for LLM conversations.
//!
//! These types model the conversation protocol and are designed to be
//! bidirectionally compatible with the Anthropic Messages API JSON format.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The role of a message participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => f.write_str("user"),
            Self::Assistant => f.write_str("assistant"),
            Self::System => f.write_str("system"),
        }
    }
}

/// A content block within a message.
///
/// Uses `#[serde(tag = "type")]` internal tagging so JSON looks like:
/// `{"type": "text", "text": "..."}` or `{"type": "tool_use", "id": "...", ...}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content.
    Text { text: String },

    /// A tool invocation requested by the assistant.
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },

    /// The result of a tool invocation, sent as a user message.
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },

    /// An image (base64 encoded).
    Image { source: ImageSource },

    /// Extended thinking content from the model.
    Thinking { thinking: String },
}

impl ContentBlock {
    /// Create a text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create a tool use content block.
    pub fn tool_use(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Create a tool result content block.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error,
        }
    }

    /// Returns the text content if this is a `Text` block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Returns true if this is a `ToolUse` block.
    pub const fn is_tool_use(&self) -> bool {
        matches!(self, Self::ToolUse { .. })
    }

    /// Returns true if this is a `ToolResult` block.
    pub const fn is_tool_result(&self) -> bool {
        matches!(self, Self::ToolResult { .. })
    }

    /// Returns true if this is an `Image` block.
    pub const fn is_image(&self) -> bool {
        matches!(self, Self::Image { .. })
    }
}

/// Image source data (base64 encoded).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageSource {
    /// Source type, typically "base64".
    #[serde(rename = "type")]
    pub source_type: String,

    /// MIME type, e.g. "image/png", "image/jpeg".
    pub media_type: String,

    /// Base64-encoded image data.
    pub data: String,
}

impl ImageSource {
    /// Create a new base64 image source.
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            source_type: "base64".to_string(),
            media_type: media_type.into(),
            data: data.into(),
        }
    }
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a new message with the given role and content blocks.
    pub fn new(role: Role, content: Vec<ContentBlock>) -> Self {
        Self { role, content }
    }

    /// Create a user message with text content.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create an assistant message with text content.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create a system message with text content.
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create a user message containing a tool result.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::tool_result(tool_use_id, content, is_error)],
        }
    }

    /// Extract all text content from this message, joined with newlines.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(ContentBlock::as_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns true if this message contains any tool use blocks.
    pub fn has_tool_use(&self) -> bool {
        self.content.iter().any(ContentBlock::is_tool_use)
    }

    /// Returns true if this message contains any tool result blocks.
    pub fn has_tool_result(&self) -> bool {
        self.content.iter().any(ContentBlock::is_tool_result)
    }

    /// Iterate over all tool use blocks in this message.
    pub fn tool_uses(&self) -> impl Iterator<Item = (&str, &str, &Value)> {
        self.content.iter().filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some((id.as_str(), name.as_str(), input)),
            _ => None,
        })
    }

    /// Rough token estimate for this message (chars / 4 heuristic).
    ///
    /// This is intentionally imprecise — accurate counts require a tokenizer.
    /// Used for budget checks where an approximate answer is sufficient.
    pub fn estimated_tokens(&self) -> u64 {
        let char_count: usize = self
            .content
            .iter()
            .map(|block| match block {
                ContentBlock::Text { text } => text.len(),
                ContentBlock::ToolUse { name, input, .. } => name.len() + input.to_string().len(),
                ContentBlock::ToolResult { content, .. } => content.len(),
                ContentBlock::Image { .. } => 1000, // images are ~fixed cost
                ContentBlock::Thinking { thinking } => thinking.len(),
            })
            .sum();

        // ~4 chars per token is a reasonable English-text heuristic
        (char_count as u64) / 4 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn role_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&Role::Assistant).unwrap(),
            "\"assistant\""
        );
        assert_eq!(serde_json::to_string(&Role::System).unwrap(), "\"system\"");
    }

    #[test]
    fn role_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<Role>("\"user\"").unwrap(),
            Role::User
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"assistant\"").unwrap(),
            Role::Assistant
        );
    }

    #[test]
    fn text_block_roundtrip() {
        let block = ContentBlock::text("hello world");
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json, json!({"type": "text", "text": "hello world"}));

        let decoded: ContentBlock = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, block);
    }

    #[test]
    fn tool_use_block_roundtrip() {
        let block =
            ContentBlock::tool_use("toolu_01A", "read_file", json!({"path": "/tmp/test.txt"}));
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            json!({
                "type": "tool_use",
                "id": "toolu_01A",
                "name": "read_file",
                "input": {"path": "/tmp/test.txt"}
            })
        );

        let decoded: ContentBlock = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, block);
    }

    #[test]
    fn tool_result_block_roundtrip() {
        let block = ContentBlock::tool_result("toolu_01A", "file contents here", false);
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            json!({
                "type": "tool_result",
                "tool_use_id": "toolu_01A",
                "content": "file contents here",
                "is_error": false
            })
        );

        let decoded: ContentBlock = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, block);
    }

    #[test]
    fn tool_result_is_error_defaults_to_false() {
        let json = json!({
            "type": "tool_result",
            "tool_use_id": "toolu_01A",
            "content": "ok"
        });
        let block: ContentBlock = serde_json::from_value(json).unwrap();
        match block {
            ContentBlock::ToolResult { is_error, .. } => assert!(!is_error),
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn image_block_roundtrip() {
        let block = ContentBlock::Image {
            source: ImageSource::base64("image/png", "iVBOR..."),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/png",
                    "data": "iVBOR..."
                }
            })
        );

        let decoded: ContentBlock = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, block);
    }

    #[test]
    fn message_roundtrip_anthropic_format() {
        let msg = Message::user("Hello, Claude!");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(
            json,
            json!({
                "role": "user",
                "content": [{"type": "text", "text": "Hello, Claude!"}]
            })
        );

        let decoded: Message = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn assistant_message_with_tool_use() {
        let msg = Message::new(
            Role::Assistant,
            vec![
                ContentBlock::text("Let me read that file."),
                ContentBlock::tool_use("toolu_01A", "read_file", json!({"path": "src/main.rs"})),
            ],
        );

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "assistant");
        assert_eq!(json["content"].as_array().unwrap().len(), 2);
        assert_eq!(json["content"][0]["type"], "text");
        assert_eq!(json["content"][1]["type"], "tool_use");
        assert_eq!(json["content"][1]["name"], "read_file");

        assert!(msg.has_tool_use());
        assert_eq!(msg.text(), "Let me read that file.");

        let mut tool_uses: Vec<_> = msg.tool_uses().collect();
        assert_eq!(tool_uses.len(), 1);
        let (id, name, input) = tool_uses.remove(0);
        assert_eq!(id, "toolu_01A");
        assert_eq!(name, "read_file");
        assert_eq!(input, &json!({"path": "src/main.rs"}));

        // Roundtrip
        let decoded: Message = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn message_text_extraction() {
        let msg = Message::new(
            Role::Assistant,
            vec![
                ContentBlock::text("line 1"),
                ContentBlock::tool_use("id1", "bash", json!({})),
                ContentBlock::text("line 2"),
            ],
        );
        assert_eq!(msg.text(), "line 1\nline 2");
    }

    #[test]
    fn message_estimated_tokens_nonzero() {
        let msg = Message::user("Hello world");
        assert!(msg.estimated_tokens() > 0);
    }

    #[test]
    fn role_display() {
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::System.to_string(), "system");
    }

    #[test]
    fn message_tool_result_constructor() {
        let msg = Message::tool_result("toolu_01", "result text", false);
        assert_eq!(msg.role, Role::User);
        assert!(msg.has_tool_result());
        assert!(!msg.has_tool_use());
    }

    #[test]
    fn message_tool_result_is_error() {
        let msg = Message::tool_result("toolu_01", "error!", true);
        match &msg.content[0] {
            ContentBlock::ToolResult { is_error, .. } => assert!(is_error),
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn message_no_tool_use() {
        let msg = Message::user("just text");
        assert!(!msg.has_tool_use());
        assert_eq!(msg.tool_uses().count(), 0);
    }

    #[test]
    fn message_system_constructor() {
        let msg = Message::system("You are a helpful assistant.");
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.text(), "You are a helpful assistant.");
    }

    #[test]
    fn content_block_as_text() {
        assert_eq!(ContentBlock::text("hi").as_text(), Some("hi"));
        let tool = ContentBlock::tool_use("id", "name", json!({}));
        assert_eq!(tool.as_text(), None);
    }

    #[test]
    fn message_estimated_tokens_image() {
        let msg = Message::new(
            Role::User,
            vec![ContentBlock::Image {
                source: ImageSource::base64("image/png", "abc"),
            }],
        );
        // Images have a fixed ~1000 char cost / 4 = ~250 tokens
        assert!(msg.estimated_tokens() >= 250);
    }

    // ─── Additional coverage tests ───

    #[test]
    fn role_rejects_invalid_string() {
        let result = serde_json::from_str::<Role>("\"moderator\"");
        assert!(result.is_err());
    }

    #[test]
    fn role_rejects_uppercase() {
        let result = serde_json::from_str::<Role>("\"User\"");
        assert!(result.is_err());
    }

    #[test]
    fn role_system_serde_roundtrip() {
        let json = serde_json::to_string(&Role::System).unwrap();
        assert_eq!(serde_json::from_str::<Role>(&json).unwrap(), Role::System);
    }

    #[test]
    fn full_anthropic_api_request_roundtrip() {
        // Simulate a full multi-turn Anthropic API conversation payload
        let messages = vec![
            Message::user("What files are in /tmp?"),
            Message::new(
                Role::Assistant,
                vec![
                    ContentBlock::text("Let me check that for you."),
                    ContentBlock::tool_use("toolu_01", "bash", json!({"command": "ls /tmp"})),
                ],
            ),
            Message::tool_result("toolu_01", "file1.txt\nfile2.txt", false),
            Message::assistant("There are two files: file1.txt and file2.txt."),
        ];

        let json = serde_json::to_value(&messages).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 4);

        // Verify Anthropic API structure
        assert_eq!(arr[0]["role"], "user");
        assert_eq!(arr[1]["role"], "assistant");
        assert_eq!(arr[1]["content"][1]["type"], "tool_use");
        assert_eq!(arr[2]["role"], "user");
        assert_eq!(arr[2]["content"][0]["type"], "tool_result");
        assert_eq!(arr[3]["role"], "assistant");

        // Full roundtrip
        let decoded: Vec<Message> = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.len(), 4);
        assert_eq!(decoded[0], messages[0]);
        assert_eq!(decoded[1], messages[1]);
        assert_eq!(decoded[2], messages[2]);
        assert_eq!(decoded[3], messages[3]);
    }

    #[test]
    fn tool_result_error_roundtrip() {
        let msg = Message::tool_result("toolu_02", "command not found: xyz", true);
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["content"][0]["is_error"], true);

        let decoded: Message = serde_json::from_value(json).unwrap();
        assert!(decoded.has_tool_result());
        match &decoded.content[0] {
            ContentBlock::ToolResult {
                is_error, content, ..
            } => {
                assert!(is_error);
                assert_eq!(content, "command not found: xyz");
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn message_multiple_tool_uses() {
        let msg = Message::new(
            Role::Assistant,
            vec![
                ContentBlock::tool_use("t1", "read", json!({"path": "a.rs"})),
                ContentBlock::tool_use("t2", "read", json!({"path": "b.rs"})),
                ContentBlock::text("Reading both files."),
            ],
        );
        assert!(msg.has_tool_use());
        let uses: Vec<_> = msg.tool_uses().collect();
        assert_eq!(uses.len(), 2);
        assert_eq!(uses[0].1, "read");
        assert_eq!(uses[1].1, "read");
    }

    #[test]
    fn message_empty_content() {
        let msg = Message::new(Role::User, vec![]);
        assert_eq!(msg.text(), "");
        assert!(!msg.has_tool_use());
        assert!(!msg.has_tool_result());
        assert_eq!(msg.tool_uses().count(), 0);
        assert_eq!(msg.estimated_tokens(), 1); // 0/4 + 1
    }

    #[test]
    fn content_block_is_tool_use_and_result() {
        let tu = ContentBlock::tool_use("id", "name", json!({}));
        assert!(tu.is_tool_use());
        assert!(!tu.is_tool_result());

        let tr = ContentBlock::tool_result("id", "ok", false);
        assert!(tr.is_tool_result());
        assert!(!tr.is_tool_use());

        let text = ContentBlock::text("hi");
        assert!(!text.is_tool_use());
        assert!(!text.is_tool_result());
    }

    #[test]
    fn content_block_is_image() {
        let img = ContentBlock::Image {
            source: ImageSource::base64("image/png", "data"),
        };
        assert!(img.is_image());
        assert!(!ContentBlock::text("hello").is_image());
    }

    #[test]
    fn image_source_base64_constructor() {
        let src = ImageSource::base64("image/jpeg", "data123");
        assert_eq!(src.source_type, "base64");
        assert_eq!(src.media_type, "image/jpeg");
        assert_eq!(src.data, "data123");
    }

    #[test]
    fn message_estimated_tokens_tool_use() {
        let msg = Message::new(
            Role::Assistant,
            vec![ContentBlock::tool_use(
                "id",
                "bash",
                json!({"command": "echo hello world"}),
            )],
        );
        assert!(msg.estimated_tokens() > 0);
    }

    #[test]
    fn message_text_only_text_blocks() {
        let msg = Message::new(
            Role::User,
            vec![
                ContentBlock::text("hello"),
                ContentBlock::Image {
                    source: ImageSource::base64("image/png", "data"),
                },
                ContentBlock::tool_result("id", "output", false),
                ContentBlock::text("world"),
            ],
        );
        // text() should only join Text blocks
        assert_eq!(msg.text(), "hello\nworld");
    }

    // ─── Thinking content block tests ───

    #[test]
    fn thinking_block_roundtrip() {
        let block = ContentBlock::Thinking {
            thinking: "Let me reason about this...".into(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "thinking");
        assert_eq!(json["thinking"], "Let me reason about this...");
        let decoded: ContentBlock = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, block);
    }

    #[test]
    fn thinking_block_not_in_text() {
        let msg = Message::new(
            Role::Assistant,
            vec![
                ContentBlock::Thinking {
                    thinking: "internal reasoning".into(),
                },
                ContentBlock::text("visible answer"),
            ],
        );
        // text() should NOT include thinking content
        assert_eq!(msg.text(), "visible answer");
    }

    #[test]
    fn thinking_block_estimated_tokens() {
        let msg = Message::new(
            Role::Assistant,
            vec![ContentBlock::Thinking {
                thinking: "a".repeat(400),
            }],
        );
        // 400 chars / 4 = 100 tokens + 1
        assert_eq!(msg.estimated_tokens(), 101);
    }

    #[test]
    fn thinking_block_is_not_tool_use_or_result() {
        let block = ContentBlock::Thinking {
            thinking: "thinking".into(),
        };
        assert!(!block.is_tool_use());
        assert!(!block.is_tool_result());
        assert!(block.as_text().is_none());
    }
}
