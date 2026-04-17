//! Proactive suggestion engine — forks a lightweight mini-agent to
//! speculate about the user's next step and emits ranked suggestions as
//! `core::Event::ProactiveSuggestion`.
//!
//! Matches CCB's `feature('PROACTIVE')` pattern: the open-source build
//! ships only a placeholder (`src/commands/proactive.js` is empty), with
//! the real implementation shipped through a feature-flag overlay. crab
//! mirrors that posture — the module tree exists so the full engine can
//! drop in later without reshaping re-exports, but the three sibling
//! files below are deliberately stubs today.
//!
//! The `proactive` Cargo feature controls whether this module participates
//! in the default build. It is currently inert (default off) and unused
//! by downstream wiring; flipping it on only compiles the stubs. A
//! follow-up will add a real `MiniAgent` runner, a suggestion cache, and
//! session-level integration.

pub mod cache;
pub mod mini_agent;
pub mod suggestion;
