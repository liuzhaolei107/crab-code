//! Welcome cell — ambient startup panel.
//!
//! Shown at the top of the transcript when any of these is true:
//! - the current binary version differs from `global_state.last_welcome_version`
//! - the current project has no `CRAB.md` (new-project hint)
//! - the `CRAB_FORCE_FULL_LOGO` env var is truthy
//!
//! Not included in the transcript overlay — ambient context is not
//! conversation content.
//!
//! Layout is a three-column card:
//!
//! ```text
//! ╭─ Crab Code v0.1.0 ─────────────────────────────────────────╮
//! │  Welcome back!          Recent activity                    │
//! │                         2h ago   refactor tui header       │
//! │   ╭◉───◉╮               5h ago   fix bash shell resolution │
//! │   ╰█████╯               1d ago   add welcome cell          │
//! │    ╵╵╵╵╵                                                   │
//! │                         What's new                         │
//! │  First time?            • dropped cmd.exe fallback…        │
//! │  Press /help            • collapse parallel reads…         │
//! ╰────────────────────────────────────────────────────────────╯
//! ```
//!
//! Narrow terminals degrade gracefully: below 70 cols the card becomes a
//! two-column layout; below 50 cols a single column.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;
use crate::theme::accents::CLAUDE_DARK as CRAB_COLOR;

/// Three-line crab mascot, shared with the `HeaderBar`.
const CRAB_ART: &str = include_str!("../../../assets/header-logo.txt");

const MIN_WIDE_WIDTH: u16 = 70;
const MIN_TWO_COL_WIDTH: u16 = 50;

/// Ambient welcome panel shown at startup.
#[derive(Debug, Clone)]
pub struct WelcomeCell {
    version: String,
    recent_sessions: Vec<(String, String)>,
    whats_new: Vec<String>,
    show_project_hint: bool,
}

impl WelcomeCell {
    #[must_use]
    pub fn new(
        version: String,
        recent_sessions: Vec<(String, String)>,
        whats_new: Vec<String>,
        show_project_hint: bool,
    ) -> Self {
        Self {
            version,
            recent_sessions,
            whats_new,
            show_project_hint,
        }
    }
}

impl HistoryCell for WelcomeCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        if width >= MIN_WIDE_WIDTH {
            render_wide(self)
        } else if width >= MIN_TWO_COL_WIDTH {
            render_two_col(self)
        } else {
            render_single_col(self)
        }
    }

    /// Welcome is ambient context, not transcript content — omit from the
    /// transcript overlay so Ctrl+O stays focused on the actual exchange.
    fn transcript_lines(&self, _width: u16) -> Vec<Line<'static>> {
        Vec::new()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ─── Layouts ──────────────────────────────────────────────────────────────

fn render_wide(cell: &WelcomeCell) -> Vec<Line<'static>> {
    let mut out = Vec::new();

    // Title row.
    out.push(Line::from(vec![
        Span::styled(
            "✦ ",
            Style::default().fg(CRAB_COLOR).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Crab Code {}", cell.version),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    out.push(Line::default());

    // Three columns assembled line-by-line.
    let art_lines = CRAB_ART
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let recent_lines = format_recent(&cell.recent_sessions);
    let whats_new_lines = format_whats_new(&cell.whats_new);
    let hint_lines = format_hint(cell.show_project_hint);

    // Left column combines mascot + hint below it; right column combines
    // "Recent activity" then "What's new".
    let mut left: Vec<Line<'static>> = Vec::new();
    for line in &art_lines {
        left.push(Line::from(Span::styled(
            line.clone(),
            Style::default().fg(CRAB_COLOR),
        )));
    }
    left.push(Line::default());
    left.extend(hint_lines);

    let mut right: Vec<Line<'static>> = Vec::new();
    right.push(header_span("Recent activity"));
    right.extend(recent_lines);
    right.push(Line::default());
    right.push(header_span("What's new"));
    right.extend(whats_new_lines);

    // Zip the two columns, padding with blanks until both drain.
    let rows = left.len().max(right.len());
    let left_width = 18usize; // widest art line is ~10 chars + padding
    for i in 0..rows {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let left_line = left.get(i).cloned().unwrap_or_default();
        let left_text: String = left_line
            .spans
            .iter()
            .map(|s| s.content.as_ref().to_string())
            .collect();
        let left_vis = left_text.chars().count();
        // Preserve the left line's styled spans, then pad.
        spans.extend(left_line.spans);
        let pad = left_width.saturating_sub(left_vis);
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
        if let Some(right_line) = right.get(i) {
            spans.extend(right_line.spans.iter().cloned());
        }
        out.push(Line::from(spans));
    }

    out.push(Line::default());
    out
}

fn render_two_col(cell: &WelcomeCell) -> Vec<Line<'static>> {
    // Same as wide but drops the crab art; hint moves beneath the columns.
    let mut out = Vec::new();
    out.push(Line::from(vec![
        Span::styled(
            "✦ ",
            Style::default().fg(CRAB_COLOR).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Crab Code {}", cell.version),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    out.push(Line::default());

    out.push(header_span("Recent activity"));
    out.extend(format_recent(&cell.recent_sessions));
    out.push(Line::default());
    out.push(header_span("What's new"));
    out.extend(format_whats_new(&cell.whats_new));

    let hint = format_hint(cell.show_project_hint);
    if !hint.is_empty() {
        out.push(Line::default());
        out.extend(hint);
    }
    out.push(Line::default());
    out
}

fn render_single_col(cell: &WelcomeCell) -> Vec<Line<'static>> {
    // Minimal: just title + what's new + hint. Sidebar dupes recent activity
    // on narrow layouts, so omit it here to avoid noise.
    let mut out = Vec::new();
    out.push(Line::from(vec![
        Span::styled(
            "✦ ",
            Style::default().fg(CRAB_COLOR).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("Crab Code {}", cell.version),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    if !cell.whats_new.is_empty() {
        out.push(Line::default());
        out.push(header_span("What's new"));
        out.extend(format_whats_new(&cell.whats_new));
    }
    let hint = format_hint(cell.show_project_hint);
    if !hint.is_empty() {
        out.push(Line::default());
        out.extend(hint);
    }
    out.push(Line::default());
    out
}

// ─── Formatters ───────────────────────────────────────────────────────────

fn header_span(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default()
            .fg(CRAB_COLOR)
            .add_modifier(Modifier::BOLD),
    ))
}

fn format_recent(sessions: &[(String, String)]) -> Vec<Line<'static>> {
    if sessions.is_empty() {
        return vec![Line::from(Span::styled(
            "  (no recent sessions)",
            Style::default().fg(Color::DarkGray),
        ))];
    }
    sessions
        .iter()
        .map(|(name, ago)| {
            Line::from(vec![
                Span::styled(
                    format!("  {ago:<6} "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(name.clone(), Style::default().fg(Color::Gray)),
            ])
        })
        .collect()
}

fn format_whats_new(items: &[String]) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![Line::from(Span::styled(
            "  (no release notes)",
            Style::default().fg(Color::DarkGray),
        ))];
    }
    items
        .iter()
        .map(|line| {
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(CRAB_COLOR)),
                Span::styled(line.clone(), Style::default().fg(Color::Gray)),
            ])
        })
        .collect()
}

fn format_hint(show_project_hint: bool) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let dim = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC);
    out.push(Line::from(Span::styled("First time?", dim)));
    out.push(Line::from(Span::styled("Press /help", dim)));
    if show_project_hint {
        out.push(Line::default());
        out.push(Line::from(Span::styled(
            "No CRAB.md yet —",
            dim,
        )));
        out.push(Line::from(Span::styled(
            "consider creating one",
            dim,
        )));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> WelcomeCell {
        WelcomeCell::new(
            "0.1.0".into(),
            vec![
                ("refactor tui header".into(), "2h".into()),
                ("bash shell fix".into(), "5h".into()),
            ],
            vec!["dropped cmd fallback".into(), "collapse parallel reads".into()],
            true,
        )
    }

    #[test]
    fn wide_layout_has_title_plus_columns() {
        let cell = sample();
        let lines = cell.display_lines(120);
        assert!(!lines.is_empty());
        let title: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(title.contains("Crab Code 0.1.0"));
    }

    #[test]
    fn narrow_layout_drops_recent_activity() {
        let cell = sample();
        let lines = cell.display_lines(40);
        // Flatten to text
        let flat: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!flat.contains("Recent activity"));
        assert!(flat.contains("What's new"));
    }

    #[test]
    fn transcript_lines_are_empty() {
        let cell = sample();
        assert!(cell.transcript_lines(120).is_empty());
    }

    #[test]
    fn empty_recent_shows_placeholder() {
        let cell = WelcomeCell::new(
            "0.1.0".into(),
            Vec::new(),
            vec!["note".into()],
            false,
        );
        let lines = cell.display_lines(120);
        let flat: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(flat.contains("no recent sessions"));
    }

    #[test]
    fn empty_whats_new_shows_placeholder() {
        let cell = WelcomeCell::new(
            "0.1.0".into(),
            vec![("x".into(), "1h".into())],
            Vec::new(),
            false,
        );
        let lines = cell.display_lines(120);
        let flat: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(flat.contains("no release notes"));
    }

    #[test]
    fn project_hint_appears_when_requested() {
        let cell = sample();
        let lines = cell.display_lines(120);
        let flat: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(flat.contains("CRAB.md"));
    }

    #[test]
    fn project_hint_omitted_when_disabled() {
        let cell = WelcomeCell::new(
            "0.1.0".into(),
            vec![("x".into(), "1h".into())],
            vec!["n".into()],
            false,
        );
        let lines = cell.display_lines(120);
        let flat: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref().to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!flat.contains("CRAB.md"));
    }
}
