use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::keybindings::KeyContext;

use super::OverlayAction;

pub struct TranscriptState {
    pub scroll_offset: usize,
}

impl TranscriptState {
    #[must_use]
    pub fn new() -> Self {
        Self { scroll_offset: 0 }
    }
}

impl Default for TranscriptState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HelpState {
    pub scroll_offset: usize,
}

impl HelpState {
    #[must_use]
    pub fn new() -> Self {
        Self { scroll_offset: 0 }
    }
}

impl Default for HelpState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PermissionDialogState {
    pub request_id: String,
    pub tool_name: String,
    pub summary: String,
    pub selected_option: usize,
}

impl PermissionDialogState {
    #[must_use]
    pub fn new(request_id: String, tool_name: String, summary: String) -> Self {
        Self {
            request_id,
            tool_name,
            summary,
            selected_option: 0,
        }
    }
}

pub enum OverlayKind {
    Transcript(TranscriptState),
    Help(HelpState),
    Permission(PermissionDialogState),
}

impl OverlayKind {
    pub fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match self {
            Self::Transcript(state) => handle_transcript_key(state, key),
            Self::Help(state) => handle_help_key(state, key),
            Self::Permission(state) => handle_permission_key(state, key),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Transcript(_state) => render_transcript(area, buf),
            Self::Help(_state) => render_help(area, buf),
            Self::Permission(state) => render_permission(state, area, buf),
        }
    }

    pub fn contexts(&self) -> Vec<KeyContext> {
        match self {
            Self::Transcript(_) => vec![KeyContext::Transcript],
            Self::Help(_) => vec![KeyContext::Help],
            Self::Permission(_) => vec![KeyContext::Permission],
        }
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Transcript(_) => "transcript",
            Self::Help(_) => "help",
            Self::Permission(_) => "permission",
        }
    }
}

fn handle_transcript_key(state: &mut TranscriptState, key: KeyEvent) -> OverlayAction {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => OverlayAction::Dismiss,
        KeyCode::Up | KeyCode::Char('k') => {
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
            OverlayAction::Consumed
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.scroll_offset = state.scroll_offset.saturating_add(1);
            OverlayAction::Consumed
        }
        _ => OverlayAction::Passthrough,
    }
}

fn handle_help_key(state: &mut HelpState, key: KeyEvent) -> OverlayAction {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => OverlayAction::Dismiss,
        KeyCode::Up | KeyCode::Char('k') => {
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
            OverlayAction::Consumed
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.scroll_offset = state.scroll_offset.saturating_add(1);
            OverlayAction::Consumed
        }
        _ => OverlayAction::Passthrough,
    }
}

fn handle_permission_key(state: &mut PermissionDialogState, key: KeyEvent) -> OverlayAction {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Char('y' | 'n') | KeyCode::Enter | KeyCode::Esc => OverlayAction::Dismiss,
        KeyCode::Up => {
            state.selected_option = state.selected_option.saturating_sub(1);
            OverlayAction::Consumed
        }
        KeyCode::Down => {
            state.selected_option = (state.selected_option + 1).min(3);
            OverlayAction::Consumed
        }
        _ => OverlayAction::Passthrough,
    }
}

fn render_transcript(_area: Rect, _buf: &mut Buffer) {
    // Rendering will be implemented in Phase 3
}

fn render_help(_area: Rect, _buf: &mut Buffer) {
    // Rendering will be implemented in Phase 3
}

fn render_permission(_state: &PermissionDialogState, _area: Rect, _buf: &mut Buffer) {
    // Rendering will be implemented in Phase 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn transcript_esc_dismisses() {
        let mut overlay = OverlayKind::Transcript(TranscriptState::new());
        assert!(matches!(
            overlay.handle_key(key(KeyCode::Esc)),
            OverlayAction::Dismiss
        ));
    }

    #[test]
    fn transcript_scroll() {
        let mut overlay = OverlayKind::Transcript(TranscriptState::new());
        overlay.handle_key(key(KeyCode::Down));
        if let OverlayKind::Transcript(s) = &overlay {
            assert_eq!(s.scroll_offset, 1);
        }
    }

    #[test]
    fn help_q_dismisses() {
        let mut overlay = OverlayKind::Help(HelpState::new());
        assert!(matches!(
            overlay.handle_key(key(KeyCode::Char('q'))),
            OverlayAction::Dismiss
        ));
    }

    #[test]
    fn permission_y_dismisses() {
        let mut overlay = OverlayKind::Permission(PermissionDialogState::new(
            "1".into(),
            "Bash".into(),
            "ls".into(),
        ));
        assert!(matches!(
            overlay.handle_key(key(KeyCode::Char('y'))),
            OverlayAction::Dismiss
        ));
    }

    #[test]
    fn permission_nav() {
        let mut overlay = OverlayKind::Permission(PermissionDialogState::new(
            "1".into(),
            "Bash".into(),
            "ls".into(),
        ));
        overlay.handle_key(key(KeyCode::Down));
        if let OverlayKind::Permission(s) = &overlay {
            assert_eq!(s.selected_option, 1);
        }
    }

    #[test]
    fn overlay_names() {
        assert_eq!(
            OverlayKind::Transcript(TranscriptState::new()).name(),
            "transcript"
        );
        assert_eq!(OverlayKind::Help(HelpState::new()).name(), "help");
        assert_eq!(
            OverlayKind::Permission(PermissionDialogState::new(
                "1".into(),
                "B".into(),
                "s".into()
            ))
            .name(),
            "permission"
        );
    }

    #[test]
    fn overlay_contexts() {
        assert_eq!(
            OverlayKind::Transcript(TranscriptState::new()).contexts(),
            vec![KeyContext::Transcript]
        );
        assert_eq!(
            OverlayKind::Help(HelpState::new()).contexts(),
            vec![KeyContext::Help]
        );
    }
}
