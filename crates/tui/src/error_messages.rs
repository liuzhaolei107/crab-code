//! Friendly classifier for raw LLM/API error strings.
//!
//! The agent runtime hands errors to the TUI as a single string (see
//! `AppEvent::AgentError`). Raw strings tend to be JSON envelopes or
//! `error sending request` from reqwest. Showing them as-is is loud and
//! unhelpful.
//!
//! [`classify_error`] inspects the raw text, picks a friendly replacement
//! when it matches a known pattern, and returns a [`SystemKind`] severity
//! so the renderer can pick the right colour. Unknown errors fall back to
//! a truncated dump in red.
//!
//! Severity policy:
//! - Transient / server-side / retryable issues → `Warning` (yellow)
//! - Configuration / hard failures user must act on → `Error` (red)

use crate::history::cells::SystemKind;

/// Maximum body length for unknown error messages before truncation.
const MAX_UNKNOWN_ERROR_CHARS: usize = 1000;

/// Classify a raw error string and return a friendly replacement plus severity.
///
/// The output text is what should land in the conversation transcript —
/// callers do not need to add prefixes or do extra truncation.
#[must_use]
pub fn classify_error(raw: &str) -> (String, SystemKind) {
    let lower = raw.to_lowercase();

    // Order matters: more specific patterns first.

    // Rate limit / 429
    if lower.contains("rate limit")
        || lower.contains("rate_limited")
        || lower.contains("status=429")
        || lower.contains("retry after")
    {
        return (
            "Rate limit reached. Please wait a moment and retry.".into(),
            SystemKind::Warning,
        );
    }

    // Context length / prompt too long
    if lower.contains("context_length_exceeded")
        || lower.contains("prompt is too long")
        || lower.contains("context limit")
        || lower.contains("maximum context length")
    {
        return (
            "Context limit reached \u{00b7} use /compact or /clear to continue.".into(),
            SystemKind::Error,
        );
    }

    // Quota / credit / billing
    if lower.contains("credit balance")
        || lower.contains("quota")
        || lower.contains("billing")
        || lower.contains("insufficient_quota")
    {
        return (
            "Credit balance too low \u{00b7} add funds to continue.".into(),
            SystemKind::Error,
        );
    }

    // Auth: invalid api key, 401, unauthorized, token revoked
    if lower.contains("invalid api key")
        || lower.contains("invalid_api_key")
        || lower.contains("status=401")
        || lower.contains("unauthorized")
        || lower.contains("token revoked")
        || lower.contains("token_revoked")
    {
        return (
            "Invalid or revoked API key \u{00b7} check your provider credentials.".into(),
            SystemKind::Error,
        );
    }

    // Server overloaded / 503 / 502 / 504
    if lower.contains("status=503")
        || lower.contains("status=502")
        || lower.contains("status=504")
        || lower.contains("overloaded")
        || lower.contains("service unavailable")
        || lower.contains("bad gateway")
        || lower.contains("gateway timeout")
    {
        return (
            "API is overloaded. Please retry in a moment.".into(),
            SystemKind::Warning,
        );
    }

    // Timeout
    if lower.contains("timed out") || lower.contains("timeout") {
        return (
            "Request timed out. Please retry.".into(),
            SystemKind::Warning,
        );
    }

    // Network / DNS / connection
    if lower.contains("dns error")
        || lower.contains("connection refused")
        || lower.contains("error sending request")
        || lower.contains("connection reset")
    {
        return (
            "Network error. Check your connection and retry.".into(),
            SystemKind::Warning,
        );
    }

    // Org disabled / custom off-switch / abuse
    if lower.contains("organization") && lower.contains("disabled") {
        return (
            "Organization access disabled \u{00b7} contact your administrator.".into(),
            SystemKind::Error,
        );
    }

    // Unknown error: dump original text, truncated, in red.
    let truncated = if raw.chars().count() > MAX_UNKNOWN_ERROR_CHARS {
        let head: String = raw.chars().take(MAX_UNKNOWN_ERROR_CHARS).collect();
        format!("{head}\u{2026}")
    } else {
        raw.to_string()
    };
    (truncated, SystemKind::Error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_warning() {
        let (text, kind) = classify_error("rate limited, retry after 5000ms");
        assert_eq!(kind, SystemKind::Warning);
        assert!(text.contains("Rate limit"));
    }

    #[test]
    fn context_limit_error() {
        let (text, kind) = classify_error("API error: status=400, message=context_length_exceeded");
        assert_eq!(kind, SystemKind::Error);
        assert!(text.contains("Context limit"));
    }

    #[test]
    fn credit_balance_error() {
        let (text, kind) = classify_error("credit balance too low");
        assert_eq!(kind, SystemKind::Error);
        assert!(text.contains("Credit balance"));
    }

    #[test]
    fn invalid_api_key_error() {
        let (text, kind) = classify_error("API error: status=401, message=invalid_api_key");
        assert_eq!(kind, SystemKind::Error);
        assert!(text.contains("Invalid or revoked"));
    }

    #[test]
    fn server_overloaded_warning() {
        let (text, kind) = classify_error("API error: status=503, message=overloaded_error");
        assert_eq!(kind, SystemKind::Warning);
        assert!(text.contains("overloaded"));
    }

    #[test]
    fn timeout_warning() {
        let (text, kind) = classify_error("request timed out");
        assert_eq!(kind, SystemKind::Warning);
        assert!(text.contains("timed out"));
    }

    #[test]
    fn network_warning() {
        let (text, kind) = classify_error("error sending request: dns error: lookup failed");
        assert_eq!(kind, SystemKind::Warning);
        assert!(text.contains("Network error"));
    }

    #[test]
    fn unknown_error_falls_back_to_red_dump() {
        let raw = "some_weird_internal_failure_xyz";
        let (text, kind) = classify_error(raw);
        assert_eq!(kind, SystemKind::Error);
        assert_eq!(text, raw);
    }

    #[test]
    fn unknown_long_error_is_truncated() {
        let raw = "x".repeat(2000);
        let (text, kind) = classify_error(&raw);
        assert_eq!(kind, SystemKind::Error);
        assert_eq!(text.chars().count(), MAX_UNKNOWN_ERROR_CHARS + 1);
        assert!(text.ends_with('\u{2026}'));
    }

    #[test]
    fn pattern_matching_is_case_insensitive() {
        let (_, kind) = classify_error("RATE LIMIT EXCEEDED");
        assert_eq!(kind, SystemKind::Warning);
    }

    #[test]
    fn org_disabled_error() {
        let (text, kind) = classify_error("organization has been disabled");
        assert_eq!(kind, SystemKind::Error);
        assert!(text.contains("Organization"));
    }
}
