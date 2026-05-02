//! Proactive suggestion engine — forks a lightweight mini-agent to
//! speculate about the user's next step and emits ranked suggestions as
//! `core::Event::ProactiveSuggestion`.
//!
//! The module tree exists so the full engine can drop in later without
//! reshaping re-exports, but the three sibling files below are
//! deliberately stubs today.
//!
//! The `proactive` Cargo feature controls whether this module participates
//! in the default build. It is currently inert (default off) and unused
//! by downstream wiring; flipping it on only compiles the stubs. A
//! follow-up will add a real `MiniAgent` runner, a suggestion cache, and
//! session-level integration.

pub mod cache;
pub mod mini_agent;
pub mod suggestion;
