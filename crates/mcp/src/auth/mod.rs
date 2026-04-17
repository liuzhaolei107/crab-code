//! MCP server authentication — `OAuth2` (Phase 7-auth.1+), API keys, token refresh.
//!
//! Supports multiple authentication methods for MCP server connections:
//! - [`McpAuthMethod::None`] — no authentication
//! - [`McpAuthMethod::ApiKey`] — static API key sent as header or query parameter
//! - [`McpAuthMethod::OAuth2`] — authorisation code flow with PKCE
//!
//! ## Module layout
//!
//! - [`types`] — `McpAuthMethod` / `ApiKeyConfig` / `OAuthConfig` / `AuthToken`
//! - [`api_key`] — API-key resolver (env-var expansion)
//! - [`pkce`] — PKCE code verifier + SHA-256 challenge (RFC 7636)
//! - [`discovery`] — RFC 9728 resource metadata + RFC 8414 server metadata
//! - [`store`] — persistent per-server token store (`~/.crab/mcp/tokens/`)
//!
//! The full `OAuth2` flow (auth URL builder, callback server, token exchange,
//! refresh) is in-progress: see the Phase 7-auth implementation plan.

pub mod api_key;
pub mod discovery;
pub mod pkce;
pub mod store;
pub mod types;

pub use discovery::{
    AuthServerMetadata, ResourceMetadata, discover_auth_server, discover_resource,
};
pub use pkce::PkceChallenge;
pub use store::{TokenStore, default_token_dir};
pub use types::{ApiKeyConfig, AuthToken, McpAuthMethod, OAuthConfig};

use std::collections::HashMap;

/// Manages authentication state for multiple MCP servers.
///
/// Stores resolved tokens per server and delegates to the appropriate flow
/// based on [`McpAuthMethod`]. The `OAuth2` flow is still being built out
/// in subsequent Phase 7-auth sub-PRs; API-key auth works end-to-end today.
pub struct McpAuthManager {
    /// Cached tokens keyed by server name.
    tokens: HashMap<String, AuthToken>,
}

impl McpAuthManager {
    /// Create a new manager with no cached tokens.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    /// Authenticate with an MCP server using the specified method.
    ///
    /// - [`McpAuthMethod::None`] returns an empty token immediately.
    /// - [`McpAuthMethod::ApiKey`] resolves env-var references and stores
    ///   the value.
    /// - [`McpAuthMethod::OAuth2`] is not yet wired end-to-end; currently
    ///   returns an error directing callers to the in-progress flow.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails (env var missing, OAuth
    /// flow error, network failure, etc.).
    pub async fn authenticate(
        &mut self,
        server_name: &str,
        method: &McpAuthMethod,
    ) -> crab_common::Result<AuthToken> {
        let token = match method {
            McpAuthMethod::None => AuthToken {
                access_token: String::new(),
                token_type: "None".into(),
                expires_at: None,
                refresh_token: None,
            },
            McpAuthMethod::ApiKey(config) => api_key::resolve_api_key(config),
            McpAuthMethod::OAuth2(_) => {
                return Err(crab_common::Error::Config(format!(
                    "OAuth2 auth for '{server_name}' is still being built out — \
                     PKCE / discovery / token store are ready, but the \
                     authorization-code + callback server + token exchange + \
                     refresh flow will land in Phase 7-auth.2-5. \
                     Use API key auth until then."
                )));
            }
        };

        self.tokens.insert(server_name.to_string(), token.clone());
        Ok(token)
    }

    /// Refresh an expired token using its refresh token.
    ///
    /// Not yet implemented (Phase 7-auth.4).
    ///
    /// # Errors
    ///
    /// Currently always returns an error.
    pub async fn refresh_token(
        &mut self,
        server_name: &str,
        token: &AuthToken,
    ) -> crab_common::Result<AuthToken> {
        if token.refresh_token.is_none() {
            return Err(crab_common::Error::Config(
                "no refresh token available".into(),
            ));
        }
        Err(crab_common::Error::Config(format!(
            "token refresh for '{server_name}' not yet implemented (Phase 7-auth.4)"
        )))
    }

    /// Get the cached token for a server, if one exists and is not expired.
    #[must_use]
    pub fn get_valid_token(&self, server_name: &str) -> Option<&AuthToken> {
        self.tokens.get(server_name).filter(|t| !t.is_expired())
    }

    /// Remove the cached token for a server.
    pub fn clear_token(&mut self, server_name: &str) {
        self.tokens.remove(server_name);
    }
}

impl Default for McpAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for McpAuthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpAuthManager")
            .field("cached_servers", &self.tokens.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_starts_empty() {
        let mgr = McpAuthManager::new();
        assert!(mgr.tokens.is_empty());
    }

    #[test]
    fn clear_token_removes_entry() {
        let mut mgr = McpAuthManager::new();
        mgr.tokens.insert(
            "test-server".into(),
            AuthToken {
                access_token: "tok".into(),
                token_type: "Bearer".into(),
                expires_at: None,
                refresh_token: None,
            },
        );
        mgr.clear_token("test-server");
        assert!(mgr.tokens.is_empty());
    }

    #[tokio::test]
    async fn authenticate_none_returns_empty_token() {
        let mut mgr = McpAuthManager::new();
        let tok = mgr.authenticate("any", &McpAuthMethod::None).await.unwrap();
        assert_eq!(tok.token_type, "None");
        assert!(tok.access_token.is_empty());
    }

    #[tokio::test]
    async fn authenticate_api_key_delegates_to_api_key_module() {
        let mut mgr = McpAuthManager::new();
        let method = McpAuthMethod::ApiKey(ApiKeyConfig {
            key: "sk-static".into(),
            location: "header".into(),
            name: "Authorization".into(),
        });
        let tok = mgr.authenticate("github", &method).await.unwrap();
        assert_eq!(tok.access_token, "sk-static");
        assert_eq!(tok.token_type, "ApiKey");
        assert!(mgr.get_valid_token("github").is_some());
    }

    #[tokio::test]
    async fn authenticate_oauth_returns_informative_error() {
        let mut mgr = McpAuthManager::new();
        let method = McpAuthMethod::OAuth2(OAuthConfig {
            client_id: "id".into(),
            client_secret: None,
            auth_url: "https://auth.example.com/authorize".into(),
            token_url: "https://auth.example.com/token".into(),
            redirect_uri: "http://localhost:0/callback".into(),
            scopes: vec![],
        });
        let err = mgr.authenticate("svc", &method).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Phase 7-auth"), "got: {msg}");
    }

    #[tokio::test]
    async fn refresh_without_token_errors() {
        let mut mgr = McpAuthManager::new();
        let tok = AuthToken {
            access_token: "a".into(),
            token_type: "Bearer".into(),
            expires_at: Some(1),
            refresh_token: None,
        };
        let err = mgr.refresh_token("svc", &tok).await.unwrap_err();
        assert!(err.to_string().contains("no refresh token"));
    }

    #[test]
    fn get_valid_token_filters_expired() {
        let mut mgr = McpAuthManager::new();
        mgr.tokens.insert(
            "fresh".into(),
            AuthToken {
                access_token: "a".into(),
                token_type: "Bearer".into(),
                expires_at: None, // never expires
                refresh_token: None,
            },
        );
        mgr.tokens.insert(
            "stale".into(),
            AuthToken {
                access_token: "b".into(),
                token_type: "Bearer".into(),
                expires_at: Some(1), // epoch + 1s — long expired
                refresh_token: None,
            },
        );
        assert!(mgr.get_valid_token("fresh").is_some());
        assert!(mgr.get_valid_token("stale").is_none());
    }
}
