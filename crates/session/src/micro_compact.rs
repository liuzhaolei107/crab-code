//! Micro-compaction: replace individual large tool results with summaries.
//!
//! Unlike full conversation compaction (which re-summarizes the entire history),
//! micro-compaction targets individual tool results that exceed a token threshold.
//! This preserves the conversation structure while reducing token usage for
//! bloated tool outputs (e.g., large file reads, verbose command output).
//!
//! Maps to Claude Code's `microCompact.ts`.
//!
//! # Strategy
//!
//! 1. Scan messages for `ToolResult` content blocks.
//! 2. Estimate tokens for each result (chars / 4 heuristic).
//! 3. If a result exceeds `max_tool_result_tokens`, replace it with a
//!    summary targeting `summary_target_tokens`.
//! 4. Return the modified message list and statistics.
//!
//! # Relationship to `compaction.rs`
//!
//! The `CompactionStrategy::Microcompact` level in `compaction.rs` delegates
//! to this module for the actual per-result replacement logic.

use crab_core::message::{ContentBlock, Message};

// ─── Configuration ──────────────────────────────────────────────────────

/// Configuration for micro-compaction.
#[derive(Debug, Clone)]
pub struct MicroCompactConfig {
    /// Maximum token count for a single tool result before it is compacted.
    /// Results with estimated tokens above this threshold are summarized.
    pub max_tool_result_tokens: usize,
    /// Target token count for the summary that replaces a compacted result.
    pub summary_target_tokens: usize,
}

impl Default for MicroCompactConfig {
    fn default() -> Self {
        Self {
            max_tool_result_tokens: 500,
            summary_target_tokens: 100,
        }
    }
}

// ─── Result ─────────────────────────────────────────────────────────────

/// Result of micro-compaction applied to a conversation.
#[derive(Debug, Clone)]
pub struct MicroCompactResult {
    /// The messages after compaction (with large tool results replaced).
    pub messages: Vec<Message>,
    /// Number of tool results that were compacted.
    pub compacted_count: usize,
    /// Estimated total tokens saved by compaction.
    pub tokens_saved: usize,
}

// ─── Public API ─────────────────────────────────────────────────────────

/// Micro-compact: replace individual large tool results with summaries,
/// without re-summarizing the entire conversation.
///
/// Scans all messages for `ToolResult` content blocks that exceed the
/// configured token threshold and replaces their content with brief
/// summaries.
///
/// Messages that do not contain tool results are passed through unchanged.
///
/// # Arguments
///
/// * `messages` — the conversation messages to scan
/// * `config` — micro-compaction thresholds
///
/// # Returns
///
/// A [`MicroCompactResult`] with the (possibly modified) messages and
/// statistics about what was compacted.
pub fn micro_compact(messages: &[Message], config: &MicroCompactConfig) -> MicroCompactResult {
    let mut result_messages = Vec::with_capacity(messages.len());
    let mut compacted_count = 0;
    let mut tokens_saved = 0;

    for msg in messages {
        let mut new_content = Vec::with_capacity(msg.content.len());
        let mut message_modified = false;

        for block in &msg.content {
            match block {
                ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } => {
                    if !is_error && should_compact_result(content, config.max_tool_result_tokens) {
                        let original_tokens = estimate_tokens(content);
                        let summary =
                            summarize_tool_result("tool", content, config.summary_target_tokens);
                        let summary_tokens = estimate_tokens(&summary);

                        tokens_saved += original_tokens.saturating_sub(summary_tokens);
                        compacted_count += 1;
                        message_modified = true;

                        new_content.push(ContentBlock::ToolResult {
                            tool_use_id: tool_use_id.clone(),
                            content: summary,
                            is_error: *is_error,
                        });
                    } else {
                        new_content.push(block.clone());
                    }
                }
                _ => {
                    new_content.push(block.clone());
                }
            }
        }

        if message_modified {
            result_messages.push(Message::new(msg.role, new_content));
        } else {
            result_messages.push(msg.clone());
        }
    }

    MicroCompactResult {
        messages: result_messages,
        compacted_count,
        tokens_saved,
    }
}

/// Check if a specific tool result exceeds the token threshold.
///
/// Uses a rough `chars / 4` heuristic for token estimation.
#[must_use]
pub fn should_compact_result(result: &str, max_tokens: usize) -> bool {
    estimate_tokens(result) > max_tokens
}

/// Generate a brief summary of a tool result.
///
/// In the current stub implementation, this truncates the result and adds
/// a summary marker. A real implementation would use a small/fast LLM to
/// generate a meaningful summary.
///
/// # Arguments
///
/// * `tool_name` — name of the tool that produced the result
/// * `result` — the original tool result text
/// * `target_tokens` — target summary length in tokens
pub fn summarize_tool_result(tool_name: &str, result: &str, target_tokens: usize) -> String {
    let original_tokens = estimate_tokens(result);
    let target_chars = target_tokens * 4; // reverse the heuristic

    if result.len() <= target_chars {
        return result.to_string();
    }

    // Extract the first and last portions to preserve context.
    let head_chars = target_chars * 2 / 3;
    let tail_chars = target_chars / 3;

    let head = safe_truncate(result, head_chars);
    let tail = safe_tail(result, tail_chars);

    format!(
        "[micro-compacted: {tool_name} output ({original_tokens} tokens -> ~{target_tokens} tokens)]\n\
         {head}\n\
         [...{} tokens omitted...]\n\
         {tail}",
        original_tokens.saturating_sub(target_tokens)
    )
}

// ─── Internal helpers ───────────────────────────────────────────────────

/// Rough token estimate: chars / 4.
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Safely truncate a string to approximately `max_chars` at a char boundary.
fn safe_truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        return s;
    }
    // Find the last char boundary at or before max_chars.
    let mut end = max_chars;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Safely get the last `max_chars` of a string at a char boundary.
fn safe_tail(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        return s;
    }
    let start = s.len() - max_chars;
    let mut adjusted = start;
    while adjusted < s.len() && !s.is_char_boundary(adjusted) {
        adjusted += 1;
    }
    &s[adjusted..]
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::message::Role;

    fn make_tool_result_msg(tool_use_id: &str, content: &str, is_error: bool) -> Message {
        Message::new(
            Role::User,
            vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.to_string(),
                content: content.to_string(),
                is_error,
            }],
        )
    }

    fn make_text_msg(role: Role, text: &str) -> Message {
        Message::new(role, vec![ContentBlock::text(text)])
    }

    // ── Config ─────────────────────────────────────────────────────

    #[test]
    fn default_config() {
        let config = MicroCompactConfig::default();
        assert_eq!(config.max_tool_result_tokens, 500);
        assert_eq!(config.summary_target_tokens, 100);
    }

    // ── should_compact_result ──────────────────────────────────────

    #[test]
    fn short_result_not_compacted() {
        assert!(!should_compact_result("short text", 100));
    }

    #[test]
    fn long_result_is_compacted() {
        let long_text = "x".repeat(2004); // 2004 chars / 4 = 501 tokens > 500
        assert!(should_compact_result(&long_text, 500));
    }

    #[test]
    fn result_at_threshold_not_compacted() {
        let text = "x".repeat(2000); // exactly 500 tokens
        assert!(!should_compact_result(&text, 500));
    }

    // ── summarize_tool_result ──────────────────────────────────────

    #[test]
    fn summarize_short_result_unchanged() {
        let result = "short output";
        let summary = summarize_tool_result("bash", result, 100);
        assert_eq!(summary, result);
    }

    #[test]
    fn summarize_long_result_truncated() {
        let long_result = "x".repeat(10000);
        let summary = summarize_tool_result("bash", &long_result, 50);
        assert!(summary.contains("[micro-compacted:"));
        assert!(summary.contains("omitted"));
        assert!(summary.len() < long_result.len());
    }

    // ── micro_compact ──────────────────────────────────────────────

    #[test]
    fn compact_empty_messages() {
        let result = micro_compact(&[], &MicroCompactConfig::default());
        assert!(result.messages.is_empty());
        assert_eq!(result.compacted_count, 0);
        assert_eq!(result.tokens_saved, 0);
    }

    #[test]
    fn compact_no_tool_results() {
        let messages = vec![
            make_text_msg(Role::User, "hello"),
            make_text_msg(Role::Assistant, "hi there"),
        ];
        let result = micro_compact(&messages, &MicroCompactConfig::default());
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.compacted_count, 0);
    }

    #[test]
    fn compact_small_tool_results_unchanged() {
        let messages = vec![make_tool_result_msg("id1", "short output", false)];
        let config = MicroCompactConfig::default();
        let result = micro_compact(&messages, &config);
        assert_eq!(result.compacted_count, 0);

        match &result.messages[0].content[0] {
            ContentBlock::ToolResult { content, .. } => {
                assert_eq!(content, "short output");
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn compact_large_tool_result() {
        let large_output = "x".repeat(10000); // ~2500 tokens, way above default 500
        let messages = vec![make_tool_result_msg("id1", &large_output, false)];
        let config = MicroCompactConfig::default();
        let result = micro_compact(&messages, &config);

        assert_eq!(result.compacted_count, 1);
        assert!(result.tokens_saved > 0);

        match &result.messages[0].content[0] {
            ContentBlock::ToolResult { content, .. } => {
                assert!(content.contains("[micro-compacted:"));
                assert!(content.len() < large_output.len());
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn compact_preserves_error_results() {
        let large_error = "e".repeat(10000);
        let messages = vec![make_tool_result_msg("id1", &large_error, true)];
        let config = MicroCompactConfig::default();
        let result = micro_compact(&messages, &config);

        // Error results should NOT be compacted.
        assert_eq!(result.compacted_count, 0);
        match &result.messages[0].content[0] {
            ContentBlock::ToolResult { content, .. } => {
                assert_eq!(content.len(), large_error.len());
            }
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn compact_mixed_messages() {
        let large = "x".repeat(10000);
        let messages = vec![
            make_text_msg(Role::User, "do something"),
            make_text_msg(Role::Assistant, "calling tool..."),
            make_tool_result_msg("id1", &large, false),
            make_tool_result_msg("id2", "small", false),
            make_text_msg(Role::Assistant, "done"),
        ];

        let config = MicroCompactConfig::default();
        let result = micro_compact(&messages, &config);

        assert_eq!(result.messages.len(), 5);
        assert_eq!(result.compacted_count, 1); // only the large one
    }

    // ── Helper tests ───────────────────────────────────────────────

    #[test]
    fn estimate_tokens_basic() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("1234"), 1);
        assert_eq!(estimate_tokens("12345678"), 2);
    }

    #[test]
    fn safe_truncate_within_bounds() {
        assert_eq!(safe_truncate("hello", 10), "hello");
    }

    #[test]
    fn safe_truncate_at_boundary() {
        assert_eq!(safe_truncate("hello world", 5), "hello");
    }

    #[test]
    fn safe_tail_within_bounds() {
        assert_eq!(safe_tail("hello", 10), "hello");
    }

    #[test]
    fn safe_tail_at_boundary() {
        assert_eq!(safe_tail("hello world", 5), "world");
    }
}
