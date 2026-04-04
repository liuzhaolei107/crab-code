//! Google Vertex AI adapter (feature = "vertex").
//!
//! Wraps `AnthropicClient` with different auth (GCP) and base URL.
//! The underlying protocol is the same Anthropic Messages API.

use crate::anthropic::AnthropicClient;

/// Configuration for Google Vertex AI access.
#[cfg(feature = "vertex")]
pub struct VertexConfig {
    pub project_id: String,
    pub region: String,
    pub model_id: String,
}

/// Create an `AnthropicClient` configured for Google Vertex AI.
///
/// Vertex uses the same Anthropic Messages API format but with
/// GCP authentication and a different endpoint.
#[cfg(feature = "vertex")]
pub fn create_vertex_client(_config: &VertexConfig) -> AnthropicClient {
    // TODO: construct AnthropicClient with Vertex auth + base_url
    todo!("vertex client construction")
}
