//! PKCE (RFC 7636) code verifier + SHA-256 challenge generation.
//!
//! The verifier is a high-entropy random string (43–128 URL-safe characters);
//! the challenge is `BASE64URL(SHA256(verifier))` with no padding. Both are
//! sent to the authorisation server to prove possession of the verifier
//! during the token exchange step.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt as _;
use sha2::{Digest, Sha256};

/// Minimum verifier length per RFC 7636 §4.1.
const VERIFIER_MIN_LEN: usize = 43;
/// Maximum verifier length per RFC 7636 §4.1.
const VERIFIER_MAX_LEN: usize = 128;
/// Default verifier length (well above the minimum for extra entropy).
const VERIFIER_DEFAULT_LEN: usize = 64;

/// A PKCE code verifier + its derived S256 challenge.
///
/// The verifier is kept secret until the token-exchange step. The challenge
/// (and the literal string `"S256"`) travel with the initial authorisation
/// redirect.
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    verifier: String,
    challenge: String,
}

impl PkceChallenge {
    /// Generate a fresh `PkceChallenge` using the S256 method.
    ///
    /// Uses `rand::rng()` (OS entropy) to produce `VERIFIER_DEFAULT_LEN`
    /// URL-safe base64 characters, then derives `BASE64URL(SHA256(verifier))`
    /// without padding.
    #[must_use]
    pub fn new() -> Self {
        Self::with_len(VERIFIER_DEFAULT_LEN)
    }

    /// Generate a `PkceChallenge` with a specific verifier length.
    ///
    /// # Panics
    ///
    /// Panics if `len` is outside `[43, 128]` — those bounds come from
    /// RFC 7636 §4.1 and values outside them are non-compliant.
    #[must_use]
    pub fn with_len(len: usize) -> Self {
        assert!(
            (VERIFIER_MIN_LEN..=VERIFIER_MAX_LEN).contains(&len),
            "PKCE verifier length must be between {VERIFIER_MIN_LEN} and {VERIFIER_MAX_LEN} per RFC 7636 §4.1"
        );

        // Raw byte count that, once base64url-encoded, yields `len` chars.
        // BASE64URL without padding produces ceil(4 * n / 3) characters.
        let raw_bytes = len.div_ceil(4) * 3;
        let mut buf = vec![0u8; raw_bytes];
        let mut rng = rand::rng();
        rng.fill(&mut buf[..]);

        let mut verifier = URL_SAFE_NO_PAD.encode(&buf);
        verifier.truncate(len);

        let challenge_bytes = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);

        Self {
            verifier,
            challenge,
        }
    }

    /// The code verifier — keep secret until token exchange.
    #[must_use]
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// The code challenge — send with the initial authorisation redirect.
    #[must_use]
    pub fn challenge(&self) -> &str {
        &self.challenge
    }

    /// The PKCE challenge method string. Always `"S256"` — `"plain"` is not
    /// supported and considered insecure.
    #[must_use]
    pub fn method(&self) -> &'static str {
        "S256"
    }
}

impl Default for PkceChallenge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_length_in_spec_range() {
        let p = PkceChallenge::new();
        assert!(
            p.verifier().len() >= VERIFIER_MIN_LEN && p.verifier().len() <= VERIFIER_MAX_LEN,
            "verifier length {} out of RFC 7636 bounds",
            p.verifier().len()
        );
    }

    #[test]
    fn verifier_uses_url_safe_charset() {
        let p = PkceChallenge::new();
        for c in p.verifier().chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_',
                "verifier contains disallowed char: {c:?}"
            );
        }
    }

    #[test]
    fn challenge_is_sha256_of_verifier() {
        let p = PkceChallenge::new();
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(p.verifier().as_bytes()));
        assert_eq!(p.challenge(), expected);
    }

    #[test]
    fn method_is_s256() {
        let p = PkceChallenge::new();
        assert_eq!(p.method(), "S256");
    }

    #[test]
    fn two_challenges_are_unique() {
        let a = PkceChallenge::new();
        let b = PkceChallenge::new();
        assert_ne!(a.verifier(), b.verifier());
        assert_ne!(a.challenge(), b.challenge());
    }

    #[test]
    fn custom_length_honoured() {
        let p = PkceChallenge::with_len(43);
        assert_eq!(p.verifier().len(), 43);
        let p = PkceChallenge::with_len(128);
        assert_eq!(p.verifier().len(), 128);
    }

    #[test]
    #[should_panic(expected = "PKCE verifier length must be between")]
    fn rejects_too_short() {
        let _ = PkceChallenge::with_len(42);
    }

    #[test]
    #[should_panic(expected = "PKCE verifier length must be between")]
    fn rejects_too_long() {
        let _ = PkceChallenge::with_len(129);
    }

    #[test]
    fn known_vector_rfc_7636_appendix_b() {
        // RFC 7636 Appendix B test vector for S256 method.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        let actual = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(actual, expected_challenge);
    }
}
