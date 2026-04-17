//! API-key authentication flow — resolves `ApiKeyConfig` into an `AuthToken`.
//!
//! The only runtime work for API-key auth is env-var expansion (`${VAR}`);
//! the token carries no expiry and does not need refresh.

use super::types::{ApiKeyConfig, AuthToken};
use crate::env_expansion::expand_env_vars;

/// Resolve an `ApiKeyConfig` into an `AuthToken` ready to attach to requests.
///
/// Performs `${VAR}` env expansion on the key value so configuration files
/// can reference secrets via environment variables rather than embedding
/// them directly.
pub fn resolve_api_key(config: &ApiKeyConfig) -> AuthToken {
    let key = expand_env_vars(&config.key);
    AuthToken {
        access_token: key,
        token_type: "ApiKey".into(),
        expires_at: None,
        refresh_token: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_literal_key() {
        let config = ApiKeyConfig {
            key: "sk-literal".into(),
            location: "header".into(),
            name: "Authorization".into(),
        };
        let tok = resolve_api_key(&config);
        assert_eq!(tok.access_token, "sk-literal");
        assert_eq!(tok.token_type, "ApiKey");
        assert!(tok.expires_at.is_none());
        assert!(tok.refresh_token.is_none());
    }

    // Note: env-var expansion roundtrip is covered by
    // `env_expansion::tests` at the crate root; duplicating a set_env
    // test here is blocked by the workspace `unsafe_code = "forbid"`
    // lint (Rust 2024 made `std::env::set_var` unsafe). The literal-key
    // and unset-var tests here cover the `resolve_api_key` wrapper.

    #[test]
    fn missing_env_var_expands_to_empty() {
        let config = ApiKeyConfig {
            key: "${CRAB_TEST_NEVER_SET_UNIQUE_8821}".into(),
            location: "header".into(),
            name: "Authorization".into(),
        };
        let tok = resolve_api_key(&config);
        // Behaviour per env_expansion: unset vars resolve to empty string.
        assert_eq!(tok.access_token, "");
    }
}
