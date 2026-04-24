//! Tool rejection cell — shows what was rejected with optional rich preview.

use crab_core::tool::{ToolDisplayResult, ToolDisplayStyle as DS};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

/// A tool invocation rejected by the user.
///
/// Rendered with a dim gray header so it reads as "informational — this
/// tool use was declined," not as an error (which would use red). Preview
/// lines supplied by `Tool::format_rejected()` keep their own styling so
/// diffs and highlights remain legible.
#[derive(Debug, Clone)]
pub struct ToolRejectedCell {
    summary: String,
    display: Option<ToolDisplayResult>,
}

impl ToolRejectedCell {
    #[must_use]
    pub fn new(summary: impl Into<String>, display: Option<ToolDisplayResult>) -> Self {
        Self {
            summary: summary.into(),
            display,
        }
    }

    fn style_for(ds: Option<DS>) -> Style {
        match ds {
            Some(DS::Error | DS::DiffRemove) => Style::default().fg(Color::Red),
            Some(DS::DiffAdd) => Style::default().fg(Color::Green),
            Some(DS::DiffContext | DS::Muted) => Style::default().fg(Color::DarkGray),
            Some(DS::Highlight) => Style::default().fg(Color::Cyan),
            _ => Style::default().fg(Color::Gray),
        }
    }
}

impl HistoryCell for ToolRejectedCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let header_style = Style::default().fg(Color::DarkGray);
        let mut out = vec![Line::from(vec![
            Span::styled("  \u{2298} ", header_style),
            Span::styled(self.summary.clone(), header_style),
        ])];

        if let Some(display) = &self.display {
            for dl in &display.lines {
                let style = Self::style_for(dl.style);
                out.push(Line::from(Span::styled(format!("    {}", dl.text), style)));
            }
        }

        out.push(Line::default());
        out
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crab_core::tool::{ToolDisplayLine, ToolDisplayResult, ToolDisplayStyle};
    use ratatui::style::Modifier;

    #[test]
    fn plain_rejection_renders_summary() {
        let cell = ToolRejectedCell::new("Run rejected (ls)", None);
        let lines = cell.display_lines(80);
        assert!(lines.len() >= 2); // header + blank
        let text: String = lines[0].spans.iter().map(|s| &*s.content).collect();
        assert!(text.contains("Run rejected"));
    }

    #[test]
    fn rich_rejection_renders_preview_lines() {
        let display = ToolDisplayResult {
            lines: vec![
                ToolDisplayLine::new("echo hello", ToolDisplayStyle::Highlight),
                ToolDisplayLine::new("echo world", ToolDisplayStyle::Highlight),
            ],
            preview_lines: 2,
        };
        let cell = ToolRejectedCell::new("Run rejected", Some(display));
        let lines = cell.display_lines(80);
        // header + 2 preview + blank
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn header_uses_dim_gray_not_red() {
        // Rejection is informational, not an error — the header must not
        // use Red or BOLD so it reads as "declined this call," matching
        // the dim style used for the same state elsewhere in the TUI.
        let cell = ToolRejectedCell::new("Run rejected (ls)", None);
        let lines = cell.display_lines(80);
        let header = &lines[0];
        for span in &header.spans {
            assert_eq!(span.style.fg, Some(Color::DarkGray));
            assert!(!span.style.add_modifier.contains(Modifier::BOLD));
        }
    }
}
