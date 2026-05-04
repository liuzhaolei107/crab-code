//! Live tool-progress cell — single dim status line indented under the
//! preceding `ToolCallCell`. Replaced by `ToolResultCell` once the tool
//! finishes.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

const CONT_GLYPH: &str = "  \u{23bf}  "; // matches CCB's "  ⎿  " indent

#[derive(Debug, Clone)]
pub struct ToolProgressCell {
    #[allow(dead_code)]
    tool_name: String,
    tail_output: String,
    total_lines: usize,
    elapsed_secs: f64,
}

impl ToolProgressCell {
    #[must_use]
    pub fn new(
        tool_name: String,
        tail_output: String,
        total_lines: usize,
        elapsed_secs: f64,
    ) -> Self {
        Self {
            tool_name,
            tail_output,
            total_lines,
            elapsed_secs,
        }
    }

    fn last_tail_line(&self) -> Option<&str> {
        self.tail_output.lines().next_back()
    }

    fn status_text(&self) -> String {
        let body = self.last_tail_line().unwrap_or("Running…");
        let mut suffix_parts = Vec::new();
        if self.elapsed_secs >= 0.05 {
            suffix_parts.push(format!("{:.1}s", self.elapsed_secs));
        }
        if self.total_lines > 0 {
            suffix_parts.push(format!("{} lines", self.total_lines));
        }
        if suffix_parts.is_empty() {
            body.to_string()
        } else {
            format!("{body} ({})", suffix_parts.join(", "))
        }
    }
}

impl HistoryCell for ToolProgressCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let glyph_style = Style::default().fg(Color::DarkGray);
        let dim = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);
        vec![Line::from(vec![
            Span::styled(CONT_GLYPH, glyph_style),
            Span::styled(self.status_text(), dim),
        ])]
    }

    fn search_text(&self) -> String {
        self.tail_output.clone()
    }

    fn is_finalized(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flatten(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn renders_single_status_line_with_running_label_when_no_output() {
        let cell = ToolProgressCell::new("bash".into(), String::new(), 0, 0.0);
        let lines = cell.display_lines(80);
        assert_eq!(lines.len(), 1);
        let text = flatten(&lines[0]);
        assert!(text.contains("Running"));
        assert!(text.contains('\u{23bf}'));
    }

    #[test]
    fn shows_last_tail_line_with_elapsed_and_count() {
        let tail = "Compiling foo\nLinking foo".to_string();
        let cell = ToolProgressCell::new("bash".into(), tail, 2, 1.25);
        let lines = cell.display_lines(80);
        assert_eq!(lines.len(), 1);
        let text = flatten(&lines[0]);
        assert!(text.contains("Linking foo"));
        assert!(text.contains("1.2s"));
        assert!(text.contains("2 lines"));
    }

    #[test]
    fn search_text_returns_tail() {
        let cell = ToolProgressCell::new("bash".into(), "needle\nhaystack".into(), 2, 0.1);
        assert!(cell.search_text().contains("needle"));
    }

    #[test]
    fn never_finalized() {
        let cell = ToolProgressCell::new("bash".into(), String::new(), 0, 0.0);
        assert!(!cell.is_finalized());
    }
}
