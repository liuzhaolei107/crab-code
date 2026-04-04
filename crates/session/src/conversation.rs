use crab_core::conversation::Conversation as CoreConversation;
use crab_core::message::Message;
use crab_core::model::TokenUsage;

/// Session-level conversation: wraps the core `Conversation` and adds
/// session metadata (id, system prompt, context window, cumulative usage).
pub struct Conversation {
    pub id: String,
    pub system_prompt: String,
    /// Underlying core conversation (message history + turn tracking).
    pub inner: CoreConversation,
    /// Cumulative token usage across all API calls in this session.
    pub total_usage: TokenUsage,
    /// Maximum context window size (in tokens) for the active model.
    pub context_window: u64,
}

impl Conversation {
    pub fn new(id: String, system_prompt: String, context_window: u64) -> Self {
        Self {
            id,
            system_prompt,
            inner: CoreConversation::new(),
            total_usage: TokenUsage::default(),
            context_window,
        }
    }

    /// Append a message (delegates to core conversation).
    pub fn push(&mut self, msg: Message) {
        self.inner.push(msg);
    }

    /// Access all messages as a slice.
    pub fn messages(&self) -> &[Message] {
        self.inner.messages()
    }

    /// Number of completed turns.
    pub fn turn_count(&self) -> usize {
        self.inner.turn_count()
    }

    /// Rough token estimate for the entire conversation (delegates to core).
    pub fn estimated_tokens(&self) -> u64 {
        self.inner.estimated_tokens()
    }

    /// Whether the context window usage exceeds 80%, triggering compaction.
    pub fn needs_compaction(&self) -> bool {
        if self.context_window == 0 {
            return false;
        }
        self.estimated_tokens() > self.context_window * 80 / 100
    }

    /// Record token usage from an API response.
    pub fn record_usage(&mut self, usage: TokenUsage) {
        self.total_usage += usage;
    }
}
