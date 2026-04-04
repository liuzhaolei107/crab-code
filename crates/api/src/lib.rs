//! LLM API clients for Crab Code.
//!
//! Two independent clients — Anthropic (Messages API) and OpenAI-compatible
//! (Chat Completions) — dispatched via the `LlmBackend` enum.
//! No dynamic trait dispatch; compile-time determined, exhaustive match.

pub mod anthropic;
pub mod cache;
pub mod error;
pub mod openai;
pub mod rate_limit;
pub mod types;

#[cfg(feature = "bedrock")]
pub mod bedrock;
#[cfg(feature = "vertex")]
pub mod vertex;

use futures::future::Either;
use futures::stream::Stream;

use crate::error::Result;
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

/// LLM backend enum — provider count is bounded (2 standards + 2 cloud variants).
///
/// Enum dispatch: zero dynamic dispatch overhead, exhaustive match ensures no variant missed.
pub enum LlmBackend {
    Anthropic(anthropic::AnthropicClient),
    OpenAi(openai::OpenAiClient),
    #[cfg(feature = "bedrock")]
    Bedrock(anthropic::AnthropicClient),
    #[cfg(feature = "vertex")]
    Vertex(anthropic::AnthropicClient),
}

impl LlmBackend {
    /// Stream a message request, returning internal `StreamEvent`s.
    pub fn stream_message<'a>(
        &'a self,
        req: MessageRequest<'a>,
    ) -> impl Stream<Item = Result<StreamEvent>> + Send + 'a {
        match self {
            Self::Anthropic(c) => Either::Left(c.stream(req)),
            Self::OpenAi(c) => Either::Right(c.stream(req)),
            #[cfg(feature = "bedrock")]
            Self::Bedrock(c) => Either::Left(c.stream(req)),
            #[cfg(feature = "vertex")]
            Self::Vertex(c) => Either::Left(c.stream(req)),
        }
    }

    /// Non-streaming message send.
    ///
    /// # Errors
    ///
    /// Returns `ApiError` on HTTP, JSON, or API-level errors.
    pub async fn send_message(&self, req: MessageRequest<'_>) -> Result<MessageResponse> {
        match self {
            Self::Anthropic(c) => c.send(req).await,
            Self::OpenAi(c) => c.send(req).await,
            #[cfg(feature = "bedrock")]
            Self::Bedrock(c) => c.send(req).await,
            #[cfg(feature = "vertex")]
            Self::Vertex(c) => c.send(req).await,
        }
    }

    /// Provider name string.
    #[must_use]
    pub const fn name(&self) -> &str {
        match self {
            Self::Anthropic(_) => "anthropic",
            Self::OpenAi(_) => "openai",
            #[cfg(feature = "bedrock")]
            Self::Bedrock(_) => "bedrock",
            #[cfg(feature = "vertex")]
            Self::Vertex(_) => "vertex",
        }
    }
}

/// Create an `LlmBackend` from settings.
///
/// Routes to the appropriate client based on `api_provider`:
/// - `"openai"`, `"ollama"`, `"deepseek"` → `OpenAiClient`
/// - Everything else (including `None`) → `AnthropicClient`
#[must_use]
pub fn create_backend(settings: &crab_config::Settings) -> LlmBackend {
    if let Some("openai" | "ollama" | "deepseek" | "vllm") = settings.api_provider.as_deref() {
        let base_url = settings
            .api_base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        let api_key = settings
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());
        LlmBackend::OpenAi(openai::OpenAiClient::new(base_url, api_key))
    } else {
        let base_url = settings
            .api_base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");
        let auth = crab_auth::create_auth_provider(settings);
        LlmBackend::Anthropic(anthropic::AnthropicClient::new(base_url, auth))
    }
}
