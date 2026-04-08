//! Settings schema validation.
//!
//! Validates settings JSON values against expected types, ranges, and
//! inter-field constraints. Returns all errors found (not just the first),
//! with optional suggestions for common mistakes.

use std::fmt;

/// A single validation error with optional remediation suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Dot-separated field path, e.g. `"mcpServers.myServer.command"`.
    pub field: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional suggestion for how to fix the error.
    pub suggestion: Option<String>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, " (suggestion: {suggestion})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// Known top-level settings fields and their expected types.
const KNOWN_FIELDS: &[(&str, &str)] = &[
    ("apiProvider", "string"),
    ("apiBaseUrl", "string"),
    ("model", "string"),
    ("maxTokens", "number"),
    ("permissionMode", "string"),
    ("allowedTools", "array"),
    ("deniedTools", "array"),
    ("mcpServers", "object"),
    ("hooks", "object"),
    ("featureFlags", "object"),
    ("customInstructions", "string"),
    ("theme", "string"),
    ("gitContext", "object"),
    ("autoMemoryEnabled", "boolean"),
    ("autoMemoryDirectory", "string"),
];

/// Valid permission mode values.
const VALID_PERMISSION_MODES: &[&str] = &[
    "default",
    "acceptEdits",
    "accept-edits",
    "trust-project",
    "dontAsk",
    "dont-ask",
    "bypassPermissions",
    "dangerously",
    "plan",
];

/// Valid API provider values.
const VALID_PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    "ollama",
    "bedrock",
    "vertex",
    "deepseek",
    "custom",
];

/// Validate an entire settings JSON value.
///
/// Returns an empty `Vec` if the settings are valid.
pub fn validate_settings(settings: &serde_json::Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let Some(obj) = settings.as_object() else {
        errors.push(ValidationError {
            field: "<root>".into(),
            message: "settings must be a JSON object".into(),
            suggestion: None,
        });
        return errors;
    };

    // Check for unknown fields
    for key in obj.keys() {
        if !KNOWN_FIELDS.iter().any(|(k, _)| *k == key.as_str()) {
            errors.push(ValidationError {
                field: key.clone(),
                message: format!("unknown settings field '{key}'"),
                suggestion: Some("check spelling or remove this field".into()),
            });
        }
    }

    // Type checks for known fields
    for &(field, expected_type) in KNOWN_FIELDS {
        if let Some(value) = obj.get(field) {
            let type_ok = match expected_type {
                "string" => value.is_string(),
                "number" => value.is_number(),
                "boolean" => value.is_boolean(),
                "object" => value.is_object(),
                "array" => value.is_array(),
                _ => true,
            };
            if !type_ok {
                errors.push(ValidationError {
                    field: field.into(),
                    message: format!("expected {expected_type}, got {}", value_type_name(value)),
                    suggestion: None,
                });
            }
        }
    }

    // Validate maxTokens range
    if let Some(max_tokens) = obj.get("maxTokens").and_then(serde_json::Value::as_u64)
        && (max_tokens == 0 || max_tokens > 1_000_000)
    {
        errors.push(ValidationError {
            field: "maxTokens".into(),
            message: "must be between 1 and 1,000,000".into(),
            suggestion: Some("typical values: 4096, 8192, 16384".into()),
        });
    }

    // Validate permissionMode
    if let Some(mode) = obj.get("permissionMode").and_then(|v| v.as_str())
        && !VALID_PERMISSION_MODES.contains(&mode)
    {
        errors.push(ValidationError {
            field: "permissionMode".into(),
            message: format!("unknown permission mode '{mode}'"),
            suggestion: Some(format!(
                "valid modes: {}",
                VALID_PERMISSION_MODES.join(", ")
            )),
        });
    }

    // Validate apiProvider
    if let Some(provider) = obj.get("apiProvider").and_then(|v| v.as_str())
        && !VALID_PROVIDERS.contains(&provider)
    {
        errors.push(ValidationError {
            field: "apiProvider".into(),
            message: format!("unknown API provider '{provider}'"),
            suggestion: Some(format!("valid providers: {}", VALID_PROVIDERS.join(", "))),
        });
    }

    // Validate mcpServers entries
    if let Some(servers) = obj.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, config) in servers {
            errors.extend(validate_mcp_server_config(name, config));
        }
    }

    // Validate hooks
    if let Some(hooks) = obj.get("hooks").and_then(|v| v.as_object()) {
        for (trigger, entries) in hooks {
            if let Some(arr) = entries.as_array() {
                for entry in arr {
                    errors.extend(validate_hook_entry_inner(trigger, entry));
                }
            }
        }
    }

    errors
}

/// Validate a single permission rule string.
pub fn validate_permission_rule(rule: &str) -> Result<(), ValidationError> {
    let rule = rule.trim();
    if rule.is_empty() {
        return Err(ValidationError {
            field: "permission_rule".into(),
            message: "rule cannot be empty".into(),
            suggestion: None,
        });
    }

    // Check for common syntax errors
    if rule.contains('(') && !rule.contains(')') {
        return Err(ValidationError {
            field: "permission_rule".into(),
            message: format!("unmatched '(' in rule '{rule}'"),
            suggestion: Some("close with ')' e.g. Bash(command:git*)".into()),
        });
    }

    if !rule.contains('(') && rule.contains(')') {
        return Err(ValidationError {
            field: "permission_rule".into(),
            message: format!("unmatched ')' in rule '{rule}'"),
            suggestion: None,
        });
    }

    Ok(())
}

/// Validate an MCP server configuration object.
pub fn validate_mcp_server_config(
    server_name: &str,
    config: &serde_json::Value,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let prefix = format!("mcpServers.{server_name}");

    let Some(obj) = config.as_object() else {
        errors.push(ValidationError {
            field: prefix,
            message: "server config must be an object".into(),
            suggestion: None,
        });
        return errors;
    };

    // Must have either "command" (stdio) or "url" (SSE/WebSocket)
    let has_command = obj.get("command").and_then(|v| v.as_str()).is_some();
    let has_url = obj.get("url").and_then(|v| v.as_str()).is_some();

    if !has_command && !has_url {
        errors.push(ValidationError {
            field: prefix,
            message: "must have either 'command' (stdio) or 'url' (SSE/WebSocket)".into(),
            suggestion: Some("add command: \"npx server\" or url: \"http://...\"".into()),
        });
    }

    errors
}

/// Validate a hook configuration entry.
pub fn validate_hook_entry(entry: &serde_json::Value) -> Vec<ValidationError> {
    validate_hook_entry_inner("hooks", entry)
}

fn validate_hook_entry_inner(trigger: &str, entry: &serde_json::Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let Some(obj) = entry.as_object() else {
        errors.push(ValidationError {
            field: format!("hooks.{trigger}"),
            message: "hook entry must be an object".into(),
            suggestion: None,
        });
        return errors;
    };

    // Must have "command" field
    if let Some(cmd) = obj.get("command") {
        if let Some(s) = cmd.as_str() {
            if s.trim().is_empty() {
                errors.push(ValidationError {
                    field: format!("hooks.{trigger}.command"),
                    message: "command cannot be empty".into(),
                    suggestion: None,
                });
            }
        } else {
            errors.push(ValidationError {
                field: format!("hooks.{trigger}.command"),
                message: "command must be a string".into(),
                suggestion: None,
            });
        }
    }

    errors
}

/// Get a human-readable type name for a JSON value.
fn value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_display() {
        let err = ValidationError {
            field: "maxTokens".into(),
            message: "must be a positive integer".into(),
            suggestion: Some("use a value like 4096".into()),
        };
        let s = err.to_string();
        assert!(s.contains("maxTokens"));
        assert!(s.contains("positive integer"));
        assert!(s.contains("4096"));
    }

    #[test]
    fn validation_error_display_no_suggestion() {
        let err = ValidationError {
            field: "model".into(),
            message: "must be a string".into(),
            suggestion: None,
        };
        let s = err.to_string();
        assert!(s.contains("model"));
        assert!(!s.contains("suggestion"));
    }

    #[test]
    fn validate_valid_settings() {
        let settings = serde_json::json!({
            "model": "claude-3-sonnet",
            "maxTokens": 4096,
            "permissionMode": "default"
        });
        let errors = validate_settings(&settings);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn validate_unknown_field() {
        let settings = serde_json::json!({"unknownField": true});
        let errors = validate_settings(&settings);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unknown"));
    }

    #[test]
    fn validate_wrong_type() {
        let settings = serde_json::json!({"maxTokens": "not a number"});
        let errors = validate_settings(&settings);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("expected number"));
    }

    #[test]
    fn validate_bad_permission_mode() {
        let settings = serde_json::json!({"permissionMode": "invalid"});
        let errors = validate_settings(&settings);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown permission mode"));
    }

    #[test]
    fn validate_mcp_server_no_command_or_url() {
        let errors = validate_mcp_server_config("test", &serde_json::json!({"args": []}));
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("command"));
    }

    #[test]
    fn validate_mcp_server_with_command() {
        let errors =
            validate_mcp_server_config("test", &serde_json::json!({"command": "npx server"}));
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_permission_rule_valid() {
        assert!(validate_permission_rule("Bash(command:git*)").is_ok());
        assert!(validate_permission_rule("*").is_ok());
        assert!(validate_permission_rule("Edit").is_ok());
    }

    #[test]
    fn validate_permission_rule_unmatched_paren() {
        assert!(validate_permission_rule("Bash(command:git*").is_err());
    }

    #[test]
    fn validate_permission_rule_empty() {
        assert!(validate_permission_rule("").is_err());
    }

    #[test]
    fn validate_hook_entry_valid() {
        let entry = serde_json::json!({"command": "echo check"});
        let errors = validate_hook_entry(&entry);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_hook_entry_empty_command() {
        let entry = serde_json::json!({"command": ""});
        let errors = validate_hook_entry(&entry);
        assert!(!errors.is_empty());
    }

    #[test]
    fn validate_non_object_root() {
        let errors = validate_settings(&serde_json::json!("not an object"));
        assert!(!errors.is_empty());
    }
}
