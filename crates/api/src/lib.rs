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
    // Bedrock and Vertex are the same Anthropic API with different auth + base_url
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
