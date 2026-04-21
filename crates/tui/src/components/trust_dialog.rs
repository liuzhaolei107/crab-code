//! Project trust dialog overlay — confirms user trusts project-level settings.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Widget};

use crate::app_event::AppEvent;
use crate::keybindings::KeyContext;
use crate::overlay::{Overlay, OverlayAction};
use crate::traits::Renderable;

pub struct TrustDialogOverlay {
    project_path: String,
    has_settings: bool,
    has_crab_md: bool,
    selected: usize,
}

impl TrustDialogOverlay {
    pub fn new(project_path: String, has_settings: bool, has_crab_md: bool) -> Self {
        Self {
            project_path,
            has_settings,
            has_crab_md,
            selected: 0,
        }
    }
}

impl Renderable for TrustDialogOverlay {
    #[allow(clippy::cast_possible_truncation)]
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let popup = centered_popup(area, 64, 16);
        Widget::render(Clear, popup, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Trust This Project? ")
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::new(2, 2, 1, 1));
        let inner = block.inner(popup);
        Widget::render(block, popup, buf);

        let mut lines: Vec<Line<'_>> = vec![
            Line::from(""),
            Line::from(Span::styled(
                &*self.project_path,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "This project contains configuration that will",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "affect how Crab Code behaves:",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
        ];

        if self.has_settings {
            lines.push(Line::from(vec![
                Span::styled("  \u{2022} ", Style::default().fg(Color::Yellow)),
                Span::styled(".crab/settings.json", Style::default().fg(Color::White)),
            ]));
        }
        if self.has_crab_md {
            lines.push(Line::from(vec![
                Span::styled("  \u{2022} ", Style::default().fg(Color::Yellow)),
                Span::styled("CRAB.md", Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(""));

        let accept_style = if self.selected == 0 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let deny_style = if self.selected == 1 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        lines.push(Line::from(vec![
            Span::styled("  [ Trust ] ", accept_style),
            Span::styled("  ", Style::default()),
            Span::styled("[ Skip (bare mode) ] ", deny_style),
        ]));

        for (i, line) in lines.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            Widget::render(
                line.clone(),
                Rect {
                    x: inner.x,
                    y: inner.y + i as u16,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        16
    }
}

impl Overlay for TrustDialogOverlay {
    fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Char('y' | 'Y') => OverlayAction::Execute(AppEvent::TrustAccepted {
                project_path: self.project_path.clone(),
            }),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                OverlayAction::Execute(AppEvent::TrustDenied)
            }
            KeyCode::Enter => {
                if self.selected == 0 {
                    OverlayAction::Execute(AppEvent::TrustAccepted {
                        project_path: self.project_path.clone(),
                    })
                } else {
                    OverlayAction::Execute(AppEvent::TrustDenied)
                }
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                self.selected = 1 - self.selected;
                OverlayAction::Consumed
            }
            _ => OverlayAction::Passthrough,
        }
    }

    fn contexts(&self) -> Vec<KeyContext> {
        vec![KeyContext::Permission]
    }

    fn name(&self) -> &'static str {
        "trust_dialog"
    }
}

fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn trust_y_accepts() {
        let mut overlay = TrustDialogOverlay::new("/my/project".into(), true, true);
        let action = overlay.handle_key(key(KeyCode::Char('y')));
        assert!(matches!(
            action,
            OverlayAction::Execute(AppEvent::TrustAccepted { .. })
        ));
    }

    #[test]
    fn trust_n_denies() {
        let mut overlay = TrustDialogOverlay::new("/my/project".into(), true, false);
        let action = overlay.handle_key(key(KeyCode::Char('n')));
        assert!(matches!(
            action,
            OverlayAction::Execute(AppEvent::TrustDenied)
        ));
    }

    #[test]
    fn trust_esc_denies() {
        let mut overlay = TrustDialogOverlay::new("/my/project".into(), false, true);
        let action = overlay.handle_key(key(KeyCode::Esc));
        assert!(matches!(
            action,
            OverlayAction::Execute(AppEvent::TrustDenied)
        ));
    }

    #[test]
    fn trust_enter_default_accepts() {
        let mut overlay = TrustDialogOverlay::new("/my/project".into(), true, true);
        let action = overlay.handle_key(key(KeyCode::Enter));
        assert!(matches!(
            action,
            OverlayAction::Execute(AppEvent::TrustAccepted { .. })
        ));
    }

    #[test]
    fn trust_tab_toggles() {
        let mut overlay = TrustDialogOverlay::new("/my/project".into(), true, true);
        assert_eq!(overlay.selected, 0);

        overlay.handle_key(key(KeyCode::Tab));
        assert_eq!(overlay.selected, 1);

        let action = overlay.handle_key(key(KeyCode::Enter));
        assert!(matches!(
            action,
            OverlayAction::Execute(AppEvent::TrustDenied)
        ));
    }

    #[test]
    fn trust_render_no_panic() {
        let overlay = TrustDialogOverlay::new("/test/project".into(), true, true);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        overlay.render(area, &mut buf);
    }
}
