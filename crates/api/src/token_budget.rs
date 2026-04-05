//! Token budget management — auto-calculate `max_tokens` based on context window.
//!
//! Ensures requests don't exceed the model's context window by computing
//! the available output budget from: `context_window - input_tokens - reserved`.

use crate::capabilities::ModelCapabilities;

/// Configuration for token budget calculation.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// Total context window size (tokens).
    context_window: u32,
    /// Maximum output tokens the model supports.
    max_output_tokens: u32,
    /// Reserved tokens for system prompt overhead (default: 1000).
    reserved_tokens: u32,
    /// Minimum output tokens to request (default: 256).
    min_output_tokens: u32,
}

impl TokenBudget {
    /// Create a budget from model capabilities.
    #[must_use]
    pub fn from_capabilities(caps: &ModelCapabilities) -> Self {
        Self {
            context_window: caps.context_window,
            max_output_tokens: caps.max_output_tokens,
            reserved_tokens: 1000,
            min_output_tokens: 256,
        }
    }

    /// Create a budget with explicit values.
    #[must_use]
    pub fn new(context_window: u32, max_output_tokens: u32) -> Self {
        Self {
            context_window,
            max_output_tokens,
            reserved_tokens: 1000,
            min_output_tokens: 256,
        }
    }

    /// Set the reserved token count (for system prompt overhead, etc.).
    #[must_use]
    pub fn with_reserved(mut self, reserved: u32) -> Self {
        self.reserved_tokens = reserved;
        self
    }

    /// Set the minimum output token count.
    #[must_use]
    pub fn with_min_output(mut self, min: u32) -> Self {
        self.min_output_tokens = min;
        self
    }

    /// Calculate the optimal `max_tokens` given current input token usage.
    ///
    /// Formula: `min(max_output_tokens, context_window - input_tokens - reserved)`
    /// Clamped to `[min_output_tokens, max_output_tokens]`.
    ///
    /// If the input already exceeds the context window (minus reserved + `min_output`),
    /// returns `min_output_tokens` as a safety floor.
    #[must_use]
    pub fn calculate_max_tokens(&self, input_tokens: u32) -> u32 {
        let available = self
            .context_window
            .saturating_sub(input_tokens)
            .saturating_sub(self.reserved_tokens);

        available
            .max(self.min_output_tokens)
            .min(self.max_output_tokens)
    }

    /// Check whether the input token count exceeds the safe threshold.
    ///
    /// Returns `true` when `input_tokens` + reserved + `min_output` > `context_window`,
    /// meaning the model may not have enough room to generate a useful response.
    #[must_use]
    pub fn is_over_budget(&self, input_tokens: u32) -> bool {
        input_tokens
            .saturating_add(self.reserved_tokens)
            .saturating_add(self.min_output_tokens)
            > self.context_window
    }

    /// Returns the context window utilization as a percentage (0-100).
    #[must_use]
    pub fn utilization_percent(&self, input_tokens: u32) -> u32 {
        if self.context_window == 0 {
            return 0;
        }
        #[allow(clippy::cast_possible_truncation)]
        {
            (u64::from(input_tokens) * 100 / u64::from(self.context_window)) as u32
        }
    }

    /// Remaining capacity in tokens (`context_window` - input - reserved).
    #[must_use]
    pub fn remaining_capacity(&self, input_tokens: u32) -> u32 {
        self.context_window
            .saturating_sub(input_tokens)
            .saturating_sub(self.reserved_tokens)
    }
}

/// Convenience: calculate `max_tokens` from capabilities and current input usage.
#[must_use]
pub fn auto_max_tokens(caps: &ModelCapabilities, input_tokens: u32) -> u32 {
    TokenBudget::from_capabilities(caps).calculate_max_tokens(input_tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_budget() -> TokenBudget {
        TokenBudget::new(200_000, 16_000)
    }

    #[test]
    fn basic_budget_calculation() {
        let budget = make_budget();
        // 200k context, 10k input, 1k reserved → 189k available, capped at 16k
        let max = budget.calculate_max_tokens(10_000);
        assert_eq!(max, 16_000);
    }

    #[test]
    fn budget_limited_by_available_space() {
        let budget = TokenBudget::new(10_000, 16_000);
        // 10k context, 8k input, 1k reserved → 1k available
        let max = budget.calculate_max_tokens(8_000);
        assert_eq!(max, 1_000);
    }

    #[test]
    fn budget_floor_at_min_output() {
        let budget = TokenBudget::new(10_000, 16_000);
        // 10k context, 9.9k input, 1k reserved → 0 available → floor at 256
        let max = budget.calculate_max_tokens(9_900);
        assert_eq!(max, 256);
    }

    #[test]
    fn budget_exceeds_context_window() {
        let budget = make_budget();
        // Input exceeds context window entirely
        let max = budget.calculate_max_tokens(250_000);
        assert_eq!(max, 256); // min_output_tokens floor
    }

    #[test]
    fn is_over_budget_false_normal() {
        let budget = make_budget();
        assert!(!budget.is_over_budget(10_000));
    }

    #[test]
    fn is_over_budget_true_near_limit() {
        let budget = TokenBudget::new(10_000, 4_096);
        // 9_800 + 1000 reserved + 256 min = 11_056 > 10_000
        assert!(budget.is_over_budget(9_800));
    }

    #[test]
    fn utilization_percent_empty() {
        let budget = make_budget();
        assert_eq!(budget.utilization_percent(0), 0);
    }

    #[test]
    fn utilization_percent_half() {
        let budget = make_budget();
        assert_eq!(budget.utilization_percent(100_000), 50);
    }

    #[test]
    fn utilization_percent_full() {
        let budget = make_budget();
        assert_eq!(budget.utilization_percent(200_000), 100);
    }

    #[test]
    fn utilization_percent_zero_window() {
        let budget = TokenBudget::new(0, 4_096);
        assert_eq!(budget.utilization_percent(1_000), 0);
    }

    #[test]
    fn remaining_capacity_normal() {
        let budget = make_budget();
        // 200k - 50k - 1k reserved = 149k
        assert_eq!(budget.remaining_capacity(50_000), 149_000);
    }

    #[test]
    fn remaining_capacity_saturates() {
        let budget = make_budget();
        assert_eq!(budget.remaining_capacity(250_000), 0);
    }

    #[test]
    fn with_reserved_modifies() {
        let budget = TokenBudget::new(10_000, 4_096).with_reserved(2_000);
        // 10k - 5k - 2k reserved = 3k
        assert_eq!(budget.calculate_max_tokens(5_000), 3_000);
    }

    #[test]
    fn with_min_output_modifies() {
        let budget = TokenBudget::new(10_000, 4_096).with_min_output(512);
        // 10k - 9.5k - 1k reserved = 0 → floor at 512
        let max = budget.calculate_max_tokens(9_500);
        assert_eq!(max, 512);
    }

    #[test]
    fn from_capabilities() {
        let caps = ModelCapabilities::anthropic("claude-sonnet-4-20250514");
        let budget = TokenBudget::from_capabilities(&caps);
        let max = budget.calculate_max_tokens(10_000);
        assert_eq!(max, 16_000); // capped at sonnet's max_output_tokens
    }

    #[test]
    fn auto_max_tokens_convenience() {
        let caps = ModelCapabilities::anthropic("claude-opus-4-20250514");
        let max = auto_max_tokens(&caps, 10_000);
        assert_eq!(max, 32_000); // opus max_output_tokens
    }

    #[test]
    fn auto_max_tokens_small_window() {
        let caps = ModelCapabilities::unknown("tiny-model");
        // unknown: context_window=128k, max_output=4096
        let max = auto_max_tokens(&caps, 127_000);
        // 128k - 127k - 1k reserved = 0 → floor at 256, capped at 4096
        assert_eq!(max, 256);
    }

    #[test]
    fn budget_exact_boundary() {
        // Exactly at the point where available == max_output
        let budget = TokenBudget::new(20_000, 4_096).with_reserved(0);
        let max = budget.calculate_max_tokens(15_904);
        assert_eq!(max, 4_096);
    }

    #[test]
    fn budget_one_below_boundary() {
        let budget = TokenBudget::new(20_000, 4_096).with_reserved(0);
        let max = budget.calculate_max_tokens(15_905);
        assert_eq!(max, 4_095);
    }
}
