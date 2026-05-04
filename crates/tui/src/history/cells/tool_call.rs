//! Tool-invocation cell — colored `● {name} {detail}` header.
//!
//! Each tool category gets a distinct icon color via `Tool::display_color()`.
//! The dot changes color based on tool lifecycle status:
//! - Running: category color (White/Cyan/etc.)
//! - Success: Green
//! - Error: Red

use crab_core::tool::ToolDisplayStyle as DS;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::ToolCallStatus;
use crate::history::HistoryCell;

#[derive(Debug, Clone)]
pub struct ToolCallCell {
    name: String,
    summary: Option<String>,
    color: Option<DS>,
    status: ToolCallStatus,
    frame: u64,
}

impl ToolCallCell {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        summary: Option<String>,
        color: Option<DS>,
        status: ToolCallStatus,
    ) -> Self {
        Self {
            name: name.into(),
            summary,
            color,
            status,
            frame: 0,
        }
    }

    pub fn set_frame(&mut self, frame: u64) {
        self.frame = frame;
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn label(&self) -> &str {
        self.summary.as_deref().unwrap_or(&self.name)
    }

    fn icon_color(&self) -> Color {
        match self.status {
            ToolCallStatus::Success => Color::Green,
            ToolCallStatus::Error => Color::Red,
            ToolCallStatus::Running => match self.color {
                Some(DS::Highlight) => Color::Cyan,
                Some(DS::DiffAdd) => Color::Green,
                Some(DS::DiffRemove | DS::Error) => Color::Red,
                Some(DS::Muted) => Color::DarkGray,
                _ => Color::White,
            },
        }
    }

    fn parse_summary(&self) -> (&str, Option<&str>) {
        let label = self.label();
        if let Some(paren_start) = label.find('(')
            && label.ends_with(')')
        {
            let tool_part = label[..paren_start].trim();
            let detail = &label[paren_start + 1..label.len() - 1];
            return (tool_part, Some(detail));
        }
        (label, None)
    }

    fn animated_dots(&self) -> &'static str {
        if !matches!(self.status, ToolCallStatus::Running) {
            return "";
        }
        match self.frame % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            _ => "...",
        }
    }
}

impl HistoryCell for ToolCallCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let icon_color = self.icon_color();
        let (tool_part, detail) = self.parse_summary();

        let mut spans = vec![
            Span::styled("● ", Style::default().fg(icon_color)),
            Span::styled(
                tool_part.to_string(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        let dots = self.animated_dots();
        if !dots.is_empty() {
            spans.push(Span::styled(
                dots.to_string(),
                Style::default().fg(Color::DarkGray),
            ));
        }

        if let Some(detail) = detail {
            let dim_paren = Style::default().fg(Color::DarkGray);
            spans.push(Span::styled("(", dim_paren));
            spans.push(Span::styled(
                detail.to_string(),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(")", dim_paren));
        }

        vec![Line::from(spans)]
    }

    fn transcript_animation_tick(&self) -> Option<u64> {
        if matches!(self.status, ToolCallStatus::Running) {
            Some(self.frame)
        } else {
            None
        }
    }

    fn is_finalized(&self) -> bool {
        !matches!(self.status, ToolCallStatus::Running)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_summary_when_present() {
        let cell = ToolCallCell::new(
            "read",
            Some("Read (src/lib.rs)".into()),
            Some(DS::Muted),
            ToolCallStatus::Running,
        );
        let lines = cell.display_lines(80);
        let text: String = lines[0].spans.iter().map(|s| &*s.content).collect();
        assert!(text.contains("Read"));
        assert!(text.contains("src/lib.rs"));
    }

    #[test]
    fn falls_back_to_name() {
        let cell = ToolCallCell::new("bash", None, None, ToolCallStatus::Running);
        let text: String = cell.display_lines(80)[0]
            .spans
            .iter()
            .map(|s| &*s.content)
            .collect();
        assert!(text.contains("bash"));
    }

    #[test]
    fn icon_color_matches_display_style_when_running() {
        let cell = ToolCallCell::new("bash", None, Some(DS::Highlight), ToolCallStatus::Running);
        assert_eq!(cell.icon_color(), Color::Cyan);

        let cell = ToolCallCell::new("edit", None, Some(DS::DiffAdd), ToolCallStatus::Running);
        assert_eq!(cell.icon_color(), Color::Green);

        let cell = ToolCallCell::new("read", None, Some(DS::Muted), ToolCallStatus::Running);
        assert_eq!(cell.icon_color(), Color::DarkGray);
    }

    #[test]
    fn icon_color_green_on_success() {
        let cell = ToolCallCell::new("bash", None, Some(DS::Highlight), ToolCallStatus::Success);
        assert_eq!(cell.icon_color(), Color::Green);
    }

    #[test]
    fn icon_color_red_on_error() {
        let cell = ToolCallCell::new("bash", None, Some(DS::Highlight), ToolCallStatus::Error);
        assert_eq!(cell.icon_color(), Color::Red);
    }

    #[test]
    fn summary_parsed_into_name_and_detail() {
        let cell = ToolCallCell::new(
            "bash",
            Some("Run (ls -la)".into()),
            None,
            ToolCallStatus::Running,
        );
        let (name, detail) = cell.parse_summary();
        assert_eq!(name, "Run");
        assert_eq!(detail, Some("ls -la"));
    }

    #[test]
    fn summary_without_parens_stays_whole() {
        let cell = ToolCallCell::new(
            "bash",
            Some("Run command".into()),
            None,
            ToolCallStatus::Running,
        );
        let (name, detail) = cell.parse_summary();
        assert_eq!(name, "Run command");
        assert_eq!(detail, None);
    }

    #[test]
    fn multi_span_rendering() {
        let cell = ToolCallCell::new(
            "edit",
            Some("Update (src/main.rs)".into()),
            Some(DS::DiffAdd),
            ToolCallStatus::Running,
        );
        let lines = cell.display_lines(80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 5);
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Green));
        assert_eq!(lines[0].spans[1].style.fg, Some(Color::White));
        assert_eq!(lines[0].spans[2].content.as_ref(), "(");
        assert_eq!(lines[0].spans[3].style.fg, Some(Color::Cyan));
        assert_eq!(lines[0].spans[4].content.as_ref(), ")");
    }

    #[test]
    fn animated_dots_appear_when_running_with_frame() {
        let mut cell = ToolCallCell::new("bash", None, None, ToolCallStatus::Running);
        cell.set_frame(2);
        let lines = cell.display_lines(80);
        let text: String = lines[0].spans.iter().map(|s| &*s.content).collect();
        assert!(text.contains("bash.."));
        assert_eq!(lines[0].spans.len(), 3);
        assert_eq!(lines[0].spans[2].content.as_ref(), "..");
        assert_eq!(lines[0].spans[2].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn animated_dots_cycle_through_frames() {
        let mut cell = ToolCallCell::new("bash", None, None, ToolCallStatus::Running);
        cell.set_frame(0);
        assert_eq!(cell.animated_dots(), "");
        cell.set_frame(1);
        assert_eq!(cell.animated_dots(), ".");
        cell.set_frame(2);
        assert_eq!(cell.animated_dots(), "..");
        cell.set_frame(3);
        assert_eq!(cell.animated_dots(), "...");
        cell.set_frame(4);
        assert_eq!(cell.animated_dots(), "");
    }

    #[test]
    fn no_dots_when_not_running() {
        let mut cell = ToolCallCell::new("bash", None, None, ToolCallStatus::Success);
        cell.set_frame(2);
        assert_eq!(cell.animated_dots(), "");
        let lines = cell.display_lines(80);
        assert_eq!(lines[0].spans.len(), 2);
    }

    #[test]
    fn animation_tick_only_when_running() {
        let mut cell = ToolCallCell::new("bash", None, None, ToolCallStatus::Running);
        cell.set_frame(7);
        assert_eq!(cell.transcript_animation_tick(), Some(7));

        let mut cell = ToolCallCell::new("bash", None, None, ToolCallStatus::Success);
        cell.set_frame(7);
        assert_eq!(cell.transcript_animation_tick(), None);

        let mut cell = ToolCallCell::new("bash", None, None, ToolCallStatus::Error);
        cell.set_frame(7);
        assert_eq!(cell.transcript_animation_tick(), None);
    }
}
