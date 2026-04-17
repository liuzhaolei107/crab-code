//! JWT issue / verify for crab-proto connection tokens.
//!
//! HS256 (HMAC-SHA256) over a shared secret is used instead of
//! asymmetric signing because the server is the only issuer *and* the
//! only verifier — asymmetric gains no security here and costs a
//! keypair-management story. If we ever add delegated issuance we will
//! add an asymmetric sibling.
//!
//! Claims:
//!
//! - `sub` — session id the client is allowed to attach to (empty string
//!   for tokens that may create new sessions).
//! - `dev` — device id (trusted-device identifier, used for revocation).
//! - `iat` / `exp` — standard issued-at / expiry, seconds since epoch.

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT claim body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Session id this token can attach to; empty means "may create".
    pub sub: String,
    /// Device identifier for per-device revocation.
    pub dev: String,
    /// Issued-at, seconds since epoch.
    pub iat: u64,
    /// Expiry, seconds since epoch.
    pub exp: u64,
}

/// Signing / verifying errors surfaced to callers.
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("clock went backwards relative to epoch")]
    BadSystemTime,
    #[error("jwt library error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

/// Issue a new token that is valid for `ttl_seconds` starting now.
///
/// `secret` is the shared-secret bytes; reused verbatim for verify.
/// `session_id` may be empty for tokens that are allowed to create a
/// new session (subject becomes the session id after the `session/create`
/// round-trip succeeds).
pub fn sign(
    secret: &[u8],
    session_id: &str,
    device_id: &str,
    ttl_seconds: u64,
) -> Result<String, JwtError> {
    let iat = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| JwtError::BadSystemTime)?
        .as_secs();
    let claims = Claims {
        sub: session_id.to_string(),
        dev: device_id.to_string(),
        iat,
        exp: iat + ttl_seconds,
    };
    let header = Header::new(jsonwebtoken::Algorithm::HS256);
    let token = jsonwebtoken::encode(&header, &claims, &EncodingKey::from_secret(secret))?;
    Ok(token)
}

/// Verify a token and return its claims on success. Rejects expired
/// tokens automatically (jsonwebtoken checks `exp` by default).
pub fn verify(secret: &[u8], token: &str) -> Result<Claims, JwtError> {
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    // Require exp; don't look for aud/iss (we don't issue those).
    validation.set_required_spec_claims(&["exp", "sub", "iat"]);
    validation.validate_exp = true;
    let data =
        jsonwebtoken::decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"a-shared-secret-at-least-32-bytes!";

    #[test]
    fn roundtrip_preserves_claims() {
        let token = sign(SECRET, "sess_42", "dev_home", 60).unwrap();
        let claims = verify(SECRET, &token).unwrap();
        assert_eq!(claims.sub, "sess_42");
        assert_eq!(claims.dev, "dev_home");
        assert!(claims.exp > claims.iat);
        assert_eq!(claims.exp - claims.iat, 60);
    }

    #[test]
    fn different_secret_rejects() {
        let token = sign(SECRET, "s", "d", 60).unwrap();
        let other: &[u8] = b"a-totally-different-secret-at-least-32!";
        let err = verify(other, &token).unwrap_err();
        assert!(matches!(err, JwtError::Jwt(_)));
    }

    #[test]
    fn tampered_token_rejects() {
        let mut token = sign(SECRET, "s", "d", 60).unwrap();
        // Flip a byte in the middle (signature segment).
        let pos = token.len() - 5;
        let flipped = token.chars().nth(pos).unwrap();
        let replacement = if flipped == 'a' { 'b' } else { 'a' };
        let bytes: Vec<u8> = token
            .bytes()
            .enumerate()
            .map(|(i, b)| if i == pos { replacement as u8 } else { b })
            .collect();
        token = String::from_utf8(bytes).unwrap();
        assert!(verify(SECRET, &token).is_err());
    }

    #[test]
    fn expired_token_rejects() {
        // Hand-craft a token that expired an hour ago by setting ttl = 0
        // and then waiting zero time — jsonwebtoken's default validation
        // has a small clock skew allowance, so bump ttl negative via a
        // direct encode call instead.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = Claims {
            sub: "s".into(),
            dev: "d".into(),
            iat: now - 7200,
            exp: now - 3600,
        };
        let token = jsonwebtoken::encode(
            &Header::new(jsonwebtoken::Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap();
        assert!(verify(SECRET, &token).is_err());
    }

    #[test]
    fn empty_session_id_is_allowed() {
        // A "may-create" token carries an empty subject.
        let token = sign(SECRET, "", "dev_home", 60).unwrap();
        let claims = verify(SECRET, &token).unwrap();
        assert!(claims.sub.is_empty());
    }
}
