//! Denial tracking for permission decisions.
//!
//! Maps to CCB `utils/permissions/denialTracking.ts`.
//!
//! Tracks consecutive and total permission denials so the system can warn the
//! user or the agent when it appears stuck in a denial loop (e.g., repeatedly
//! trying a denied tool).

use std::time::Instant;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single denial event record.
#[derive(Debug, Clone)]
pub struct DenialRecord {
    /// Name of the tool that was denied.
    pub tool_name: String,
    /// When the denial occurred.
    pub timestamp: Instant,
    /// Human-readable reason for the denial.
    pub reason: String,
}

/// Tracks permission denials and detects denial loops.
///
/// Maintains a log of recent denials and counts consecutive denials so the
/// agent loop or UI can surface warnings like "You have been denied 3 times
/// in a row -- consider a different approach."
#[derive(Debug)]
pub struct DenialTracker {
    /// All recorded denials (most recent last).
    denials: Vec<DenialRecord>,
    /// Number of consecutive denials without an intervening allow.
    consecutive_count: u32,
}

impl Default for DenialTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DenialTracker {
    /// Create a new, empty denial tracker.
    pub fn new() -> Self {
        Self {
            denials: Vec::new(),
            consecutive_count: 0,
        }
    }

    /// Record a new denial event.
    pub fn record_denial(&mut self, tool_name: &str, reason: &str) {
        todo!(
            "Record denial for tool '{tool_name}' with reason '{reason}', \
             increment consecutive_count, push DenialRecord"
        )
    }

    /// Return the number of consecutive denials since the last allow/reset.
    pub fn consecutive_denials(&self) -> u32 {
        self.consecutive_count
    }

    /// Check whether the denial count has reached a threshold that warrants
    /// a warning to the user or agent.
    ///
    /// Default threshold: 3 consecutive denials.
    pub fn should_warn(&self) -> bool {
        todo!("Return true if consecutive_count >= warning threshold (3)")
    }

    /// Reset the consecutive denial counter (called after a successful allow).
    pub fn reset(&mut self) {
        todo!("Reset consecutive_count to 0")
    }

    /// Return a slice of all recorded denial records.
    pub fn history(&self) -> &[DenialRecord] {
        &self.denials
    }

    /// Return the total number of denials ever recorded.
    pub fn total_denials(&self) -> usize {
        self.denials.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_has_zero_denials() {
        let tracker = DenialTracker::new();
        assert_eq!(tracker.consecutive_denials(), 0);
        assert_eq!(tracker.total_denials(), 0);
        assert!(tracker.history().is_empty());
    }

    // Additional tests to be added with implementation:
    // - record_denial increments consecutive_count
    // - should_warn returns false below threshold
    // - should_warn returns true at/above threshold
    // - reset clears consecutive_count but preserves history
}
