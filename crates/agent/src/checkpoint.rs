//! Checkpoint system: create snapshots of conversation state at specific
//! turns for later restoration.

use std::collections::HashMap;

// ── Data model ─────────────────────────────────────────────────────────

/// Unique identifier for a checkpoint.
pub type CheckpointId = u64;

/// A snapshot of conversation state at a point in time.
#[derive(Debug, Clone)]
pub struct ConversationSnapshot {
    /// Messages up to this point.
    pub messages: Vec<String>,
    /// Active branch id at snapshot time.
    pub active_branch: u64,
    /// Any tool state keys to preserve.
    pub tool_state: HashMap<String, String>,
}

/// A named checkpoint with metadata.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub turn_number: usize,
    pub timestamp_ms: u64,
    pub label: String,
    pub snapshot: ConversationSnapshot,
}

/// Compact summary for listing checkpoints.
#[derive(Debug, Clone)]
pub struct CheckpointSummary {
    pub id: CheckpointId,
    pub turn_number: usize,
    pub timestamp_ms: u64,
    pub label: String,
    pub message_count: usize,
}

// ── Checkpoint manager ─────────────────────────────────────────────────

/// Manages conversation checkpoints with auto-save capability.
#[derive(Debug, Clone)]
pub struct CheckpointManager {
    checkpoints: HashMap<CheckpointId, Checkpoint>,
    next_id: CheckpointId,
    /// Create a checkpoint every N turns (0 = disabled).
    pub auto_checkpoint_interval: u32,
    /// Turns since last auto-checkpoint.
    turns_since_checkpoint: u32,
    /// Maximum number of checkpoints to retain.
    pub max_checkpoints: usize,
}

impl CheckpointManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            checkpoints: HashMap::new(),
            next_id: 1,
            auto_checkpoint_interval: 0,
            turns_since_checkpoint: 0,
            max_checkpoints: 50,
        }
    }

    /// Create with auto-checkpoint interval.
    #[must_use]
    pub fn with_auto_interval(interval: u32) -> Self {
        Self {
            auto_checkpoint_interval: interval,
            ..Self::new()
        }
    }

    /// Manually create a checkpoint.
    pub fn create_checkpoint(
        &mut self,
        label: &str,
        turn_number: usize,
        timestamp_ms: u64,
        snapshot: ConversationSnapshot,
    ) -> CheckpointId {
        let id = self.next_id;
        self.next_id += 1;
        let cp = Checkpoint {
            id,
            turn_number,
            timestamp_ms,
            label: label.to_string(),
            snapshot,
        };
        self.checkpoints.insert(id, cp);
        self.turns_since_checkpoint = 0;

        // Evict oldest if over capacity.
        while self.checkpoints.len() > self.max_checkpoints {
            if let Some(oldest_id) = self
                .checkpoints
                .values()
                .min_by_key(|c| c.turn_number)
                .map(|c| c.id)
            {
                self.checkpoints.remove(&oldest_id);
            } else {
                break;
            }
        }

        id
    }

    /// Notify the manager that a turn has completed. Returns `true` if an
    /// auto-checkpoint should be created (caller must supply the snapshot).
    pub fn notify_turn(&mut self) -> bool {
        if self.auto_checkpoint_interval == 0 {
            return false;
        }
        self.turns_since_checkpoint += 1;
        self.turns_since_checkpoint >= self.auto_checkpoint_interval
    }

    /// Reset the auto-checkpoint counter (called after creating a checkpoint).
    pub fn reset_turn_counter(&mut self) {
        self.turns_since_checkpoint = 0;
    }

    /// Retrieve a checkpoint's snapshot for restoration.
    #[must_use]
    pub fn restore_checkpoint(&self, id: CheckpointId) -> Option<&ConversationSnapshot> {
        self.checkpoints.get(&id).map(|cp| &cp.snapshot)
    }

    /// Get a checkpoint by id.
    #[must_use]
    pub fn get_checkpoint(&self, id: CheckpointId) -> Option<&Checkpoint> {
        self.checkpoints.get(&id)
    }

    /// List all checkpoints sorted by turn number.
    #[must_use]
    pub fn list_checkpoints(&self) -> Vec<CheckpointSummary> {
        let mut summaries: Vec<CheckpointSummary> = self
            .checkpoints
            .values()
            .map(|cp| CheckpointSummary {
                id: cp.id,
                turn_number: cp.turn_number,
                timestamp_ms: cp.timestamp_ms,
                label: cp.label.clone(),
                message_count: cp.snapshot.messages.len(),
            })
            .collect();
        summaries.sort_by_key(|s| s.turn_number);
        summaries
    }

    /// Number of stored checkpoints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    /// Whether there are no checkpoints.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }

    /// Delete a checkpoint.
    pub fn delete_checkpoint(&mut self, id: CheckpointId) -> bool {
        self.checkpoints.remove(&id).is_some()
    }

    /// Clear all checkpoints.
    pub fn clear(&mut self) {
        self.checkpoints.clear();
        self.turns_since_checkpoint = 0;
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(msgs: &[&str]) -> ConversationSnapshot {
        ConversationSnapshot {
            messages: msgs.iter().map(|s| s.to_string()).collect(),
            active_branch: 0,
            tool_state: HashMap::new(),
        }
    }

    #[test]
    fn new_manager_empty() {
        let mgr = CheckpointManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn create_checkpoint_basic() {
        let mut mgr = CheckpointManager::new();
        let id = mgr.create_checkpoint("start", 0, 1000, make_snapshot(&["hello"]));
        assert_eq!(mgr.len(), 1);
        let cp = mgr.get_checkpoint(id).unwrap();
        assert_eq!(cp.label, "start");
        assert_eq!(cp.turn_number, 0);
        assert_eq!(cp.timestamp_ms, 1000);
    }

    #[test]
    fn create_multiple_checkpoints() {
        let mut mgr = CheckpointManager::new();
        let id1 = mgr.create_checkpoint("cp1", 0, 100, make_snapshot(&["a"]));
        let id2 = mgr.create_checkpoint("cp2", 5, 200, make_snapshot(&["a", "b"]));
        assert_eq!(mgr.len(), 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn restore_checkpoint() {
        let mut mgr = CheckpointManager::new();
        let id = mgr.create_checkpoint("snap", 3, 300, make_snapshot(&["x", "y", "z"]));
        let snap = mgr.restore_checkpoint(id).unwrap();
        assert_eq!(snap.messages.len(), 3);
        assert_eq!(snap.messages[2], "z");
    }

    #[test]
    fn restore_nonexistent() {
        let mgr = CheckpointManager::new();
        assert!(mgr.restore_checkpoint(999).is_none());
    }

    #[test]
    fn list_checkpoints_sorted() {
        let mut mgr = CheckpointManager::new();
        mgr.create_checkpoint("late", 10, 1000, make_snapshot(&[]));
        mgr.create_checkpoint("early", 2, 200, make_snapshot(&[]));
        mgr.create_checkpoint("mid", 5, 500, make_snapshot(&[]));
        let list = mgr.list_checkpoints();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].label, "early");
        assert_eq!(list[1].label, "mid");
        assert_eq!(list[2].label, "late");
    }

    #[test]
    fn auto_checkpoint_disabled() {
        let mut mgr = CheckpointManager::new();
        assert!(!mgr.notify_turn());
        assert!(!mgr.notify_turn());
    }

    #[test]
    fn auto_checkpoint_triggers() {
        let mut mgr = CheckpointManager::with_auto_interval(3);
        assert!(!mgr.notify_turn()); // 1
        assert!(!mgr.notify_turn()); // 2
        assert!(mgr.notify_turn()); // 3 — trigger
        mgr.reset_turn_counter();
        assert!(!mgr.notify_turn()); // 1 again
    }

    #[test]
    fn max_checkpoints_eviction() {
        let mut mgr = CheckpointManager::new();
        mgr.max_checkpoints = 3;
        for i in 0..5 {
            mgr.create_checkpoint(&format!("cp{i}"), i, i as u64 * 100, make_snapshot(&[]));
        }
        assert_eq!(mgr.len(), 3);
        // Oldest (turn 0, 1) should be evicted
        let list = mgr.list_checkpoints();
        assert!(list[0].turn_number >= 2);
    }

    #[test]
    fn delete_checkpoint() {
        let mut mgr = CheckpointManager::new();
        let id = mgr.create_checkpoint("del", 0, 0, make_snapshot(&[]));
        assert!(mgr.delete_checkpoint(id));
        assert!(mgr.is_empty());
    }

    #[test]
    fn delete_nonexistent() {
        let mut mgr = CheckpointManager::new();
        assert!(!mgr.delete_checkpoint(999));
    }

    #[test]
    fn clear_checkpoints() {
        let mut mgr = CheckpointManager::new();
        mgr.create_checkpoint("a", 0, 0, make_snapshot(&[]));
        mgr.create_checkpoint("b", 1, 100, make_snapshot(&[]));
        mgr.clear();
        assert!(mgr.is_empty());
    }

    #[test]
    fn snapshot_tool_state() {
        let mut snap = make_snapshot(&["msg"]);
        snap.tool_state.insert("cwd".into(), "/tmp".into());
        let mut mgr = CheckpointManager::new();
        let id = mgr.create_checkpoint("with_state", 0, 0, snap);
        let restored = mgr.restore_checkpoint(id).unwrap();
        assert_eq!(restored.tool_state.get("cwd").unwrap(), "/tmp");
    }

    #[test]
    fn default_manager() {
        let mgr = CheckpointManager::default();
        assert!(mgr.is_empty());
        assert_eq!(mgr.auto_checkpoint_interval, 0);
        assert_eq!(mgr.max_checkpoints, 50);
    }

    #[test]
    fn checkpoint_summary_message_count() {
        let mut mgr = CheckpointManager::new();
        mgr.create_checkpoint("cp", 5, 500, make_snapshot(&["a", "b", "c"]));
        let list = mgr.list_checkpoints();
        assert_eq!(list[0].message_count, 3);
    }

    #[test]
    fn with_auto_interval() {
        let mgr = CheckpointManager::with_auto_interval(10);
        assert_eq!(mgr.auto_checkpoint_interval, 10);
    }
}
