//! Anthropic Messages API client — HTTP + SSE + retry.

use futures::stream::{self, Stream};

use crate::error::{ApiError, Result};
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

/// Anthropic Messages API client.
#[allow(dead_code)]
pub struct AnthropicClient {
    http: reqwest::Client,
    base_url: String,
    auth: Box<dyn crab_auth::AuthProvider>,
}

impl AnthropicClient {
    pub fn new(base_url: &str, auth: Box<dyn crab_auth::AuthProvider>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            base_url: base_url.to_string(),
            auth,
        }
    }

    /// Streaming call — POST /v1/messages with stream: true.
    ///
    /// Returns a stream of `StreamEvent` mapped from Anthropic SSE events.
    pub fn stream<'a>(
        &'a self,
        _req: MessageRequest<'a>,
    ) -> impl Stream<Item = Result<StreamEvent>> + Send + 'a {
        // TODO: implement SSE streaming
        // 1. MessageRequest → AnthropicRequest (via convert::to_anthropic_request)
        // 2. POST /v1/messages with stream: true
        // 3. Parse Anthropic SSE: message_start / content_block_delta / message_stop
        // 4. convert::sse_event_to_stream_event() → internal StreamEvent
        stream::empty()
    }

    /// Non-streaming call — POST `/v1/messages`.
    ///
    /// # Errors
    ///
    /// Returns `ApiError` on HTTP, JSON, or API-level errors.
    #[allow(clippy::unused_async)]
    pub async fn send(&self, _req: MessageRequest<'_>) -> Result<MessageResponse> {
        // TODO: implement non-streaming request
        // 1. MessageRequest → AnthropicRequest (via convert::to_anthropic_request)
        // 2. POST /v1/messages
        // 3. Parse AnthropicResponse
        // 4. convert::from_anthropic_response() → (Message, TokenUsage)
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
