//! System / informational cell — dim italic text with `⎿` glyph prefix,
//! with optional severity level (Info/Warning/Error).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

/// Severity level for a system message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SystemKind {
    #[default]
    Info,
    Warning,
    Error,
}

/// A system message (interrupt notice, meta info, etc.).
#[derive(Debug, Clone)]
pub struct SystemCell {
    text: String,
    kind: SystemKind,
}

impl SystemCell {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SystemKind::Info,
        }
    }

    #[must_use]
    pub fn with_kind(mut self, kind: SystemKind) -> Self {
        self.kind = kind;
        self
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl HistoryCell for SystemCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        match self.kind {
            SystemKind::Info => {
                let glyph_style = Style::default().fg(Color::DarkGray);
                let text_style = Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC);
                vec![Line::from(vec![
                    Span::styled("  \u{23bf}  ", glyph_style),
                    Span::styled(self.text.clone(), text_style),
                ])]
            }
            SystemKind::Warning => {
                let color = Color::Yellow;
                vec![Line::from(vec![
                    Span::styled("● ", Style::default().fg(color)),
                    Span::styled(self.text.clone(), Style::default().fg(color)),
                ])]
            }
            SystemKind::Error => {
                let color = Color::Red;
                vec![Line::from(vec![
                    Span::styled("● ", Style::default().fg(color)),
                    Span::styled(self.text.clone(), Style::default().fg(color)),
                ])]
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_has_glyph_prefix_and_italic_gray_text() {
        let cell = SystemCell::new("note");
        let lines = cell.display_lines(80);
        let glyph: String = lines[0].spans[0].content.to_string();
        assert!(glyph.contains('\u{23bf}'));
        let style = lines[0].spans[1].style;
        assert_eq!(style.fg, Some(Color::DarkGray));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn warning_uses_bullet_and_yellow() {
        let cell = SystemCell::new("watch out").with_kind(SystemKind::Warning);
        let lines = cell.display_lines(80);
        assert!(lines[0].spans[0].content.contains('●'));
        assert_eq!(lines[0].spans[1].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn error_uses_bullet_and_red() {
        let cell = SystemCell::new("bad").with_kind(SystemKind::Error);
        let lines = cell.display_lines(80);
        assert!(lines[0].spans[0].content.contains('●'));
        assert_eq!(lines[0].spans[1].style.fg, Some(Color::Red));
    }

    #[test]
    fn search_text_matches_body() {
        let cell = SystemCell::new("something happened");
        assert!(cell.search_text().contains("something happened"));
    }
}
