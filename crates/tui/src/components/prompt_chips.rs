use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::traits::Renderable;

pub struct PromptChip {
    pub label: String,
}

impl PromptChip {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

pub struct PromptChips {
    chips: Vec<PromptChip>,
    visible: bool,
}

impl PromptChips {
    #[must_use]
    pub fn new() -> Self {
        Self {
            chips: default_chips(),
            visible: true,
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        self.chips.get(index).map(|c| c.label.as_str())
    }

    fn build_line(&self) -> Line<'static> {
        let chip_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        let num_style = Style::default().fg(Color::Yellow);

        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, chip) in self.chips.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(format!("[{}]", i + 1), num_style));
            spans.push(Span::styled(format!(" {}", chip.label), chip_style));
        }
        Line::from(spans)
    }
}

impl Default for PromptChips {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for PromptChips {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible || area.height == 0 || area.width == 0 {
            return;
        }
        let line = self.build_line();
        Widget::render(ratatui::widgets::Paragraph::new(line), area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        u16::from(self.visible)
    }
}

fn default_chips() -> Vec<PromptChip> {
    vec![
        PromptChip::new("Fix the build error"),
        PromptChip::new("Explain this code"),
        PromptChip::new("Write tests"),
        PromptChip::new("Refactor"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_by_default() {
        let chips = PromptChips::new();
        assert!(chips.is_visible());
        assert_eq!(chips.desired_height(80), 1);
    }

    #[test]
    fn hide_makes_invisible() {
        let mut chips = PromptChips::new();
        chips.hide();
        assert!(!chips.is_visible());
        assert_eq!(chips.desired_height(80), 0);
    }

    #[test]
    fn get_chip_by_index() {
        let chips = PromptChips::new();
        assert!(chips.get(0).unwrap().contains("Fix"));
        assert!(chips.get(99).is_none());
    }

    #[test]
    fn render_no_panic() {
        let chips = PromptChips::new();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        chips.render(area, &mut buf);
    }

    #[test]
    fn build_line_has_numbers() {
        let chips = PromptChips::new();
        let line = chips.build_line();
        let text: String = line.spans.iter().map(|s| &*s.content).collect();
        assert!(text.contains("[1]"));
        assert!(text.contains("[2]"));
    }
}
