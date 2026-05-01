//! IDE integration for Crab Code.
//!
//! Connects to an IDE plugin's MCP server to receive ambient context
//! (selection, opened file, `@`-mentions) and exposes it via shared state
//! that `crab-tui` reads for display and `crab-agents` reads for prompt
//! injection.
//!
//! ## Data flow
//!
//! ```text
//! [IDE plugin hosts MCP server]
//!                │  selection_changed / at_mentioned notifications
//!                ▼
//!          IdeClient (this crate)
//!                │
//!   ┌────────────┼───────────────────────┐
//!   │            │                       │
//!   ▼            ▼                       ▼
//!  Arc<RwLock<  Arc<RwLock<       broadcast::Sender<
//!    Option<       Option<          IdeAtMention>
//!    IdeSelection>>  IdeConnection>>
//!   └──────┬───────────────┬──────────────┬─────────┘
//!          │               │              │
//!   read by tui+agent  read by tui   consumed by agent
//! ```
//!
//! Data types (`IdeSelection`, `IdeAtMention`, `IdeConnection`) live in
//! `crab-core::ide` so `crab-tui` can read them without depending on
//! this crate — both are Layer 2 services and same-layer deps are
//! forbidden.
//!
//! ## Milestones
//!
//! - **R1 (current)**: Scaffold. Types in core, empty module tree here.
//! - **R2**: `lockfile::discover()` + `IdeClient::try_connect()` via
//!   MCP WS transport + status indicator in TUI.
//! - **R3**: `injection::build_system_reminder()` + agent wires it into
//!   prompt submit path.
//! - **R4**: `@`-mention channel + TUI attach-tag row.

pub mod client;
pub mod detection;
pub mod injection;
pub mod lockfile;
pub mod notifications;
pub mod quirks;
pub mod state;

pub use client::IdeClient;
pub use state::IdeHandles;
