use crab_core::model::{CostTracker, TokenUsage};

/// Session-level cost accumulator -- thin wrapper around core `CostTracker`
/// with session-specific helpers.
#[derive(Default)]
pub struct CostAccumulator {
    pub tracker: CostTracker,
}

impl CostAccumulator {
    pub fn record(&mut self, usage: &TokenUsage, cost: f64) {
        self.tracker.record(usage, cost);
    }

    pub fn total_tokens(&self) -> u64 {
        self.tracker.total_tokens()
    }

    pub fn total_cost_usd(&self) -> f64 {
        self.tracker.total_cost_usd
    }
}
