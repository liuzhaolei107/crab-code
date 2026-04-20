use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::traits::Renderable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenWarningLevel {
    Normal,
    Warning,
    Critical,
}

pub struct TokenWarning {
    level: TokenWarningLevel,
    used: u64,
    limit: u64,
}

impl TokenWarning {
    #[must_use]
    pub fn new(used: u64, limit: u64) -> Self {
        let pct = if limit > 0 {
            (used as f64 / limit as f64) * 100.0
        } else {
            0.0
        };
        let level = if pct >= 90.0 {
            TokenWarningLevel::Critical
        } else if pct >= 80.0 {
            TokenWarningLevel::Warning
        } else {
            TokenWarningLevel::Normal
        };
        Self { level, used, limit }
    }

    #[must_use]
    pub fn level(&self) -> TokenWarningLevel {
        self.level
    }

    #[must_use]
    pub fn percentage(&self) -> f64 {
        if self.limit > 0 {
            (self.used as f64 / self.limit as f64) * 100.0
        } else {
            0.0
        }
    }

    fn message(&self) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let pct = self.percentage().round() as u32;
        match self.level {
            TokenWarningLevel::Normal => String::new(),
            TokenWarningLevel::Warning => format!("⚠ Context {pct}% full"),
            TokenWarningLevel::Critical => {
                format!("⚠ Context {pct}% full — auto-compact soon")
            }
        }
    }
}

impl Renderable for TokenWarning {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if self.level == TokenWarningLevel::Normal || area.width == 0 || area.height == 0 {
            return;
        }

        let msg = self.message();
        let style = match self.level {
            TokenWarningLevel::Warning => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            TokenWarningLevel::Critical => {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            }
            TokenWarningLevel::Normal => unreachable!(),
        };

        let line = Line::from(vec![Span::styled(msg, style)]);
        Widget::render(line, area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        u16::from(self.level != TokenWarningLevel::Normal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_below_80() {
        let w = TokenWarning::new(7000, 10000);
        assert_eq!(w.level(), TokenWarningLevel::Normal);
        assert_eq!(w.desired_height(80), 0);
    }

    #[test]
    fn warning_at_80() {
        let w = TokenWarning::new(8000, 10000);
        assert_eq!(w.level(), TokenWarningLevel::Warning);
        assert!(w.message().contains("80%"));
    }

    #[test]
    fn critical_at_90() {
        let w = TokenWarning::new(9500, 10000);
        assert_eq!(w.level(), TokenWarningLevel::Critical);
        assert!(w.message().contains("auto-compact"));
    }

    #[test]
    fn zero_limit() {
        let w = TokenWarning::new(100, 0);
        assert_eq!(w.level(), TokenWarningLevel::Normal);
    }

    #[test]
    fn render_no_panic() {
        let w = TokenWarning::new(9000, 10000);
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        w.render(area, &mut buf);
    }
}
