//! Memory aging: decay relevance over time, suggest pruning.
//!
//! Memories lose relevance as they age. This module computes an age-based
//! score and identifies memories that should be pruned to keep the memory
//! store lean and useful.

use super::memory_relevance::MemoryEntry;

// ── Scoring ───────────────────────────────────────────────────────────

/// Compute an age-decay score for a memory entry.
///
/// Both timestamps should be ISO 8601 strings. The score is in `[0.0, 1.0]`
/// where 1.0 means the memory was just created and 0.0 means it is
/// maximally old.
///
/// # Arguments
///
/// * `created_at` — ISO 8601 timestamp when the memory was created.
/// * `now` — Current ISO 8601 timestamp.
pub fn age_score(_created_at: &str, _now: &str) -> f64 {
    todo!("age_score: parse timestamps, compute exponential decay")
}

// ── Pruning ───────────────────────────────────────────────────────────

/// Determine whether a memory with the given age score should be pruned.
///
/// Returns `true` when the score falls below the given threshold.
#[must_use]
pub fn should_prune(score: f64, threshold: f64) -> bool {
    score < threshold
}

/// Suggest memory entries that should be pruned based on their age.
///
/// Returns the file paths (as strings) of memories whose age score falls
/// below the internal threshold.
#[must_use]
pub fn suggest_pruning(_memories: &[MemoryEntry]) -> Vec<String> {
    todo!("suggest_pruning: compute age_score for each entry, collect those below threshold")
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_prune_below_threshold() {
        assert!(should_prune(0.1, 0.2));
    }

    #[test]
    fn should_not_prune_above_threshold() {
        assert!(!should_prune(0.5, 0.2));
    }

    #[test]
    fn should_not_prune_at_threshold() {
        assert!(!should_prune(0.2, 0.2));
    }
}
