//! Automatic memory extraction from conversations.
//!
//! Periodically scans recent conversation messages for information worth
//! persisting as memories (user preferences, corrections, project facts,
//! reference links). Extracted memories are proposed to the memory store
//! for deduplication and persistence.
//!
//! Maps to CCB `memdir/extractMemories.ts`.

use super::memory_types::MemoryType;
use crab_core::message::Message;

// ── Types ─────────────────────────────────────────────────────────────

/// Result of a memory extraction pass.
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Memories extracted from the conversation.
    pub memories: Vec<ExtractedMemory>,
}

/// A single memory extracted from conversation content.
#[derive(Debug, Clone)]
pub struct ExtractedMemory {
    /// Classification of this memory.
    pub memory_type: MemoryType,
    /// Short identifier / file-name slug for the memory.
    pub name: String,
    /// The markdown body content of the memory.
    pub content: String,
}

// ── Extraction ────────────────────────────────────────────────────────

/// Extract memories from a slice of conversation messages.
///
/// Scans messages for patterns like user corrections ("actually, I prefer X"),
/// reference links, project configuration hints, and tool usage patterns.
///
/// # Arguments
///
/// * `messages` — The conversation messages to analyze.
///
/// # Returns
///
/// An [`ExtractionResult`] containing zero or more extracted memories.
pub fn extract_memories_from_conversation(_messages: &[Message]) -> ExtractionResult {
    todo!("extract_memories_from_conversation: scan messages for extractable knowledge")
}

/// Determine whether extraction should be attempted based on message count
/// and how recently the last extraction was run.
///
/// # Arguments
///
/// * `message_count` — Total number of messages in the conversation.
/// * `last_extraction_turn` — The message index at which extraction was
///   last performed (0 if never).
///
/// # Returns
///
/// `true` if enough new messages have accumulated to justify a scan.
#[must_use]
pub fn should_extract(message_count: usize, last_extraction_turn: usize) -> bool {
    todo!(
        "should_extract: check if {} - {} exceeds extraction interval",
        message_count,
        last_extraction_turn
    )
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_result_fields() {
        let result = ExtractionResult {
            memories: vec![ExtractedMemory {
                memory_type: MemoryType::Feedback,
                name: "prefer_tabs".into(),
                content: "User prefers tabs over spaces".into(),
            }],
        };
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].name, "prefer_tabs");
    }

    #[test]
    fn extracted_memory_debug() {
        let mem = ExtractedMemory {
            memory_type: MemoryType::User,
            name: "test".into(),
            content: "test content".into(),
        };
        let debug = format!("{mem:?}");
        assert!(debug.contains("test"));
    }
}
