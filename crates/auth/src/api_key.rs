/// Resolve an API key from environment variable or system keychain.
///
/// Checks `ANTHROPIC_API_KEY` env var first, then falls back to the system keychain.
///
/// # Errors
///
/// Returns `crab_common::Error::Auth` if no API key can be found.
pub fn resolve_api_key() -> crab_common::Result<String> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        return Ok(key);
    }

    crate::keychain::get("crab-code", "api-key").map_err(|_| {
        crab_common::Error::Auth(
            "no API key found: set ANTHROPIC_API_KEY or store in keychain".into(),
        )
    })
}
