//! IDE client — wraps an MCP client connection to a plugin-hosted
//! server and maintains the shared state.

#![allow(dead_code)] // R1 scaffolding; MCP connection wired up in R2

use crate::state::IdeHandles;

/// Errors that can occur while connecting / running against an IDE
/// plugin's MCP server.
#[derive(Debug, thiserror::Error)]
pub enum IdeClientError {
    #[error("no IDE endpoint discovered")]
    NoEndpoint,
    #[error("lockfile discovery failed: {0}")]
    Discovery(#[from] crate::lockfile::DiscoverError),
    #[error("MCP transport error: {0}")]
    Transport(String),
    #[error("authentication rejected by plugin")]
    Unauthorized,
}

/// Top-level IDE integration handle.
///
/// Owns the MCP connection and the shared state. Consumers get
/// read-only handles via `handles()`.
pub struct IdeClient {
    handles: IdeHandles,
    // R2: mcp_client: crab_mcp::Client,
    // R2: selection_tick_tx: broadcast::Sender<()>,
    // R4: mention_tx: broadcast::Sender<IdeAtMention>,
}

impl IdeClient {
    /// Attempt to discover and connect to an IDE plugin.
    ///
    /// Returns `Ok(None)` when no IDE is running / no plugin installed.
    /// Returns `Err(_)` only for hard failures (corrupt lockfile,
    /// authentication rejection, etc.) — callers should log and
    /// continue with `IdeHandles::empty()`.
    pub async fn try_connect() -> Result<Option<Self>, IdeClientError> {
        // R2: let endpoints = lockfile::discover()?;
        //     let ep = match endpoints.into_iter().next() {
        //         Some(e) => e,
        //         None => return Ok(None),
        //     };
        //     let mcp = crab_mcp::Client::connect_ws(...).await?;
        //     register notifications::register(&mcp, handles.clone()).await?;
        //     Ok(Some(Self { handles, mcp, ... }))
        Ok(None)
    }

    /// Read-side handles for TUI and agent.
    pub fn handles(&self) -> IdeHandles {
        self.handles.clone()
    }
}
