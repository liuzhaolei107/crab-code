use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Widget};

use crate::traits::Renderable;

pub struct MessagePill {
    unseen_count: usize,
    at_bottom: bool,
}

impl MessagePill {
    #[must_use]
    pub fn new(unseen_count: usize, at_bottom: bool) -> Self {
        Self {
            unseen_count,
            at_bottom,
        }
    }

    #[must_use]
    pub fn should_show(&self) -> bool {
        !self.at_bottom
    }

    fn text(&self) -> String {
        if self.unseen_count > 0 {
            format!("↓ {} new messages", self.unseen_count)
        } else {
            "Jump to bottom".to_string()
        }
    }
}

impl Renderable for MessagePill {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.should_show() || area.height == 0 || area.width < 10 {
            return;
        }

        let text = self.text();
        let pill_width = (text.len() as u16 + 4).min(area.width);
        let x = area.x + (area.width.saturating_sub(pill_width)) / 2;
        let y = area.bottom().saturating_sub(2);

        let pill_area = Rect {
            x,
            y,
            width: pill_width,
            height: 1,
        };

        Widget::render(Clear, pill_area, buf);

        let style = if self.unseen_count > 0 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };

        let line = Line::from(vec![Span::styled(format!(" {text} "), style)]);
        Widget::render(line, pill_area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        u16::from(self.should_show())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_at_bottom() {
        let pill = MessagePill::new(0, true);
        assert!(!pill.should_show());
        assert_eq!(pill.desired_height(80), 0);
    }

    #[test]
    fn shows_unseen_count() {
        let pill = MessagePill::new(5, false);
        assert!(pill.should_show());
        assert!(pill.text().contains("5 new messages"));
    }

    #[test]
    fn jump_to_bottom_when_no_unseen() {
        let pill = MessagePill::new(0, false);
        assert!(pill.should_show());
        assert!(pill.text().contains("Jump to bottom"));
    }

    #[test]
    fn render_no_panic() {
        let pill = MessagePill::new(3, false);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        pill.render(area, &mut buf);
    }
}
