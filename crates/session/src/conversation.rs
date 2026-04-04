use crab_core::message::{ContentBlock, Message};
use crab_core::model::TokenUsage;

/// Multi-turn conversation state machine.
pub struct Conversation {
    pub id: String,
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub total_usage: TokenUsage,
    pub context_window: u64,
}

impl Conversation {
    pub fn new(id: String, system_prompt: String, context_window: u64) -> Self {
        Self {
            id,
            system_prompt,
            messages: Vec::new(),
            total_usage: TokenUsage::default(),
            context_window,
        }
    }

    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Rough token estimate: `text_len` / 4. Good enough for MVP.
    pub fn estimated_tokens(&self) -> u64 {
        let text_len: usize = self
            .messages
            .iter()
            .map(|m| {
                m.content
                    .iter()
                    .map(|c| match c {
                        ContentBlock::Text { text } => text.len(),
                        _ => 100,
                    })
                    .sum::<usize>()
            })
            .sum();
        (text_len / 4) as u64
    }

    pub fn needs_compaction(&self) -> bool {
        self.estimated_tokens() > self.context_window * 80 / 100
    }
}
