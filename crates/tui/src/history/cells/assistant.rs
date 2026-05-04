//! Assistant reply cell — renders markdown body with a `⎿` corner prefix on
//! the first line and matching padding on continuation lines.

use std::cell::RefCell;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::components::syntax::SyntaxHighlighter;
use crate::components::text_utils::strip_trailing_tool_json;
use crate::history::HistoryCell;
use crate::markdown::CachedMarkdownRenderer;
use crate::theme;

const BULLET_PREFIX: &str = "● ";
const CONT_LINE_PREFIX: &str = "  ";

thread_local! {
    /// Shared per-render-thread markdown cache. Keeps expensive
    /// pulldown-cmark parses memoized by (content, theme, width).
    static SHARED_MD_CACHE: RefCell<CachedMarkdownRenderer> =
        RefCell::new(CachedMarkdownRenderer::new());
}

/// Assistant reply with markdown + code highlight.
#[derive(Debug, Clone)]
pub struct AssistantCell {
    text: String,
    pub(crate) skip_prefix: usize,
}

impl AssistantCell {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            skip_prefix: 0,
        }
    }

    #[must_use]
    pub fn with_skip(text: impl Into<String>, skip_prefix: usize) -> Self {
        Self {
            text: text.into(),
            skip_prefix,
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn push_delta(&mut self, delta: &str) {
        self.text.push_str(delta);
    }

    /// Render the markdown body up to the last newline and return any lines
    /// beyond `committed`. The cell still owns the full text so subsequent
    /// frames can re-render the streaming tail.
    pub fn render_committed_lines(&self, width: u16, committed: usize) -> Vec<Line<'static>> {
        let body = match self.text.rfind('\n') {
            Some(idx) => &self.text[..=idx],
            None => return Vec::new(),
        };
        let rendered = render_with_bullet(body, width);
        if rendered.len() <= committed {
            return Vec::new();
        }
        rendered[committed..].to_vec()
    }

    /// Render the entire current text (including any unfinished tail) and
    /// return its line count — used by callers that need to know how many
    /// lines the cell occupies right now.
    pub fn rendered_line_count(&self, width: u16) -> usize {
        render_with_bullet(&self.text, width).len()
    }
}

fn render_with_bullet(text: &str, width: u16) -> Vec<Line<'static>> {
    let clean = strip_trailing_tool_json(text);
    if clean.is_empty() {
        return Vec::new();
    }
    let theme = theme::current();
    let highlighter = SyntaxHighlighter::new();
    let prefix_width = BULLET_PREFIX.chars().count() as u16;
    let inner_width = width.saturating_sub(prefix_width).max(1);
    let md_lines: Vec<Line<'static>> = SHARED_MD_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        (*cache.render(&clean, &theme, &highlighter, inner_width)).clone()
    });
    let bullet_style = Style::default().fg(Color::DarkGray);
    let cont_style = Style::default().fg(Color::DarkGray);
    let mut out: Vec<Line<'static>> = Vec::with_capacity(md_lines.len() + 1);
    for (idx, line) in md_lines.into_iter().enumerate() {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(line.spans.len() + 1);
        if idx == 0 {
            spans.push(Span::styled(BULLET_PREFIX, bullet_style));
        } else {
            spans.push(Span::styled(CONT_LINE_PREFIX, cont_style));
        }
        spans.extend(line.spans);
        out.push(Line::from(spans));
    }
    out.push(Line::default());
    out
}

impl HistoryCell for AssistantCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut rendered = render_with_bullet(&self.text, width);
        if self.skip_prefix >= rendered.len() {
            return Vec::new();
        }
        rendered.drain(..self.skip_prefix);
        rendered
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_produces_no_lines() {
        let cell = AssistantCell::new("");
        assert!(cell.display_lines(80).is_empty());
    }

    #[test]
    fn non_empty_text_renders_with_bullet_prefix() {
        let cell = AssistantCell::new("hello");
        let lines = cell.display_lines(80);
        assert!(!lines.is_empty());
        let first: String = lines[0].spans.iter().map(|s| &*s.content).collect();
        assert!(
            first.starts_with(BULLET_PREFIX),
            "assistant text should start with bullet prefix, got {first:?}"
        );
    }

    #[test]
    fn first_line_has_bullet_prefix_continuations_padded() {
        let cell = AssistantCell::new("line one\n\nline two\n\nline three");
        let lines = cell.display_lines(80);
        // Markdown paragraphs produce multiple lines; last is trailing blank.
        let content_lines: Vec<_> = lines.iter().filter(|l| !l.spans.is_empty()).collect();
        assert!(
            content_lines.len() >= 2,
            "expected multiple content lines, got {}",
            content_lines.len()
        );

        let first: String = content_lines[0].spans.iter().map(|s| &*s.content).collect();
        assert!(
            first.starts_with(BULLET_PREFIX),
            "first line should start with bullet prefix, got {first:?}"
        );

        for line in content_lines.iter().skip(1) {
            let rendered: String = line.spans.iter().map(|s| &*s.content).collect();
            assert!(
                rendered.starts_with(CONT_LINE_PREFIX),
                "continuation line should start with padded prefix, got {rendered:?}"
            );
        }
    }

    #[test]
    fn push_delta_appends() {
        let mut cell = AssistantCell::new("hel");
        cell.push_delta("lo");
        assert_eq!(cell.text(), "hello");
    }

    #[test]
    fn search_text_contains_body() {
        let cell = AssistantCell::new("the **answer** is 42");
        let needle = "the";
        assert!(cell.search_text().contains(needle));
    }
}
