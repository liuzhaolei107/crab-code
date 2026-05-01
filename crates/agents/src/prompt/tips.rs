//! Contextual tips based on tool usage patterns and first-time feature use.
//!
//! The tip system shows non-intrusive hints in the TUI when users encounter
//! new features or common pitfalls. Tips are shown at most once per session
//! and can be permanently dismissed.

use std::collections::HashSet;

// ── Types ─────────────────────────────────────────────────────────────

/// A single contextual tip.
#[derive(Debug, Clone)]
pub struct Tip {
    /// Stable identifier for deduplication and persistence.
    pub id: &'static str,
    /// Human-readable tip message (may contain markdown).
    pub message: String,
    /// Whether this tip has been shown in the current session.
    pub shown: bool,
}

/// Registry that tracks available tips and which have been shown.
///
/// # Example
///
/// ```rust,no_run
/// use crab_agents::prompt::tips::TipRegistry;
///
/// let mut registry = TipRegistry::new();
/// if let Some(tip) = registry.for_context("bash", None) {
///     println!("Tip: {}", tip.message);
///     let id = tip.id;
///     registry.mark_shown(id);
/// }
/// ```
pub struct TipRegistry {
    /// All registered tips.
    tips: Vec<Tip>,
    /// IDs of tips that have been shown (either this session or persisted).
    shown_ids: HashSet<String>,
}

impl TipRegistry {
    /// Create a new registry pre-populated with the built-in tips.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tips: default_tips(),
            shown_ids: HashSet::new(),
        }
    }

    /// Find the most relevant unshown tip for the current context.
    ///
    /// # Arguments
    ///
    /// * `tool_name` — The tool that was just used or is about to be used.
    ///   Use `""` for a generic session-start query.
    /// * `error` — If the tool returned an error, the error message.
    ///
    /// Returns `None` if no un-shown tip matches the context.
    #[must_use]
    pub fn for_context(&self, tool_name: &str, error: Option<&str>) -> Option<&Tip> {
        // Prefer tips whose id references the tool or the error keyword.
        let lower_tool = tool_name.to_ascii_lowercase();
        if !lower_tool.is_empty()
            && let Some(tip) = self
                .tips
                .iter()
                .find(|t| !self.shown_ids.contains(t.id) && t.id.contains(lower_tool.as_str()))
        {
            return Some(tip);
        }
        if let Some(err) = error {
            let lower_err = err.to_ascii_lowercase();
            if let Some(tip) = self
                .tips
                .iter()
                .find(|t| !self.shown_ids.contains(t.id) && lower_err.contains(t.id))
            {
                return Some(tip);
            }
        }
        // Fall back to the first unshown generic tip.
        self.tips
            .iter()
            .find(|tip| !self.shown_ids.contains(tip.id))
    }

    /// Mark a tip as shown so it will not be suggested again.
    pub fn mark_shown(&mut self, id: &str) {
        self.shown_ids.insert(id.to_string());
    }

    /// Number of registered tips.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tips.len()
    }

    /// Whether the registry has no tips.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tips.is_empty()
    }
}

impl Default for TipRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in tips that ship with the registry. Kept terse because the
/// registry picks one at a time and the model repeats the message
/// verbatim to the user.
fn default_tips() -> Vec<Tip> {
    vec![
        Tip {
            id: "bash",
            message: "Use Unix shell syntax even on Windows — the Bash tool runs Git Bash.".into(),
            shown: false,
        },
        Tip {
            id: "memory",
            message:
                "Use `/memory` to browse saved user, feedback, project, and reference memories."
                    .into(),
            shown: false,
        },
        Tip {
            id: "plan",
            message: "For multi-step changes, start with `/plan` to align on an approach \
                      before editing code."
                .into(),
            shown: false,
        },
        Tip {
            id: "permissions",
            message: "Run `crab permissions audit` to review allow/deny decisions or \
                      clean up stale rules."
                .into(),
            shown: false,
        },
        Tip {
            id: "resume",
            message: "Press `Ctrl+R` to open recent sessions, or run `crab session resume <id>`."
                .into(),
            shown: false,
        },
        Tip {
            id: "doctor",
            message: "If something feels off, `crab doctor` prints the most common \
                      configuration checks in one place."
                .into(),
            shown: false,
        },
    ]
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_populates_defaults() {
        let registry = TipRegistry::new();
        assert!(!registry.is_empty());
        assert!(registry.shown_ids.is_empty());
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(TipRegistry::default().len(), TipRegistry::new().len());
    }

    #[test]
    fn for_context_matches_by_tool_name() {
        let registry = TipRegistry::new();
        let tip = registry.for_context("bash", None).unwrap();
        assert_eq!(tip.id, "bash");
    }

    #[test]
    fn for_context_falls_back_to_first_unshown() {
        let registry = TipRegistry::new();
        let tip = registry.for_context("unknown-tool", None).unwrap();
        assert!(!tip.id.is_empty());
    }

    #[test]
    fn mark_shown_hides_tip_from_future_queries() {
        let mut registry = TipRegistry::new();
        let id = registry.for_context("memory", None).unwrap().id;
        assert_eq!(id, "memory");
        registry.mark_shown(id);
        // Second call must return something else (or none) — never the same tip.
        let next = registry.for_context("memory", None);
        assert!(next.is_none_or(|t| t.id != "memory"));
    }

    #[test]
    fn tip_fields() {
        let tip = Tip {
            id: "bash_timeout",
            message: "Use the Sleep tool for long waits".into(),
            shown: false,
        };
        assert_eq!(tip.id, "bash_timeout");
        assert!(!tip.shown);
    }
}
