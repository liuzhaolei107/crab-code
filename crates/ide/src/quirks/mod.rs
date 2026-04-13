//! IDE-specific quirks.
//!
//! The core data flow (MCP notification → `IdeSelection` → TUI/agent)
//! is uniform across IDEs because the protocol is standard MCP. Put
//! **only** IDE-specific small differences here:
//!
//! - Parent-process names to look for (`detection.rs` consults these).
//! - Path conversion (WSL `/mnt/c/...` ↔ Windows `C:\...`).
//! - Environment variable names the IDE sets in its terminal.
//!
//! If you find yourself reaching for these modules to add
//! `selection_changed` handling per IDE, step back — that belongs in
//! `notifications.rs` and is IDE-agnostic.

pub mod jetbrains;
pub mod vscode;
pub mod wsl;
