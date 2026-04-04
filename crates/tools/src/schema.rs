/// Convert tool schemas to the format expected by the LLM API `tools` parameter.
#[must_use]
pub fn to_api_tools(tool_schemas: &[serde_json::Value]) -> Vec<serde_json::Value> {
    tool_schemas
        .iter()
        .map(|schema| {
            serde_json::json!({
                "type": "function",
                "function": schema,
            })
        })
        .collect()
}
