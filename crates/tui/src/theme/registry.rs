//! `ThemeRegistry` — runtime-switchable theme container.

use std::path::Path;

use super::config::{ThemeConfig, load_theme_config};
use super::theme::{Theme, ThemeName};

/// Registry that manages the active theme and allows runtime switching.
pub struct ThemeRegistry {
    current: Theme,
}

impl ThemeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: Theme::dark(),
        }
    }

    /// Create with a specific built-in theme.
    #[must_use]
    pub fn with_theme(name: ThemeName) -> Self {
        Self {
            current: Theme::by_name(name),
        }
    }

    /// Get a reference to the current theme.
    #[must_use]
    pub fn current(&self) -> &Theme {
        &self.current
    }

    /// Switch to a built-in theme by name.
    pub fn set_theme(&mut self, name: ThemeName) {
        self.current = Theme::by_name(name);
    }

    /// Apply a custom theme from a `ThemeConfig`.
    pub fn set_custom(&mut self, config: ThemeConfig) {
        self.current = config.into_theme();
    }

    /// Try to load a custom theme from a file path.
    /// Returns `true` if successfully loaded.
    pub fn load_from_file(&mut self, path: &Path) -> bool {
        if let Some(config) = load_theme_config(path) {
            self.current = config.into_theme();
            true
        } else {
            false
        }
    }

    /// List all available built-in theme names.
    #[must_use]
    pub fn available_themes() -> Vec<ThemeName> {
        vec![
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::Monokai,
            ThemeName::Solarized,
        ]
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use std::collections::HashMap;

    #[test]
    fn registry_default_is_dark() {
        let reg = ThemeRegistry::new();
        assert_eq!(reg.current().name, ThemeName::Dark);
    }

    #[test]
    fn registry_with_theme() {
        let reg = ThemeRegistry::with_theme(ThemeName::Monokai);
        assert_eq!(reg.current().name, ThemeName::Monokai);
    }

    #[test]
    fn registry_set_theme() {
        let mut reg = ThemeRegistry::new();
        reg.set_theme(ThemeName::Solarized);
        assert_eq!(reg.current().name, ThemeName::Solarized);
    }

    #[test]
    fn registry_set_custom() {
        let mut reg = ThemeRegistry::new();
        let mut colors = HashMap::new();
        colors.insert("fg".into(), "#ff0000".into());
        reg.set_custom(ThemeConfig {
            name: ThemeName::Dark,
            colors,
        });
        assert_eq!(reg.current().name, ThemeName::Custom);
        assert_eq!(reg.current().fg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn registry_load_nonexistent_file() {
        let mut reg = ThemeRegistry::new();
        assert!(!reg.load_from_file(Path::new("/no/such/file.json")));
        assert_eq!(reg.current().name, ThemeName::Dark);
    }

    #[test]
    fn registry_available_themes() {
        let themes = ThemeRegistry::available_themes();
        assert_eq!(themes.len(), 4);
        assert!(themes.contains(&ThemeName::Dark));
        assert!(themes.contains(&ThemeName::Monokai));
    }
}
