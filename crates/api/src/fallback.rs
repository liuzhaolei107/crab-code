//! Model fallback chain — automatic failover to backup models.
//!
//! When the primary model fails or times out, the chain tries the next
//! model in sequence until one succeeds or all are exhausted.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// Configuration for the model fallback chain.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FallbackConfig {
    /// Ordered list of models to try (first = primary).
    pub models: Vec<FallbackModel>,
    /// Whether to enable fallback (if false, only the first model is used).
    pub enabled: bool,
}

/// A model entry in the fallback chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackModel {
    /// Model identifier (e.g., "claude-sonnet-4-20250514").
    pub model_id: String,
    /// Provider name (e.g., "anthropic", "openai").
    pub provider: String,
    /// Per-model timeout override. If `None`, uses the global timeout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,
    /// Maximum retries for this specific model before moving to the next.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_max_retries() -> u32 {
    1
}

impl FallbackConfig {
    /// Create a single-model config (no fallback).
    #[must_use]
    pub fn single(model_id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            models: vec![FallbackModel {
                model_id: model_id.into(),
                provider: provider.into(),
                timeout: None,
                max_retries: 1,
            }],
            enabled: false,
        }
    }

    /// Create a fallback chain from a list of (`model_id`, provider) pairs.
    #[must_use]
    pub fn chain(models: Vec<(impl Into<String>, impl Into<String>)>) -> Self {
        Self {
            models: models
                .into_iter()
                .map(|(id, prov)| FallbackModel {
                    model_id: id.into(),
                    provider: prov.into(),
                    timeout: None,
                    max_retries: 1,
                })
                .collect(),
            enabled: true,
        }
    }

    /// Number of models in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Whether the chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Get the primary model.
    #[must_use]
    pub fn primary(&self) -> Option<&FallbackModel> {
        self.models.first()
    }

    /// Add a fallback model to the end of the chain.
    pub fn add_fallback(&mut self, model_id: impl Into<String>, provider: impl Into<String>) {
        self.models.push(FallbackModel {
            model_id: model_id.into(),
            provider: provider.into(),
            timeout: None,
            max_retries: 1,
        });
        if self.models.len() > 1 {
            self.enabled = true;
        }
    }
}

/// Tracks the state of a fallback chain execution.
pub struct FallbackChain {
    config: FallbackConfig,
    current_index: usize,
    current_retries: u32,
    errors: Vec<FallbackError>,
}

/// An error from a specific model in the chain.
#[derive(Debug, Clone)]
pub struct FallbackError {
    /// Which model failed.
    pub model_id: String,
    /// The error message.
    pub error: String,
    /// How many attempts were made on this model.
    pub attempts: u32,
}

/// Result of advancing the fallback chain after a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackDecision {
    /// Retry the same model.
    Retry {
        model_id: String,
        provider: String,
        attempt: u32,
    },
    /// Move to the next model in the chain.
    NextModel { model_id: String, provider: String },
    /// All models exhausted.
    Exhausted,
}

impl FallbackChain {
    /// Create a new fallback chain from config.
    #[must_use]
    pub fn new(config: FallbackConfig) -> Self {
        Self {
            config,
            current_index: 0,
            current_retries: 0,
            errors: Vec::new(),
        }
    }

    /// Get the current model to try.
    #[must_use]
    pub fn current_model(&self) -> Option<&FallbackModel> {
        self.config.models.get(self.current_index)
    }

    /// Report a failure and get the next decision.
    ///
    /// If fallback is disabled, only retries the first model up to its `max_retries`.
    pub fn on_failure(&mut self, error: &str) -> FallbackDecision {
        let Some(model) = self.config.models.get(self.current_index) else {
            return FallbackDecision::Exhausted;
        };

        self.current_retries += 1;

        // Can we retry this model?
        if self.current_retries < model.max_retries {
            return FallbackDecision::Retry {
                model_id: model.model_id.clone(),
                provider: model.provider.clone(),
                attempt: self.current_retries,
            };
        }

        // Record the error for this model
        self.errors.push(FallbackError {
            model_id: model.model_id.clone(),
            error: error.to_string(),
            attempts: self.current_retries,
        });

        // Try next model if fallback is enabled
        if self.config.enabled {
            self.current_index += 1;
            self.current_retries = 0;

            if let Some(next) = self.config.models.get(self.current_index) {
                return FallbackDecision::NextModel {
                    model_id: next.model_id.clone(),
                    provider: next.provider.clone(),
                };
            }
        }

        // Mark as exhausted so is_exhausted() returns true
        self.current_index = self.config.models.len();
        FallbackDecision::Exhausted
    }

    /// Report a success. Resets the chain for next use.
    pub fn on_success(&mut self) {
        self.current_index = 0;
        self.current_retries = 0;
        self.errors.clear();
    }

    /// Get all errors accumulated during the chain execution.
    #[must_use]
    pub fn errors(&self) -> &[FallbackError] {
        &self.errors
    }

    /// Whether the chain has been exhausted (all models failed).
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.current_index >= self.config.models.len()
    }

    /// Reset the chain to start from the first model.
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.current_retries = 0;
        self.errors.clear();
    }

    /// Whether an API error should trigger a fallback (vs. being returned directly).
    ///
    /// Only transient / server-side errors trigger fallback. Client errors (400, 401)
    /// are returned immediately since a different model won't fix them.
    #[must_use]
    pub fn should_fallback(err: &ApiError) -> bool {
        match err {
            ApiError::RateLimited { .. } | ApiError::Timeout => true,
            ApiError::Api { status, .. } => {
                *status == 429 || *status == 529 || (500..600).contains(status)
            }
            ApiError::Http(e) => e.is_timeout() || e.is_connect(),
            ApiError::Json(_) | ApiError::Sse(_) | ApiError::Common(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_config_single() {
        let config = FallbackConfig::single("claude-sonnet-4-20250514", "anthropic");
        assert_eq!(config.len(), 1);
        assert!(!config.enabled);
        assert_eq!(
            config.primary().unwrap().model_id,
            "claude-sonnet-4-20250514"
        );
    }

    #[test]
    fn fallback_config_chain() {
        let config = FallbackConfig::chain(vec![
            ("claude-sonnet-4-20250514", "anthropic"),
            ("gpt-4o", "openai"),
            ("deepseek-chat", "openai"),
        ]);
        assert_eq!(config.len(), 3);
        assert!(config.enabled);
    }

    #[test]
    fn fallback_config_default_empty() {
        let config = FallbackConfig::default();
        assert!(config.is_empty());
        assert!(!config.enabled);
        assert!(config.primary().is_none());
    }

    #[test]
    fn fallback_config_add_fallback() {
        let mut config = FallbackConfig::single("claude-sonnet-4-20250514", "anthropic");
        assert!(!config.enabled);

        config.add_fallback("gpt-4o", "openai");
        assert!(config.enabled);
        assert_eq!(config.len(), 2);
    }

    #[test]
    fn fallback_config_serde_roundtrip() {
        let config = FallbackConfig::chain(vec![
            ("claude-sonnet-4-20250514", "anthropic"),
            ("gpt-4o", "openai"),
        ]);
        let json = serde_json::to_string(&config).unwrap();
        let parsed: FallbackConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(parsed.enabled);
        assert_eq!(parsed.models[0].model_id, "claude-sonnet-4-20250514");
        assert_eq!(parsed.models[1].model_id, "gpt-4o");
    }

    #[test]
    fn chain_single_model_no_fallback() {
        let config = FallbackConfig::single("claude-sonnet-4-20250514", "anthropic");
        let mut chain = FallbackChain::new(config);

        assert_eq!(
            chain.current_model().unwrap().model_id,
            "claude-sonnet-4-20250514"
        );

        // First failure exhausts the single model (max_retries=1, so 0 retries)
        let decision = chain.on_failure("timeout");
        assert_eq!(decision, FallbackDecision::Exhausted);
        assert!(chain.is_exhausted());
    }

    #[test]
    fn chain_fallback_to_next_model() {
        let config = FallbackConfig::chain(vec![
            ("claude-sonnet-4-20250514", "anthropic"),
            ("gpt-4o", "openai"),
        ]);
        let mut chain = FallbackChain::new(config);

        let decision = chain.on_failure("rate limited");
        assert_eq!(
            decision,
            FallbackDecision::NextModel {
                model_id: "gpt-4o".into(),
                provider: "openai".into(),
            }
        );

        assert_eq!(chain.current_model().unwrap().model_id, "gpt-4o");
    }

    #[test]
    fn chain_all_models_exhausted() {
        let config = FallbackConfig::chain(vec![("model-a", "prov-a"), ("model-b", "prov-b")]);
        let mut chain = FallbackChain::new(config);

        let d1 = chain.on_failure("error-a");
        assert!(matches!(d1, FallbackDecision::NextModel { .. }));

        let d2 = chain.on_failure("error-b");
        assert_eq!(d2, FallbackDecision::Exhausted);

        assert_eq!(chain.errors().len(), 2);
        assert_eq!(chain.errors()[0].model_id, "model-a");
        assert_eq!(chain.errors()[1].model_id, "model-b");
    }

    #[test]
    fn chain_retry_before_fallback() {
        let mut config = FallbackConfig::chain(vec![("model-a", "prov-a"), ("model-b", "prov-b")]);
        config.models[0].max_retries = 3;

        let mut chain = FallbackChain::new(config);

        // First two failures should retry
        let d1 = chain.on_failure("err");
        assert_eq!(
            d1,
            FallbackDecision::Retry {
                model_id: "model-a".into(),
                provider: "prov-a".into(),
                attempt: 1,
            }
        );

        let d2 = chain.on_failure("err");
        assert_eq!(
            d2,
            FallbackDecision::Retry {
                model_id: "model-a".into(),
                provider: "prov-a".into(),
                attempt: 2,
            }
        );

        // Third failure exhausts retries, falls back
        let d3 = chain.on_failure("err");
        assert_eq!(
            d3,
            FallbackDecision::NextModel {
                model_id: "model-b".into(),
                provider: "prov-b".into(),
            }
        );
    }

    #[test]
    fn chain_on_success_resets() {
        let config = FallbackConfig::chain(vec![("model-a", "prov-a"), ("model-b", "prov-b")]);
        let mut chain = FallbackChain::new(config);

        chain.on_failure("err");
        assert!(!chain.errors().is_empty());

        chain.on_success();
        assert!(chain.errors().is_empty());
        assert_eq!(chain.current_model().unwrap().model_id, "model-a");
    }

    #[test]
    fn chain_reset() {
        let config = FallbackConfig::chain(vec![("model-a", "prov-a"), ("model-b", "prov-b")]);
        let mut chain = FallbackChain::new(config);

        chain.on_failure("err"); // moves to model-b
        chain.reset();

        assert_eq!(chain.current_model().unwrap().model_id, "model-a");
        assert!(chain.errors().is_empty());
    }

    #[test]
    fn should_fallback_transient_errors() {
        assert!(FallbackChain::should_fallback(&ApiError::Timeout));
        assert!(FallbackChain::should_fallback(&ApiError::RateLimited {
            retry_after_ms: 1000,
        }));
        assert!(FallbackChain::should_fallback(&ApiError::Api {
            status: 500,
            message: "internal".into(),
        }));
        assert!(FallbackChain::should_fallback(&ApiError::Api {
            status: 429,
            message: "rate limited".into(),
        }));
        assert!(FallbackChain::should_fallback(&ApiError::Api {
            status: 529,
            message: "overloaded".into(),
        }));
    }

    #[test]
    fn should_not_fallback_client_errors() {
        assert!(!FallbackChain::should_fallback(&ApiError::Api {
            status: 400,
            message: "bad request".into(),
        }));
        assert!(!FallbackChain::should_fallback(&ApiError::Api {
            status: 401,
            message: "unauthorized".into(),
        }));
        assert!(!FallbackChain::should_fallback(&ApiError::Sse(
            "parse".into()
        )));
    }

    #[test]
    fn fallback_error_fields() {
        let err = FallbackError {
            model_id: "test".into(),
            error: "timeout".into(),
            attempts: 3,
        };
        assert_eq!(err.model_id, "test");
        assert_eq!(err.attempts, 3);
    }

    #[test]
    fn chain_empty_config() {
        let config = FallbackConfig::default();
        let mut chain = FallbackChain::new(config);

        assert!(chain.current_model().is_none());
        assert_eq!(chain.on_failure("err"), FallbackDecision::Exhausted);
    }
}
