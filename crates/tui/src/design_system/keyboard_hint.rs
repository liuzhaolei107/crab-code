use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

pub struct KeyboardHint;

impl KeyboardHint {
    #[must_use]
    pub fn styled(text: &str) -> Span<'static> {
        Span::styled(
            format!("[{text}]"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    }

    #[must_use]
    pub fn with_label(key: &str, label: &str) -> Vec<Span<'static>> {
        vec![
            Span::styled(
                format!("[{key}]"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {label}"), Style::default().fg(Color::DarkGray)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_wraps_in_brackets() {
        let span = KeyboardHint::styled("Ctrl+K");
        assert_eq!(&*span.content, "[Ctrl+K]");
    }

    #[test]
    fn with_label_two_spans() {
        let spans = KeyboardHint::with_label("Enter", "Select");
        assert_eq!(spans.len(), 2);
        assert!(spans[0].content.contains("[Enter]"));
        assert!(spans[1].content.contains("Select"));
    }
}
