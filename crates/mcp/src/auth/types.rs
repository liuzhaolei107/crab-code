//! Data types shared across the MCP auth subsystem.

use serde::{Deserialize, Serialize};

// ─── Auth method configuration ─────────────────────────────────────────

/// Authentication method configured for an MCP server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpAuthMethod {
    /// No authentication required.
    #[default]
    None,
    /// Static API key, sent as a header or query parameter.
    ApiKey(ApiKeyConfig),
    /// `OAuth2` authorization code flow with PKCE.
    OAuth2(OAuthConfig),
}

/// Configuration for API-key authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// The API key value. May reference an environment variable via `${VAR}` syntax.
    pub key: String,
    /// Where to send the key: `"header"` (default) or `"query"`.
    #[serde(default = "default_key_location")]
    pub location: String,
    /// Header name or query parameter name (default: `"Authorization"`).
    #[serde(default = "default_header_name")]
    pub name: String,
}

fn default_key_location() -> String {
    "header".into()
}

fn default_header_name() -> String {
    "Authorization".into()
}

/// Configuration for `OAuth2` authorization code flow with PKCE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// `OAuth2` client identifier.
    pub client_id: String,
    /// `OAuth2` client secret (optional for public clients using PKCE).
    pub client_secret: Option<String>,
    /// Authorization endpoint URL.
    pub auth_url: String,
    /// Token endpoint URL.
    pub token_url: String,
    /// Redirect URI for the authorization callback.
    #[serde(default = "default_redirect_uri")]
    pub redirect_uri: String,
    /// Requested scopes.
    #[serde(default)]
    pub scopes: Vec<String>,
}

fn default_redirect_uri() -> String {
    "http://localhost:0/callback".into()
}

// ─── Auth token ────────────────────────────────────────────────────────

/// A resolved authentication token ready to attach to outbound requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// The bearer or API key token value.
    pub access_token: String,
    /// Token type (e.g., `"Bearer"`, `"ApiKey"`).
    pub token_type: String,
    /// Expiry timestamp (seconds since Unix epoch), if known.
    pub expires_at: Option<u64>,
    /// Refresh token, if the provider issued one (`OAuth2` flows).
    pub refresh_token: Option<String>,
}

impl AuthToken {
    /// Check whether the token has expired, with a 60-second grace window.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let Some(expires_at) = self.expires_at else {
            return false; // No expiry recorded = never expires (e.g. API keys).
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now + 60 >= expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_auth_method_is_none() {
        assert!(matches!(McpAuthMethod::default(), McpAuthMethod::None));
    }

    #[test]
    fn auth_method_api_key_serde_roundtrip() {
        let method = McpAuthMethod::ApiKey(ApiKeyConfig {
            key: "sk-test".into(),
            location: "header".into(),
            name: "Authorization".into(),
        });
        let json = serde_json::to_string(&method).unwrap();
        let parsed: McpAuthMethod = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, McpAuthMethod::ApiKey(_)));
    }

    #[test]
    fn oauth_config_serde() {
        let config = OAuthConfig {
            client_id: "my-client".into(),
            client_secret: None,
            auth_url: "https://auth.example.com/authorize".into(),
            token_url: "https://auth.example.com/token".into(),
            redirect_uri: "http://localhost:0/callback".into(),
            scopes: vec!["read".into(), "write".into()],
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: OAuthConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.client_id, "my-client");
        assert_eq!(parsed.scopes.len(), 2);
    }

    #[test]
    fn token_never_expires_without_timestamp() {
        let tok = AuthToken {
            access_token: "k".into(),
            token_type: "ApiKey".into(),
            expires_at: None,
            refresh_token: None,
        };
        assert!(!tok.is_expired());
    }

    #[test]
    fn token_expired_detects_past_timestamp() {
        let tok = AuthToken {
            access_token: "k".into(),
            token_type: "Bearer".into(),
            expires_at: Some(1), // Unix epoch + 1 second → long past
            refresh_token: None,
        };
        assert!(tok.is_expired());
    }

    #[test]
    fn token_expired_detects_grace_window() {
        let soon = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 30; // expires in 30s — within 60s grace window
        let tok = AuthToken {
            access_token: "k".into(),
            token_type: "Bearer".into(),
            expires_at: Some(soon),
            refresh_token: None,
        };
        assert!(tok.is_expired());
    }

    #[test]
    fn token_not_expired_when_far_future() {
        let far = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600; // 1 hour out
        let tok = AuthToken {
            access_token: "k".into(),
            token_type: "Bearer".into(),
            expires_at: Some(far),
            refresh_token: None,
        };
        assert!(!tok.is_expired());
    }
}
