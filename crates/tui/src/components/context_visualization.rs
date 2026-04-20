use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::traits::Renderable;

pub struct ContextVisualization {
    summarized_spans: usize,
    staged_count: usize,
    error_count: usize,
    visible: bool,
}

impl ContextVisualization {
    #[must_use]
    pub fn new() -> Self {
        Self {
            summarized_spans: 0,
            staged_count: 0,
            error_count: 0,
            visible: false,
        }
    }

    pub fn update(&mut self, summarized_spans: usize, staged_count: usize, error_count: usize) {
        self.summarized_spans = summarized_spans;
        self.staged_count = staged_count;
        self.error_count = error_count;
        self.visible = summarized_spans > 0;
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for ContextVisualization {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for ContextVisualization {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible || area.height == 0 || area.width == 0 {
            return;
        }

        let mut spans = vec![
            Span::styled(
                " ⊘ Context compacted: ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
            Span::styled(
                format!("{} spans summarized", self.summarized_spans),
                Style::default().fg(Color::Cyan),
            ),
        ];

        if self.staged_count > 0 {
            spans.push(Span::styled(
                format!(" · {} staged", self.staged_count),
                Style::default().fg(Color::Yellow),
            ));
        }

        if self.error_count > 0 {
            spans.push(Span::styled(
                format!(" · {} errors", self.error_count),
                Style::default().fg(Color::Red),
            ));
        }

        let line = Line::from(spans);
        let row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        Widget::render(line, row, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        u16::from(self.visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_by_default() {
        let cv = ContextVisualization::new();
        assert!(!cv.is_visible());
        assert_eq!(cv.desired_height(80), 0);
    }

    #[test]
    fn visible_after_compact() {
        let mut cv = ContextVisualization::new();
        cv.update(5, 2, 0);
        assert!(cv.is_visible());
        assert_eq!(cv.desired_height(80), 1);
    }

    #[test]
    fn render_no_panic() {
        let mut cv = ContextVisualization::new();
        cv.update(10, 3, 1);
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        cv.render(area, &mut buf);
    }
}
