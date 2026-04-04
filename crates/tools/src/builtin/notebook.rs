use crab_common::Result;
use crab_core::tool::{Tool, ToolContext, ToolOutput};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Jupyter notebook editing tool.
pub struct NotebookTool;

impl Tool for NotebookTool {
    fn name(&self) -> &'static str {
        "notebook_edit"
    }

    fn description(&self) -> &'static str {
        "Edit a cell in a Jupyter notebook"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "notebook_path": { "type": "string", "description": "Absolute path to the notebook" },
                "cell_number": { "type": "integer", "description": "0-indexed cell number" },
                "new_source": { "type": "string", "description": "New source for the cell" },
                "cell_type": { "type": "string", "enum": ["code", "markdown"] }
            },
            "required": ["notebook_path", "new_source"]
        })
    }

    fn execute(
        &self,
        _input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement notebook editing
            Ok(ToolOutput::error("not implemented"))
        })
    }

    fn requires_confirmation(&self) -> bool {
        true
    }
}
