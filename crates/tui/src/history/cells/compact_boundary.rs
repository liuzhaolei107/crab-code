use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

#[derive(Debug, Clone)]
pub struct CompactBoundaryCell {
    strategy: String,
    after_tokens: u64,
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
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let w = width as usize;
        let tokens_k = self.after_tokens / 1000;
        let label = format!(
            " context compacted ({}): {} messages removed, ~{}k tokens remaining ",
            self.strategy, self.removed_messages, tokens_k
        );

        let style = Style::default().fg(Color::DarkGray);

        let label_len = label.chars().count();
        let remaining = w.saturating_sub(label_len);
        let left = remaining / 2;
        let right = remaining.saturating_sub(left);

        let mut content = String::with_capacity(w);
        content.push_str(&"\u{2500}".repeat(left));
        content.push_str(&label);
        content.push_str(&"\u{2500}".repeat(right));

        vec![
            Line::default(),
            Line::from(Span::styled(content, style)),
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
    fn boundary_contains_label_text() {
        let cell = CompactBoundaryCell::new("summary".into(), 50000, 12);
        let lines = cell.display_lines(80);
        let text: String = lines[1]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("context compacted (summary)"));
        assert!(text.contains("12 messages removed"));
        assert!(text.contains("~50k tokens remaining"));
    }

    #[test]
    fn boundary_uses_box_drawing_rule() {
        let cell = CompactBoundaryCell::new("trim".into(), 100_000, 5);
        let lines = cell.display_lines(80);
        let text: String = lines[1]
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains('\u{2500}'));
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
