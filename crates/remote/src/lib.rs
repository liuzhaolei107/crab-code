//! claude.ai outbound client for Crab Code — `RemoteTrigger`, `ScheduleWakeup`,
//! and remote agent sessions. Does not touch local session state.
//!
//! Populated incrementally. Phase 1 only lays out the module tree.

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod permission;

#[cfg(feature = "trigger")]
pub mod trigger;

#[cfg(feature = "session")]
pub mod session;
