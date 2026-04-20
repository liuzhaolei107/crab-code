use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::traits::Renderable;

pub struct ProgressBar {
    progress: f64,
    filled_color: Color,
    empty_color: Color,
    show_percentage: bool,
}

impl ProgressBar {
    #[must_use]
    pub fn new(progress: f64) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            filled_color: Color::Cyan,
            empty_color: Color::DarkGray,
            show_percentage: true,
        }
    }

    #[must_use]
    pub fn filled_color(mut self, color: Color) -> Self {
        self.filled_color = color;
        self
    }

    #[must_use]
    pub fn empty_color(mut self, color: Color) -> Self {
        self.empty_color = color;
        self
    }

    #[must_use]
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }
}

impl Renderable for ProgressBar {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let pct_text = if self.show_percentage {
            format!(" {:.0}%", self.progress * 100.0)
        } else {
            String::new()
        };
        let bar_width = (area.width as usize).saturating_sub(pct_text.len());

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let filled = (bar_width as f64 * self.progress).round() as usize;
        let empty = bar_width.saturating_sub(filled);

        let filled_style = Style::default().bg(self.filled_color).fg(Color::Black);
        let empty_style = Style::default().bg(self.empty_color).fg(Color::DarkGray);

        let y = area.y;
        let mut x = area.x;

        for _ in 0..filled {
            if x >= area.right() {
                break;
            }
            buf[(x, y)].set_char('━').set_style(filled_style);
            x += 1;
        }
        for _ in 0..empty {
            if x >= area.right() {
                break;
            }
            buf[(x, y)].set_char('━').set_style(empty_style);
            x += 1;
        }

        if self.show_percentage {
            let pct_style = Style::default().fg(Color::White);
            for ch in pct_text.chars() {
                if x >= area.right() {
                    break;
                }
                buf[(x, y)].set_char(ch).set_style(pct_style);
                x += 1;
            }
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

impl Widget for ProgressBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Renderable::render(&self, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_progress() {
        let bar = ProgressBar::new(1.5);
        assert!((bar.progress - 1.0).abs() < f64::EPSILON);

        let bar2 = ProgressBar::new(-0.5);
        assert!(bar2.progress.abs() < f64::EPSILON);
    }

    #[test]
    fn desired_height_one() {
        let bar = ProgressBar::new(0.5);
        assert_eq!(bar.desired_height(80), 1);
    }

    #[test]
    fn render_no_panic() {
        let bar = ProgressBar::new(0.75).filled_color(Color::Green);
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        Renderable::render(&bar, area, &mut buf);
    }

    #[test]
    fn zero_width_no_panic() {
        let bar = ProgressBar::new(0.5);
        let area = Rect::new(0, 0, 0, 1);
        let mut buf = Buffer::empty(area);
        Renderable::render(&bar, area, &mut buf);
    }
}
