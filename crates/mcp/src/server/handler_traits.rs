//! `ToolHandler`, `ResourceHandler`, `PromptHandler` — pluggable handler traits
//! that the MCP server delegates to.

use serde_json::Value;

use crate::protocol::{
    McpPrompt, McpResource, McpToolDef, PromptGetResult, ResourceReadResult, ToolCallResult,
};

/// Trait for handling tool calls within the MCP server.
///
/// Implementations bridge from the MCP server to the local tool system.
/// A default implementation is provided for `Vec<Arc<dyn Tool>>`.
pub trait ToolHandler: Send + Sync {
    /// List the available tools as MCP tool definitions.
    fn list_tools(&self) -> Vec<McpToolDef>;

    /// Call a tool by name with the given JSON arguments.
    ///
    /// Returns a `ToolCallResult` suitable for JSON-RPC response.
    fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolCallResult> + Send + '_>>;
}

/// Trait for handling MCP resource requests.
///
/// Implementations expose data (files, database records, API responses, etc.)
/// as MCP resources that clients can list and read.
pub trait ResourceHandler: Send + Sync {
    /// List available resources.
    fn list_resources(&self) -> Vec<McpResource>;

    /// Read a resource by URI.
    ///
    /// Returns the resource contents, or an error message if the URI is unknown
    /// or the resource cannot be read.
    fn read_resource(
        &self,
        uri: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ResourceReadResult, String>> + Send + '_>,
    >;
}

/// Trait for handling MCP prompt requests.
///
/// Implementations expose prompt templates that clients can list and invoke
/// with arguments. Each prompt returns a list of messages suitable for LLM input.
pub trait PromptHandler: Send + Sync {
    /// List available prompts.
    fn list_prompts(&self) -> Vec<McpPrompt>;

    /// Get a prompt by name, substituting the given arguments.
    fn get_prompt(
        &self,
        name: &str,
        arguments: &std::collections::HashMap<String, String>,
    ) -> Result<PromptGetResult, String>;
}
