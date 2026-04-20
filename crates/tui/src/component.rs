use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::action::Action;
use crate::event::TuiEvent;

pub trait Component {
    fn render(&mut self, f: &mut Frame, area: Rect);

    fn focus(&mut self, _focused: bool) {}

    fn is_hover(&self, mouse_pos: (u16, u16), area: Rect) -> bool {
        let pos = ratatui::layout::Position::new(mouse_pos.0, mouse_pos.1);
        area.contains(pos)
    }

    fn keybindings(&self) -> Vec<(KeyEvent, Action)> {
        vec![]
    }

    fn handle_action(&mut self, _action: Action) -> bool {
        false
    }

    fn handle_event(&mut self, event: &TuiEvent) -> bool {
        if let TuiEvent::Key(key) = event {
            for (binding, action) in self.keybindings() {
                if binding == *key {
                    return self.handle_action(action);
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    struct StubComponent {
        focused: bool,
    }

    impl Component for StubComponent {
        fn render(&mut self, _f: &mut Frame, _area: Rect) {}

        fn focus(&mut self, focused: bool) {
            self.focused = focused;
        }

        fn keybindings(&self) -> Vec<(KeyEvent, Action)> {
            vec![(
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
                Action::Quit,
            )]
        }

        fn handle_action(&mut self, action: Action) -> bool {
            matches!(action, Action::Quit)
        }
    }

    #[test]
    fn default_handle_event_dispatches_keybinding() {
        let mut comp = StubComponent { focused: false };
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert!(comp.handle_event(&TuiEvent::Key(key)));
    }

    #[test]
    fn default_handle_event_ignores_unbound_key() {
        let mut comp = StubComponent { focused: false };
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!comp.handle_event(&TuiEvent::Key(key)));
    }

    #[test]
    fn focus_callback() {
        let mut comp = StubComponent { focused: false };
        comp.focus(true);
        assert!(comp.focused);
    }

    #[test]
    fn is_hover_default_inside() {
        let comp = StubComponent { focused: false };
        let area = Rect::new(10, 10, 20, 5);
        assert!(comp.is_hover((15, 12), area));
    }

    #[test]
    fn is_hover_default_outside() {
        let comp = StubComponent { focused: false };
        let area = Rect::new(10, 10, 20, 5);
        assert!(!comp.is_hover((5, 5), area));
    }
}
