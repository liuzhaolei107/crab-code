//! Tool result cell — truncated output, optional tool-customized styling.

use crab_core::tool::{ToolDisplayResult, ToolDisplayStyle as DS};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

/// Default row limit before showing the "... N more lines" pager row.
const DEFAULT_LIMIT: usize = 10;

/// Distinguishes result sub-types for rendering purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolResultKind {
    #[default]
    Success,
    Error,
    Canceled,
}

/// A tool execution result.
#[derive(Debug, Clone)]
pub struct ToolResultCell {
    tool_name: String,
    output: String,
    is_error: bool,
    kind: ToolResultKind,
    display: Option<ToolDisplayResult>,
    collapsed: bool,
}

impl ToolResultCell {
    #[must_use]
    pub fn new(
        tool_name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
        display: Option<ToolDisplayResult>,
        collapsed: bool,
    ) -> Self {
        let kind = if is_error {
            ToolResultKind::Error
        } else {
            ToolResultKind::Success
        };
        Self {
            tool_name: tool_name.into(),
            output: output.into(),
            is_error,
            kind,
            display,
            collapsed,
        }
    }

    #[must_use]
    pub fn with_kind(mut self, kind: ToolResultKind) -> Self {
        self.kind = kind;
        self
    }

    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    #[must_use]
    pub fn is_error(&self) -> bool {
        self.is_error
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

impl HistoryCell for ToolResultCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let mut out: Vec<Line<'static>> = Vec::new();
        let glyph_style = Style::default().fg(Color::DarkGray);

        if self.kind == ToolResultKind::Canceled {
            out.push(Line::from(vec![
                Span::styled("  \u{23bf}  ", glyph_style),
                Span::styled(
                    "Canceled",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
            out.push(Line::default());
            return out;
        }

        if let Some(display) = &self.display {
            let total = display.lines.len();
            let limit = if self.collapsed && display.preview_lines > 0 {
                display.preview_lines.min(total)
            } else {
                total
            };
            for (i, dl) in display.lines[..limit].iter().enumerate() {
                let style = Self::style_for(dl.style);
                if i == 0 {
                    out.push(Line::from(vec![
                        Span::styled("  \u{23bf}  ", glyph_style),
                        Span::styled(dl.text.clone(), style),
                    ]));
                } else {
                    out.push(Line::from(Span::styled(format!("     {}", dl.text), style)));
                }
            }
            if limit < total {
                out.push(Line::from(Span::styled(
                    format!("     ... ({} more lines, Enter to expand)", total - limit),
                    Style::default().fg(Color::DarkGray),
                )));
                out.push(Line::from(Span::styled(
                    "  Ctrl+O to expand",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )));
            }
        } else {
            let style = if self.is_error {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let lines: Vec<&str> = self.output.lines().collect();
            let limit = if self.collapsed {
                lines.len().min(DEFAULT_LIMIT)
            } else {
                lines.len()
            };
            for (i, line) in lines[..limit].iter().enumerate() {
                if i == 0 {
                    out.push(Line::from(vec![
                        Span::styled("  \u{23bf}  ", glyph_style),
                        Span::styled(line.to_string(), style),
                    ]));
                } else {
                    out.push(Line::from(Span::styled(format!("     {line}"), style)));
                }
            }
            if lines.len() > limit {
                out.push(Line::from(Span::styled(
                    format!("     ... ({} more lines)", lines.len() - limit),
                    Style::default().fg(Color::DarkGray),
                )));
                out.push(Line::from(Span::styled(
                    "  Ctrl+O to expand",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )));
            }
        }
        out.push(Line::default());
        out
    }

    /// Transcript view shows the full output with no truncation, since
    /// the overlay exists specifically to inspect long outputs.
    fn transcript_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let glyph_style = Style::default().fg(Color::DarkGray);
        if let Some(display) = &self.display {
            let mut out: Vec<Line<'static>> = display
                .lines
                .iter()
                .enumerate()
                .map(|(i, dl)| {
                    let style = Self::style_for(dl.style);
                    if i == 0 {
                        Line::from(vec![
                            Span::styled("  \u{23bf}  ", glyph_style),
                            Span::styled(dl.text.clone(), style),
                        ])
                    } else {
                        Line::from(Span::styled(format!("     {}", dl.text), style))
                    }
                })
                .collect();
            out.push(Line::default());
            return out;
        }
        let style = if self.is_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let mut out: Vec<Line<'static>> = self
            .output
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    Line::from(vec![
                        Span::styled("  \u{23bf}  ", glyph_style),
                        Span::styled(line.to_string(), style),
                    ])
                } else {
                    Line::from(Span::styled(format!("     {line}"), style))
                }
            })
            .collect();
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

    #[test]
    fn short_output_is_not_truncated() {
        let cell = ToolResultCell::new("read", "one\ntwo", false, None, true);
        let lines = cell.display_lines(80);
        // 2 body + 1 blank
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn long_output_is_truncated_with_pager() {
        let body = (0..20)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let cell = ToolResultCell::new("bash", body, false, None, true);
        let lines = cell.display_lines(80);
        // 10 body + 1 pager + 1 hint + 1 blank
        assert_eq!(lines.len(), 13);
        let pager: String = lines[10].spans.iter().map(|s| &*s.content).collect();
        assert!(pager.contains("more lines"));
        let hint: String = lines[11].spans.iter().map(|s| &*s.content).collect();
        assert!(hint.contains("Ctrl+O to expand"));
        assert!(
            lines[11].spans[0]
                .style
                .add_modifier
                .contains(Modifier::ITALIC)
        );
    }

    #[test]
    fn transcript_shows_full_output() {
        let body = (0..20)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let cell = ToolResultCell::new("bash", body, false, None, true);
        let lines = cell.transcript_lines(80);
        // 20 body + 1 blank, no pager row
        assert_eq!(lines.len(), 21);
    }

    #[test]
    fn error_gets_red_styling() {
        let cell = ToolResultCell::new("bash", "bad", true, None, true);
        let lines = cell.display_lines(80);
        // spans[0] is the glyph "  ⎿  ", spans[1] is the error text
        assert_eq!(lines[0].spans[1].style.fg, Some(Color::Red));
    }

    #[test]
    fn collapsed_display_respects_preview_lines() {
        use crab_core::tool::{ToolDisplayLine, ToolDisplayResult, ToolDisplayStyle};
        let display = ToolDisplayResult {
            lines: (0..10)
                .map(|i| ToolDisplayLine::new(format!("line-{i}"), ToolDisplayStyle::Normal))
                .collect(),
            preview_lines: 3,
        };
        let cell = ToolResultCell::new("bash", "", false, Some(display), true);
        let lines = cell.display_lines(80);
        // 3 preview + 1 pager + 1 hint + 1 blank
        assert_eq!(lines.len(), 6);
        let pager: String = lines[3].spans.iter().map(|s| &*s.content).collect();
        assert!(pager.contains("7 more lines"));
        let hint: String = lines[4].spans.iter().map(|s| &*s.content).collect();
        assert!(hint.contains("Ctrl+O to expand"));
    }

    #[test]
    fn hint_absent_when_output_fits() {
        let cell = ToolResultCell::new("read", "one\ntwo", false, None, true);
        let lines = cell.display_lines(80);
        for line in &lines {
            let text: String = line.spans.iter().map(|s| &*s.content).collect();
            assert!(!text.contains("Ctrl+O"));
        }
    }

    #[test]
    fn hint_absent_when_not_collapsed() {
        let body = (0..20)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let cell = ToolResultCell::new("bash", body, false, None, false);
        let lines = cell.display_lines(80);
        for line in &lines {
            let text: String = line.spans.iter().map(|s| &*s.content).collect();
            assert!(!text.contains("Ctrl+O"));
        }
    }

    #[test]
    fn hint_absent_in_transcript_view() {
        let body = (0..20)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let cell = ToolResultCell::new("bash", body, false, None, true);
        let lines = cell.transcript_lines(80);
        for line in &lines {
            let text: String = line.spans.iter().map(|s| &*s.content).collect();
            assert!(!text.contains("Ctrl+O"));
        }
    }

    #[test]
    fn expanded_display_shows_all_lines() {
        use crab_core::tool::{ToolDisplayLine, ToolDisplayResult, ToolDisplayStyle};
        let display = ToolDisplayResult {
            lines: (0..10)
                .map(|i| ToolDisplayLine::new(format!("line-{i}"), ToolDisplayStyle::Normal))
                .collect(),
            preview_lines: 3,
        };
        let cell = ToolResultCell::new("bash", "", false, Some(display), false);
        let lines = cell.display_lines(80);
        // 10 body + 1 blank
        assert_eq!(lines.len(), 11);
    }
}
