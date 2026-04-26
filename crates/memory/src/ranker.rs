//! LLM-driven memory ranking via sidequery.
//!
//! Gated behind the `mem-ranker` Cargo feature. Uses a lightweight LLM
//! call to select the most relevant memories from a manifest.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crab_api::LlmBackend;
use crab_api::types::MessageRequest;
use crab_core::message::Message;
use crab_core::model::ModelId;

use crate::relevance::{MemoryRanker, format_manifest};
use crate::store::MemoryFile;

const SYSTEM_PROMPT: &str = "\
You select memories relevant to the user's query.\n\
Return JSON: {\"selected\": [\"file1.md\", \"file2.md\"]}\n\
Be selective — only include clearly relevant memories.\n\
If nothing matches, return {\"selected\": []}.\
";

/// LLM-driven memory ranker using a sidequery to a fast model.
pub struct LlmMemoryRanker {
    backend: Arc<LlmBackend>,
    model: ModelId,
}

impl LlmMemoryRanker {
    /// Create a ranker using the given backend and model.
    pub fn new(backend: Arc<LlmBackend>, model: ModelId) -> Self {
        Self { backend, model }
    }
}

impl MemoryRanker for LlmMemoryRanker {
    fn rank(
        &self,
        query: &str,
        manifest: &str,
        max_count: usize,
    ) -> Pin<Box<dyn Future<Output = crab_core::Result<Vec<String>>> + Send + '_>> {
        let query = query.to_string();
        let manifest = manifest.to_string();
        Box::pin(async move {
            let user_msg = format!(
                "Query: {query}\n\nAvailable memories (select up to {max_count}):\n{manifest}"
            );

            let req = MessageRequest {
                model: self.model.clone(),
                messages: std::borrow::Cow::Owned(vec![Message::user(&user_msg)]),
                system: Some(SYSTEM_PROMPT.to_string()),
                max_tokens: 256,
                tools: vec![],
                temperature: Some(0.0),
                cache_breakpoints: vec![],
                budget_tokens: None,
                response_format: None,
                tool_choice: None,
            };

            let response = self.backend.send_message(req).await.map_err(|e| {
                crab_core::Error::Other(format!("memory ranker LLM call failed: {e}"))
            })?;

            let text = response.message.text();
            parse_ranker_response(&text, &manifest)
        })
    }
}

/// Parse the JSON response from the ranker and filter to valid filenames.
///
/// Accepts: `{"selected": ["file1.md", "file2.md"]}` or just the array.
/// Filters out any filenames not present in the manifest.
fn parse_ranker_response(response_text: &str, manifest: &str) -> crab_core::Result<Vec<String>> {
    // Try to extract JSON from the response (may be wrapped in markdown code blocks)
    let json_text = extract_json(response_text);

    // Try parsing as {"selected": [...]}
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&json_text)
        && let Some(arr) = obj.get("selected").and_then(|v| v.as_array())
    {
        let filenames: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        return Ok(filter_valid_filenames(&filenames, manifest));
    }

    // Try parsing as a plain array
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(&json_text) {
        return Ok(filter_valid_filenames(&arr, manifest));
    }

    // Couldn't parse — return empty (graceful degradation, no error)
    Ok(Vec::new())
}

/// Extract JSON from text that may be wrapped in markdown code fences.
fn extract_json(text: &str) -> String {
    let text = text.trim();
    // Strip ```json ... ``` wrapper
    if let Some(start) = text.find('{')
        && let Some(end) = text.rfind('}')
    {
        return text[start..=end].to_string();
    }
    if let Some(start) = text.find('[')
        && let Some(end) = text.rfind(']')
    {
        return text[start..=end].to_string();
    }
    text.to_string()
}

/// Keep only filenames that appear in the manifest text.
fn filter_valid_filenames(filenames: &[String], manifest: &str) -> Vec<String> {
    filenames
        .iter()
        .filter(|f| manifest.contains(f.as_str()))
        .cloned()
        .collect()
}

/// Select memories using LLM ranking, falling back to keyword scoring on error.
pub async fn select_with_ranker(
    ranker: &LlmMemoryRanker,
    memories: &[MemoryFile],
    query: &str,
    max_count: usize,
) -> Vec<MemoryFile> {
    let manifest = format_manifest(memories);

    match ranker.rank(query, &manifest, max_count).await {
        Ok(selected_filenames) if !selected_filenames.is_empty() => {
            // Return memories matching selected filenames, preserving ranker order
            selected_filenames
                .iter()
                .filter_map(|name| memories.iter().find(|m| m.filename == *name))
                .cloned()
                .collect()
        }
        _ => {
            // Fallback to keyword scoring
            let selector = crate::relevance::MemorySelector {
                max_memories: max_count,
                ..Default::default()
            };
            selector
                .select_by_keywords(memories, query)
                .into_iter()
                .map(|s| s.file)
                .collect()
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MANIFEST: &str = "\
user_role.md — Senior Rust developer [user]\n\
feedback_style.md — Prefers terse responses [feedback]\n\
project_auth.md — Working on auth rewrite [project]\n\
";

    #[test]
    fn parse_valid_json_object() {
        let response = r#"{"selected": ["user_role.md", "feedback_style.md"]}"#;
        let result = parse_ranker_response(response, SAMPLE_MANIFEST).unwrap();
        assert_eq!(result, vec!["user_role.md", "feedback_style.md"]);
    }

    #[test]
    fn parse_json_in_code_fence() {
        let response = "```json\n{\"selected\": [\"user_role.md\"]}\n```";
        let result = parse_ranker_response(response, SAMPLE_MANIFEST).unwrap();
        assert_eq!(result, vec!["user_role.md"]);
    }

    #[test]
    fn parse_plain_array() {
        let response = r#"["user_role.md", "project_auth.md"]"#;
        let result = parse_ranker_response(response, SAMPLE_MANIFEST).unwrap();
        assert_eq!(result, vec!["user_role.md", "project_auth.md"]);
    }

    #[test]
    fn parse_empty_selected() {
        let response = r#"{"selected": []}"#;
        let result = parse_ranker_response(response, SAMPLE_MANIFEST).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_empty() {
        let response = "I don't know what to select";
        let result = parse_ranker_response(response, SAMPLE_MANIFEST).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn filter_hallucinated_filenames() {
        let response = r#"{"selected": ["user_role.md", "nonexistent.md", "fake.md"]}"#;
        let result = parse_ranker_response(response, SAMPLE_MANIFEST).unwrap();
        assert_eq!(result, vec!["user_role.md"]);
    }

    #[test]
    fn extract_json_from_text() {
        assert_eq!(extract_json("  {\"a\": 1}  "), "{\"a\": 1}");
        assert_eq!(extract_json("```json\n[1,2]\n```"), "[1,2]");
        assert_eq!(extract_json("plain text"), "plain text");
    }
}
