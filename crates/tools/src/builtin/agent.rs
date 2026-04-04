use crab_common::Result;
use crab_core::tool::{Tool, ToolContext, ToolOutput, ToolSource};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Sub-agent spawning tool.
pub struct AgentTool;

impl Tool for AgentTool {
    fn name(&self) -> &'static str {
        "agent"
    }

    fn description(&self) -> &'static str {
        "Spawn a sub-agent to perform a task"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "The task for the sub-agent" }
            },
            "required": ["prompt"]
        })
    }

    fn execute(
        &self,
        _input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement sub-agent spawning
            Ok(ToolOutput::error("not implemented"))
        })
    }

    fn source(&self) -> ToolSource {
        ToolSource::AgentSpawn
    }

    fn requires_confirmation(&self) -> bool {
        true
    }
}
