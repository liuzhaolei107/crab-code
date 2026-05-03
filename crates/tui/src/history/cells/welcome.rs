use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;

#[derive(Debug, Clone)]
pub struct WelcomeCell {
    version: String,
    whats_new: String,
    show_project_hint: bool,
}

impl WelcomeCell {
    #[must_use]
    pub fn new(version: String, whats_new: String, show_project_hint: bool) -> Self {
        Self {
            version,
            whats_new,
            show_project_hint,
        }
    }
}

impl HistoryCell for WelcomeCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let mut out = Vec::with_capacity(4);
        out.push(Line::from(Span::styled(
            format!("Crab Code v{}", self.version),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        if !self.whats_new.is_empty() {
            out.push(Line::from(Span::styled(
                self.whats_new.clone(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )));
        }
        if self.show_project_hint {
            out.push(Line::from(Span::styled(
                "Found CLAUDE.md",
                Style::default().fg(Color::DarkGray),
            )));
        }
        out.push(Line::default());
        out
    }

    fn transcript_lines(&self, _width: u16) -> Vec<Line<'static>> {
        Vec::new()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flatten(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn banner_only_when_no_extras() {
        let cell = WelcomeCell::new("0.1.0".into(), String::new(), false);
        let lines = cell.display_lines(120);
        assert_eq!(lines.len(), 2);
        let text = flatten(&lines);
        assert!(text.contains("Crab Code v0.1.0"));
    }

    #[test]
    fn includes_whats_new_when_present() {
        let cell = WelcomeCell::new("0.1.0".into(), "shiny new thing".into(), false);
        let lines = cell.display_lines(120);
        assert_eq!(lines.len(), 3);
        let text = flatten(&lines);
        assert!(text.contains("Crab Code v0.1.0"));
        assert!(text.contains("shiny new thing"));
    }

    #[test]
    fn includes_project_hint_when_enabled() {
        let cell = WelcomeCell::new("0.1.0".into(), String::new(), true);
        let lines = cell.display_lines(120);
        assert_eq!(lines.len(), 3);
        let text = flatten(&lines);
        assert!(text.contains("Found CLAUDE.md"));
    }

    #[test]
    fn includes_all_when_both_set() {
        let cell = WelcomeCell::new("0.1.0".into(), "release notes".into(), true);
        let lines = cell.display_lines(120);
        assert_eq!(lines.len(), 4);
        let text = flatten(&lines);
        assert!(text.contains("Crab Code v0.1.0"));
        assert!(text.contains("release notes"));
        assert!(text.contains("Found CLAUDE.md"));
    }

    #[test]
    fn no_emoji_in_banner() {
        let cell = WelcomeCell::new("0.1.0".into(), String::new(), false);
        let text = flatten(&cell.display_lines(120));
        assert!(!text.contains('✦'));
        assert!(!text.contains('🦀'));
    }

    #[test]
    fn transcript_lines_are_empty() {
        let cell = WelcomeCell::new("0.1.0".into(), "x".into(), true);
        assert!(cell.transcript_lines(120).is_empty());
    }
}
