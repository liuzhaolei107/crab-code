use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::traits::Renderable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitAction {
    Upgrade,
    ExtraUsage,
    Login,
    Retry,
}

impl RateLimitAction {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Upgrade => "/upgrade",
            Self::ExtraUsage => "/extra-usage",
            Self::Login => "/login",
            Self::Retry => "Retry",
        }
    }

    #[must_use]
    pub const fn shortcut(self) -> char {
        match self {
            Self::Upgrade => 'u',
            Self::ExtraUsage => 'x',
            Self::Login => 'l',
            Self::Retry => 'r',
        }
    }
}

const ACTIONS: [RateLimitAction; 4] = [
    RateLimitAction::Upgrade,
    RateLimitAction::ExtraUsage,
    RateLimitAction::Login,
    RateLimitAction::Retry,
];

pub struct RateLimitCard {
    pub message: String,
    pub retry_after_secs: Option<u64>,
    pub selected: usize,
}

impl RateLimitCard {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retry_after_secs: None,
            selected: 0,
        }
    }

    #[must_use]
    pub fn with_retry_after(mut self, secs: u64) -> Self {
        self.retry_after_secs = Some(secs);
        self
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1).min(ACTIONS.len() - 1);
    }

    #[must_use]
    pub fn selected_action(&self) -> RateLimitAction {
        ACTIONS[self.selected]
    }

    #[must_use]
    pub fn action_for_key(c: char) -> Option<RateLimitAction> {
        ACTIONS.iter().find(|a| a.shortcut() == c).copied()
    }
}

impl Renderable for RateLimitCard {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 20 {
            return;
        }

        let msg_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let msg_line = Line::from(vec![
            Span::styled("⚠ ", msg_style),
            Span::styled(self.message.clone(), msg_style),
        ]);
        let msg_row = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        Widget::render(msg_line, msg_row, buf);

        if let Some(secs) = self.retry_after_secs.filter(|_| area.height > 1) {
            let retry_line = Line::from(vec![Span::styled(
                format!("  Retry in {secs}s"),
                Style::default().fg(Color::DarkGray),
            )]);
            let retry_row = Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: 1,
            };
            Widget::render(retry_line, retry_row, buf);
        }

        let actions_start = if self.retry_after_secs.is_some() {
            2
        } else {
            1
        };
        for (i, action) in ACTIONS.iter().enumerate() {
            let row_y = area.y + actions_start + i as u16;
            if row_y >= area.bottom() {
                break;
            }
            let style = if i == self.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let line = Line::from(vec![Span::styled(
                format!("  ({}) {}", action.shortcut(), action.label()),
                style,
            )]);
            let row = Rect {
                x: area.x,
                y: row_y,
                width: area.width,
                height: 1,
            };
            Widget::render(line, row, buf);
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        let base = if self.retry_after_secs.is_some() {
            2
        } else {
            1
        };
        base + ACTIONS.len() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation() {
        let mut card = RateLimitCard::new("Rate limited");
        assert_eq!(card.selected_action(), RateLimitAction::Upgrade);
        card.move_down();
        assert_eq!(card.selected_action(), RateLimitAction::ExtraUsage);
        card.move_up();
        assert_eq!(card.selected_action(), RateLimitAction::Upgrade);
    }

    #[test]
    fn key_shortcuts() {
        assert_eq!(
            RateLimitCard::action_for_key('u'),
            Some(RateLimitAction::Upgrade)
        );
        assert_eq!(
            RateLimitCard::action_for_key('r'),
            Some(RateLimitAction::Retry)
        );
        assert_eq!(RateLimitCard::action_for_key('z'), None);
    }

    #[test]
    fn desired_height() {
        let card = RateLimitCard::new("test");
        assert_eq!(card.desired_height(80), 5);

        let card_retry = RateLimitCard::new("test").with_retry_after(30);
        assert_eq!(card_retry.desired_height(80), 6);
    }

    #[test]
    fn render_no_panic() {
        let card = RateLimitCard::new("Rate limit exceeded").with_retry_after(30);
        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);
        card.render(area, &mut buf);
    }
}
