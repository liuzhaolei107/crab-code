use std::sync::OnceLock;

static CAPS: OnceLock<TerminalCapabilities> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct TerminalCapabilities {
    pub truecolor: bool,
    pub kitty_keyboard: bool,
    pub sixel: bool,
    pub osc8_hyperlinks: bool,
    pub osc52_clipboard: bool,
    pub bracketed_paste: bool,
    pub focus_events: bool,
    pub unicode_width: UnicodeSupport,
    pub term_program: String,
    pub term: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeSupport {
    Full,
    Basic,
    Ascii,
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        Self::detect()
    }
}

impl TerminalCapabilities {
    #[must_use]
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        let tp_lower = term_program.to_lowercase();

        let truecolor = colorterm == "truecolor"
            || colorterm == "24bit"
            || tp_lower.contains("iterm")
            || tp_lower.contains("kitty")
            || tp_lower.contains("wezterm");

        let kitty_keyboard =
            tp_lower.contains("kitty") || tp_lower.contains("wezterm") || tp_lower.contains("foot");

        let sixel = tp_lower.contains("foot")
            || tp_lower.contains("mlterm")
            || tp_lower.contains("wezterm");

        let osc8_hyperlinks = tp_lower.contains("iterm")
            || tp_lower.contains("kitty")
            || tp_lower.contains("wezterm")
            || term.contains("xterm");

        let osc52_clipboard = tp_lower.contains("iterm")
            || tp_lower.contains("kitty")
            || tp_lower.contains("wezterm")
            || tp_lower.contains("tmux");

        let unicode_width = if std::env::var("LANG").unwrap_or_default().contains("UTF-8")
            || std::env::var("LC_ALL")
                .unwrap_or_default()
                .contains("UTF-8")
        {
            UnicodeSupport::Full
        } else {
            UnicodeSupport::Basic
        };

        Self {
            truecolor,
            kitty_keyboard,
            sixel,
            osc8_hyperlinks,
            osc52_clipboard,
            bracketed_paste: true,
            focus_events: true,
            unicode_width,
            term_program,
            term,
        }
    }

    pub fn init() {
        CAPS.get_or_init(Self::detect);
    }

    #[must_use]
    pub fn global() -> &'static Self {
        CAPS.get_or_init(Self::detect)
    }

    #[must_use]
    pub fn summary(&self) -> Vec<(&'static str, bool)> {
        vec![
            ("Truecolor", self.truecolor),
            ("Kitty keyboard", self.kitty_keyboard),
            ("Sixel", self.sixel),
            ("OSC 8 hyperlinks", self.osc8_hyperlinks),
            ("OSC 52 clipboard", self.osc52_clipboard),
            ("Bracketed paste", self.bracketed_paste),
            ("Focus events", self.focus_events),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_struct() {
        let caps = TerminalCapabilities::detect();
        assert!(caps.bracketed_paste);
        assert!(caps.focus_events);
    }

    #[test]
    fn summary_has_entries() {
        let caps = TerminalCapabilities::detect();
        let summary = caps.summary();
        assert!(summary.len() >= 5);
    }

    #[test]
    fn global_returns_same_instance() {
        let a = TerminalCapabilities::global();
        let b = TerminalCapabilities::global();
        assert_eq!(a.term, b.term);
    }
}
