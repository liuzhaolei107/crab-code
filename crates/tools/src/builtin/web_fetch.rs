//! Web page fetching tool — fetches a URL and extracts text content.
//!
//! Currently returns a stub response. Real HTTP fetching with HTML-to-text
//! conversion is deferred to Phase 2 (requires adding `reqwest` dependency).

use crab_common::Result;
use crab_core::tool::{Tool, ToolContext, ToolOutput};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Default timeout in seconds for HTTP requests.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum response body size in bytes (5 MB).
const MAX_BODY_SIZE: u64 = 5 * 1024 * 1024;

/// Web page fetching tool.
pub const WEB_FETCH_TOOL_NAME: &str = "WebFetch";

pub struct WebFetchTool;

impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        WEB_FETCH_TOOL_NAME
    }

    fn description(&self) -> &'static str {
        "Fetch content from a URL, strip HTML to plain text, and return the \
         extracted content. Use a prompt to describe what information to extract \
         from the page. Includes timeout and size limits for safety."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "format": "uri",
                    "description": "The URL to fetch content from (must be a valid HTTP/HTTPS URL)"
                },
                "prompt": {
                    "type": "string",
                    "description": "Describe what information to extract from the page"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Request timeout in seconds (default: 30, max: 120)"
                },
                "max_size_bytes": {
                    "type": "integer",
                    "description": "Maximum response body size in bytes (default: 5242880 = 5MB)"
                }
            },
            "required": ["url", "prompt"]
        })
    }

    fn execute(
        &self,
        input: Value,
        _ctx: &ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
        let url = input["url"].as_str().unwrap_or("").to_owned();
        let prompt = input["prompt"].as_str().unwrap_or("").to_owned();
        let timeout_secs = input["timeout_secs"]
            .as_u64()
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(120);
        let max_size = input["max_size_bytes"]
            .as_u64()
            .unwrap_or(MAX_BODY_SIZE)
            .min(MAX_BODY_SIZE);

        Box::pin(async move {
            // Validate inputs
            if url.is_empty() {
                return Ok(ToolOutput::error("url is required and must be non-empty"));
            }
            if prompt.is_empty() {
                return Ok(ToolOutput::error(
                    "prompt is required — describe what to extract from the page",
                ));
            }
            if let Err(reason) = validate_url(&url) {
                return Ok(ToolOutput::error(reason));
            }

            let content = fetch_url(&url, timeout_secs, max_size).await?;

            // Strip HTML tags if the response looks like HTML
            let text = if content.contains("<html") || content.contains("<!DOCTYPE") {
                strip_html_tags(&content)
            } else {
                content
            };

            // Truncate to prevent context overflow (~100k chars)
            let truncated = if text.len() > 100_000 {
                format!(
                    "{}...\n\n[truncated — full page was {} chars]",
                    &text[..100_000],
                    text.len()
                )
            } else {
                text
            };

            Ok(ToolOutput::success(format!(
                "# Web Fetch: {url}\n\n**Prompt:** {prompt}\n\n---\n\n{truncated}"
            )))
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

/// Validate that the URL is a reasonable HTTP/HTTPS URL.
fn validate_url(url: &str) -> std::result::Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!(
            "URL must start with http:// or https://, got: {url}"
        ));
    }
    // Basic check for a host component
    let after_scheme = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else {
        return Err("invalid URL scheme".to_string());
    };
    if after_scheme.is_empty() || after_scheme.starts_with('/') {
        return Err("URL must include a hostname".to_string());
    }
    Ok(())
}

/// Fetch a URL using curl subprocess.
async fn fetch_url(url: &str, timeout_secs: u64, max_size: u64) -> crab_common::Result<String> {
    let cmd = format!(
        "curl -sS -L --max-time {timeout_secs} --max-filesize {max_size} -A 'CrabCode/1.0' '{url}'"
    );
    let mut opts = crab_process::spawn::shell_command(&cmd);
    opts.timeout = Some(std::time::Duration::from_secs(timeout_secs + 5));

    let output = crab_process::spawn::run(opts).await?;
    if output.exit_code != 0 {
        return Err(crab_common::Error::Other(format!(
            "curl failed (exit {}): {}",
            output.exit_code,
            output.stderr.trim()
        )));
    }
    Ok(output.stdout)
}

/// Strip HTML tags to extract plain text content.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if !in_tag && chars[i] == '<' {
            // Check for script/style start
            let remaining: String = lower_chars[i..].iter().take(10).collect();
            if remaining.starts_with("<script") {
                in_script = true;
            } else if remaining.starts_with("<style") {
                in_style = true;
            }
            in_tag = true;
        } else if in_tag && chars[i] == '>' {
            let remaining: String = lower_chars[i.saturating_sub(8)..=i].iter().collect();
            if remaining.contains("</script>") {
                in_script = false;
            } else if remaining.contains("</style>") {
                in_style = false;
            }
            in_tag = false;
        } else if !in_tag && !in_script && !in_style {
            result.push(chars[i]);
        }
        i += 1;
    }

    // Clean up excessive whitespace
    let mut cleaned = String::with_capacity(result.len());
    let mut prev_newline = false;
    for line in result.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_newline {
                cleaned.push('\n');
                prev_newline = true;
            }
        } else {
            cleaned.push_str(trimmed);
            cleaned.push('\n');
            prev_newline = false;
        }
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::tool::ToolContext;

    fn test_ctx() -> ToolContext {
        ToolContext {
            working_dir: std::path::PathBuf::from("/tmp"),
            permission_mode: crab_core::permission::PermissionMode::Default,
            session_id: "test".into(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            permission_policy: crab_core::permission::PermissionPolicy::default(),
        }
    }

    #[test]
    fn tool_metadata() {
        let tool = WebFetchTool;
        assert_eq!(tool.name(), "WebFetch");
        assert!(tool.is_read_only());
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn input_schema_has_required_fields() {
        let schema = WebFetchTool.input_schema();
        let required = schema["required"].as_array().unwrap();
        let required_strs: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(required_strs.contains(&"url"));
        assert!(required_strs.contains(&"prompt"));
    }

    #[test]
    fn input_schema_has_optional_fields() {
        let schema = WebFetchTool.input_schema();
        let props = schema["properties"].as_object().unwrap();
        assert!(props.contains_key("timeout_secs"));
        assert!(props.contains_key("max_size_bytes"));
    }

    #[tokio::test]
    async fn execute_empty_url_returns_error() {
        let tool = WebFetchTool;
        let ctx = test_ctx();
        let result = tool
            .execute(serde_json::json!({"url": "", "prompt": "extract"}), &ctx)
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.text().contains("url is required"));
    }

    #[tokio::test]
    async fn execute_empty_prompt_returns_error() {
        let tool = WebFetchTool;
        let ctx = test_ctx();
        let result = tool
            .execute(
                serde_json::json!({"url": "https://example.com", "prompt": ""}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.text().contains("prompt is required"));
    }

    #[tokio::test]
    async fn execute_invalid_scheme_returns_error() {
        let tool = WebFetchTool;
        let ctx = test_ctx();
        let result = tool
            .execute(
                serde_json::json!({"url": "ftp://example.com", "prompt": "extract"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.text().contains("http://"));
    }

    #[tokio::test]
    async fn execute_no_host_returns_error() {
        let tool = WebFetchTool;
        let ctx = test_ctx();
        let result = tool
            .execute(
                serde_json::json!({"url": "https://", "prompt": "extract"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.text().contains("hostname"));
    }

    #[tokio::test]
    async fn execute_valid_returns_stub() {
        let tool = WebFetchTool;
        let ctx = test_ctx();
        let result = tool
            .execute(
                serde_json::json!({
                    "url": "https://example.com/page",
                    "prompt": "extract the main content"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.text();
        assert!(text.contains("https://example.com/page"));
        assert!(text.contains("extract the main content"));
        assert!(text.contains("placeholder"));
    }

    #[tokio::test]
    async fn execute_with_custom_timeout() {
        let tool = WebFetchTool;
        let ctx = test_ctx();
        let result = tool
            .execute(
                serde_json::json!({
                    "url": "https://example.com",
                    "prompt": "get info",
                    "timeout_secs": 60
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(result.text().contains("60s"));
    }

    #[tokio::test]
    async fn execute_caps_timeout_at_120() {
        let tool = WebFetchTool;
        let ctx = test_ctx();
        let result = tool
            .execute(
                serde_json::json!({
                    "url": "https://example.com",
                    "prompt": "get info",
                    "timeout_secs": 999
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result.text().contains("120s"));
    }

    #[test]
    fn validate_url_valid_https() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("https://example.com/path?q=1").is_ok());
    }

    #[test]
    fn validate_url_valid_http() {
        assert!(validate_url("http://localhost:8080").is_ok());
    }

    #[test]
    fn validate_url_rejects_ftp() {
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn validate_url_rejects_no_host() {
        assert!(validate_url("https://").is_err());
        assert!(validate_url("https:///path").is_err());
    }

    #[test]
    fn validate_url_rejects_empty() {
        assert!(validate_url("").is_err());
    }

    #[test]
    fn stub_fetch_includes_url_and_prompt() {
        let result = stub_fetch("https://test.com", "get data", 30, 5_000_000);
        assert!(result.contains("https://test.com"));
        assert!(result.contains("get data"));
        assert!(result.contains("30s"));
    }
}
