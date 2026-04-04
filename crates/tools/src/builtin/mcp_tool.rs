use crab_common::Result;
use crab_core::tool::{Tool, ToolContext, ToolOutput, ToolSource};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Adapter that bridges an MCP tool to the native `Tool` trait.
pub struct McpToolAdapter {
    tool_name: String,
    tool_description: String,
    server_name: String,
    schema: Value,
}

impl McpToolAdapter {
    #[must_use]
    pub fn new(name: String, description: String, server_name: String, schema: Value) -> Self {
        Self {
            tool_name: name,
            tool_description: description,
            server_name,
            schema,
        }
    }
}

impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn input_schema(&self) -> Value {
        self.schema.clone()
    }

    fn execute(
        &self,
        _input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        Box::pin(async move {
            // TODO: forward call to MCP client via crab-mcp
            Ok(ToolOutput::error("not implemented"))
        })
    }

    fn source(&self) -> ToolSource {
        ToolSource::McpExternal {
            server_name: self.server_name.clone(),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }
}
