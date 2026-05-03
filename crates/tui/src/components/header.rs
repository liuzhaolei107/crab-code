//! Thin horizontal separator used between layout regions.
//!
//! The persistent header bar was retired when the TUI moved to inline
//! viewport rendering — model name and cwd now live in the welcome
//! cell at session start. This module keeps `render_separator`, which
//! is still used between the status line and the input box and between
//! the input box and the bottom bar.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

/// Render a thin horizontal separator line spanning `area.width`.
#[allow(clippy::cast_possible_truncation)]
pub fn render_separator(area: Rect, buf: &mut Buffer) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let sep = "─".repeat(area.width as usize);
    Widget::render(
        Line::from(Span::styled(&*sep, Style::default().fg(Color::DarkGray))),
        area,
        buf,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_separator_does_not_panic_on_empty_area() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 0, 0));
        render_separator(Rect::new(0, 0, 0, 0), &mut buf);
    }

    #[test]
    fn render_separator_fills_width() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        render_separator(area, &mut buf);
        let row: String = (0..area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert_eq!(row, "─────");
    }
}
