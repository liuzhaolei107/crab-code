//! Rollback mechanism: undo/redo operations with a bounded operation stack.

// ── Data model ─────────────────────────────────────────────────────────

/// The type of action that was performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    UserMessage,
    AssistantMessage,
    ToolCall,
    ToolResult,
    BranchSwitch,
    CheckpointRestore,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserMessage => write!(f, "user_message"),
            Self::AssistantMessage => write!(f, "assistant_message"),
            Self::ToolCall => write!(f, "tool_call"),
            Self::ToolResult => write!(f, "tool_result"),
            Self::BranchSwitch => write!(f, "branch_switch"),
            Self::CheckpointRestore => write!(f, "checkpoint_restore"),
        }
    }
}

/// A single entry in the undo/redo stack, capturing the state before an action.
#[derive(Debug, Clone)]
pub struct RollbackEntry {
    pub turn_number: usize,
    pub action_type: ActionType,
    /// Serialized state before this action was applied.
    pub before_state: Vec<String>,
}

// ── Undo stack ─────────────────────────────────────────────────────────

/// A bounded stack of rollback entries.
#[derive(Debug, Clone)]
pub struct UndoStack {
    entries: Vec<RollbackEntry>,
    max_depth: usize,
}

impl UndoStack {
    #[must_use]
    pub fn new(max_depth: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_depth,
        }
    }

    /// Push a new entry. Evicts the oldest if at capacity.
    pub fn push(&mut self, entry: RollbackEntry) {
        if self.entries.len() >= self.max_depth {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    /// Pop the most recent entry (for undo).
    pub fn pop(&mut self) -> Option<RollbackEntry> {
        self.entries.pop()
    }

    /// Peek at the most recent entry without removing it.
    #[must_use]
    pub fn peek(&self) -> Option<&RollbackEntry> {
        self.entries.last()
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the stack is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear the stack.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Maximum depth.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(100)
    }
}

// ── Rollback manager ───────────────────────────────────────────────────

/// Manages undo/redo operations using two stacks.
#[derive(Debug, Clone)]
pub struct RollbackManager {
    undo_stack: UndoStack,
    redo_stack: UndoStack,
    /// Current conversation messages (the live state).
    current_messages: Vec<String>,
}

impl RollbackManager {
    #[must_use]
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: UndoStack::new(max_depth),
            redo_stack: UndoStack::new(max_depth),
            current_messages: Vec::new(),
        }
    }

    /// Record an action: save current state to undo stack, then apply the
    /// new messages. Clears the redo stack.
    pub fn record_action(&mut self, action_type: ActionType, new_messages: Vec<String>) {
        let entry = RollbackEntry {
            turn_number: self.current_messages.len(),
            action_type,
            before_state: self.current_messages.clone(),
        };
        self.undo_stack.push(entry);
        self.redo_stack.clear();
        self.current_messages = new_messages;
    }

    /// Undo `steps` actions. Returns the number of steps actually undone.
    pub fn undo(&mut self, steps: u32) -> u32 {
        let mut undone = 0;
        for _ in 0..steps {
            let Some(entry) = self.undo_stack.pop() else {
                break;
            };
            // Save current state to redo stack.
            let redo_entry = RollbackEntry {
                turn_number: self.current_messages.len(),
                action_type: entry.action_type,
                before_state: self.current_messages.clone(),
            };
            self.redo_stack.push(redo_entry);
            // Restore previous state.
            self.current_messages = entry.before_state;
            undone += 1;
        }
        undone
    }

    /// Redo `steps` actions. Returns the number of steps actually redone.
    pub fn redo(&mut self, steps: u32) -> u32 {
        let mut redone = 0;
        for _ in 0..steps {
            let Some(entry) = self.redo_stack.pop() else {
                break;
            };
            let undo_entry = RollbackEntry {
                turn_number: self.current_messages.len(),
                action_type: entry.action_type,
                before_state: self.current_messages.clone(),
            };
            self.undo_stack.push(undo_entry);
            self.current_messages = entry.before_state;
            redone += 1;
        }
        redone
    }

    /// Current conversation messages.
    #[must_use]
    pub fn current_messages(&self) -> &[String] {
        &self.current_messages
    }

    /// Number of undo steps available.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of redo steps available.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    /// Whether undo is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether redo is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new(100)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── UndoStack tests ──

    #[test]
    fn undo_stack_push_pop() {
        let mut stack = UndoStack::new(5);
        stack.push(RollbackEntry {
            turn_number: 0,
            action_type: ActionType::UserMessage,
            before_state: vec!["a".into()],
        });
        assert_eq!(stack.len(), 1);
        let entry = stack.pop().unwrap();
        assert_eq!(entry.turn_number, 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn undo_stack_eviction() {
        let mut stack = UndoStack::new(2);
        for i in 0..3 {
            stack.push(RollbackEntry {
                turn_number: i,
                action_type: ActionType::UserMessage,
                before_state: vec![],
            });
        }
        assert_eq!(stack.len(), 2);
        // Oldest (turn 0) evicted; top should be turn 2
        assert_eq!(stack.peek().unwrap().turn_number, 2);
    }

    #[test]
    fn undo_stack_peek() {
        let mut stack = UndoStack::new(5);
        assert!(stack.peek().is_none());
        stack.push(RollbackEntry {
            turn_number: 1,
            action_type: ActionType::ToolCall,
            before_state: vec![],
        });
        assert_eq!(stack.peek().unwrap().turn_number, 1);
        assert_eq!(stack.len(), 1); // peek doesn't remove
    }

    #[test]
    fn undo_stack_clear() {
        let mut stack = UndoStack::new(5);
        stack.push(RollbackEntry {
            turn_number: 0,
            action_type: ActionType::UserMessage,
            before_state: vec![],
        });
        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn undo_stack_max_depth() {
        let stack = UndoStack::new(42);
        assert_eq!(stack.max_depth(), 42);
    }

    #[test]
    fn undo_stack_default() {
        let stack = UndoStack::default();
        assert_eq!(stack.max_depth(), 100);
    }

    // ── RollbackManager tests ──

    #[test]
    fn record_and_undo() {
        let mut mgr = RollbackManager::new(10);
        mgr.record_action(ActionType::UserMessage, vec!["hello".into()]);
        mgr.record_action(
            ActionType::AssistantMessage,
            vec!["hello".into(), "world".into()],
        );
        assert_eq!(mgr.current_messages().len(), 2);
        assert!(mgr.can_undo());

        let undone = mgr.undo(1);
        assert_eq!(undone, 1);
        assert_eq!(mgr.current_messages(), &["hello"]);
    }

    #[test]
    fn undo_multiple_steps() {
        let mut mgr = RollbackManager::new(10);
        mgr.record_action(ActionType::UserMessage, vec!["a".into()]);
        mgr.record_action(ActionType::AssistantMessage, vec!["a".into(), "b".into()]);
        mgr.record_action(
            ActionType::ToolCall,
            vec!["a".into(), "b".into(), "c".into()],
        );

        let undone = mgr.undo(2);
        assert_eq!(undone, 2);
        assert_eq!(mgr.current_messages(), &["a"]);
    }

    #[test]
    fn undo_more_than_available() {
        let mut mgr = RollbackManager::new(10);
        mgr.record_action(ActionType::UserMessage, vec!["a".into()]);
        let undone = mgr.undo(5);
        assert_eq!(undone, 1);
        assert!(mgr.current_messages().is_empty());
    }

    #[test]
    fn undo_on_empty() {
        let mut mgr = RollbackManager::new(10);
        assert!(!mgr.can_undo());
        assert_eq!(mgr.undo(1), 0);
    }

    #[test]
    fn redo_after_undo() {
        let mut mgr = RollbackManager::new(10);
        mgr.record_action(ActionType::UserMessage, vec!["a".into()]);
        mgr.record_action(ActionType::AssistantMessage, vec!["a".into(), "b".into()]);

        mgr.undo(1);
        assert!(mgr.can_redo());
        let redone = mgr.redo(1);
        assert_eq!(redone, 1);
        assert_eq!(mgr.current_messages(), &["a", "b"]);
    }

    #[test]
    fn redo_on_empty() {
        let mut mgr = RollbackManager::new(10);
        assert!(!mgr.can_redo());
        assert_eq!(mgr.redo(1), 0);
    }

    #[test]
    fn new_action_clears_redo() {
        let mut mgr = RollbackManager::new(10);
        mgr.record_action(ActionType::UserMessage, vec!["a".into()]);
        mgr.record_action(ActionType::AssistantMessage, vec!["a".into(), "b".into()]);
        mgr.undo(1);
        assert!(mgr.can_redo());

        // New action clears redo stack
        mgr.record_action(ActionType::UserMessage, vec!["a".into(), "c".into()]);
        assert!(!mgr.can_redo());
        assert_eq!(mgr.current_messages(), &["a", "c"]);
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut mgr = RollbackManager::new(10);
        mgr.record_action(ActionType::UserMessage, vec!["x".into()]);
        let original = mgr.current_messages().to_vec();
        mgr.undo(1);
        mgr.redo(1);
        assert_eq!(mgr.current_messages(), original.as_slice());
    }

    #[test]
    fn depth_tracking() {
        let mut mgr = RollbackManager::new(10);
        assert_eq!(mgr.undo_depth(), 0);
        assert_eq!(mgr.redo_depth(), 0);

        mgr.record_action(ActionType::UserMessage, vec!["a".into()]);
        assert_eq!(mgr.undo_depth(), 1);

        mgr.undo(1);
        assert_eq!(mgr.undo_depth(), 0);
        assert_eq!(mgr.redo_depth(), 1);
    }

    #[test]
    fn clear_history() {
        let mut mgr = RollbackManager::new(10);
        mgr.record_action(ActionType::UserMessage, vec!["a".into()]);
        mgr.undo(1);
        mgr.clear();
        assert!(!mgr.can_undo());
        assert!(!mgr.can_redo());
    }

    #[test]
    fn default_manager() {
        let mgr = RollbackManager::default();
        assert_eq!(mgr.undo_stack.max_depth(), 100);
    }

    #[test]
    fn action_type_display() {
        assert_eq!(ActionType::UserMessage.to_string(), "user_message");
        assert_eq!(ActionType::ToolCall.to_string(), "tool_call");
        assert_eq!(ActionType::BranchSwitch.to_string(), "branch_switch");
    }

    #[test]
    fn rollback_entry_fields() {
        let entry = RollbackEntry {
            turn_number: 5,
            action_type: ActionType::ToolResult,
            before_state: vec!["state".into()],
        };
        assert_eq!(entry.turn_number, 5);
        assert_eq!(entry.action_type, ActionType::ToolResult);
        assert_eq!(entry.before_state, vec!["state"]);
    }
}
