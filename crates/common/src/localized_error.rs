//! Localized error messages — error catalog with multi-language support.

use crate::i18n::{Locale, format_message};
use std::collections::HashMap;

// ── Localized error ────────────────────────────────────────────────────

/// An error with an error code and a locale-resolved message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedError {
    /// Machine-readable error code (e.g. "E1001").
    pub code: String,
    /// Human-readable message in the resolved locale.
    pub message: String,
}

impl std::fmt::Display for LocalizedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for LocalizedError {}

// ── Error catalog ──────────────────────────────────────────────────────

/// Registry mapping error codes to multi-language message templates.
#[derive(Debug, Clone, Default)]
pub struct ErrorCatalog {
    /// `error_code -> (locale -> message_template)`.
    entries: HashMap<String, HashMap<Locale, String>>,
}

impl ErrorCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or update translations for an error code.
    pub fn register(
        &mut self,
        code: impl Into<String>,
        translations: impl IntoIterator<Item = (Locale, String)>,
    ) {
        let entry = self.entries.entry(code.into()).or_default();
        for (locale, text) in translations {
            entry.insert(locale, text);
        }
    }

    /// Register a single translation for an error code.
    pub fn register_one(
        &mut self,
        code: impl Into<String>,
        locale: Locale,
        text: impl Into<String>,
    ) {
        self.entries
            .entry(code.into())
            .or_default()
            .insert(locale, text.into());
    }

    /// Get the raw template for an error code and locale.
    ///
    /// Falls back to English if the requested locale is not available.
    #[must_use]
    pub fn get_template(&self, code: &str, locale: Locale) -> Option<&str> {
        let translations = self.entries.get(code)?;
        if let Some(text) = translations.get(&locale) {
            return Some(text.as_str());
        }
        // Fallback to English
        translations.get(&Locale::En).map(String::as_str)
    }

    /// Localize an error code into a [`LocalizedError`] with parameter substitution.
    #[must_use]
    pub fn localize(&self, code: &str, locale: Locale, args: &[(&str, &str)]) -> LocalizedError {
        let message = self.get_template(code, locale).map_or_else(
            || format!("Unknown error: {code}"),
            |tmpl| format_message(tmpl, args),
        );

        LocalizedError {
            code: code.to_string(),
            message,
        }
    }

    /// Localize without parameters.
    #[must_use]
    pub fn localize_simple(&self, code: &str, locale: Locale) -> LocalizedError {
        self.localize(code, locale, &[])
    }

    /// Localize a `crab_common::Error` into a human-readable string.
    #[must_use]
    pub fn localize_error(&self, error: &crate::Error, locale: Locale) -> String {
        let code = error_to_code(error);
        let detail = error.to_string();
        let err = self.localize(code, locale, &[("detail", &detail)]);
        err.message
    }

    /// Whether a code is registered.
    #[must_use]
    pub fn contains(&self, code: &str) -> bool {
        self.entries.contains_key(code)
    }

    /// Number of registered error codes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the catalog is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All registered error codes.
    #[must_use]
    pub fn codes(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }
}

// ── Built-in error mapping ─────────────────────────────────────────────

/// Map a `crab_common::Error` variant to a canonical error code.
#[must_use]
pub fn error_to_code(error: &crate::Error) -> &'static str {
    match error {
        crate::Error::Io(_) => "E1001",
        crate::Error::Config(_) => "E2001",
        crate::Error::Api(_) => "E3001",
        crate::Error::Auth(_) => "E4001",
        crate::Error::Tool(_) => "E5001",
        crate::Error::Permission(_) => "E6001",
        crate::Error::Other(_) => "E9999",
    }
}

/// Create a catalog pre-populated with core error translations (En + `ZhCn`).
#[must_use]
pub fn default_catalog() -> ErrorCatalog {
    let mut cat = ErrorCatalog::new();

    cat.register(
        "E1001",
        [
            (Locale::En, "I/O error: {detail}".to_string()),
            (Locale::ZhCn, "I/O 错误：{detail}".to_string()),
        ],
    );
    cat.register(
        "E2001",
        [
            (Locale::En, "Configuration error: {detail}".to_string()),
            (Locale::ZhCn, "配置错误：{detail}".to_string()),
        ],
    );
    cat.register(
        "E3001",
        [
            (Locale::En, "API error: {detail}".to_string()),
            (Locale::ZhCn, "API 错误：{detail}".to_string()),
        ],
    );
    cat.register(
        "E4001",
        [
            (Locale::En, "Authentication error: {detail}".to_string()),
            (Locale::ZhCn, "认证错误：{detail}".to_string()),
        ],
    );
    cat.register(
        "E5001",
        [
            (Locale::En, "Tool execution error: {detail}".to_string()),
            (Locale::ZhCn, "工具执行错误：{detail}".to_string()),
        ],
    );
    cat.register(
        "E6001",
        [
            (Locale::En, "Permission denied: {detail}".to_string()),
            (Locale::ZhCn, "权限不足：{detail}".to_string()),
        ],
    );
    cat.register(
        "E9999",
        [
            (Locale::En, "Unexpected error: {detail}".to_string()),
            (Locale::ZhCn, "未知错误：{detail}".to_string()),
        ],
    );

    cat
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_catalog() -> ErrorCatalog {
        let mut cat = ErrorCatalog::new();
        cat.register(
            "E0001",
            [
                (Locale::En, "File not found: {path}".to_string()),
                (Locale::ZhCn, "文件未找到：{path}".to_string()),
            ],
        );
        cat
    }

    // ── LocalizedError ─────────────────────────────────────────────────

    #[test]
    fn localized_error_display() {
        let err = LocalizedError {
            code: "E0001".into(),
            message: "File not found".into(),
        };
        assert_eq!(err.to_string(), "[E0001] File not found");
    }

    #[test]
    fn localized_error_is_error_trait() {
        let err = LocalizedError {
            code: "E0001".into(),
            message: "test".into(),
        };
        let _: &dyn std::error::Error = &err;
    }

    // ── ErrorCatalog ───────────────────────────────────────────────────

    #[test]
    fn catalog_new_is_empty() {
        let cat = ErrorCatalog::new();
        assert!(cat.is_empty());
        assert_eq!(cat.len(), 0);
    }

    #[test]
    fn catalog_register_and_get_template() {
        let cat = test_catalog();
        assert_eq!(
            cat.get_template("E0001", Locale::En),
            Some("File not found: {path}")
        );
        assert_eq!(
            cat.get_template("E0001", Locale::ZhCn),
            Some("文件未找到：{path}")
        );
    }

    #[test]
    fn catalog_fallback_to_english() {
        let cat = test_catalog();
        // Japanese not registered, falls back to En
        assert_eq!(
            cat.get_template("E0001", Locale::Ja),
            Some("File not found: {path}")
        );
    }

    #[test]
    fn catalog_missing_code_returns_none() {
        let cat = test_catalog();
        assert_eq!(cat.get_template("E9999", Locale::En), None);
    }

    #[test]
    fn catalog_localize_with_args() {
        let cat = test_catalog();
        let err = cat.localize("E0001", Locale::En, &[("path", "/tmp/x")]);
        assert_eq!(err.code, "E0001");
        assert_eq!(err.message, "File not found: /tmp/x");
    }

    #[test]
    fn catalog_localize_zh_cn() {
        let cat = test_catalog();
        let err = cat.localize("E0001", Locale::ZhCn, &[("path", "/tmp/x")]);
        assert_eq!(err.message, "文件未找到：/tmp/x");
    }

    #[test]
    fn catalog_localize_unknown_code() {
        let cat = test_catalog();
        let err = cat.localize_simple("E9999", Locale::En);
        assert_eq!(err.message, "Unknown error: E9999");
    }

    #[test]
    fn catalog_localize_simple() {
        let mut cat = ErrorCatalog::new();
        cat.register_one("E0002", Locale::En, "Something broke");
        let err = cat.localize_simple("E0002", Locale::En);
        assert_eq!(err.message, "Something broke");
    }

    #[test]
    fn catalog_contains() {
        let cat = test_catalog();
        assert!(cat.contains("E0001"));
        assert!(!cat.contains("E0002"));
    }

    #[test]
    fn catalog_codes() {
        let cat = test_catalog();
        assert_eq!(cat.codes(), vec!["E0001"]);
    }

    #[test]
    fn catalog_len() {
        let cat = test_catalog();
        assert_eq!(cat.len(), 1);
        assert!(!cat.is_empty());
    }

    // ── error_to_code ──────────────────────────────────────────────────

    #[test]
    fn error_to_code_mapping() {
        assert_eq!(error_to_code(&crate::Error::Config("x".into())), "E2001");
        assert_eq!(error_to_code(&crate::Error::Api("x".into())), "E3001");
        assert_eq!(error_to_code(&crate::Error::Auth("x".into())), "E4001");
        assert_eq!(error_to_code(&crate::Error::Tool("x".into())), "E5001");
        assert_eq!(
            error_to_code(&crate::Error::Permission("x".into())),
            "E6001"
        );
        assert_eq!(error_to_code(&crate::Error::Other("x".into())), "E9999");
    }

    #[test]
    fn error_to_code_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        assert_eq!(error_to_code(&crate::Error::Io(io_err)), "E1001");
    }

    // ── default_catalog ────────────────────────────────────────────────

    #[test]
    fn default_catalog_has_core_errors() {
        let cat = default_catalog();
        assert_eq!(cat.len(), 7);
        assert!(cat.contains("E1001"));
        assert!(cat.contains("E2001"));
        assert!(cat.contains("E3001"));
        assert!(cat.contains("E4001"));
        assert!(cat.contains("E5001"));
        assert!(cat.contains("E6001"));
        assert!(cat.contains("E9999"));
    }

    #[test]
    fn default_catalog_localize_error_en() {
        let cat = default_catalog();
        let err = crate::Error::Config("bad toml".into());
        let msg = cat.localize_error(&err, Locale::En);
        assert_eq!(msg, "Configuration error: config error: bad toml");
    }

    #[test]
    fn default_catalog_localize_error_zh() {
        let cat = default_catalog();
        let err = crate::Error::Auth("token expired".into());
        let msg = cat.localize_error(&err, Locale::ZhCn);
        assert_eq!(msg, "认证错误：auth error: token expired");
    }

    #[test]
    fn register_one_works() {
        let mut cat = ErrorCatalog::new();
        cat.register_one("E0010", Locale::En, "Oops");
        cat.register_one("E0010", Locale::Fr, "Oups");
        assert_eq!(cat.get_template("E0010", Locale::En), Some("Oops"));
        assert_eq!(cat.get_template("E0010", Locale::Fr), Some("Oups"));
    }
}
