//! User input cell — the `❯ {text}` row.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::history::HistoryCell;
use crate::theme::accents::CLAUDE_DARK;

const TRUNCATE_THRESHOLD: usize = 10_000;
const HEAD_TAIL_CHARS: usize = 2_500;
const PROMPT_GLYPH: &str = "❯ ";
const CONT_INDENT: &str = "  ";

#[derive(Debug, Clone)]
pub struct UserCell {
    text: String,
}

impl UserCell {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl HistoryCell for UserCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let total = self.text.chars().count();
        let prompt_style = Style::default()
            .fg(CLAUDE_DARK)
            .add_modifier(Modifier::BOLD);
        let body_style = Style::default().fg(Color::White);
        let trunc_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        let mut lines: Vec<Line<'static>> = Vec::new();

        if total > TRUNCATE_THRESHOLD {
            let head: String = self.text.chars().take(HEAD_TAIL_CHARS).collect();
            let tail: String = self
                .text
                .chars()
                .skip(total.saturating_sub(HEAD_TAIL_CHARS))
                .collect();
            let marker = format!("...[truncated, {total} chars total]...");

            push_wrapped(&mut lines, &head, width, prompt_style, body_style, true);
            push_wrapped(&mut lines, &marker, width, trunc_style, trunc_style, false);
            push_wrapped(&mut lines, &tail, width, body_style, body_style, false);
        } else {
            push_wrapped(
                &mut lines,
                &self.text,
                width,
                prompt_style,
                body_style,
                true,
            );
        }

        lines.push(Line::default());
        lines
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn push_wrapped(
    out: &mut Vec<Line<'static>>,
    text: &str,
    width: u16,
    first_prefix_style: Style,
    body_style: Style,
    use_glyph_first: bool,
) {
    let prompt_w = PROMPT_GLYPH.chars().count();
    let avail = (width as usize).saturating_sub(prompt_w).max(1);

    let mut first_in_block = true;
    for raw_line in text.split('\n') {
        let chars: Vec<char> = raw_line.chars().collect();
        if chars.is_empty() {
            let prefix = if first_in_block && use_glyph_first {
                PROMPT_GLYPH
            } else {
                CONT_INDENT
            };
            let prefix_style = if first_in_block && use_glyph_first {
                first_prefix_style
            } else {
                body_style
            };
            out.push(Line::from(vec![Span::styled(prefix, prefix_style)]));
            first_in_block = false;
            continue;
        }

        let mut start = 0;
        let mut first_chunk = true;
        while start < chars.len() {
            let end = (start + avail).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            let (prefix, prefix_style) = if first_in_block && first_chunk && use_glyph_first {
                (PROMPT_GLYPH, first_prefix_style)
            } else {
                (CONT_INDENT, body_style)
            };
            out.push(Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(chunk, body_style),
            ]));
            start = end;
            first_chunk = false;
            first_in_block = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_prompt_glyph_and_text() {
        let cell = UserCell::new("hi");
        let lines = cell.display_lines(80);
        assert_eq!(lines.len(), 2);
        let rendered: String = lines[0].spans.iter().map(|s| &*s.content).collect();
        assert!(rendered.starts_with("❯ "));
        assert!(rendered.contains("hi"));
    }

    #[test]
    fn desired_height_matches_line_count() {
        let cell = UserCell::new("some text");
        assert_eq!(cell.desired_height(80), 2);
    }

    #[test]
    fn search_text_includes_body() {
        let cell = UserCell::new("searchable text");
        assert!(cell.search_text().contains("searchable text"));
    }

    #[test]
    fn long_message_truncates_with_marker() {
        let body = "x".repeat(15_000);
        let cell = UserCell::new(body);
        let lines = cell.display_lines(200);
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(joined.contains("...[truncated, 15000 chars total]..."));
        let x_count = joined.chars().filter(|c| *c == 'x').count();
        assert_eq!(x_count, HEAD_TAIL_CHARS * 2);
    }

    #[test]
    fn wraps_long_single_line() {
        let cell = UserCell::new("x".repeat(50));
        let lines = cell.display_lines(20);
        assert!(lines.len() > 2);
        let first: String = lines[0].spans.iter().map(|s| &*s.content).collect();
        assert!(first.starts_with("❯ "));
        let second: String = lines[1].spans.iter().map(|s| &*s.content).collect();
        assert!(second.starts_with("  "));
    }
}
