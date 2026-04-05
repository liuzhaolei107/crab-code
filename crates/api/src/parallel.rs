//! Parallel inference — race multiple models and take the fastest response.
//!
//! Sends the same request to multiple models concurrently. The first successful
//! response wins; remaining requests are cancelled via `CancellationToken`.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::types::MessageResponse;

/// Configuration for parallel inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Models to race against each other.
    pub models: Vec<ParallelModel>,
    /// Global timeout for the entire race (all models).
    #[serde(default = "default_race_timeout")]
    pub race_timeout: Duration,
    /// Strategy for selecting the winner.
    #[serde(default)]
    pub strategy: RaceStrategy,
    /// Whether to cancel losing requests (saves API cost).
    #[serde(default = "default_true")]
    pub cancel_losers: bool,
}

fn default_race_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_true() -> bool {
    true
}

/// A model participating in parallel inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelModel {
    /// Model identifier.
    pub model_id: String,
    /// Provider name.
    pub provider: String,
    /// Weight for weighted-random selection (higher = more likely to be picked).
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_weight() -> u32 {
    1
}

/// Strategy for picking the winner in a race.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RaceStrategy {
    /// First successful response wins.
    #[default]
    Fastest,
    /// Collect all responses and pick the longest (most detailed).
    MostDetailed,
    /// Collect all and pick by quality heuristic (token count, etc.).
    BestQuality,
}

impl std::fmt::Display for RaceStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fastest => write!(f, "fastest"),
            Self::MostDetailed => write!(f, "most_detailed"),
            Self::BestQuality => write!(f, "best_quality"),
        }
    }
}

impl ParallelConfig {
    /// Create a config that races two models.
    #[must_use]
    pub fn race(
        model_a: (impl Into<String>, impl Into<String>),
        model_b: (impl Into<String>, impl Into<String>),
    ) -> Self {
        Self {
            models: vec![
                ParallelModel {
                    model_id: model_a.0.into(),
                    provider: model_a.1.into(),
                    weight: 1,
                },
                ParallelModel {
                    model_id: model_b.0.into(),
                    provider: model_b.1.into(),
                    weight: 1,
                },
            ],
            race_timeout: default_race_timeout(),
            strategy: RaceStrategy::Fastest,
            cancel_losers: true,
        }
    }

    /// Number of models in the race.
    #[must_use]
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Whether no models are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Add a model to the race.
    pub fn add_model(&mut self, model_id: impl Into<String>, provider: impl Into<String>) {
        self.models.push(ParallelModel {
            model_id: model_id.into(),
            provider: provider.into(),
            weight: 1,
        });
    }

    /// Set the race strategy.
    #[must_use]
    pub fn with_strategy(mut self, strategy: RaceStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the race timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.race_timeout = timeout;
        self
    }
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            race_timeout: default_race_timeout(),
            strategy: RaceStrategy::Fastest,
            cancel_losers: true,
        }
    }
}

/// Result of a parallel inference race.
#[derive(Debug, Clone)]
pub struct RaceResult {
    /// The winning response.
    pub response: MessageResponse,
    /// Which model produced the winning response.
    pub winner_model: String,
    /// Which provider the winner came from.
    pub winner_provider: String,
    /// How long the winning response took.
    pub latency: Duration,
    /// Results from all models (for `MostDetailed` / `BestQuality` strategies).
    pub all_results: Vec<ModelResult>,
}

/// Result from a single model in the race.
#[derive(Debug, Clone)]
pub struct ModelResult {
    /// Model identifier.
    pub model_id: String,
    /// Provider name.
    pub provider: String,
    /// The response, if successful.
    pub response: Option<MessageResponse>,
    /// Error message, if failed.
    pub error: Option<String>,
    /// How long this model took.
    pub latency: Duration,
}

impl ModelResult {
    /// Whether this model succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.response.is_some()
    }
}

/// Select the best response from multiple results based on strategy.
#[must_use]
pub fn select_winner(results: &[ModelResult], strategy: RaceStrategy) -> Option<usize> {
    if results.is_empty() {
        return None;
    }

    let successful: Vec<(usize, &ModelResult)> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| r.is_success())
        .collect();

    if successful.is_empty() {
        return None;
    }

    match strategy {
        RaceStrategy::Fastest => successful
            .iter()
            .min_by_key(|(_, r)| r.latency)
            .map(|(i, _)| *i),
        RaceStrategy::MostDetailed => successful
            .iter()
            .max_by_key(|(_, r)| {
                r.response
                    .as_ref()
                    .map_or(0, |resp| resp.message.text().len())
            })
            .map(|(i, _)| *i),
        RaceStrategy::BestQuality => {
            // Heuristic: prefer responses with more output tokens (more thorough)
            // but penalize very short responses
            successful
                .iter()
                .max_by_key(|(_, r)| {
                    r.response.as_ref().map_or(0, |resp| {
                        let text_len = resp.message.text().len();
                        #[allow(clippy::cast_possible_truncation)]
                        let token_score = resp.usage.output_tokens as usize;
                        // Weighted combination: output tokens matter more
                        token_score * 2 + text_len
                    })
                })
                .map(|(i, _)| *i)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::message::Message;
    use crab_core::model::TokenUsage;

    fn test_response(text: &str, output_tokens: u64) -> MessageResponse {
        MessageResponse {
            id: "msg_01".into(),
            message: Message::assistant(text),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
            },
        }
    }

    #[test]
    fn parallel_config_race() {
        let config = ParallelConfig::race(
            ("claude-sonnet-4-20250514", "anthropic"),
            ("gpt-4o", "openai"),
        );
        assert_eq!(config.len(), 2);
        assert_eq!(config.strategy, RaceStrategy::Fastest);
        assert!(config.cancel_losers);
    }

    #[test]
    fn parallel_config_default() {
        let config = ParallelConfig::default();
        assert!(config.is_empty());
        assert_eq!(config.strategy, RaceStrategy::Fastest);
    }

    #[test]
    fn parallel_config_add_model() {
        let mut config = ParallelConfig::default();
        config.add_model("model-a", "prov-a");
        config.add_model("model-b", "prov-b");
        assert_eq!(config.len(), 2);
    }

    #[test]
    fn parallel_config_builder() {
        let config = ParallelConfig::race(("a", "pa"), ("b", "pb"))
            .with_strategy(RaceStrategy::MostDetailed)
            .with_timeout(Duration::from_secs(30));
        assert_eq!(config.strategy, RaceStrategy::MostDetailed);
        assert_eq!(config.race_timeout, Duration::from_secs(30));
    }

    #[test]
    fn parallel_config_serde_roundtrip() {
        let config = ParallelConfig::race(
            ("claude-sonnet-4-20250514", "anthropic"),
            ("gpt-4o", "openai"),
        )
        .with_strategy(RaceStrategy::BestQuality);
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ParallelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.strategy, RaceStrategy::BestQuality);
    }

    #[test]
    fn race_strategy_display() {
        assert_eq!(RaceStrategy::Fastest.to_string(), "fastest");
        assert_eq!(RaceStrategy::MostDetailed.to_string(), "most_detailed");
        assert_eq!(RaceStrategy::BestQuality.to_string(), "best_quality");
    }

    #[test]
    fn race_strategy_default() {
        assert_eq!(RaceStrategy::default(), RaceStrategy::Fastest);
    }

    #[test]
    fn select_winner_fastest() {
        let results = vec![
            ModelResult {
                model_id: "slow".into(),
                provider: "p".into(),
                response: Some(test_response("slow response", 50)),
                error: None,
                latency: Duration::from_secs(5),
            },
            ModelResult {
                model_id: "fast".into(),
                provider: "p".into(),
                response: Some(test_response("fast response", 30)),
                error: None,
                latency: Duration::from_secs(1),
            },
        ];

        let winner = select_winner(&results, RaceStrategy::Fastest).unwrap();
        assert_eq!(winner, 1); // "fast"
    }

    #[test]
    fn select_winner_most_detailed() {
        let results = vec![
            ModelResult {
                model_id: "short".into(),
                provider: "p".into(),
                response: Some(test_response("hi", 5)),
                error: None,
                latency: Duration::from_secs(1),
            },
            ModelResult {
                model_id: "long".into(),
                provider: "p".into(),
                response: Some(test_response("a very detailed and thorough response", 100)),
                error: None,
                latency: Duration::from_secs(3),
            },
        ];

        let winner = select_winner(&results, RaceStrategy::MostDetailed).unwrap();
        assert_eq!(winner, 1); // "long"
    }

    #[test]
    fn select_winner_best_quality() {
        let results = vec![
            ModelResult {
                model_id: "low-tokens".into(),
                provider: "p".into(),
                response: Some(test_response("short", 10)),
                error: None,
                latency: Duration::from_secs(1),
            },
            ModelResult {
                model_id: "high-tokens".into(),
                provider: "p".into(),
                response: Some(test_response("detailed", 200)),
                error: None,
                latency: Duration::from_secs(2),
            },
        ];

        let winner = select_winner(&results, RaceStrategy::BestQuality).unwrap();
        assert_eq!(winner, 1); // "high-tokens" — higher output token count
    }

    #[test]
    fn select_winner_skips_failures() {
        let results = vec![
            ModelResult {
                model_id: "failed".into(),
                provider: "p".into(),
                response: None,
                error: Some("timeout".into()),
                latency: Duration::from_millis(100), // fastest but failed
            },
            ModelResult {
                model_id: "success".into(),
                provider: "p".into(),
                response: Some(test_response("ok", 20)),
                error: None,
                latency: Duration::from_secs(2),
            },
        ];

        let winner = select_winner(&results, RaceStrategy::Fastest).unwrap();
        assert_eq!(winner, 1); // only successful result
    }

    #[test]
    fn select_winner_empty() {
        assert!(select_winner(&[], RaceStrategy::Fastest).is_none());
    }

    #[test]
    fn select_winner_all_failed() {
        let results = vec![ModelResult {
            model_id: "failed".into(),
            provider: "p".into(),
            response: None,
            error: Some("err".into()),
            latency: Duration::from_secs(1),
        }];
        assert!(select_winner(&results, RaceStrategy::Fastest).is_none());
    }

    #[test]
    fn model_result_is_success() {
        let success = ModelResult {
            model_id: "m".into(),
            provider: "p".into(),
            response: Some(test_response("ok", 10)),
            error: None,
            latency: Duration::from_secs(1),
        };
        assert!(success.is_success());

        let failure = ModelResult {
            model_id: "m".into(),
            provider: "p".into(),
            response: None,
            error: Some("err".into()),
            latency: Duration::from_secs(1),
        };
        assert!(!failure.is_success());
    }

    #[test]
    fn race_result_fields() {
        let result = RaceResult {
            response: test_response("winner", 50),
            winner_model: "claude-sonnet-4-20250514".into(),
            winner_provider: "anthropic".into(),
            latency: Duration::from_secs(2),
            all_results: vec![],
        };
        assert_eq!(result.winner_model, "claude-sonnet-4-20250514");
        assert_eq!(result.latency, Duration::from_secs(2));
    }
}
