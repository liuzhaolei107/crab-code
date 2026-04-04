use std::path::Path;

/// Configuration for an MCP server to connect to.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransportConfig,
}

/// Transport-specific configuration for an MCP server.
#[derive(Debug, Clone)]
pub enum McpTransportConfig {
    Stdio {
        command: String,
        args: Vec<String>,
    },
    Sse {
        url: String,
    },
    #[cfg(feature = "ws")]
    WebSocket {
        url: String,
    },
}

/// Discover MCP servers from settings files.
///
/// Reads `~/.crab/settings.json` and project-level settings
/// to build the list of MCP servers to connect to.
pub fn discover_servers(_settings_path: &Path) -> crab_common::Result<Vec<McpServerConfig>> {
    // Parse mcp_servers from settings.json
    todo!()
}
