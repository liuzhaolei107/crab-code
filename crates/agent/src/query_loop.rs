use crab_core::event::Event;
use crab_core::message::{ContentBlock, Message};
use crab_core::tool::{ToolContext, ToolOutput};
use crab_session::Conversation;
use tokio::sync::mpsc;

/// Core agent loop: user input -> LLM API call (SSE) -> parse tool calls ->
/// execute tools -> serialize results -> next round.
/// Exits when the model produces a final message without tool calls.
pub async fn query_loop(
    _conversation: &mut Conversation,
    // TODO: add api: &LlmBackend and tools: &ToolExecutor params once those crates are ready
    _event_tx: mpsc::Sender<Event>,
) -> crab_common::Result<()> {
    todo!()
}

/// Partition tool calls into read (concurrent) and write (sequential) groups.
pub fn partition_tool_calls(
    blocks: &[ContentBlock],
) -> (Vec<ToolCallRef<'_>>, Vec<ToolCallRef<'_>>) {
    let mut reads = Vec::new();
    let mut writes = Vec::new();
    for block in blocks {
        if let ContentBlock::ToolUse { id, name, input } = block {
            // TODO: check tool registry for is_read_only to route into reads
            let call = ToolCallRef { id, name, input };
            writes.push(call);
            let _ = &mut reads; // suppress unused warning
        }
    }
    (reads, writes)
}

pub struct ToolCallRef<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub input: &'a serde_json::Value,
}

/// Streaming tool executor -- starts tool execution as soon as
/// a `tool_use` block's JSON is fully parsed during SSE streaming.
pub struct StreamingToolExecutor {
    pub pending: Vec<tokio::task::JoinHandle<(String, crab_common::Result<ToolOutput>)>>,
}

impl StreamingToolExecutor {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Spawn a tool execution as soon as its input JSON is complete.
    pub fn spawn_early(
        &mut self,
        id: &str,
        name: String,
        input: serde_json::Value,
        ctx: ToolContext,
        tool_fn: impl FnOnce(String, serde_json::Value, ToolContext) -> tokio::task::JoinHandle<(String, crab_common::Result<ToolOutput>)>,
    ) {
        let handle = tool_fn(name, input, ctx);
        // Re-wrap with the original id
        let _ = id; // id is captured by the caller's closure
        self.pending.push(handle);
    }

    /// Collect all pending tool results after `message_stop`.
    pub async fn collect_all(
        &mut self,
    ) -> Vec<(String, crab_common::Result<ToolOutput>)> {
        let mut results = Vec::new();
        for handle in self.pending.drain(..) {
            results.push(handle.await.expect("tool task panicked"));
        }
        results
    }
}

impl Default for StreamingToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a tool result `Message` (role: User) from tool outputs.
pub fn tool_results_message(results: Vec<(String, Result<ToolOutput, crab_common::Error>)>) -> Message {
    let content: Vec<ContentBlock> = results
        .into_iter()
        .map(|(id, result)| {
            let (text, is_error) = match result {
                Ok(output) => (output.text(), output.is_error),
                Err(e) => (e.to_string(), true),
            };
            ContentBlock::tool_result(id, text, is_error)
        })
        .collect();
    Message::new(crab_core::message::Role::User, content)
}
