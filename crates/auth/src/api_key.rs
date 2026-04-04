pub fn resolve_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .or_else(|| crate::keychain::get("crab-code", "api-key").ok())
}
