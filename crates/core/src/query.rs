//! Query-loop tags shared between `crab-engine`, `crab-agents`, and UI.
//!
//! The actual loop and stop-reason machinery live in `crab-engine`; this
//! module only carries the labels that need to cross crate boundaries.

use serde::{Deserialize, Serialize};

/// Who or what initiated the current query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QuerySource {
    #[default]
    Repl,
    Agent {
        agent_id: String,
    },
    Compact,
    Sdk,
    Print,
    SessionMemory,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_default_is_repl() {
        assert_eq!(QuerySource::default(), QuerySource::Repl);
    }

    #[test]
    fn source_serde_roundtrip() {
        let s = QuerySource::Agent {
            agent_id: "worker-1".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: QuerySource = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
