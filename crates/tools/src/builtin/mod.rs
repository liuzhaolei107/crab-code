pub mod agent;
pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod mcp_tool;
pub mod notebook;
pub mod read;
pub mod task;
pub mod web_fetch;
pub mod web_search;
pub mod write;

use crate::registry::ToolRegistry;

/// Register all built-in tools with the given registry.
pub fn register_all_builtins(_registry: &mut ToolRegistry) {
    // TODO: register each tool
}
