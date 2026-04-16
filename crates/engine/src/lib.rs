//! Raw query loop for Crab Code.
//!
//! This crate will hold the low-level "messages + backend + tool executor →
//! streaming events" loop extracted from `crab-agent` in Phase 3 of the v2.3
//! restructure. Phase 1 leaves the modules empty so that source migration in
//! Phase 3 has a stable destination.

pub mod effort;
#[path = "loop.rs"]
pub mod query_loop;
pub mod stop_hooks;
pub mod streaming;
pub mod token_budget;
pub mod tool_orchestration;
