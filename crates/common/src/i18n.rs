//! Internationalization (i18n) framework — locale management, message bundles,
//! and parameterized message formatting.

use std::collections::HashMap;
use std::sync::OnceLock;

// ── Locale ─────────────────────────────────────────────────────────────

/// Supported locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// English (default).
    En,
    /// Simplified Chinese.
    ZhCn,
    /// Traditional Chinese.
    ZhTw,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
    /// Spanish.
    Es,
    /// French.
    Fr,
    /// German.
    De,
}

impl Locale {
    /// Parse a locale string (e.g. `"en"`, `"zh-CN"`, `"zh_CN"`).
    #[must_use]
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            "en" | "en_us" | "en_gb" => Some(Self::En),
            "zh_cn" | "zh" => Some(Self::ZhCn),
            "zh_tw" | "zh_hant" => Some(Self::ZhTw),
            "ja" | "ja_jp" => Some(Self::Ja),
            "ko" | "ko_kr" => Some(Self::Ko),
            "es" | "es_es" => Some(Self::Es),
            "fr" | "fr_fr" => Some(Self::Fr),
            "de" | "de_de" => Some(Self::De),
            _ => None,
        }
    }

    /// BCP-47 style tag for this locale.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::De => "de",
        }
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.tag())
    }
}

// ── Config ─────────────────────────────────────────────────────────────

/// Configuration for the i18n system.
#[derive(Debug, Clone)]
pub struct I18nConfig {
    /// Primary locale.
    pub default_locale: Locale,
    /// Locale to try when a key is missing in the default locale.
    pub fallback_locale: Locale,
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            default_locale: Locale::En,
            fallback_locale: Locale::En,
        }
    }
}

// ── Bundle ─────────────────────────────────────────────────────────────

/// A collection of translated messages keyed by message ID.
///
/// Inner map: `message_key -> (locale -> translated_text)`.
#[derive(Debug, Clone, Default)]
pub struct I18nBundle {
    messages: HashMap<String, HashMap<Locale, String>>,
    config: I18nConfig,
}

impl I18nBundle {
    /// Create an empty bundle with the given config.
    #[must_use]
    pub fn new(config: I18nConfig) -> Self {
        Self {
            messages: HashMap::new(),
            config,
        }
    }

    /// Insert a single translation.
    pub fn insert(&mut self, key: impl Into<String>, locale: Locale, text: impl Into<String>) {
        self.messages
            .entry(key.into())
            .or_default()
            .insert(locale, text.into());
    }

    /// Insert translations for a key across multiple locales at once.
    pub fn insert_many(
        &mut self,
        key: impl Into<String>,
        translations: impl IntoIterator<Item = (Locale, String)>,
    ) {
        let entry = self.messages.entry(key.into()).or_default();
        for (locale, text) in translations {
            entry.insert(locale, text);
        }
    }

    /// Look up a message in the default locale, falling back as configured.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.get_for(key, self.config.default_locale)
    }

    /// Look up a message for a specific locale with fallback.
    #[must_use]
    pub fn get_for(&self, key: &str, locale: Locale) -> Option<&str> {
        let translations = self.messages.get(key)?;
        if let Some(text) = translations.get(&locale) {
            return Some(text.as_str());
        }
        if locale != self.config.fallback_locale
            && let Some(text) = translations.get(&self.config.fallback_locale)
        {
            return Some(text.as_str());
        }
        None
    }

    /// Format a message with named parameters: `{name}` placeholders.
    #[must_use]
    pub fn format(&self, key: &str, args: &[(&str, &str)]) -> Option<String> {
        self.format_for(key, self.config.default_locale, args)
    }

    /// Format a message for a specific locale with named parameters.
    #[must_use]
    pub fn format_for(&self, key: &str, locale: Locale, args: &[(&str, &str)]) -> Option<String> {
        let template = self.get_for(key, locale)?;
        Some(format_message(template, args))
    }

    /// Number of message keys in the bundle.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the bundle has no messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// All registered message keys.
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        self.messages.keys().map(String::as_str).collect()
    }

    /// Check if a key exists for the given locale (no fallback).
    #[must_use]
    pub fn has_translation(&self, key: &str, locale: Locale) -> bool {
        self.messages
            .get(key)
            .is_some_and(|m| m.contains_key(&locale))
    }

    /// Reference to the config.
    #[must_use]
    pub fn config(&self) -> &I18nConfig {
        &self.config
    }

    /// Set a new default locale.
    pub fn set_locale(&mut self, locale: Locale) {
        self.config.default_locale = locale;
    }
}

// ── Format helper ──────────────────────────────────────────────────────

/// Replace `{name}` placeholders in `template` with values from `args`.
#[must_use]
pub fn format_message(template: &str, args: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for &(name, value) in args {
        let placeholder = format!("{{{name}}}");
        result = result.replace(&placeholder, value);
    }
    result
}

// ── Global bundle (optional convenience) ───────────────────────────────

static GLOBAL_BUNDLE: OnceLock<I18nBundle> = OnceLock::new();

/// Initialize the global i18n bundle. Can only be called once.
///
/// Returns `Err` with the bundle back if already initialized.
pub fn init_global(bundle: I18nBundle) -> Result<(), I18nBundle> {
    GLOBAL_BUNDLE.set(bundle)
}

/// Get a reference to the global bundle, if initialized.
#[must_use]
pub fn global_bundle() -> Option<&'static I18nBundle> {
    GLOBAL_BUNDLE.get()
}

/// Translate a key using the global bundle.
///
/// Returns the key itself if the bundle is not initialized or the key is missing.
#[must_use]
pub fn translate(key: &str) -> String {
    global_bundle()
        .and_then(|b| b.get(key))
        .map_or_else(|| key.to_string(), String::from)
}

/// Translate a key with arguments using the global bundle.
#[must_use]
pub fn translate_fmt(key: &str, args: &[(&str, &str)]) -> String {
    global_bundle()
        .and_then(|b| b.format(key, args))
        .unwrap_or_else(|| key.to_string())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundle() -> I18nBundle {
        let mut bundle = I18nBundle::new(I18nConfig::default());
        bundle.insert("greeting", Locale::En, "Hello");
        bundle.insert("greeting", Locale::ZhCn, "你好");
        bundle.insert("greeting", Locale::Ja, "こんにちは");
        bundle.insert(
            "welcome",
            Locale::En,
            "Welcome, {name}! You have {count} messages.",
        );
        bundle.insert(
            "welcome",
            Locale::ZhCn,
            "{name}，欢迎！你有 {count} 条消息。",
        );
        bundle
    }

    // ── Locale ─────────────────────────────────────────────────────────

    #[test]
    fn locale_from_str_loose() {
        assert_eq!(Locale::from_str_loose("en"), Some(Locale::En));
        assert_eq!(Locale::from_str_loose("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_str_loose("zh_cn"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_str_loose("zh-TW"), Some(Locale::ZhTw));
        assert_eq!(Locale::from_str_loose("ja"), Some(Locale::Ja));
        assert_eq!(Locale::from_str_loose("ko"), Some(Locale::Ko));
        assert_eq!(Locale::from_str_loose("es"), Some(Locale::Es));
        assert_eq!(Locale::from_str_loose("fr"), Some(Locale::Fr));
        assert_eq!(Locale::from_str_loose("de"), Some(Locale::De));
        assert_eq!(Locale::from_str_loose("xx"), None);
    }

    #[test]
    fn locale_tag() {
        assert_eq!(Locale::En.tag(), "en");
        assert_eq!(Locale::ZhCn.tag(), "zh-CN");
        assert_eq!(Locale::ZhTw.tag(), "zh-TW");
    }

    #[test]
    fn locale_display() {
        assert_eq!(Locale::En.to_string(), "en");
        assert_eq!(Locale::ZhCn.to_string(), "zh-CN");
    }

    // ── I18nConfig ─────────────────────────────────────────────────────

    #[test]
    fn default_config() {
        let config = I18nConfig::default();
        assert_eq!(config.default_locale, Locale::En);
        assert_eq!(config.fallback_locale, Locale::En);
    }

    // ── I18nBundle ─────────────────────────────────────────────────────

    #[test]
    fn bundle_insert_and_get() {
        let bundle = sample_bundle();
        assert_eq!(bundle.get("greeting"), Some("Hello"));
    }

    #[test]
    fn bundle_get_for_locale() {
        let bundle = sample_bundle();
        assert_eq!(bundle.get_for("greeting", Locale::ZhCn), Some("你好"));
        assert_eq!(bundle.get_for("greeting", Locale::Ja), Some("こんにちは"));
    }

    #[test]
    fn bundle_fallback_to_default() {
        let bundle = sample_bundle();
        // Korean not added, falls back to En
        assert_eq!(bundle.get_for("greeting", Locale::Ko), Some("Hello"));
    }

    #[test]
    fn bundle_missing_key_returns_none() {
        let bundle = sample_bundle();
        assert_eq!(bundle.get("nonexistent"), None);
    }

    #[test]
    fn bundle_format() {
        let bundle = sample_bundle();
        let result = bundle.format("welcome", &[("name", "Alice"), ("count", "3")]);
        assert_eq!(
            result,
            Some("Welcome, Alice! You have 3 messages.".to_string())
        );
    }

    #[test]
    fn bundle_format_for_locale() {
        let bundle = sample_bundle();
        let result = bundle.format_for(
            "welcome",
            Locale::ZhCn,
            &[("name", "Alice"), ("count", "3")],
        );
        assert_eq!(result, Some("Alice，欢迎！你有 3 条消息。".to_string()));
    }

    #[test]
    fn bundle_format_missing_key() {
        let bundle = sample_bundle();
        assert_eq!(bundle.format("nope", &[]), None);
    }

    #[test]
    fn bundle_len_and_is_empty() {
        let bundle = sample_bundle();
        assert_eq!(bundle.len(), 2);
        assert!(!bundle.is_empty());

        let empty = I18nBundle::new(I18nConfig::default());
        assert!(empty.is_empty());
    }

    #[test]
    fn bundle_keys() {
        let bundle = sample_bundle();
        let mut keys = bundle.keys();
        keys.sort();
        assert_eq!(keys, vec!["greeting", "welcome"]);
    }

    #[test]
    fn bundle_has_translation() {
        let bundle = sample_bundle();
        assert!(bundle.has_translation("greeting", Locale::En));
        assert!(bundle.has_translation("greeting", Locale::ZhCn));
        assert!(!bundle.has_translation("greeting", Locale::Ko));
    }

    #[test]
    fn bundle_set_locale() {
        let mut bundle = sample_bundle();
        bundle.set_locale(Locale::ZhCn);
        assert_eq!(bundle.get("greeting"), Some("你好"));
    }

    #[test]
    fn bundle_insert_many() {
        let mut bundle = I18nBundle::new(I18nConfig::default());
        bundle.insert_many(
            "ok",
            [
                (Locale::En, "OK".to_string()),
                (Locale::ZhCn, "好的".to_string()),
            ],
        );
        assert_eq!(bundle.get("ok"), Some("OK"));
        assert_eq!(bundle.get_for("ok", Locale::ZhCn), Some("好的"));
    }

    // ── format_message ─────────────────────────────────────────────────

    #[test]
    fn format_message_basic() {
        let result = format_message("Hello, {name}!", &[("name", "World")]);
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn format_message_no_args() {
        let result = format_message("No params here", &[]);
        assert_eq!(result, "No params here");
    }

    #[test]
    fn format_message_missing_placeholder() {
        let result = format_message("{missing} stays", &[]);
        assert_eq!(result, "{missing} stays");
    }

    #[test]
    fn format_message_multiple() {
        let result = format_message("{a} and {b}", &[("a", "first"), ("b", "second")]);
        assert_eq!(result, "first and second");
    }

    // ── translate (global) ─────────────────────────────────────────────

    #[test]
    fn translate_without_global_returns_key() {
        // Global may or may not be initialized by other tests,
        // but a missing key always returns the key itself.
        let result = translate("__nonexistent_test_key__");
        assert_eq!(result, "__nonexistent_test_key__");
    }
}
