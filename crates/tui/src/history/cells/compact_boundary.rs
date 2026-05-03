use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

#[derive(Debug, Clone)]
pub struct CompactBoundaryCell {
    #[allow(dead_code)]
    strategy: String,
    #[allow(dead_code)]
    after_tokens: u64,
    #[allow(dead_code)]
    removed_messages: usize,
}

impl CompactBoundaryCell {
    #[must_use]
    pub fn new(strategy: String, after_tokens: u64, removed_messages: usize) -> Self {
        Self {
            strategy,
            after_tokens,
            removed_messages,
        }
    }
}

impl HistoryCell for CompactBoundaryCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let style = Style::default().fg(Color::DarkGray);
        vec![
            Line::default(),
            Line::from(Span::styled(
                "\u{273b} Conversation compacted (Ctrl+O for history)",
                style,
            )),
            Line::default(),
        ]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_renders_three_lines() {
        let cell = CompactBoundaryCell::new("summary".into(), 50000, 12);
        let lines = cell.display_lines(80);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn boundary_first_and_last_lines_are_blank() {
        let cell = CompactBoundaryCell::new("summary".into(), 50000, 12);
        let lines = cell.display_lines(80);
        assert!(lines[0].spans.is_empty());
        assert!(lines[2].spans.is_empty());
    }

    #[test]
    fn boundary_contains_compacted_label() {
        let cell = CompactBoundaryCell::new("summary".into(), 50000, 12);
        let lines = cell.display_lines(80);
        let text: String = lines[1]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("Conversation compacted"));
        assert!(text.contains("Ctrl+O"));
    }

    #[test]
    fn boundary_uses_star_glyph() {
        let cell = CompactBoundaryCell::new("trim".into(), 100_000, 5);
        let lines = cell.display_lines(80);
        let text: String = lines[1]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains('\u{273b}'));
    }

    #[test]
    fn boundary_line_uses_dark_gray() {
        let cell = CompactBoundaryCell::new("summary".into(), 50000, 12);
        let lines = cell.display_lines(80);
        for span in &lines[1].spans {
            assert_eq!(span.style.fg, Some(Color::DarkGray));
        }
    }

    #[test]
    fn boundary_narrow_width_does_not_panic() {
        let cell = CompactBoundaryCell::new("trim".into(), 1000, 1);
        let lines = cell.display_lines(20);
        assert_eq!(lines.len(), 3);
    }
}
