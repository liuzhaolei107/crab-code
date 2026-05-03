//! `ThemeConfig` — the JSON-loadable theme override format.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::parse::parse_color;
use super::theme::{Theme, ThemeName, default_theme_name};

/// JSON-serializable theme configuration with string-based colors.
/// This is the format used for `~/.crab/theme.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_name")]
    pub name: ThemeName,

    #[serde(default)]
    pub colors: HashMap<String, String>,
}

impl ThemeConfig {
    /// Convert into a `Theme`, starting from the named base theme
    /// and overriding any colors specified in the `colors` map.
    #[must_use]
    pub fn into_theme(self) -> Theme {
        let mut theme = Theme::by_name(self.name);
        theme.name = ThemeName::Custom;

        for (key, value) in &self.colors {
            if let Some(color) = parse_color(value) {
                match key.as_str() {
                    "fg" => theme.fg = color,
                    "bg" => theme.bg = color,
                    "muted" => theme.muted = color,
                    "heading" => theme.heading = color,
                    "inline_code_fg" => theme.inline_code_fg = color,
                    "inline_code_bg" => theme.inline_code_bg = color,
                    "link" => theme.link = color,
                    "list_marker" => theme.list_marker = color,
                    "blockquote" => theme.blockquote = color,
                    "diff_add_fg" => theme.diff_add_fg = color,
                    "diff_add_bg" => theme.diff_add_bg = color,
                    "diff_remove_fg" => theme.diff_remove_fg = color,
                    "diff_remove_bg" => theme.diff_remove_bg = color,
                    "diff_hunk" => theme.diff_hunk = color,
                    "syntax_keyword" => theme.syntax_keyword = color,
                    "syntax_string" => theme.syntax_string = color,
                    "syntax_comment" => theme.syntax_comment = color,
                    "syntax_function" => theme.syntax_function = color,
                    "syntax_type" => theme.syntax_type = color,
                    "syntax_number" => theme.syntax_number = color,
                    "border" => theme.border = color,
                    "error" => theme.error = color,
                    "warning" => theme.warning = color,
                    "success" => theme.success = color,
                    "accent" => theme.accent = color,
                    "text_bright" => theme.text_bright = color,
                    "text_dim" => theme.text_dim = color,
                    "highlight_bg" => theme.highlight_bg = color,
                    "highlight_fg" => theme.highlight_fg = color,
                    "selection_bg" => theme.selection_bg = color,
                    "selection_fg" => theme.selection_fg = color,
                    _ => {}
                }
            }
        }

        theme
    }
}

/// Load a custom theme from a JSON file.
/// Returns `None` if the file doesn't exist or can't be parsed.
#[must_use]
pub fn load_theme_config(path: &Path) -> Option<ThemeConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn theme_config_overrides_new_role_fields() {
        let mut colors = HashMap::new();
        colors.insert("accent".into(), "#123456".into());
        colors.insert("highlight_bg".into(), "red".into());
        let config = ThemeConfig {
            name: ThemeName::Dark,
            colors,
        };
        let theme = config.into_theme();
        assert_eq!(theme.accent, Color::Rgb(0x12, 0x34, 0x56));
        assert_eq!(theme.highlight_bg, Color::Red);
        // Unchanged field stays at dark default.
        assert_eq!(theme.highlight_fg, Color::Black);
    }

    #[test]
    fn theme_config_into_theme_default() {
        let config = ThemeConfig {
            name: ThemeName::Dark,
            colors: HashMap::new(),
        };
        let theme = config.into_theme();
        assert_eq!(theme.name, ThemeName::Custom);
        assert_eq!(theme.fg, Color::White);
    }

    #[test]
    fn theme_config_overrides_colors() {
        let mut colors = HashMap::new();
        colors.insert("fg".into(), "#ff0000".into());
        colors.insert("error".into(), "blue".into());
        let config = ThemeConfig {
            name: ThemeName::Dark,
            colors,
        };
        let theme = config.into_theme();
        assert_eq!(theme.fg, Color::Rgb(255, 0, 0));
        assert_eq!(theme.error, Color::Blue);
        assert_eq!(theme.bg, Color::Reset);
    }

    #[test]
    fn theme_config_based_on_light() {
        let config = ThemeConfig {
            name: ThemeName::Light,
            colors: HashMap::new(),
        };
        let theme = config.into_theme();
        assert_eq!(theme.fg, Color::Black);
    }

    #[test]
    fn theme_config_json_roundtrip() {
        let mut colors = HashMap::new();
        colors.insert("fg".into(), "#ffffff".into());
        let config = ThemeConfig {
            name: ThemeName::Monokai,
            colors,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: ThemeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, ThemeName::Monokai);
        assert_eq!(back.colors.get("fg").unwrap(), "#ffffff");
    }

    #[test]
    fn theme_config_from_json_string() {
        let json = r##"{
            "name": "solarized",
            "colors": {
                "fg": "rgb(200, 200, 200)",
                "bg": "#002b36",
                "error": "red"
            }
        }"##;
        let config: ThemeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, ThemeName::Solarized);
        let theme = config.into_theme();
        assert_eq!(theme.fg, Color::Rgb(200, 200, 200));
        assert_eq!(theme.bg, Color::Rgb(0, 43, 54));
        assert_eq!(theme.error, Color::Red);
    }

    #[test]
    fn load_theme_config_missing_file() {
        let result = load_theme_config(Path::new("/nonexistent/theme.json"));
        assert!(result.is_none());
    }
}
