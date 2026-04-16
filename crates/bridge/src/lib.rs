//! WebSocket bridge server — exposes a live Crab Code session to remote
//! clients (IDE extensions, claude.ai web, peer crab instances).
//!
//! This crate is populated incrementally; Phase 1 only lays out the module
//! tree. See `docs/superpowers/specs/2026-04-17-crate-restructure-design.md`
//! §3 for the full design.

pub mod auth;
pub mod config;
pub mod protocol;
pub mod remote_core;
pub mod server;
pub mod session;
pub mod status;
pub mod transport;
pub mod webhook;

#[cfg(feature = "rest-api")]
pub mod api;
