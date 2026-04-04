use std::sync::Arc;

use crab_core::permission::PermissionMode;
use crab_core::tool::{ToolContext, ToolOutput};

use crate::registry::ToolRegistry;

/// Unified tool executor with permission checks.
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
}

impl ToolExecutor {
    #[must_use]
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// Returns a reference to the underlying registry.
    #[must_use]
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Execute a tool by name with permission checks.
    ///
    /// Permission decision matrix (mode x `tool_type` x `path_scope`):
    ///
    /// | PermissionMode | read_only | write(project) | write(outside) | dangerous | mcp_external | denied_list |
    /// |----------------|-----------|---------------|----------------|-----------|-------------|-------------|
    /// | Default        | Allow     | Prompt        | Prompt         | Prompt    | Prompt      | Deny        |
    /// | TrustProject   | Allow     | Allow         | Prompt         | Prompt    | Prompt      | Deny        |
    /// | Dangerously    | Allow     | Allow         | Allow          | Allow     | Allow       | Deny        |
    pub async fn execute(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> crab_common::Result<ToolOutput> {
        let tool = self
            .registry
            .get(tool_name)
            .ok_or_else(|| crab_common::Error::Other(format!("tool not found: {tool_name}")))?;

        // 1. Check denied list — denied in all modes
        if self.is_denied(tool_name, ctx) {
            return Ok(ToolOutput::error(format!(
                "tool '{tool_name}' is denied by policy"
            )));
        }

        // 2. Dangerously mode short-circuit (after denied check)
        if ctx.permission_mode == PermissionMode::Dangerously {
            return tool.execute(input, ctx).await;
        }

        // TODO: implement full permission matrix check
        tool.execute(input, ctx).await
    }

    #[allow(clippy::unused_self)]
    fn is_denied(&self, tool_name: &str, ctx: &ToolContext) -> bool {
        ctx.permission_policy.denied_tools.iter().any(|pattern| {
            globset::Glob::new(pattern)
                .ok()
                .and_then(|g| g.compile_matcher().is_match(tool_name).then_some(()))
                .is_some()
        })
    }
}
