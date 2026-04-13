//! Shared-state container for IDE data.
//!
//! Holds `Arc<RwLock<_>>` handles that both the owning `IdeClient`
//! (writer) and downstream consumers (TUI status bar, agent prompt
//! injection) share. Handles are cheap to clone.

use crab_core::ide::{IdeConnection, IdeSelection};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Read-side handles for consumers.
///
/// Cloned out of `IdeClient::handles()` and passed to `crab-tui` and
/// `crab-agent` at startup. When no IDE is connected, the outer
/// `Option` stays `None` on all clones.
#[derive(Clone, Default)]
pub struct IdeHandles {
    pub selection: Arc<RwLock<Option<IdeSelection>>>,
    pub connection: Arc<RwLock<Option<IdeConnection>>>,
}

impl IdeHandles {
    /// Empty handles — useful when IDE integration is disabled so
    /// downstream code can always assume non-None handles.
    pub fn empty() -> Self {
        Self::default()
    }
}
