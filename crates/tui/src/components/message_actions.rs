//! Per-message action popup menu.
//!
//! Opened on Alt+M, targets the most-recent user message by default. Handles
//! Copy / Edit / Delete / Rewind via the overlay trait — routes each choice
//! back through a `MessageAction` `AppEvent` that the App loop applies to
//! the targeted message index.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::app_event::AppEvent;
use crate::keybindings::KeyContext;
use crate::overlay::{Overlay, OverlayAction};
use crate::traits::Renderable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageAction {
    Copy,
    Edit,
    Delete,
    Rewind,
}

impl MessageAction {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Edit => "Edit",
            Self::Delete => "Delete",
            Self::Rewind => "Rewind to here",
        }
    }

    #[must_use]
    pub const fn shortcut(self) -> char {
        match self {
            Self::Copy => 'c',
            Self::Edit => 'e',
            Self::Delete => 'd',
            Self::Rewind => 'r',
        }
    }
}

const ALL_ACTIONS: [MessageAction; 4] = [
    MessageAction::Copy,
    MessageAction::Edit,
    MessageAction::Delete,
    MessageAction::Rewind,
];

pub struct MessageActionsMenu {
    pub message_index: usize,
    pub selected: usize,
}

impl MessageActionsMenu {
    #[must_use]
    pub fn new(message_index: usize) -> Self {
        Self {
            message_index,
            selected: 0,
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1).min(ALL_ACTIONS.len() - 1);
    }

    #[must_use]
    pub fn selected_action(&self) -> MessageAction {
        ALL_ACTIONS[self.selected]
    }

    #[must_use]
    pub fn action_for_key(c: char) -> Option<MessageAction> {
        ALL_ACTIONS.iter().find(|a| a.shortcut() == c).copied()
    }
}

impl Renderable for MessageActionsMenu {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let width = 24u16;
        let height = (ALL_ACTIONS.len() as u16) + 2;
        if area.width < width || area.height < height {
            return;
        }

        let popup_area = Rect {
            x: area.right().saturating_sub(width + 1),
            y: area.y,
            width,
            height,
        };

        Widget::render(Clear, popup_area, buf);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Actions ");
        let inner = block.inner(popup_area);
        Widget::render(block, popup_area, buf);

        for (i, action) in ALL_ACTIONS.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let style = if i == self.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let shortcut_style = if i == self.selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let line = Line::from(vec![
                Span::styled(format!(" ({}) ", action.shortcut()), shortcut_style),
                Span::styled(action.label().to_string(), style),
            ]);
            let row = Rect {
                x: inner.x,
                y: inner.y + i as u16,
                width: inner.width,
                height: 1,
            };
            Widget::render(line, row, buf);
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        ALL_ACTIONS.len() as u16 + 2
    }
}

impl MessageActionsMenu {
    fn event_for(&self, action: MessageAction) -> AppEvent {
        let index = self.message_index;
        match action {
            MessageAction::Copy => AppEvent::MessageCopy { index },
            MessageAction::Edit => AppEvent::MessageEdit { index },
            MessageAction::Delete => AppEvent::MessageDelete { index },
            MessageAction::Rewind => AppEvent::MessageRewind { index },
        }
    }
}

impl Overlay for MessageActionsMenu {
    fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => OverlayAction::Dismiss,
            KeyCode::Enter => OverlayAction::Execute(self.event_for(self.selected_action())),
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                OverlayAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                OverlayAction::Consumed
            }
            KeyCode::Char(c) => {
                if let Some(action) = Self::action_for_key(c) {
                    OverlayAction::Execute(self.event_for(action))
                } else {
                    OverlayAction::Passthrough
                }
            }
            _ => OverlayAction::Passthrough,
        }
    }

    fn contexts(&self) -> Vec<KeyContext> {
        vec![KeyContext::Chat]
    }

    fn name(&self) -> &'static str {
        "message_actions"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation() {
        let mut menu = MessageActionsMenu::new(0);
        assert_eq!(menu.selected_action(), MessageAction::Copy);
        menu.move_down();
        assert_eq!(menu.selected_action(), MessageAction::Edit);
        menu.move_down();
        menu.move_down();
        assert_eq!(menu.selected_action(), MessageAction::Rewind);
        menu.move_down();
        assert_eq!(menu.selected_action(), MessageAction::Rewind);
        menu.move_up();
        assert_eq!(menu.selected_action(), MessageAction::Delete);
    }

    #[test]
    fn key_shortcuts() {
        assert_eq!(
            MessageActionsMenu::action_for_key('c'),
            Some(MessageAction::Copy)
        );
        assert_eq!(
            MessageActionsMenu::action_for_key('e'),
            Some(MessageAction::Edit)
        );
        assert_eq!(MessageActionsMenu::action_for_key('x'), None);
    }

    #[test]
    fn render_no_panic() {
        let menu = MessageActionsMenu::new(0);
        let area = Rect::new(0, 0, 80, 10);
        let mut buf = Buffer::empty(area);
        menu.render(area, &mut buf);
    }

    #[test]
    fn labels_nonempty() {
        for action in &ALL_ACTIONS {
            assert!(!action.label().is_empty());
        }
    }
}
