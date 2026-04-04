use crate::protocol::ServerCapabilities;

/// MCP server — exposes local tools to external MCP clients.
pub struct McpServer {
    _capabilities: ServerCapabilities,
}

impl McpServer {
    /// Create a new MCP server with the given capabilities.
    pub fn new(capabilities: ServerCapabilities) -> Self {
        Self {
            _capabilities: capabilities,
        }
    }

    /// Start serving on the given transport (stdio, SSE, or WebSocket).
    pub async fn serve(&self) -> crab_common::Result<()> {
        todo!()
    }
}
