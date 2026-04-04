use crab_common::Result;
use crab_core::tool::{Tool, ToolContext, ToolOutput};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Task creation tool.
pub struct TaskCreateTool;

impl Tool for TaskCreateTool {
    fn name(&self) -> &'static str {
        "task_create"
    }

    fn description(&self) -> &'static str {
        "Create a new task"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "subject": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["subject", "description"]
        })
    }

    fn execute(
        &self,
        _input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement task creation
            Ok(ToolOutput::error("not implemented"))
        })
    }
}

/// Task listing tool.
pub struct TaskListTool;

impl Tool for TaskListTool {
    fn name(&self) -> &'static str {
        "task_list"
    }

    fn description(&self) -> &'static str {
        "List all tasks"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute(
        &self,
        _input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement task listing
            Ok(ToolOutput::error("not implemented"))
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

/// Task update tool.
pub struct TaskUpdateTool;

impl Tool for TaskUpdateTool {
    fn name(&self) -> &'static str {
        "task_update"
    }

    fn description(&self) -> &'static str {
        "Update an existing task"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string" },
                "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
            },
            "required": ["task_id"]
        })
    }

    fn execute(
        &self,
        _input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement task update
            Ok(ToolOutput::error("not implemented"))
        })
    }
}

/// Task retrieval tool.
pub struct TaskGetTool;

impl Tool for TaskGetTool {
    fn name(&self) -> &'static str {
        "task_get"
    }

    fn description(&self) -> &'static str {
        "Get details of a specific task"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string" }
            },
            "required": ["task_id"]
        })
    }

    fn execute(
        &self,
        _input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement task retrieval
            Ok(ToolOutput::error("not implemented"))
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }
}
