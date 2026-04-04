pub mod api_key;
#[cfg(feature = "bedrock")]
pub mod bedrock_auth;
pub mod error;
pub mod keychain;
pub mod oauth;

pub use error::AuthError;

use std::future::Future;
use std::pin::Pin;

pub enum AuthMethod {
    ApiKey(String),
    OAuth(OAuthToken),
}

pub struct OAuthToken {
    pub access_token: String,
}

pub trait AuthProvider: Send + Sync {
    fn get_auth(
        &self,
    ) -> Pin<Box<dyn Future<Output = crab_common::Result<AuthMethod>> + Send + '_>>;
    fn refresh(&self) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>>;
}

/// Simple API key auth provider.
pub struct ApiKeyProvider {
    key: String,
}

impl ApiKeyProvider {
    #[must_use]
    pub fn new(key: String) -> Self {
        Self { key }
    }
}

impl AuthProvider for ApiKeyProvider {
    fn get_auth(
        &self,
    ) -> Pin<Box<dyn Future<Output = crab_common::Result<AuthMethod>> + Send + '_>> {
        let key = self.key.clone();
        Box::pin(async move { Ok(AuthMethod::ApiKey(key)) })
    }

    fn refresh(&self) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

/// Create an auth provider from settings.
///
/// Resolves API key from: settings → `ANTHROPIC_API_KEY` env var → keychain.
pub fn create_auth_provider(settings: &crab_config::Settings) -> Box<dyn AuthProvider> {
    let key = settings
        .api_key
        .clone()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .unwrap_or_default();
    Box::new(ApiKeyProvider::new(key))
}
