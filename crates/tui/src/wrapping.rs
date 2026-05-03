//! URL-aware line wrapping for inline-viewport scrollback rendering.
//!
//! Standard text wrapping splits at hyphens and slashes, which mangles URLs
//! and makes them unclickable. This module's adaptive path detects URL-like
//! tokens and switches to a wrapping configuration that keeps them intact
//! while still letting non-URL words break normally on the same line.
//!
//! Used by `insert_history` to wrap finalized history lines before writing
//! them above the viewport.

use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use std::borrow::Cow;
use std::ops::Range;
use textwrap::Options;

/// Wrapping options. Mirrors the relevant subset of `textwrap::Options` but
/// uses ratatui `Line` for indents so styled prefixes survive wrapping.
#[derive(Debug, Clone)]
pub struct RtOptions<'a> {
    pub width: usize,
    pub initial_indent: Line<'a>,
    pub subsequent_indent: Line<'a>,
    pub break_words: bool,
    pub word_separator: textwrap::WordSeparator,
    pub word_splitter: textwrap::WordSplitter,
}

impl RtOptions<'_> {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            initial_indent: Line::default(),
            subsequent_indent: Line::default(),
            break_words: true,
            word_separator: textwrap::WordSeparator::new(),
            word_splitter: textwrap::WordSplitter::HyphenSplitter,
        }
    }

    #[must_use]
    pub fn word_separator(mut self, sep: textwrap::WordSeparator) -> Self {
        self.word_separator = sep;
        self
    }

    #[must_use]
    pub fn word_splitter(mut self, splitter: textwrap::WordSplitter) -> Self {
        self.word_splitter = splitter;
        self
    }

    #[must_use]
    pub fn break_words(mut self, value: bool) -> Self {
        self.break_words = value;
        self
    }
}

impl From<usize> for RtOptions<'_> {
    fn from(width: usize) -> Self {
        Self::new(width)
    }
}

/// Returns `true` if any whitespace-delimited token in `line` looks like a URL.
#[must_use]
pub fn line_contains_url_like(line: &Line<'_>) -> bool {
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    text_contains_url_like(&text)
}

/// Returns `true` if `line` has both a URL-like token and at least one
/// substantive non-URL token (decorative markers like `> `, `1.`, `│` don't
/// count as substantive).
#[must_use]
pub fn line_has_mixed_url_and_non_url_tokens(line: &Line<'_>) -> bool {
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    text_has_mixed_url_and_non_url_tokens(&text)
}

fn text_contains_url_like(text: &str) -> bool {
    text.split_ascii_whitespace().any(is_url_like_token)
}

fn text_has_mixed_url_and_non_url_tokens(text: &str) -> bool {
    let mut saw_url = false;
    let mut saw_non_url = false;
    for raw_token in text.split_ascii_whitespace() {
        if is_url_like_token(raw_token) {
            saw_url = true;
        } else if is_substantive_non_url_token(raw_token) {
            saw_non_url = true;
        }
        if saw_url && saw_non_url {
            return true;
        }
    }
    false
}

fn is_url_like_token(raw_token: &str) -> bool {
    let token = trim_url_token(raw_token);
    !token.is_empty() && (is_absolute_url_like(token) || is_bare_url_like(token))
}

fn is_substantive_non_url_token(raw_token: &str) -> bool {
    let token = trim_url_token(raw_token);
    if token.is_empty() || is_decorative_marker_token(raw_token, token) {
        return false;
    }
    token.chars().any(char::is_alphanumeric)
}

fn is_decorative_marker_token(raw_token: &str, token: &str) -> bool {
    let raw = raw_token.trim();
    matches!(
        raw,
        "-" | "*"
            | "+"
            | "•"
            | "◦"
            | "▪"
            | ">"
            | "|"
            | "│"
            | "┆"
            | "└"
            | "├"
            | "┌"
            | "┐"
            | "┘"
            | "┼"
    ) || is_ordered_list_marker(raw, token)
}

fn is_ordered_list_marker(raw_token: &str, token: &str) -> bool {
    token.chars().all(|c| c.is_ascii_digit())
        && (raw_token.ends_with('.') || raw_token.ends_with(')'))
}

fn trim_url_token(token: &str) -> &str {
    token.trim_matches(|c: char| {
        matches!(
            c,
            '(' | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | ','
                | '.'
                | ';'
                | ':'
                | '!'
                | '\''
                | '"'
        )
    })
}

fn is_absolute_url_like(token: &str) -> bool {
    if !token.contains("://") {
        return false;
    }
    if let Ok(url) = url::Url::parse(token) {
        let scheme = url.scheme().to_ascii_lowercase();
        if matches!(
            scheme.as_str(),
            "http" | "https" | "ftp" | "ftps" | "ws" | "wss"
        ) {
            return url.host_str().is_some();
        }
        return true;
    }
    has_valid_scheme_prefix(token)
}

fn has_valid_scheme_prefix(token: &str) -> bool {
    let Some((scheme, rest)) = token.split_once("://") else {
        return false;
    };
    if scheme.is_empty() || rest.is_empty() {
        return false;
    }
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

fn is_bare_url_like(token: &str) -> bool {
    let (host_port, has_trailer) = match token.find(['/', '?', '#']) {
        Some(idx) => (&token[..idx], true),
        None => (token, false),
    };
    if host_port.is_empty() {
        return false;
    }
    if !has_trailer && !host_port.to_ascii_lowercase().starts_with("www.") {
        return false;
    }
    let (host, port) = split_host_and_port(host_port);
    if host.is_empty() {
        return false;
    }
    if let Some(port) = port
        && !is_valid_port(port)
    {
        return false;
    }
    host.eq_ignore_ascii_case("localhost") || is_ipv4(host) || is_domain_name(host)
}

fn split_host_and_port(host_port: &str) -> (&str, Option<&str>) {
    if host_port.starts_with('[') {
        return (host_port, None);
    }
    if let Some((host, port)) = host_port.rsplit_once(':')
        && !host.is_empty()
        && !port.is_empty()
        && port.chars().all(|c| c.is_ascii_digit())
    {
        return (host, Some(port));
    }
    (host_port, None)
}

fn is_valid_port(port: &str) -> bool {
    if port.is_empty() || port.len() > 5 || !port.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    port.parse::<u16>().is_ok()
}

fn is_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts
        .iter()
        .all(|part| !part.is_empty() && part.parse::<u8>().is_ok())
}

fn is_domain_name(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    if !host.contains('.') {
        return false;
    }
    let mut labels = host.split('.');
    let Some(tld) = labels.next_back() else {
        return false;
    };
    if !is_tld(tld) {
        return false;
    }
    labels.all(is_domain_label)
}

fn is_tld(label: &str) -> bool {
    (2..=63).contains(&label.len()) && label.chars().all(|c| c.is_ascii_alphabetic())
}

fn is_domain_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }
    let first = label.chars().next().unwrap_or('-');
    let last = label.chars().next_back().unwrap_or('-');
    first.is_ascii_alphanumeric()
        && last.is_ascii_alphanumeric()
        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn url_preserving_wrap_options(opts: RtOptions<'_>) -> RtOptions<'_> {
    opts.word_separator(textwrap::WordSeparator::AsciiSpace)
        .word_splitter(textwrap::WordSplitter::Custom(split_non_url_word))
        .break_words(false)
}

fn split_non_url_word(word: &str) -> Vec<usize> {
    if is_url_like_token(word) {
        return Vec::new();
    }
    word.char_indices().skip(1).map(|(idx, _)| idx).collect()
}

/// Wrap a single ratatui `Line`, preserving URL tokens intact when present.
#[must_use]
pub fn adaptive_wrap_line<'a>(line: &'a Line<'a>, base: RtOptions<'a>) -> Vec<Line<'a>> {
    let selected = if line_contains_url_like(line) {
        url_preserving_wrap_options(base)
    } else {
        base
    };
    word_wrap_line(line, selected)
}

fn word_wrap_line<'a>(line: &'a Line<'a>, rt_opts: RtOptions<'a>) -> Vec<Line<'a>> {
    let mut flat = String::new();
    let mut span_bounds: Vec<(Range<usize>, Style)> = Vec::new();
    let mut acc = 0usize;
    for s in &line.spans {
        let text = s.content.as_ref();
        let start = acc;
        flat.push_str(text);
        acc += text.len();
        span_bounds.push((start..acc, s.style));
    }

    let opts = Options::new(rt_opts.width)
        .break_words(rt_opts.break_words)
        .word_separator(rt_opts.word_separator)
        .word_splitter(rt_opts.word_splitter);

    let initial_width = opts
        .width
        .saturating_sub(rt_opts.initial_indent.width())
        .max(1);
    let initial_ranges = wrap_ranges_trim(&flat, opts.clone().width(initial_width));
    let Some(first_range) = initial_ranges.first() else {
        return vec![rt_opts.initial_indent.clone()];
    };

    let mut out: Vec<Line<'a>> = Vec::new();
    let mut first_line = rt_opts.initial_indent.clone().style(line.style);
    {
        let sliced = slice_line_spans(line, &span_bounds, first_range);
        first_line
            .spans
            .extend(sliced.spans.into_iter().map(|s| s.patch_style(line.style)));
        out.push(first_line);
    }

    let base = first_range.end;
    let leading = flat[base..].chars().take_while(|c| *c == ' ').count();
    let base = base + leading;
    let subsequent_width = opts
        .width
        .saturating_sub(rt_opts.subsequent_indent.width())
        .max(1);
    let remaining_ranges = wrap_ranges_trim(&flat[base..], opts.width(subsequent_width));
    for r in &remaining_ranges {
        if r.is_empty() {
            continue;
        }
        let mut subsequent_line = rt_opts.subsequent_indent.clone().style(line.style);
        let offset_range = (r.start + base)..(r.end + base);
        let sliced = slice_line_spans(line, &span_bounds, &offset_range);
        subsequent_line
            .spans
            .extend(sliced.spans.into_iter().map(|s| s.patch_style(line.style)));
        out.push(subsequent_line);
    }

    out
}

fn wrap_ranges_trim<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    let mut lines: Vec<Range<usize>> = Vec::new();
    let mut cursor = 0usize;
    for line in &textwrap::wrap(text, &opts) {
        match line {
            Cow::Borrowed(slice) => {
                let start = skip_leading_whitespace(text, cursor);
                let end = (start + slice.len()).min(text.len());
                lines.push(start..end);
                cursor = end;
            }
            Cow::Owned(slice) => {
                let mapped = map_owned_to_range(text, cursor, slice);
                lines.push(mapped.clone());
                cursor = mapped.end;
            }
        }
    }
    lines
}

fn skip_leading_whitespace(text: &str, from: usize) -> usize {
    text[from..]
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(text.len(), |(i, _)| from + i)
}

fn map_owned_to_range(text: &str, cursor: usize, wrapped: &str) -> Range<usize> {
    let mut start = cursor;
    while start < text.len() && !wrapped.starts_with(' ') {
        let Some(ch) = text[start..].chars().next() else {
            break;
        };
        if ch != ' ' {
            break;
        }
        start += ch.len_utf8();
    }
    let mut end = start;
    let mut saw_source_char = false;
    let mut chars = wrapped.chars().peekable();
    while let Some(ch) = chars.next() {
        if end < text.len() {
            let src = text[end..].chars().next().unwrap_or('\0');
            if ch == src {
                end += src.len_utf8();
                saw_source_char = true;
                continue;
            }
        }
        if ch == '-' && chars.peek().is_none() {
            continue;
        }
        if !saw_source_char {
            continue;
        }
        break;
    }
    start..end
}

fn slice_line_spans<'a>(
    original: &'a Line<'a>,
    span_bounds: &[(Range<usize>, Style)],
    range: &Range<usize>,
) -> Line<'a> {
    let start_byte = range.start;
    let end_byte = range.end;
    let mut acc: Vec<Span<'a>> = Vec::new();
    for (i, (range, style)) in span_bounds.iter().enumerate() {
        let s = range.start;
        let e = range.end;
        if e <= start_byte {
            continue;
        }
        if s >= end_byte {
            break;
        }
        let seg_start = start_byte.max(s);
        let seg_end = end_byte.min(e);
        if seg_end > seg_start {
            let local_start = seg_start - s;
            let local_end = seg_end - s;
            let content = original.spans[i].content.as_ref();
            let slice = &content[local_start..local_end];
            acc.push(Span {
                style: *style,
                content: Cow::Borrowed(slice),
            });
        }
        if e >= end_byte {
            break;
        }
    }
    Line {
        style: original.style,
        alignment: original.alignment,
        spans: acc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concat_line(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn url_token_kept_intact() {
        let line = Line::from("https://example.com/very/long/path/with/many/segments");
        let out = adaptive_wrap_line(&line, RtOptions::new(20));
        assert_eq!(out.len(), 1);
        assert!(concat_line(&out[0]).contains("https://example.com"));
    }

    #[test]
    fn non_url_long_token_wraps() {
        let line = Line::from("a_very_long_token_without_spaces_to_force_wrapping");
        let out = adaptive_wrap_line(&line, RtOptions::new(20));
        assert!(out.len() > 1);
    }

    #[test]
    fn mixed_line_keeps_url_breaks_other_words() {
        let line = Line::from("see https://ex.com/path tail words here");
        let out = adaptive_wrap_line(&line, RtOptions::new(20));
        assert!(
            out.iter()
                .any(|l| concat_line(l).contains("https://ex.com")),
            "URL must remain on a single physical line"
        );
    }

    #[test]
    fn plain_prose_wraps_at_word_boundaries() {
        let line = Line::from("hello world foo bar baz");
        let out = adaptive_wrap_line(&line, RtOptions::new(10));
        assert!(out.len() > 1);
        for l in &out {
            assert!(!concat_line(l).starts_with(' '));
        }
    }

    #[test]
    fn empty_line_yields_single_empty_output() {
        let line = Line::from("");
        let out = adaptive_wrap_line(&line, RtOptions::new(10));
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "");
    }

    #[test]
    fn line_contains_url_like_detects_across_spans() {
        let line = Line::from(vec![
            "see ".into(),
            "https://example.com/a/b".into(),
            " for details".into(),
        ]);
        assert!(line_contains_url_like(&line));
    }

    #[test]
    fn line_contains_url_like_rejects_file_paths() {
        let line = Line::from("src/main.rs:42");
        assert!(!line_contains_url_like(&line));
    }

    #[test]
    fn mixed_detection_ignores_pipe_decoration() {
        let line = Line::from("  │ https://example.com/path");
        assert!(!line_has_mixed_url_and_non_url_tokens(&line));
    }

    #[test]
    fn mixed_detection_finds_real_prose() {
        let line = Line::from("see https://example.com for details");
        assert!(line_has_mixed_url_and_non_url_tokens(&line));
    }
}
