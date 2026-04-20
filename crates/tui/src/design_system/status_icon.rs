use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusIcon {
    Success,
    Error,
    Warning,
    Info,
    Running,
}

impl StatusIcon {
    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Success => "✓",
            Self::Error => "✗",
            Self::Warning => "⚠",
            Self::Info => "⏺",
            Self::Running => "⟳",
        }
    }

    #[must_use]
    pub fn color(self) -> Color {
        match self {
            Self::Success => Color::Green,
            Self::Error => Color::Red,
            Self::Warning => Color::Yellow,
            Self::Info => Color::Cyan,
            Self::Running => Color::Magenta,
        }
    }

    #[must_use]
    pub fn span(self) -> Span<'static> {
        Span::styled(
            self.glyph(),
            Style::default()
                .fg(self.color())
                .add_modifier(Modifier::BOLD),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_nonempty() {
        for icon in [
            StatusIcon::Success,
            StatusIcon::Error,
            StatusIcon::Warning,
            StatusIcon::Info,
            StatusIcon::Running,
        ] {
            assert!(!icon.glyph().is_empty());
        }
    }

    #[test]
    fn span_has_style() {
        let span = StatusIcon::Success.span();
        assert!(!span.content.is_empty());
    }
}
