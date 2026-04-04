use crate::protocol::McpToolDef;
use crate::transport::Transport;

/// MCP client — connects to an external MCP server, discovers tools/resources,
/// and forwards tool calls.
pub struct McpClient {
    transport: Box<dyn Transport>,
    server_name: String,
    tools: Vec<McpToolDef>,
}

impl McpClient {
    /// Connect to an MCP server: perform handshake and discover capabilities.
    pub async fn connect(
        transport: Box<dyn Transport>,
        server_name: &str,
    ) -> crab_common::Result<Self> {
        // 1. Send initialize request
        // 2. Receive server capabilities
        // 3. Fetch tools/list
        let _ = (&transport, server_name);
        todo!()
    }

    /// Call a tool on the connected MCP server.
    pub async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> crab_common::Result<serde_json::Value> {
        let _ = (name, input);
        todo!()
    }

    /// Read a resource from the connected MCP server.
    pub async fn read_resource(&self, uri: &str) -> crab_common::Result<String> {
        let _ = uri;
        todo!()
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Get the list of tools discovered from this server.
    pub fn tools(&self) -> &[McpToolDef] {
        &self.tools
    }

    /// Get a reference to the underlying transport.
    pub fn transport(&self) -> &dyn Transport {
        &*self.transport
    }
}
