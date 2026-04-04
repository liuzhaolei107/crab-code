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
