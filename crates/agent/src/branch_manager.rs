//! Conversation branch management: create, switch, merge, and visualize
//! dialogue branches within a session.

use std::collections::HashMap;

// ── Data model ─────────────────────────────────────────────────────────

/// Unique identifier for a branch.
pub type BranchId = u64;

/// A single conversation branch.
#[derive(Debug, Clone)]
pub struct Branch {
    pub id: BranchId,
    pub name: String,
    /// Parent branch (None for the root/main branch).
    pub parent_id: Option<BranchId>,
    /// Turn number at which this branch was forked from the parent.
    pub fork_point: usize,
    /// Messages belonging to this branch (turn-indexed content strings).
    pub messages: Vec<String>,
}

/// Compact summary of a branch for listing.
#[derive(Debug, Clone)]
pub struct BranchSummary {
    pub id: BranchId,
    pub name: String,
    pub parent_id: Option<BranchId>,
    pub fork_point: usize,
    pub message_count: usize,
    pub is_active: bool,
}

/// Tree-format visualization of the branch hierarchy.
#[derive(Debug, Clone)]
pub struct BranchTree {
    pub lines: Vec<String>,
}

impl std::fmt::Display for BranchTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

// ── Branch manager ─────────────────────────────────────────────────────

/// Manages a tree of conversation branches.
#[derive(Debug, Clone)]
pub struct BranchManager {
    branches: HashMap<BranchId, Branch>,
    active_branch: BranchId,
    next_id: BranchId,
}

impl BranchManager {
    /// Create a new manager with a default "main" branch.
    #[must_use]
    pub fn new() -> Self {
        let mut branches = HashMap::new();
        let main = Branch {
            id: 0,
            name: "main".into(),
            parent_id: None,
            fork_point: 0,
            messages: Vec::new(),
        };
        branches.insert(0, main);
        Self {
            branches,
            active_branch: 0,
            next_id: 1,
        }
    }

    /// The currently active branch id.
    #[must_use]
    pub fn active_branch_id(&self) -> BranchId {
        self.active_branch
    }

    /// Get a branch by id.
    #[must_use]
    pub fn get_branch(&self, id: BranchId) -> Option<&Branch> {
        self.branches.get(&id)
    }

    /// Add a message to the active branch.
    pub fn push_message(&mut self, content: String) {
        if let Some(branch) = self.branches.get_mut(&self.active_branch) {
            branch.messages.push(content);
        }
    }

    /// Number of branches.
    #[must_use]
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// Create a new branch forking from the active branch at a given turn.
    ///
    /// Messages up to `fork_from_turn` are copied from the parent. Returns
    /// the new branch id, or `None` if `fork_from_turn` exceeds the parent
    /// message count.
    pub fn create_branch(&mut self, name: &str, fork_from_turn: usize) -> Option<BranchId> {
        let parent = self.branches.get(&self.active_branch)?;
        if fork_from_turn > parent.messages.len() {
            return None;
        }
        let forked_messages = parent.messages[..fork_from_turn].to_vec();
        let id = self.next_id;
        self.next_id += 1;
        let branch = Branch {
            id,
            name: name.to_string(),
            parent_id: Some(self.active_branch),
            fork_point: fork_from_turn,
            messages: forked_messages,
        };
        self.branches.insert(id, branch);
        Some(id)
    }

    /// Switch the active branch. Returns `false` if the branch doesn't exist.
    pub fn switch_branch(&mut self, branch_id: BranchId) -> bool {
        if self.branches.contains_key(&branch_id) {
            self.active_branch = branch_id;
            true
        } else {
            false
        }
    }

    /// Merge `source` into `target`: append messages from source that are
    /// beyond the fork point. Returns `false` if either branch doesn't exist.
    pub fn merge_branch(&mut self, source: BranchId, target: BranchId) -> bool {
        let Some(src) = self.branches.get(&source).cloned() else {
            return false;
        };
        let Some(tgt) = self.branches.get_mut(&target) else {
            return false;
        };
        // Append messages from source that are after the fork point.
        let new_messages: Vec<String> = src.messages.iter().skip(src.fork_point).cloned().collect();
        tgt.messages.extend(new_messages);
        true
    }

    /// List all branches with summaries.
    #[must_use]
    pub fn list_branches(&self) -> Vec<BranchSummary> {
        let mut summaries: Vec<BranchSummary> = self
            .branches
            .values()
            .map(|b| BranchSummary {
                id: b.id,
                name: b.name.clone(),
                parent_id: b.parent_id,
                fork_point: b.fork_point,
                message_count: b.messages.len(),
                is_active: b.id == self.active_branch,
            })
            .collect();
        summaries.sort_by_key(|s| s.id);
        summaries
    }

    /// Build a tree visualization of branches.
    #[must_use]
    pub fn branch_tree(&self) -> BranchTree {
        let mut lines = Vec::new();
        // Start from root branches (no parent).
        let mut roots: Vec<BranchId> = self
            .branches
            .values()
            .filter(|b| b.parent_id.is_none())
            .map(|b| b.id)
            .collect();
        roots.sort_unstable();
        for root in roots {
            self.build_tree_lines(root, "", true, &mut lines);
        }
        BranchTree { lines }
    }

    fn build_tree_lines(&self, id: BranchId, prefix: &str, is_last: bool, lines: &mut Vec<String>) {
        let Some(branch) = self.branches.get(&id) else {
            return;
        };
        let connector = if prefix.is_empty() {
            ""
        } else if is_last {
            "└─ "
        } else {
            "├─ "
        };
        let active_marker = if id == self.active_branch { " *" } else { "" };
        lines.push(format!(
            "{prefix}{connector}{} ({} msgs, fork@{}){active_marker}",
            branch.name,
            branch.messages.len(),
            branch.fork_point
        ));

        // Find children.
        let mut children: Vec<BranchId> = self
            .branches
            .values()
            .filter(|b| b.parent_id == Some(id))
            .map(|b| b.id)
            .collect();
        children.sort_unstable();

        let child_prefix = if prefix.is_empty() {
            String::new()
        } else if is_last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        };

        for (i, child_id) in children.iter().enumerate() {
            let last = i == children.len() - 1;
            self.build_tree_lines(*child_id, &child_prefix, last, lines);
        }
    }

    /// Delete a branch (cannot delete the active branch or the main branch).
    pub fn delete_branch(&mut self, id: BranchId) -> bool {
        if id == 0 || id == self.active_branch {
            return false;
        }
        self.branches.remove(&id).is_some()
    }
}

impl Default for BranchManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_has_main_branch() {
        let mgr = BranchManager::new();
        assert_eq!(mgr.branch_count(), 1);
        assert_eq!(mgr.active_branch_id(), 0);
        let main = mgr.get_branch(0).unwrap();
        assert_eq!(main.name, "main");
        assert!(main.parent_id.is_none());
    }

    #[test]
    fn push_message() {
        let mut mgr = BranchManager::new();
        mgr.push_message("hello".into());
        mgr.push_message("world".into());
        let main = mgr.get_branch(0).unwrap();
        assert_eq!(main.messages.len(), 2);
        assert_eq!(main.messages[1], "world");
    }

    #[test]
    fn create_branch_forks_messages() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        mgr.push_message("m2".into());
        mgr.push_message("m3".into());
        let bid = mgr.create_branch("feature", 2).unwrap();
        assert_eq!(mgr.branch_count(), 2);
        let branch = mgr.get_branch(bid).unwrap();
        assert_eq!(branch.name, "feature");
        assert_eq!(branch.fork_point, 2);
        assert_eq!(branch.messages.len(), 2); // m1, m2
        assert_eq!(branch.parent_id, Some(0));
    }

    #[test]
    fn create_branch_invalid_fork_point() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        assert!(mgr.create_branch("bad", 5).is_none());
    }

    #[test]
    fn create_branch_at_zero() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        let bid = mgr.create_branch("empty", 0).unwrap();
        let branch = mgr.get_branch(bid).unwrap();
        assert!(branch.messages.is_empty());
    }

    #[test]
    fn switch_branch() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        let bid = mgr.create_branch("alt", 1).unwrap();
        assert!(mgr.switch_branch(bid));
        assert_eq!(mgr.active_branch_id(), bid);
        // Push to new branch
        mgr.push_message("alt_msg".into());
        assert_eq!(mgr.get_branch(bid).unwrap().messages.len(), 2); // forked m1 + alt_msg
        // Main unchanged
        assert_eq!(mgr.get_branch(0).unwrap().messages.len(), 1);
    }

    #[test]
    fn switch_branch_invalid() {
        let mut mgr = BranchManager::new();
        assert!(!mgr.switch_branch(999));
        assert_eq!(mgr.active_branch_id(), 0);
    }

    #[test]
    fn merge_branch() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        mgr.push_message("m2".into());
        let bid = mgr.create_branch("feat", 1).unwrap();
        mgr.switch_branch(bid);
        mgr.push_message("feat_msg".into());
        // feat now has: [m1, feat_msg], fork_point=1
        // Merge feat into main: should append "feat_msg" to main
        assert!(mgr.merge_branch(bid, 0));
        let main = mgr.get_branch(0).unwrap();
        assert_eq!(main.messages.len(), 3); // m1, m2, feat_msg
        assert_eq!(main.messages[2], "feat_msg");
    }

    #[test]
    fn merge_branch_invalid() {
        let mgr = BranchManager::new();
        // Can't merge into non-mutable context, but let's test with mut
        let mut mgr = mgr;
        assert!(!mgr.merge_branch(999, 0));
        assert!(!mgr.merge_branch(0, 999));
    }

    #[test]
    fn list_branches() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        mgr.create_branch("feat", 1);
        let list = mgr.list_branches();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "main");
        assert!(list[0].is_active);
        assert_eq!(list[1].name, "feat");
        assert!(!list[1].is_active);
    }

    #[test]
    fn branch_tree_visualization() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        mgr.create_branch("feat-a", 1);
        mgr.create_branch("feat-b", 1);
        let tree = mgr.branch_tree();
        assert!(!tree.lines.is_empty());
        let text = tree.to_string();
        assert!(text.contains("main"));
        assert!(text.contains("feat-a"));
        assert!(text.contains("feat-b"));
    }

    #[test]
    fn delete_branch() {
        let mut mgr = BranchManager::new();
        let bid = mgr.create_branch("temp", 0).unwrap();
        assert!(mgr.delete_branch(bid));
        assert_eq!(mgr.branch_count(), 1);
        assert!(mgr.get_branch(bid).is_none());
    }

    #[test]
    fn cannot_delete_main() {
        let mut mgr = BranchManager::new();
        assert!(!mgr.delete_branch(0));
    }

    #[test]
    fn cannot_delete_active() {
        let mut mgr = BranchManager::new();
        let bid = mgr.create_branch("active", 0).unwrap();
        mgr.switch_branch(bid);
        assert!(!mgr.delete_branch(bid));
    }

    #[test]
    fn delete_nonexistent() {
        let mut mgr = BranchManager::new();
        assert!(!mgr.delete_branch(999));
    }

    #[test]
    fn default_manager() {
        let mgr = BranchManager::default();
        assert_eq!(mgr.branch_count(), 1);
    }

    #[test]
    fn active_marker_in_tree() {
        let mgr = BranchManager::new();
        let tree = mgr.branch_tree();
        let text = tree.to_string();
        assert!(text.contains("*")); // main is active
    }

    #[test]
    fn multiple_levels() {
        let mut mgr = BranchManager::new();
        mgr.push_message("m1".into());
        let b1 = mgr.create_branch("level1", 1).unwrap();
        mgr.switch_branch(b1);
        mgr.push_message("l1m".into());
        let b2 = mgr.create_branch("level2", 2).unwrap();
        assert_eq!(mgr.branch_count(), 3);
        let b2_branch = mgr.get_branch(b2).unwrap();
        assert_eq!(b2_branch.parent_id, Some(b1));
    }
}
