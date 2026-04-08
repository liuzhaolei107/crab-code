//! JWT session token generation and validation.
//!
//! Session tokens are used to authenticate IDE extensions connecting
//! to the bridge. Tokens are short-lived and scoped to a specific
//! session.

use serde::{Deserialize, Serialize};

/// Claims embedded in a session token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    /// Session ID this token is scoped to.
    pub session_id: String,
    /// Issued-at timestamp (Unix epoch seconds).
    pub iat: u64,
    /// Expiration timestamp (Unix epoch seconds).
    pub exp: u64,
    /// Subject (client identifier).
    pub sub: String,
}

/// Default token validity duration (1 hour).
const DEFAULT_TTL_SECS: u64 = 3600;

/// Generate a new session token.
///
/// Creates a JSON-serialized token scoped to the given session ID.
/// A proper JWT with cryptographic signing should replace this in
/// production; the current implementation is suitable for local-only
/// IPC where both endpoints are trusted.
pub fn generate_token(session_id: &str, client_id: &str) -> crab_common::Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let claims = SessionClaims {
        session_id: session_id.to_string(),
        iat: now,
        exp: now + DEFAULT_TTL_SECS,
        sub: client_id.to_string(),
    };

    serde_json::to_string(&claims)
        .map_err(|e| crab_common::Error::Auth(format!("failed to serialize session token: {e}")))
}

/// Validate a session token and extract claims.
///
/// Deserializes the token and checks expiration.
pub fn validate_token(token: &str) -> crab_common::Result<SessionClaims> {
    let claims: SessionClaims = serde_json::from_str(token)
        .map_err(|e| crab_common::Error::Auth(format!("invalid session token: {e}")))?;

    if is_expired(&claims) {
        return Err(crab_common::Error::Auth("session token has expired".into()));
    }

    Ok(claims)
}

/// Check if a token has expired.
#[must_use]
pub fn is_expired(claims: &SessionClaims) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    claims.exp <= now
}

/// Compute the default expiration timestamp from now.
#[must_use]
pub fn default_expiration() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + DEFAULT_TTL_SECS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_claims_serde() {
        let claims = SessionClaims {
            session_id: "sess_123".into(),
            iat: 1000,
            exp: 2000,
            sub: "client_abc".into(),
        };
        let json = serde_json::to_string(&claims).unwrap();
        let parsed: SessionClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.session_id, "sess_123");
        assert_eq!(parsed.iat, 1000);
        assert_eq!(parsed.exp, 2000);
    }

    #[test]
    fn expired_claims() {
        let claims = SessionClaims {
            session_id: "s".into(),
            iat: 0,
            exp: 1, // expired long ago
            sub: "c".into(),
        };
        assert!(is_expired(&claims));
    }

    #[test]
    fn future_claims_not_expired() {
        let claims = SessionClaims {
            session_id: "s".into(),
            iat: 0,
            exp: u64::MAX,
            sub: "c".into(),
        };
        assert!(!is_expired(&claims));
    }

    #[test]
    fn default_expiration_is_future() {
        let exp = default_expiration();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(exp > now);
    }
}
