//! Error classification for API responses.
//!
//! `ErrorCategory` classifies API errors into actionable categories.
//! `classify_error` maps HTTP status codes and response bodies to categories.
//! `is_retryable` determines which errors are worth retrying.

use std::fmt;

/// Classification of an API error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Temporary server-side issue (500, 502, 503).
    Transient,
    /// Rate limit exceeded (429, 529).
    RateLimit,
    /// Authentication/authorization failure (401, 403).
    Auth,
    /// Client-side invalid request (400, 422).
    InvalidRequest,
    /// Server error that is unlikely to resolve on retry (501).
    ServerError,
    /// Network-level error (DNS, connection refused, etc.).
    NetworkError,
    /// Request timed out.
    Timeout,
    /// Unknown / unclassified error.
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transient => write!(f, "transient"),
            Self::RateLimit => write!(f, "rate_limit"),
            Self::Auth => write!(f, "auth"),
            Self::InvalidRequest => write!(f, "invalid_request"),
            Self::ServerError => write!(f, "server_error"),
            Self::NetworkError => write!(f, "network_error"),
            Self::Timeout => write!(f, "timeout"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Classify an error based on HTTP status code and optional response body.
#[must_use]
pub fn classify_error(status_code: u16, body: &str) -> ErrorCategory {
    match status_code {
        401 | 403 => ErrorCategory::Auth,
        400 | 422 => ErrorCategory::InvalidRequest,
        429 | 529 => ErrorCategory::RateLimit,
        408 => ErrorCategory::Timeout,
        500 | 502 | 503 => ErrorCategory::Transient,
        501 => ErrorCategory::ServerError,
        0 => {
            // No HTTP status — likely a network-level error.
            let lower = body.to_lowercase();
            if lower.contains("timeout") || lower.contains("timed out") {
                ErrorCategory::Timeout
            } else if lower.contains("dns")
                || lower.contains("connection refused")
                || lower.contains("network")
            {
                ErrorCategory::NetworkError
            } else {
                ErrorCategory::Unknown
            }
        }
        _ if status_code >= 500 => ErrorCategory::Transient,
        _ => ErrorCategory::Unknown,
    }
}

/// Whether the given error category is worth retrying.
#[must_use]
pub fn is_retryable(category: ErrorCategory) -> bool {
    matches!(
        category,
        ErrorCategory::Transient
            | ErrorCategory::RateLimit
            | ErrorCategory::Timeout
            | ErrorCategory::NetworkError
    )
}

/// Generate a user-friendly error message for the given category.
#[must_use]
pub fn error_to_user_message(category: ErrorCategory) -> String {
    match category {
        ErrorCategory::Transient => {
            "The API server is temporarily unavailable. Retrying...".to_string()
        }
        ErrorCategory::RateLimit => "Rate limit exceeded. Waiting before retrying...".to_string(),
        ErrorCategory::Auth => {
            "Authentication failed. Please check your API key or credentials.".to_string()
        }
        ErrorCategory::InvalidRequest => {
            "The request was invalid. Please check your input.".to_string()
        }
        ErrorCategory::ServerError => {
            "The API server returned an error. This may require manual intervention.".to_string()
        }
        ErrorCategory::NetworkError => {
            "Network error. Please check your internet connection.".to_string()
        }
        ErrorCategory::Timeout => {
            "The request timed out. Retrying with a shorter context...".to_string()
        }
        ErrorCategory::Unknown => "An unexpected error occurred.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_auth_errors() {
        assert_eq!(classify_error(401, ""), ErrorCategory::Auth);
        assert_eq!(classify_error(403, ""), ErrorCategory::Auth);
    }

    #[test]
    fn classify_invalid_request() {
        assert_eq!(classify_error(400, ""), ErrorCategory::InvalidRequest);
        assert_eq!(classify_error(422, ""), ErrorCategory::InvalidRequest);
    }

    #[test]
    fn classify_rate_limit() {
        assert_eq!(classify_error(429, ""), ErrorCategory::RateLimit);
        assert_eq!(classify_error(529, ""), ErrorCategory::RateLimit);
    }

    #[test]
    fn classify_transient() {
        assert_eq!(classify_error(500, ""), ErrorCategory::Transient);
        assert_eq!(classify_error(502, ""), ErrorCategory::Transient);
        assert_eq!(classify_error(503, ""), ErrorCategory::Transient);
        assert_eq!(classify_error(504, ""), ErrorCategory::Transient); // >= 500
    }

    #[test]
    fn classify_server_error() {
        assert_eq!(classify_error(501, ""), ErrorCategory::ServerError);
    }

    #[test]
    fn classify_timeout() {
        assert_eq!(classify_error(408, ""), ErrorCategory::Timeout);
    }

    #[test]
    fn classify_network_from_body() {
        assert_eq!(
            classify_error(0, "DNS resolution failed"),
            ErrorCategory::NetworkError
        );
        assert_eq!(
            classify_error(0, "connection refused"),
            ErrorCategory::NetworkError
        );
    }

    #[test]
    fn classify_timeout_from_body() {
        assert_eq!(
            classify_error(0, "request timed out"),
            ErrorCategory::Timeout
        );
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(classify_error(0, ""), ErrorCategory::Unknown);
        assert_eq!(classify_error(418, "teapot"), ErrorCategory::Unknown);
    }

    #[test]
    fn retryable_categories() {
        assert!(is_retryable(ErrorCategory::Transient));
        assert!(is_retryable(ErrorCategory::RateLimit));
        assert!(is_retryable(ErrorCategory::Timeout));
        assert!(is_retryable(ErrorCategory::NetworkError));
        assert!(!is_retryable(ErrorCategory::Auth));
        assert!(!is_retryable(ErrorCategory::InvalidRequest));
        assert!(!is_retryable(ErrorCategory::ServerError));
        assert!(!is_retryable(ErrorCategory::Unknown));
    }

    #[test]
    fn user_messages_non_empty() {
        let categories = [
            ErrorCategory::Transient,
            ErrorCategory::RateLimit,
            ErrorCategory::Auth,
            ErrorCategory::InvalidRequest,
            ErrorCategory::ServerError,
            ErrorCategory::NetworkError,
            ErrorCategory::Timeout,
            ErrorCategory::Unknown,
        ];
        for cat in &categories {
            let msg = error_to_user_message(*cat);
            assert!(!msg.is_empty(), "empty message for {cat}");
        }
    }

    #[test]
    fn category_display() {
        assert_eq!(ErrorCategory::RateLimit.to_string(), "rate_limit");
        assert_eq!(ErrorCategory::Timeout.to_string(), "timeout");
    }
}
