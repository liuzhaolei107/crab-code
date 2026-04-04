//! AWS Bedrock adapter (feature = "bedrock").
//!
//! Wraps `AnthropicClient` with different auth (AWS SigV4) and base URL.
//! The underlying protocol is the same Anthropic Messages API.

use crate::anthropic::AnthropicClient;

/// Configuration for AWS Bedrock runtime access.
#[cfg(feature = "bedrock")]
pub struct BedrockConfig {
    pub region: String,
    pub model_id: String,
}

/// Create an `AnthropicClient` configured for AWS Bedrock.
///
/// Bedrock uses the same Anthropic Messages API format but with
/// AWS SigV4 authentication and a different endpoint.
#[cfg(feature = "bedrock")]
pub fn create_bedrock_client(_config: &BedrockConfig) -> AnthropicClient {
    // TODO: construct AnthropicClient with Bedrock auth + base_url
    todo!("bedrock client construction")
}
