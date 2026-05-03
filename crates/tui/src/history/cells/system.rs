//! System / informational cell — dim italic text with `⎿` glyph prefix.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

/// A system message (interrupt notice, meta info, etc.).
#[derive(Debug, Clone)]
pub struct SystemCell {
    text: String,
}

impl SystemCell {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl HistoryCell for SystemCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let glyph_style = Style::default().fg(Color::DarkGray);
        let text_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        vec![Line::from(vec![
            Span::styled("  \u{23bf}  ", glyph_style),
            Span::styled(self.text.clone(), text_style),
        ])]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_glyph_prefix_and_italic_gray_text() {
        let cell = SystemCell::new("note");
        let lines = cell.display_lines(80);
        let glyph: String = lines[0].spans[0].content.to_string();
        assert!(glyph.contains('\u{23bf}'));
        let style = lines[0].spans[1].style;
        assert_eq!(style.fg, Some(Color::DarkGray));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn search_text_matches_body() {
        let cell = SystemCell::new("something happened");
        assert!(cell.search_text().contains("something happened"));
    }
}
