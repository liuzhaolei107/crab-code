//! `crab-acp` — [Agent Client Protocol](https://agentclientprotocol.com) server glue.
//!
//! ACP lets editors (Zed, Neovim, Helix, …) drive external AI coding
//! agents the way LSP lets them drive language servers. This crate
//! wires the upstream [`agent_client_protocol`] SDK to stdio so that a
//! user's editor can spawn `crab` as an ACP-speaking child process.
//!
//! ## Architecture
//!
//! The wire types and builder come from the upstream SDK
//! (`agent-client-protocol = 0.11`, Zed's official Rust crate,
//! Apache-2.0). This crate provides the stdio transport via
//! [`serve_stdio`] and re-exports the SDK surface that composition
//! roots need, so `cli` / `daemon` don't add the SDK as a direct dep.
//!
//! Composition roots (`crates/cli/`) configure an
//! [`Agent`](agent_client_protocol::Agent) builder with request/notification
//! handlers, then hand it to [`serve_stdio`].

pub mod server;

pub use agent_client_protocol as sdk;
pub use server::{AcpServeError, serve_stdio};
