//! `RemoteClient` — outbound crab-proto client.
//!
//! Paired with [`super::server::RemoteServer`]; speaks the same wire and
//! follows the same lifecycle (initialize → session create/attach →
//! `send_input` / cancel → events). Target is any server that speaks
//! crab-proto — another crab instance, a user-built bot, or a
//! third-party adapter.
//!
//! Architecture: one background task owns the WebSocket. Public API
//! methods push work to it through an `mpsc::Sender<Outbox>` and await
//! the reply on a per-request oneshot. Server-initiated notifications
//! fan out over a `broadcast::Sender<SessionEventParams>` so multiple
//! consumers (TUI, logger, tests) can listen independently.

#[allow(clippy::module_inception)]
mod client;
pub mod config;
pub mod error;

pub use client::RemoteClient;
pub use config::ClientConfig;
pub use error::ClientError;
