use crab_core::event::Event;
use crab_core::message::ContentBlock;
use crab_session::Conversation;
use tokio::sync::mpsc;

/// Core agent loop: user input -> LLM API call (SSE) -> parse tool calls ->
/// execute tools -> serialize results -> next round.
/// Exits when the model produces a final message without tool calls.
pub async fn query_loop(
    _conversation: &mut Conversation,
    // TODO: add api: &LlmBackend and tools: &ToolExecutor params once those crates have skeletons
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
            // TODO: check tool registry for is_read_only
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
    pub pending:
        Vec<tokio::task::JoinHandle<(String, crab_common::Result<crab_core::tool::ToolOutput>)>>,
}

impl StreamingToolExecutor {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub async fn collect_all(
        &mut self,
    ) -> Vec<(String, crab_common::Result<crab_core::tool::ToolOutput>)> {
        let mut results = Vec::new();
        for handle in self.pending.drain(..) {
            results.push(handle.await.expect("tool task panicked"));
        }
        results
    }
}
