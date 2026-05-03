//! Live tool-progress cell — renders an in-progress tool's last few output
//! lines while it's still running, then gets replaced by `ToolResultCell`
//! once the tool finishes.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

/// Header glyph for the in-progress line. Uses the CCB spinner frame `✶`.
const HEADER_GLYPH: &str = "\u{2736}";

/// How many tail-output lines to render under the header.
const TAIL_LINES: usize = 5;

#[derive(Debug, Clone)]
pub struct ToolProgressCell {
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

    fn header_line(&self) -> Line<'static> {
        let label = format!(
            "{HEADER_GLYPH} {name}  {elapsed:.1}s  {lines} lines",
            name = self.tool_name,
            elapsed = self.elapsed_secs,
            lines = self.total_lines,
        );
        Line::from(Span::styled(
            label,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ))
    }
}

impl HistoryCell for ToolProgressCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::with_capacity(TAIL_LINES + 2);
        lines.push(self.header_line());

        if self.tail_output.is_empty() {
            return lines;
        }

        let inner_width = width.saturating_sub(5) as usize;
        let raw_lines: Vec<&str> = self.tail_output.lines().collect();
        let start = raw_lines.len().saturating_sub(TAIL_LINES);
        let dim = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);
        let glyph_style = Style::default().fg(Color::DarkGray);

        for (i, raw) in raw_lines[start..].iter().enumerate() {
            if inner_width == 0 || raw.len() <= inner_width {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("  \u{23bf}  ", glyph_style),
                        Span::styled(raw.to_string(), dim),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(format!("     {raw}"), dim)));
                }
            } else {
                for (ci, chunk) in raw.as_bytes().chunks(inner_width).enumerate() {
                    let s = String::from_utf8_lossy(chunk).into_owned();
                    if i == 0 && ci == 0 {
                        lines.push(Line::from(vec![
                            Span::styled("  \u{23bf}  ", glyph_style),
                            Span::styled(s, dim),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled(format!("     {s}"), dim)));
                    }
                }
            }
        }
        lines
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

    #[test]
    fn header_includes_name_elapsed_and_lines() {
        let cell = ToolProgressCell::new("bash".into(), String::new(), 42, 1.25);
        let lines = cell.display_lines(80);
        assert_eq!(lines.len(), 1);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("bash"), "expected tool name: {text}");
        assert!(text.contains("1.2"), "expected elapsed seconds: {text}");
        assert!(text.contains("42 lines"), "expected line count: {text}");
    }

    #[test]
    fn renders_at_most_five_tail_lines() {
        let tail = (1..=20)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let cell = ToolProgressCell::new("bash".into(), tail, 20, 0.5);
        let lines = cell.display_lines(80);
        // 1 header + 5 tail lines max
        assert_eq!(lines.len(), 1 + TAIL_LINES);
        let last: String = lines
            .last()
            .unwrap()
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            last.contains("line 20"),
            "expected most recent line: {last}"
        );
    }

    #[test]
    fn empty_tail_renders_only_header() {
        let cell = ToolProgressCell::new("bash".into(), String::new(), 0, 0.0);
        assert_eq!(cell.display_lines(80).len(), 1);
    }

    #[test]
    fn search_text_returns_tail() {
        let cell = ToolProgressCell::new("bash".into(), "needle\nhaystack".into(), 2, 0.1);
        assert!(cell.search_text().contains("needle"));
    }
}
