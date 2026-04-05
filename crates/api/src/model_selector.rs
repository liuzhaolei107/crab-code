//! Automatic model selection based on task type.
//!
//! Maps task categories (code generation, Q&A, translation, etc.) to
//! optimal models based on configurable rules and provider availability.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Task type categories for model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Code generation, completion, refactoring.
    CodeGeneration,
    /// Code review, explanation, analysis.
    CodeReview,
    /// General question answering and conversation.
    QuestionAnswering,
    /// Translation between natural languages.
    Translation,
    /// Summarization of text or code.
    Summarization,
    /// Creative writing, brainstorming.
    Creative,
    /// Data analysis, structured output.
    DataAnalysis,
    /// Math, logic, reasoning tasks.
    Reasoning,
    /// General-purpose (no specific category).
    General,
}

impl TaskType {
    /// All known task types.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::CodeGeneration,
            Self::CodeReview,
            Self::QuestionAnswering,
            Self::Translation,
            Self::Summarization,
            Self::Creative,
            Self::DataAnalysis,
            Self::Reasoning,
            Self::General,
        ]
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CodeGeneration => write!(f, "code_generation"),
            Self::CodeReview => write!(f, "code_review"),
            Self::QuestionAnswering => write!(f, "question_answering"),
            Self::Translation => write!(f, "translation"),
            Self::Summarization => write!(f, "summarization"),
            Self::Creative => write!(f, "creative"),
            Self::DataAnalysis => write!(f, "data_analysis"),
            Self::Reasoning => write!(f, "reasoning"),
            Self::General => write!(f, "general"),
        }
    }
}

/// A model recommendation with priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    /// Model identifier.
    pub model_id: String,
    /// Provider name.
    pub provider: String,
    /// Priority (lower = preferred).
    pub priority: u32,
    /// Why this model is recommended for this task type.
    pub reason: String,
}

/// Model selector — picks the best model for a given task type.
pub struct ModelSelector {
    /// Rules mapping task types to recommended models.
    rules: HashMap<TaskType, Vec<ModelRecommendation>>,
    /// Default model when no rule matches.
    default_model: String,
    /// Default provider.
    default_provider: String,
}

impl ModelSelector {
    /// Create a new selector with a default model.
    #[must_use]
    pub fn new(default_model: impl Into<String>, default_provider: impl Into<String>) -> Self {
        Self {
            rules: HashMap::new(),
            default_model: default_model.into(),
            default_provider: default_provider.into(),
        }
    }

    /// Create a selector pre-loaded with sensible defaults for common providers.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut selector = Self::new("claude-sonnet-4-20250514", "anthropic");

        selector.add_rule(
            TaskType::CodeGeneration,
            "claude-sonnet-4-20250514",
            "anthropic",
            1,
            "Strong at code generation with tool use",
        );
        selector.add_rule(
            TaskType::CodeReview,
            "claude-sonnet-4-20250514",
            "anthropic",
            1,
            "Thorough code analysis and explanation",
        );
        selector.add_rule(
            TaskType::Reasoning,
            "claude-sonnet-4-20250514",
            "anthropic",
            1,
            "Strong reasoning capabilities",
        );
        selector.add_rule(
            TaskType::Creative,
            "claude-sonnet-4-20250514",
            "anthropic",
            1,
            "Nuanced creative writing",
        );
        selector.add_rule(
            TaskType::QuestionAnswering,
            "claude-sonnet-4-20250514",
            "anthropic",
            1,
            "Accurate factual answers",
        );
        selector.add_rule(
            TaskType::General,
            "claude-sonnet-4-20250514",
            "anthropic",
            1,
            "Good general-purpose model",
        );

        selector
    }

    /// Add a model recommendation rule.
    pub fn add_rule(
        &mut self,
        task_type: TaskType,
        model_id: impl Into<String>,
        provider: impl Into<String>,
        priority: u32,
        reason: impl Into<String>,
    ) {
        let rec = ModelRecommendation {
            model_id: model_id.into(),
            provider: provider.into(),
            priority,
            reason: reason.into(),
        };

        self.rules.entry(task_type).or_default().push(rec);

        // Keep sorted by priority
        if let Some(recs) = self.rules.get_mut(&task_type) {
            recs.sort_by_key(|r| r.priority);
        }
    }

    /// Select the best model for a task type.
    ///
    /// Returns the highest-priority recommendation, or the default model.
    #[must_use]
    pub fn select(&self, task_type: TaskType) -> (&str, &str) {
        self.rules
            .get(&task_type)
            .and_then(|recs| recs.first())
            .map_or(
                (self.default_model.as_str(), self.default_provider.as_str()),
                |rec| (rec.model_id.as_str(), rec.provider.as_str()),
            )
    }

    /// Get all recommendations for a task type, sorted by priority.
    #[must_use]
    pub fn recommendations(&self, task_type: TaskType) -> &[ModelRecommendation] {
        self.rules.get(&task_type).map_or(&[], Vec::as_slice)
    }

    /// Select the best model, excluding specific providers.
    ///
    /// Useful when a provider is down and you want to find an alternative.
    #[must_use]
    pub fn select_excluding(
        &self,
        task_type: TaskType,
        excluded_providers: &[&str],
    ) -> (&str, &str) {
        if let Some(recs) = self.rules.get(&task_type) {
            for rec in recs {
                if !excluded_providers.contains(&rec.provider.as_str()) {
                    return (&rec.model_id, &rec.provider);
                }
            }
        }

        // Fall back to default if not excluded
        if !excluded_providers.contains(&self.default_provider.as_str()) {
            return (&self.default_model, &self.default_provider);
        }

        // Everything excluded — return default anyway (caller decides)
        (&self.default_model, &self.default_provider)
    }

    /// Detect task type from user input using keyword matching.
    #[must_use]
    pub fn detect_task_type(input: &str) -> TaskType {
        let lower = input.to_lowercase();

        // Check keywords in priority order (most specific first)
        if contains_any(
            &lower,
            &[
                "translate",
                "translation",
                "翻译",
                "convert to",
                "in japanese",
                "in chinese",
                "in spanish",
                "in french",
                "in german",
            ],
        ) {
            return TaskType::Translation;
        }

        if contains_any(
            &lower,
            &[
                "write code",
                "implement",
                "create function",
                "generate",
                "scaffold",
                "boilerplate",
                "add feature",
                "new file",
                "write a",
                "build",
            ],
        ) {
            return TaskType::CodeGeneration;
        }

        if contains_any(
            &lower,
            &[
                "review",
                "explain",
                "what does",
                "how does",
                "analyze",
                "code review",
                "walk through",
            ],
        ) {
            return TaskType::CodeReview;
        }

        if contains_any(
            &lower,
            &[
                "summarize",
                "summary",
                "tldr",
                "brief",
                "condense",
                "key points",
            ],
        ) {
            return TaskType::Summarization;
        }

        if contains_any(
            &lower,
            &[
                "data",
                "csv",
                "json",
                "parse",
                "extract",
                "structured",
                "table",
                "format",
            ],
        ) {
            return TaskType::DataAnalysis;
        }

        if contains_any(
            &lower,
            &[
                "math",
                "calculate",
                "proof",
                "logic",
                "reason",
                "solve",
                "equation",
            ],
        ) {
            return TaskType::Reasoning;
        }

        if contains_any(
            &lower,
            &[
                "creative",
                "story",
                "poem",
                "brainstorm",
                "imagine",
                "fiction",
            ],
        ) {
            return TaskType::Creative;
        }

        if contains_any(
            &lower,
            &["what is", "who is", "when", "where", "why", "how to", "?"],
        ) {
            return TaskType::QuestionAnswering;
        }

        TaskType::General
    }

    /// Number of task types with rules.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// The default model.
    #[must_use]
    pub fn default_model(&self) -> (&str, &str) {
        (&self.default_model, &self.default_provider)
    }
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_type_all() {
        assert_eq!(TaskType::all().len(), 9);
    }

    #[test]
    fn task_type_display() {
        assert_eq!(TaskType::CodeGeneration.to_string(), "code_generation");
        assert_eq!(
            TaskType::QuestionAnswering.to_string(),
            "question_answering"
        );
        assert_eq!(TaskType::General.to_string(), "general");
    }

    #[test]
    fn task_type_serde_roundtrip() {
        let tt = TaskType::CodeGeneration;
        let json = serde_json::to_string(&tt).unwrap();
        assert_eq!(json, "\"code_generation\"");
        let parsed: TaskType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, tt);
    }

    #[test]
    fn selector_new() {
        let selector = ModelSelector::new("test-model", "test-provider");
        assert_eq!(selector.default_model(), ("test-model", "test-provider"));
        assert_eq!(selector.rule_count(), 0);
    }

    #[test]
    fn selector_with_defaults() {
        let selector = ModelSelector::with_defaults();
        assert!(selector.rule_count() > 0);
        let (model, provider) = selector.select(TaskType::CodeGeneration);
        assert_eq!(provider, "anthropic");
        assert!(model.contains("claude"));
    }

    #[test]
    fn selector_add_rule() {
        let mut selector = ModelSelector::new("default", "default-prov");
        selector.add_rule(
            TaskType::Translation,
            "gpt-4o",
            "openai",
            1,
            "Good at translation",
        );

        let (model, provider) = selector.select(TaskType::Translation);
        assert_eq!(model, "gpt-4o");
        assert_eq!(provider, "openai");
    }

    #[test]
    fn selector_fallback_to_default() {
        let selector = ModelSelector::new("default-model", "default-prov");
        let (model, provider) = selector.select(TaskType::Creative);
        assert_eq!(model, "default-model");
        assert_eq!(provider, "default-prov");
    }

    #[test]
    fn selector_priority_ordering() {
        let mut selector = ModelSelector::new("default", "prov");
        selector.add_rule(TaskType::CodeGeneration, "low-priority", "prov", 10, "ok");
        selector.add_rule(TaskType::CodeGeneration, "high-priority", "prov", 1, "best");

        let (model, _) = selector.select(TaskType::CodeGeneration);
        assert_eq!(model, "high-priority");
    }

    #[test]
    fn selector_recommendations() {
        let mut selector = ModelSelector::new("default", "prov");
        selector.add_rule(TaskType::Reasoning, "model-a", "prov-a", 1, "reason a");
        selector.add_rule(TaskType::Reasoning, "model-b", "prov-b", 2, "reason b");

        let recs = selector.recommendations(TaskType::Reasoning);
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].model_id, "model-a");
        assert_eq!(recs[1].model_id, "model-b");
    }

    #[test]
    fn selector_recommendations_empty() {
        let selector = ModelSelector::new("default", "prov");
        assert!(selector.recommendations(TaskType::Creative).is_empty());
    }

    #[test]
    fn selector_excluding_provider() {
        let mut selector = ModelSelector::new("default", "anthropic");
        selector.add_rule(
            TaskType::CodeGeneration,
            "claude",
            "anthropic",
            1,
            "primary",
        );
        selector.add_rule(TaskType::CodeGeneration, "gpt-4o", "openai", 2, "backup");

        let (model, provider) = selector.select_excluding(TaskType::CodeGeneration, &["anthropic"]);
        assert_eq!(model, "gpt-4o");
        assert_eq!(provider, "openai");
    }

    #[test]
    fn selector_excluding_all_falls_back() {
        let selector = ModelSelector::new("fallback", "fallback-prov");
        let (model, _) =
            selector.select_excluding(TaskType::General, &["anthropic", "openai", "fallback-prov"]);
        // Returns default even when excluded (caller decides what to do)
        assert_eq!(model, "fallback");
    }

    #[test]
    fn detect_code_generation() {
        assert_eq!(
            ModelSelector::detect_task_type("implement a new auth module"),
            TaskType::CodeGeneration
        );
        assert_eq!(
            ModelSelector::detect_task_type("write code for sorting"),
            TaskType::CodeGeneration
        );
    }

    #[test]
    fn detect_code_review() {
        assert_eq!(
            ModelSelector::detect_task_type("review this function"),
            TaskType::CodeReview
        );
        assert_eq!(
            ModelSelector::detect_task_type("explain what does this do"),
            TaskType::CodeReview
        );
    }

    #[test]
    fn detect_translation() {
        assert_eq!(
            ModelSelector::detect_task_type("translate this to Japanese"),
            TaskType::Translation
        );
        assert_eq!(
            ModelSelector::detect_task_type("翻译这段文字"),
            TaskType::Translation
        );
    }

    #[test]
    fn detect_summarization() {
        assert_eq!(
            ModelSelector::detect_task_type("summarize this document"),
            TaskType::Summarization
        );
    }

    #[test]
    fn detect_reasoning() {
        assert_eq!(
            ModelSelector::detect_task_type("solve this equation"),
            TaskType::Reasoning
        );
    }

    #[test]
    fn detect_creative() {
        assert_eq!(
            ModelSelector::detect_task_type("brainstorm ideas for a story"),
            TaskType::Creative
        );
    }

    #[test]
    fn detect_question_answering() {
        assert_eq!(
            ModelSelector::detect_task_type("what is a closure?"),
            TaskType::QuestionAnswering
        );
    }

    #[test]
    fn detect_data_analysis() {
        assert_eq!(
            ModelSelector::detect_task_type("parse this CSV data"),
            TaskType::DataAnalysis
        );
    }

    #[test]
    fn detect_general() {
        assert_eq!(
            ModelSelector::detect_task_type("hello there"),
            TaskType::General
        );
    }

    #[test]
    fn model_recommendation_fields() {
        let rec = ModelRecommendation {
            model_id: "test".into(),
            provider: "prov".into(),
            priority: 1,
            reason: "best for this".into(),
        };
        assert_eq!(rec.model_id, "test");
        assert_eq!(rec.priority, 1);
    }
}
