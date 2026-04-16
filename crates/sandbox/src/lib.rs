//! Process sandbox — trait-only core with platform backends behind feature
//! flags. Consumed by `crab-tools` for shell execution (Bash / `PowerShell`).
//!
//! Populated incrementally. Phase 1 only lays out the module tree.

pub mod backend;
pub mod config;
pub mod doctor;
pub mod error;
pub mod policy;
pub mod violation;
