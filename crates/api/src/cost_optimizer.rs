//! Cost optimization for model selection.
//!
//! `CostOptimizer` selects the cheapest model that meets quality requirements
//! for a given task complexity. `BudgetTracker` monitors cumulative spending.

use std::fmt;

use crate::usage_stats::cost_estimate;

// ---------------------------------------------------------------------------
// TaskComplexity
// ---------------------------------------------------------------------------

/// How complex a task is — drives model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskComplexity {
    /// Simple tasks (summarization, formatting, Q&A).
    Simple,
    /// Moderate tasks (code review, explanation, search).
    Moderate,
    /// Complex tasks (multi-step reasoning, large refactors).
    Complex,
    /// Critical tasks (security-sensitive, production deployments).
    Critical,
}

impl fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple => write!(f, "simple"),
            Self::Moderate => write!(f, "moderate"),
            Self::Complex => write!(f, "complex"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ---------------------------------------------------------------------------
// ModelSelection
// ---------------------------------------------------------------------------

/// Result of model selection by the cost optimizer.
#[derive(Debug, Clone)]
pub struct ModelSelection {
    /// Selected model ID.
    pub model_id: String,
    /// Why this model was selected.
    pub reason: String,
    /// Estimated cost per 1K input + 1K output tokens (USD).
    pub estimated_cost_per_1k: Option<f64>,
}

// ---------------------------------------------------------------------------
// ModelTier
// ---------------------------------------------------------------------------

/// A model tier with associated metadata.
#[derive(Debug, Clone)]
struct ModelTier {
    model_id: String,
    min_complexity: TaskComplexity,
}

// ---------------------------------------------------------------------------
// CostOptimizer
// ---------------------------------------------------------------------------

/// Selects the cheapest model that satisfies quality requirements.
#[derive(Debug)]
pub struct CostOptimizer {
    /// Models ordered from cheapest to most expensive.
    tiers: Vec<ModelTier>,
}

impl Default for CostOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl CostOptimizer {
    /// Create with default tiers (haiku < sonnet < opus).
    #[must_use]
    pub fn new() -> Self {
        Self {
            tiers: vec![
                ModelTier {
                    model_id: "claude-haiku-3-5".into(),
                    min_complexity: TaskComplexity::Simple,
                },
                ModelTier {
                    model_id: "claude-sonnet-4-20250514".into(),
                    min_complexity: TaskComplexity::Moderate,
                },
                ModelTier {
                    model_id: "claude-opus-4-20250514".into(),
                    min_complexity: TaskComplexity::Complex,
                },
            ],
        }
    }

    /// Create with custom model tiers (ordered cheapest to most expensive).
    ///
    /// Each entry is `(model_id, min_complexity)`.
    #[must_use]
    pub fn with_tiers(tiers: Vec<(impl Into<String>, TaskComplexity)>) -> Self {
        Self {
            tiers: tiers
                .into_iter()
                .map(|(id, complexity)| ModelTier {
                    model_id: id.into(),
                    min_complexity: complexity,
                })
                .collect(),
        }
    }

    /// Select the cheapest model that can handle the given complexity.
    #[must_use]
    pub fn select_model(&self, complexity: TaskComplexity) -> ModelSelection {
        // Find the cheapest model whose min_complexity covers the task.
        // Walk from cheapest to most expensive; pick the first one that
        // can handle this complexity level or higher.
        //
        // A model with min_complexity=Simple can handle Simple tasks.
        // A model with min_complexity=Moderate can handle Simple and Moderate.
        // We want: the cheapest model where complexity >= min_complexity.
        //
        // Actually, the tiers represent "this model is appropriate starting
        // from this complexity level". So for a Complex task, we want a model
        // whose min_complexity <= Complex. Walk from expensive to cheap and
        // pick the cheapest that qualifies... or more simply:
        // find the tier that best matches.

        // Strategy: for the requested complexity, find the tier where
        // min_complexity is closest to (but not exceeding) the requested level.
        // This gives the cheapest adequate model.

        let mut best: Option<&ModelTier> = None;
        for tier in &self.tiers {
            if tier.min_complexity <= complexity {
                // This tier can handle the complexity. Among all that can,
                // pick the one with the highest min_complexity (most specific/cheapest
                // that still qualifies).
                if best.is_none_or(|b| tier.min_complexity > b.min_complexity) {
                    best = Some(tier);
                }
            }
        }

        // If nothing matches (e.g., complexity is Critical but no tier covers it),
        // fall back to the most expensive tier.
        let tier = best.or_else(|| self.tiers.last());

        tier.map_or_else(
            || ModelSelection {
                model_id: String::new(),
                reason: "no models configured".into(),
                estimated_cost_per_1k: None,
            },
            |t| {
                let cost_per_1k = cost_estimate(&t.model_id, 1000, 1000);
                ModelSelection {
                    model_id: t.model_id.clone(),
                    reason: format!(
                        "{} complexity -> {} (tier: {})",
                        complexity, t.model_id, t.min_complexity
                    ),
                    estimated_cost_per_1k: cost_per_1k,
                }
            },
        )
    }

    /// Select model with budget constraint — may downgrade if over budget.
    #[must_use]
    pub fn select_within_budget(
        &self,
        complexity: TaskComplexity,
        budget: &BudgetTracker,
    ) -> ModelSelection {
        if budget.is_over_budget() {
            // Over budget — force cheapest model regardless of complexity.
            if let Some(cheapest) = self.tiers.first() {
                return ModelSelection {
                    model_id: cheapest.model_id.clone(),
                    reason: format!(
                        "budget exceeded (${:.4}/{:.4}) — downgraded to {}",
                        budget.total_spent, budget.budget_limit, cheapest.model_id
                    ),
                    estimated_cost_per_1k: cost_estimate(&cheapest.model_id, 1000, 1000),
                };
            }
        }

        // Near budget (>80%) — downgrade one tier if possible.
        if budget.utilization() > 0.8 && complexity > TaskComplexity::Simple {
            let downgraded = match complexity {
                TaskComplexity::Critical => TaskComplexity::Complex,
                TaskComplexity::Complex => TaskComplexity::Moderate,
                TaskComplexity::Moderate | TaskComplexity::Simple => TaskComplexity::Simple,
            };
            let mut selection = self.select_model(downgraded);
            selection.reason = format!(
                "budget near limit ({:.0}%) — downgraded from {} to {}",
                budget.utilization() * 100.0,
                complexity,
                downgraded
            );
            return selection;
        }

        self.select_model(complexity)
    }
}

// ---------------------------------------------------------------------------
// BudgetTracker
// ---------------------------------------------------------------------------

/// Tracks cumulative spending against a budget limit.
#[derive(Debug, Clone)]
pub struct BudgetTracker {
    /// Budget limit in USD.
    pub budget_limit: f64,
    /// Total spent so far in USD.
    pub total_spent: f64,
    /// Number of requests made.
    pub request_count: u32,
}

impl BudgetTracker {
    /// Create a tracker with the given budget limit (USD).
    #[must_use]
    pub fn new(budget_limit: f64) -> Self {
        Self {
            budget_limit,
            total_spent: 0.0,
            request_count: 0,
        }
    }

    /// Record spending for a request.
    pub fn record_spend(&mut self, amount: f64) {
        self.total_spent += amount;
        self.request_count += 1;
    }

    /// Remaining budget.
    #[must_use]
    pub fn remaining(&self) -> f64 {
        (self.budget_limit - self.total_spent).max(0.0)
    }

    /// Budget utilization (0.0–1.0+).
    #[must_use]
    pub fn utilization(&self) -> f64 {
        if self.budget_limit <= 0.0 {
            return 0.0;
        }
        self.total_spent / self.budget_limit
    }

    /// Whether the budget has been exceeded.
    #[must_use]
    pub fn is_over_budget(&self) -> bool {
        self.total_spent >= self.budget_limit
    }

    /// Reset tracker.
    pub fn reset(&mut self) {
        self.total_spent = 0.0;
        self.request_count = 0;
    }
}

impl fmt::Display for BudgetTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Budget: ${:.4} / ${:.4} ({:.0}% used, {} requests)",
            self.total_spent,
            self.budget_limit,
            self.utilization() * 100.0,
            self.request_count
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complexity_display() {
        assert_eq!(TaskComplexity::Simple.to_string(), "simple");
        assert_eq!(TaskComplexity::Critical.to_string(), "critical");
    }

    #[test]
    fn complexity_ordering() {
        assert!(TaskComplexity::Simple < TaskComplexity::Moderate);
        assert!(TaskComplexity::Moderate < TaskComplexity::Complex);
        assert!(TaskComplexity::Complex < TaskComplexity::Critical);
    }

    #[test]
    fn select_simple_task() {
        let optimizer = CostOptimizer::new();
        let selection = optimizer.select_model(TaskComplexity::Simple);
        assert_eq!(selection.model_id, "claude-haiku-3-5");
        assert!(selection.estimated_cost_per_1k.is_some());
    }

    #[test]
    fn select_moderate_task() {
        let optimizer = CostOptimizer::new();
        let selection = optimizer.select_model(TaskComplexity::Moderate);
        assert_eq!(selection.model_id, "claude-sonnet-4-20250514");
    }

    #[test]
    fn select_complex_task() {
        let optimizer = CostOptimizer::new();
        let selection = optimizer.select_model(TaskComplexity::Complex);
        assert_eq!(selection.model_id, "claude-opus-4-20250514");
    }

    #[test]
    fn select_critical_falls_to_best() {
        let optimizer = CostOptimizer::new();
        let selection = optimizer.select_model(TaskComplexity::Critical);
        // Critical > Complex, so opus (the highest tier that qualifies) is selected
        assert_eq!(selection.model_id, "claude-opus-4-20250514");
    }

    #[test]
    fn custom_tiers() {
        let optimizer = CostOptimizer::with_tiers(vec![
            ("gpt-3.5-turbo", TaskComplexity::Simple),
            ("gpt-4o", TaskComplexity::Complex),
        ]);
        let selection = optimizer.select_model(TaskComplexity::Simple);
        assert_eq!(selection.model_id, "gpt-3.5-turbo");

        let selection = optimizer.select_model(TaskComplexity::Complex);
        assert_eq!(selection.model_id, "gpt-4o");
    }

    #[test]
    fn empty_tiers() {
        let tiers: Vec<(String, TaskComplexity)> = vec![];
        let optimizer = CostOptimizer::with_tiers(tiers);
        let selection = optimizer.select_model(TaskComplexity::Simple);
        assert!(selection.model_id.is_empty());
        assert!(selection.reason.contains("no models"));
    }

    #[test]
    fn budget_tracker_basic() {
        let mut budget = BudgetTracker::new(1.0);
        assert_eq!(budget.remaining(), 1.0);
        assert!(!budget.is_over_budget());
        assert_eq!(budget.request_count, 0);

        budget.record_spend(0.3);
        budget.record_spend(0.5);
        assert_eq!(budget.request_count, 2);
        assert!((budget.remaining() - 0.2).abs() < 0.001);
        assert!((budget.utilization() - 0.8).abs() < 0.001);
    }

    #[test]
    fn budget_tracker_over_budget() {
        let mut budget = BudgetTracker::new(0.5);
        budget.record_spend(0.6);
        assert!(budget.is_over_budget());
        assert_eq!(budget.remaining(), 0.0);
    }

    #[test]
    fn budget_tracker_display() {
        let mut budget = BudgetTracker::new(1.0);
        budget.record_spend(0.5);
        let s = budget.to_string();
        assert!(s.contains("50%"));
        assert!(s.contains("1 requests"));
    }

    #[test]
    fn budget_tracker_reset() {
        let mut budget = BudgetTracker::new(1.0);
        budget.record_spend(0.5);
        budget.reset();
        assert_eq!(budget.total_spent, 0.0);
        assert_eq!(budget.request_count, 0);
    }

    #[test]
    fn budget_tracker_zero_limit() {
        let budget = BudgetTracker::new(0.0);
        assert_eq!(budget.utilization(), 0.0);
    }

    #[test]
    fn select_within_budget_normal() {
        let optimizer = CostOptimizer::new();
        let budget = BudgetTracker::new(10.0);
        let selection = optimizer.select_within_budget(TaskComplexity::Complex, &budget);
        assert_eq!(selection.model_id, "claude-opus-4-20250514");
    }

    #[test]
    fn select_within_budget_over_limit() {
        let optimizer = CostOptimizer::new();
        let mut budget = BudgetTracker::new(1.0);
        budget.record_spend(1.5);
        let selection = optimizer.select_within_budget(TaskComplexity::Critical, &budget);
        // Should downgrade to cheapest
        assert_eq!(selection.model_id, "claude-haiku-3-5");
        assert!(selection.reason.contains("budget exceeded"));
    }

    #[test]
    fn select_within_budget_near_limit() {
        let optimizer = CostOptimizer::new();
        let mut budget = BudgetTracker::new(1.0);
        budget.record_spend(0.85);
        let selection = optimizer.select_within_budget(TaskComplexity::Complex, &budget);
        // Should downgrade one tier
        assert_eq!(selection.model_id, "claude-sonnet-4-20250514");
        assert!(selection.reason.contains("near limit"));
    }
}
