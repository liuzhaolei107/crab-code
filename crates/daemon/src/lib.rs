//! `crab-daemon` — background daemon for Crab Code.
//!
//! The `crab-daemon` binary runs this library; external consumers (e.g.
//! a management CLI) can depend on the crate to speak the IPC protocol
//! and inspect the session pool directly.

pub mod protocol;
pub mod server;
pub mod session_pool;
