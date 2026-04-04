//! OpenAI-compatible Chat Completions API client — HTTP + SSE + retry.

use futures::stream::{self, Stream};

use crate::error::{ApiError, Result};
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

/// Chat Completions API client.
#[allow(dead_code)]
pub struct OpenAiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl OpenAiClient {
    pub fn new(base_url: &str, api_key: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            base_url: base_url.to_string(),
            api_key,
        }
    }

    /// Streaming call — POST `/v1/chat/completions` with `stream: true`.
    ///
    /// Returns a stream of `StreamEvent` mapped from SSE chunks.
    pub fn stream<'a>(
        &'a self,
        _req: MessageRequest<'a>,
    ) -> impl Stream<Item = Result<StreamEvent>> + Send + 'a {
        // TODO: implement SSE streaming
        // 1. MessageRequest → ChatCompletionRequest (via convert::to_chat_completion_request)
        // 2. POST /v1/chat/completions with stream: true
        // 3. Parse SSE: data: {"choices":[{"delta":...}]}
        // 4. convert::chunk_to_stream_event() → internal StreamEvent
        stream::empty()
    }

    /// Non-streaming call — POST `/v1/chat/completions`.
    ///
    /// # Errors
    ///
    /// Returns `ApiError` on HTTP, JSON, or API-level errors.
    #[allow(clippy::unused_async)]
    pub async fn send(&self, _req: MessageRequest<'_>) -> Result<MessageResponse> {
        // TODO: implement non-streaming request
        Err(ApiError::Api {
            status: 0,
            message: "not yet implemented".to_string(),
        })
    }

    /// Base URL for this client.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
