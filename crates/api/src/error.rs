/// API layer error types.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: status={status}, message={message}")]
    Api { status: u16, message: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SSE stream error: {0}")]
    Sse(String),

    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("request timed out")]
    Timeout,

    #[error(transparent)]
    Common(#[from] crab_common::Error),
}

/// Convenience result type for the api crate.
pub type Result<T> = std::result::Result<T, ApiError>;
