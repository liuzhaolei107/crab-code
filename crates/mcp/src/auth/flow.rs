//! Authorisation URL construction + state/nonce generation.
//!
//! Builds the initial redirect URL that takes the user to the OAuth
//! provider's authorisation endpoint. Includes:
//!
//! - `response_type=code` (authorisation code flow)
//! - `client_id`, `redirect_uri`, `scope`
//! - `state` (CSRF protection; random ~32 chars)
//! - `code_challenge` + `code_challenge_method=S256` (PKCE, from [`super::pkce`])
//!
//! The resulting URL is opened in the user's default browser by
//! [`crate::auth::callback::await_callback`]'s caller (usually
//! `webbrowser::open`).

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt as _;

use super::pkce::PkceChallenge;
use super::types::OAuthConfig;

/// Everything the caller needs to complete an OAuth flow after redirecting
/// the browser to [`Self::authorize_url`].
#[derive(Debug, Clone)]
pub struct AuthorizationRequest {
    /// The full URL to open in the user's browser.
    pub authorize_url: String,
    /// CSRF `state` parameter — callback must return this exact string.
    pub state: String,
    /// The PKCE challenge; the verifier is held here for later token exchange.
    pub pkce: PkceChallenge,
    /// Redirect URI that was encoded into `authorize_url` — the callback
    /// server must listen on this exact URL for the provider's redirect.
    pub redirect_uri: String,
}

impl AuthorizationRequest {
    /// Build an authorization request from an `OAuthConfig`.
    ///
    /// Generates a fresh PKCE challenge + CSRF state, and encodes all
    /// parameters into the authorisation URL.
    #[must_use]
    pub fn build(config: &OAuthConfig) -> Self {
        let pkce = PkceChallenge::new();
        let state = random_state();
        let redirect_uri = config.redirect_uri.clone();
        let authorize_url = build_authorize_url(config, &pkce, &state);
        Self {
            authorize_url,
            state,
            pkce,
            redirect_uri,
        }
    }
}

/// Generate a random CSRF state string (32 URL-safe chars).
///
/// The provider echoes this back on the callback; the callback handler
/// verifies it matches to prevent cross-site request forgery.
#[must_use]
pub fn random_state() -> String {
    let mut buf = [0u8; 24];
    rand::rng().fill(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Assemble the authorization URL with all required query parameters.
fn build_authorize_url(config: &OAuthConfig, pkce: &PkceChallenge, state: &str) -> String {
    let mut url = config.auth_url.clone();
    let separator = if url.contains('?') { '&' } else { '?' };
    url.push(separator);

    // Stable ordering matches most provider examples; order isn't semantically
    // required, but stable ordering helps when comparing captured redirect
    // URLs in tests / debugging.
    let scope = config.scopes.join(" ");
    let params = [
        ("response_type", "code"),
        ("client_id", config.client_id.as_str()),
        ("redirect_uri", config.redirect_uri.as_str()),
        ("scope", scope.as_str()),
        ("state", state),
        ("code_challenge", pkce.challenge()),
        ("code_challenge_method", pkce.method()),
    ];

    let encoded: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect();
    url.push_str(&encoded.join("&"));
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> OAuthConfig {
        OAuthConfig {
            client_id: "test-client".into(),
            client_secret: None,
            auth_url: "https://auth.example.com/oauth/authorize".into(),
            token_url: "https://auth.example.com/oauth/token".into(),
            redirect_uri: "http://localhost:12345/callback".into(),
            scopes: vec!["read".into(), "write:repo".into()],
        }
    }

    #[test]
    fn authorize_url_contains_all_params() {
        let req = AuthorizationRequest::build(&sample_config());
        let u = &req.authorize_url;
        assert!(u.starts_with("https://auth.example.com/oauth/authorize?"));
        assert!(u.contains("response_type=code"));
        assert!(u.contains("client_id=test-client"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(u.contains(&format!("code_challenge={}", req.pkce.challenge())));
        assert!(u.contains(&format!("state={}", req.state)));
        // Scopes space-encoded
        assert!(u.contains("scope=read%20write%3Arepo"));
        // Redirect URI percent-encoded
        assert!(u.contains("redirect_uri=http%3A%2F%2Flocalhost%3A12345%2Fcallback"));
    }

    #[test]
    fn authorize_url_preserves_existing_query() {
        let mut cfg = sample_config();
        cfg.auth_url = "https://auth.example.com/authorize?tenant=acme".into();
        let req = AuthorizationRequest::build(&cfg);
        // Pre-existing `?tenant=acme` should be preserved with `&` separator
        assert!(
            req.authorize_url
                .starts_with("https://auth.example.com/authorize?tenant=acme&")
        );
        assert!(req.authorize_url.contains("response_type=code"));
    }

    #[test]
    fn state_is_url_safe_and_high_entropy() {
        let a = random_state();
        let b = random_state();
        assert_ne!(a, b);
        assert!(a.len() >= 32);
        for c in a.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_');
        }
    }

    #[test]
    fn build_returns_matching_components() {
        let cfg = sample_config();
        let req = AuthorizationRequest::build(&cfg);
        assert_eq!(req.redirect_uri, cfg.redirect_uri);
        assert!(!req.state.is_empty());
        assert_eq!(req.pkce.method(), "S256");
    }

    #[test]
    fn two_builds_use_unique_pkce_and_state() {
        let cfg = sample_config();
        let a = AuthorizationRequest::build(&cfg);
        let b = AuthorizationRequest::build(&cfg);
        assert_ne!(a.state, b.state);
        assert_ne!(a.pkce.verifier(), b.pkce.verifier());
    }
}
