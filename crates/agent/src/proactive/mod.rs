//! Proactive suggestion engine — forks a lightweight mini-agent to
//! speculate about the user's next step and emits ranked suggestions
//! as `core::Event::ProactiveSuggestion`.
//!
//! Populated incrementally. Phase 4 only lays out the module tree.
//! See spec §6.1 (design) and `core::proactive` (shared types).

pub mod cache;
pub mod mini_agent;
pub mod suggestion;
