//! `Theme` and `ThemeName` — the canonical theme palette type.

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

use super::accents::Accents;
use super::agents;
use super::osc::Detection;
use super::shimmer;

/// Named built-in themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    Dark,
    Light,
    Monokai,
    Solarized,
    Custom,
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dark => write!(f, "dark"),
            Self::Light => write!(f, "light"),
            Self::Monokai => write!(f, "monokai"),
            Self::Solarized => write!(f, "solarized"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

pub(super) fn default_theme_name() -> ThemeName {
    ThemeName::Dark
}

/// Color theme for the TUI.
///
/// Defines all semantic colors used across the UI. Components reference
/// theme fields instead of hard-coding colors, making it easy to switch
/// between dark and light themes.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name identifier.
    pub name: ThemeName,

    // ─── General ───
    /// Default foreground.
    pub fg: Color,
    /// Default background.
    pub bg: Color,
    /// Muted/secondary text.
    pub muted: Color,

    // ─── Markdown ───
    /// Heading text color.
    pub heading: Color,
    /// Bold style modifier (combined with current fg).
    pub bold: Modifier,
    /// Italic style modifier.
    pub italic: Modifier,
    /// Inline code foreground.
    pub inline_code_fg: Color,
    /// Inline code background.
    pub inline_code_bg: Color,
    /// Link text color.
    pub link: Color,
    /// List bullet/number color.
    pub list_marker: Color,
    /// Block quote bar color.
    pub blockquote: Color,

    // ─── Diff ───
    /// Added line foreground.
    pub diff_add_fg: Color,
    /// Added line background.
    pub diff_add_bg: Color,
    /// Removed line foreground.
    pub diff_remove_fg: Color,
    /// Removed line background.
    pub diff_remove_bg: Color,
    /// Diff hunk header color.
    pub diff_hunk: Color,

    // ─── Syntax (fallback for non-syntect rendering) ───
    /// Keyword color.
    pub syntax_keyword: Color,
    /// String literal color.
    pub syntax_string: Color,
    /// Comment color.
    pub syntax_comment: Color,
    /// Function name color.
    pub syntax_function: Color,
    /// Type/class name color.
    pub syntax_type: Color,
    /// Number literal color.
    pub syntax_number: Color,

    // ─── UI chrome ───
    /// Status bar / border color.
    pub border: Color,
    /// Error text color.
    pub error: Color,
    /// Warning text color.
    pub warning: Color,
    /// Success text color.
    pub success: Color,
    /// Accent color — used for cyan-style UI highlights (frame chars,
    /// selection indicators, timestamps). Distinct from `link` which is
    /// specifically for clickable/URL targets.
    pub accent: Color,
    /// Bright foreground — emphasized text against a dark background.
    /// Distinct from `fg` (default body) and `muted` (de-emphasized).
    pub text_bright: Color,
    /// Dim foreground — lighter than `fg` but brighter than `muted`.
    /// Used for secondary text that should still be readable.
    pub text_dim: Color,
    /// Background for the "current" highlighted item (e.g. active search match).
    pub highlight_bg: Color,
    /// Foreground for the "current" highlighted item — paired with `highlight_bg`.
    pub highlight_fg: Color,
    /// Background for non-current selection / other matches.
    pub selection_bg: Color,
    /// Foreground for non-current selection — paired with `selection_bg`.
    pub selection_fg: Color,
}

impl Theme {
    /// Default dark theme (terminal-friendly 256-color palette).
    #[must_use]
    pub fn dark() -> Self {
        Self {
            name: ThemeName::Dark,
            fg: Color::White,
            bg: Color::Reset,
            muted: Color::DarkGray,

            heading: Color::Cyan,
            bold: Modifier::BOLD,
            italic: Modifier::ITALIC,
            inline_code_fg: Color::Yellow,
            inline_code_bg: Color::Reset,
            link: Color::Blue,
            list_marker: Color::DarkGray,
            blockquote: Color::DarkGray,

            diff_add_fg: Color::Green,
            diff_add_bg: Color::Reset,
            diff_remove_fg: Color::Red,
            diff_remove_bg: Color::Reset,
            diff_hunk: Color::Cyan,

            syntax_keyword: Color::Magenta,
            syntax_string: Color::Green,
            syntax_comment: Color::DarkGray,
            syntax_function: Color::Yellow,
            syntax_type: Color::Cyan,
            syntax_number: Color::LightRed,

            border: Color::DarkGray,
            error: Color::Red,
            warning: Color::Yellow,
            success: Color::Green,
            accent: Color::Cyan,
            text_bright: Color::White,
            text_dim: Color::Gray,
            highlight_bg: Color::Yellow,
            highlight_fg: Color::Black,
            selection_bg: Color::DarkGray,
            selection_fg: Color::White,
        }
    }

    /// Light theme for terminals with light backgrounds.
    #[must_use]
    pub fn light() -> Self {
        Self {
            name: ThemeName::Light,
            fg: Color::Black,
            bg: Color::Reset,
            muted: Color::Gray,

            heading: Color::DarkGray,
            bold: Modifier::BOLD,
            italic: Modifier::ITALIC,
            inline_code_fg: Color::Rgb(139, 0, 0),
            inline_code_bg: Color::Rgb(240, 240, 240),
            link: Color::Blue,
            list_marker: Color::Gray,
            blockquote: Color::Gray,

            diff_add_fg: Color::Rgb(0, 100, 0),
            diff_add_bg: Color::Rgb(230, 255, 230),
            diff_remove_fg: Color::Rgb(139, 0, 0),
            diff_remove_bg: Color::Rgb(255, 230, 230),
            diff_hunk: Color::Blue,

            syntax_keyword: Color::Rgb(128, 0, 128),
            syntax_string: Color::Rgb(0, 128, 0),
            syntax_comment: Color::Gray,
            syntax_function: Color::Rgb(0, 0, 139),
            syntax_type: Color::Rgb(0, 128, 128),
            syntax_number: Color::Rgb(255, 69, 0),

            border: Color::Gray,
            error: Color::Red,
            warning: Color::Rgb(204, 120, 0),
            success: Color::Rgb(0, 128, 0),
            accent: Color::Blue,
            text_bright: Color::Black,
            text_dim: Color::DarkGray,
            highlight_bg: Color::Rgb(255, 230, 128),
            highlight_fg: Color::Black,
            selection_bg: Color::Gray,
            selection_fg: Color::Black,
        }
    }

    /// Monokai-inspired theme with warm, vibrant colors.
    #[must_use]
    pub fn monokai() -> Self {
        Self {
            name: ThemeName::Monokai,
            fg: Color::Rgb(248, 248, 242),
            bg: Color::Rgb(39, 40, 34),
            muted: Color::Rgb(117, 113, 94),

            heading: Color::Rgb(102, 217, 239),
            bold: Modifier::BOLD,
            italic: Modifier::ITALIC,
            inline_code_fg: Color::Rgb(230, 219, 116),
            inline_code_bg: Color::Rgb(49, 50, 44),
            link: Color::Rgb(102, 217, 239),
            list_marker: Color::Rgb(117, 113, 94),
            blockquote: Color::Rgb(117, 113, 94),

            diff_add_fg: Color::Rgb(166, 226, 46),
            diff_add_bg: Color::Rgb(39, 40, 34),
            diff_remove_fg: Color::Rgb(249, 38, 114),
            diff_remove_bg: Color::Rgb(39, 40, 34),
            diff_hunk: Color::Rgb(102, 217, 239),

            syntax_keyword: Color::Rgb(249, 38, 114),
            syntax_string: Color::Rgb(230, 219, 116),
            syntax_comment: Color::Rgb(117, 113, 94),
            syntax_function: Color::Rgb(166, 226, 46),
            syntax_type: Color::Rgb(102, 217, 239),
            syntax_number: Color::Rgb(174, 129, 255),

            border: Color::Rgb(117, 113, 94),
            error: Color::Rgb(249, 38, 114),
            warning: Color::Rgb(230, 219, 116),
            success: Color::Rgb(166, 226, 46),
            accent: Color::Rgb(102, 217, 239),
            text_bright: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(181, 181, 166),
            highlight_bg: Color::Rgb(230, 219, 116),
            highlight_fg: Color::Rgb(39, 40, 34),
            selection_bg: Color::Rgb(73, 72, 62),
            selection_fg: Color::Rgb(248, 248, 242),
        }
    }

    /// Solarized Dark theme with carefully chosen contrast.
    #[must_use]
    pub fn solarized() -> Self {
        Self {
            name: ThemeName::Solarized,
            fg: Color::Rgb(131, 148, 150),
            bg: Color::Rgb(0, 43, 54),
            muted: Color::Rgb(88, 110, 117),

            heading: Color::Rgb(38, 139, 210),
            bold: Modifier::BOLD,
            italic: Modifier::ITALIC,
            inline_code_fg: Color::Rgb(203, 75, 22),
            inline_code_bg: Color::Rgb(7, 54, 66),
            link: Color::Rgb(38, 139, 210),
            list_marker: Color::Rgb(88, 110, 117),
            blockquote: Color::Rgb(88, 110, 117),

            diff_add_fg: Color::Rgb(133, 153, 0),
            diff_add_bg: Color::Rgb(0, 43, 54),
            diff_remove_fg: Color::Rgb(220, 50, 47),
            diff_remove_bg: Color::Rgb(0, 43, 54),
            diff_hunk: Color::Rgb(108, 113, 196),

            syntax_keyword: Color::Rgb(133, 153, 0),
            syntax_string: Color::Rgb(42, 161, 152),
            syntax_comment: Color::Rgb(88, 110, 117),
            syntax_function: Color::Rgb(38, 139, 210),
            syntax_type: Color::Rgb(181, 137, 0),
            syntax_number: Color::Rgb(203, 75, 22),

            border: Color::Rgb(88, 110, 117),
            error: Color::Rgb(220, 50, 47),
            warning: Color::Rgb(181, 137, 0),
            success: Color::Rgb(133, 153, 0),
            accent: Color::Rgb(38, 139, 210),
            text_bright: Color::Rgb(238, 232, 213),
            text_dim: Color::Rgb(147, 161, 161),
            highlight_bg: Color::Rgb(181, 137, 0),
            highlight_fg: Color::Rgb(0, 43, 54),
            selection_bg: Color::Rgb(7, 54, 66),
            selection_fg: Color::Rgb(238, 232, 213),
        }
    }

    /// Get a built-in theme by name.
    #[must_use]
    pub fn by_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Light => Self::light(),
            ThemeName::Monokai => Self::monokai(),
            ThemeName::Solarized => Self::solarized(),
            ThemeName::Dark | ThemeName::Custom => Self::dark(),
        }
    }

    /// Helper: create a ratatui `Style` from foreground color.
    #[must_use]
    pub fn style_fg(&self, fg: Color) -> Style {
        Style::default().fg(fg)
    }

    /// The 8-slot agent accent palette for this theme's brightness.
    #[must_use]
    pub fn agents_palette(&self) -> [Color; 8] {
        match self.name {
            ThemeName::Light => agents::AGENTS_PALETTE_LIGHT,
            _ => agents::AGENTS_PALETTE_DARK,
        }
    }

    /// Pick an agent color by slot index.
    #[must_use]
    pub fn agent_color(&self, index: usize) -> Color {
        agents::agent_color(&self.agents_palette(), index)
    }

    /// The reserved brand / status accents for this theme.
    #[must_use]
    pub fn accents(&self) -> Accents {
        match self.name {
            ThemeName::Light => Accents::light(),
            ThemeName::Monokai => Accents::monokai(),
            ThemeName::Solarized => Accents::solarized(),
            ThemeName::Dark | ThemeName::Custom => Accents::dark(),
        }
    }

    /// Shimmer color for a column within a span painted with `base`.
    #[must_use]
    pub fn shimmer_at(&self, base: Color, column: u16, width: u16, phase: f32) -> Color {
        shimmer::shimmer_at(base, column, width, phase)
    }

    /// Select a dark vs. light base theme from an OSC background probe.
    #[must_use]
    pub fn from_detection(detection: Detection) -> Self {
        match detection {
            Detection::Light => Self::light(),
            _ => Self::dark(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeRegistry;

    #[test]
    fn dark_theme_defaults() {
        let theme = Theme::dark();
        assert_eq!(theme.name, ThemeName::Dark);
        assert_eq!(theme.fg, Color::White);
        assert_eq!(theme.diff_add_fg, Color::Green);
        assert_eq!(theme.diff_remove_fg, Color::Red);
    }

    #[test]
    fn dark_theme_new_role_defaults_are_byte_identical_to_prior_literals() {
        let t = Theme::dark();
        assert_eq!(t.accent, Color::Cyan);
        assert_eq!(t.text_bright, Color::White);
        assert_eq!(t.text_dim, Color::Gray);
        assert_eq!(t.highlight_bg, Color::Yellow);
        assert_eq!(t.highlight_fg, Color::Black);
        assert_eq!(t.selection_bg, Color::DarkGray);
        assert_eq!(t.selection_fg, Color::White);
    }

    #[test]
    fn all_builtin_themes_populate_new_role_fields() {
        for name in ThemeRegistry::available_themes() {
            let t = Theme::by_name(name);
            assert_ne!(t.accent, Color::Reset, "{name:?} missing accent");
            assert_ne!(t.text_bright, Color::Reset, "{name:?} missing text_bright");
            assert_ne!(t.text_dim, Color::Reset, "{name:?} missing text_dim");
            assert_ne!(
                t.highlight_bg,
                Color::Reset,
                "{name:?} missing highlight_bg"
            );
            assert_ne!(
                t.highlight_fg,
                Color::Reset,
                "{name:?} missing highlight_fg"
            );
            assert_ne!(
                t.selection_bg,
                Color::Reset,
                "{name:?} missing selection_bg"
            );
            assert_ne!(
                t.selection_fg,
                Color::Reset,
                "{name:?} missing selection_fg"
            );
        }
    }

    #[test]
    fn light_theme_differs_from_dark() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert_ne!(dark.fg, light.fg);
        assert_eq!(light.name, ThemeName::Light);
    }

    #[test]
    fn monokai_theme() {
        let theme = Theme::monokai();
        assert_eq!(theme.name, ThemeName::Monokai);
        assert_eq!(theme.fg, Color::Rgb(248, 248, 242));
        assert_eq!(theme.syntax_keyword, Color::Rgb(249, 38, 114));
    }

    #[test]
    fn solarized_theme() {
        let theme = Theme::solarized();
        assert_eq!(theme.name, ThemeName::Solarized);
        assert_eq!(theme.bg, Color::Rgb(0, 43, 54));
        assert_eq!(theme.syntax_keyword, Color::Rgb(133, 153, 0));
    }

    #[test]
    fn default_is_dark() {
        let def = Theme::default();
        assert_eq!(def.fg, Color::White);
        assert_eq!(def.name, ThemeName::Dark);
    }

    #[test]
    fn style_fg_helper() {
        let theme = Theme::dark();
        let style = theme.style_fg(Color::Red);
        assert_eq!(style.fg, Some(Color::Red));
    }

    #[test]
    fn by_name_returns_correct_theme() {
        assert_eq!(Theme::by_name(ThemeName::Dark).name, ThemeName::Dark);
        assert_eq!(Theme::by_name(ThemeName::Light).name, ThemeName::Light);
        assert_eq!(Theme::by_name(ThemeName::Monokai).name, ThemeName::Monokai);
        assert_eq!(
            Theme::by_name(ThemeName::Solarized).name,
            ThemeName::Solarized
        );
        assert_eq!(Theme::by_name(ThemeName::Custom).fg, Color::White);
    }

    #[test]
    fn theme_name_display() {
        assert_eq!(ThemeName::Dark.to_string(), "dark");
        assert_eq!(ThemeName::Monokai.to_string(), "monokai");
        assert_eq!(ThemeName::Solarized.to_string(), "solarized");
        assert_eq!(ThemeName::Custom.to_string(), "custom");
    }

    #[test]
    fn theme_name_serde() {
        let json = serde_json::to_string(&ThemeName::Monokai).unwrap();
        assert_eq!(json, r#""monokai""#);
        let back: ThemeName = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ThemeName::Monokai);
    }
}
