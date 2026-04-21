//! First-run onboarding overlay — multi-screen welcome wizard.

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

const TOTAL_STEPS: usize = 3;

pub struct OnboardingOverlay {
    step: usize,
}

impl OnboardingOverlay {
    pub fn new() -> Self {
        Self { step: 0 }
    }
}

impl Renderable for OnboardingOverlay {
    #[allow(clippy::cast_possible_truncation)]
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let popup = centered_popup(area, 64, 18);
        Widget::render(Clear, popup, buf);

        let title = match self.step {
            0 => " Welcome to Crab Code ",
            1 => " Theme ",
            2 => " Keybindings ",
            _ => " Welcome ",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::new(2, 2, 1, 1));
        let inner = block.inner(popup);
        Widget::render(block, popup, buf);

        let lines = content_lines(self.step);
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

        let footer = footer_line(self.step);
        if inner.height > 1 {
            Widget::render(
                footer,
                Rect {
                    x: inner.x,
                    y: inner.y + inner.height - 1,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        18
    }
}

impl Overlay for OnboardingOverlay {
    fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc => OverlayAction::Execute(AppEvent::OnboardingCompleted),
            KeyCode::Enter | KeyCode::Right => {
                if self.step + 1 >= TOTAL_STEPS {
                    OverlayAction::Execute(AppEvent::OnboardingCompleted)
                } else {
                    self.step += 1;
                    OverlayAction::Consumed
                }
            }
            KeyCode::Left => {
                self.step = self.step.saturating_sub(1);
                OverlayAction::Consumed
            }
            _ => OverlayAction::Passthrough,
        }
    }

    fn contexts(&self) -> Vec<KeyContext> {
        vec![KeyContext::ScrollBox]
    }

    fn name(&self) -> &'static str {
        "onboarding"
    }
}

fn content_lines(step: usize) -> Vec<Line<'static>> {
    match step {
        0 => vec![
            Line::from(""),
            Line::from(Span::styled(
                "Crab Code — Agentic Coding CLI",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "An open-source, Rust-native coding assistant",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "supporting any LLM provider.",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Enter  ", Style::default().fg(Color::Cyan)),
                Span::styled("Submit prompt", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+C ", Style::default().fg(Color::Cyan)),
                Span::styled("Cancel current request", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  /help  ", Style::default().fg(Color::Cyan)),
                Span::styled("Show all commands", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Esc    ", Style::default().fg(Color::Cyan)),
                Span::styled("Close overlays / cancel", Style::default().fg(Color::White)),
            ]),
        ],
        1 => vec![
            Line::from(""),
            Line::from(Span::styled(
                "Color Theme",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Crab Code auto-detects your terminal theme.",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "If colors look wrong, you can override it:",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  settings.json: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "\"theme\": \"dark\" | \"light\"",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Run /config to open settings.",
                Style::default().fg(Color::DarkGray),
            )),
        ],
        2 => vec![
            Line::from(""),
            Line::from(Span::styled(
                "Keybindings",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Ctrl+R  ", Style::default().fg(Color::Cyan)),
                Span::styled("Search input history", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+L  ", Style::default().fg(Color::Cyan)),
                Span::styled("Clear screen", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+X  ", Style::default().fg(Color::Cyan)),
                Span::styled("External editor", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Alt+P   ", Style::default().fg(Color::Cyan)),
                Span::styled("Model picker", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Vim keybindings available in settings.",
                Style::default().fg(Color::DarkGray),
            )),
        ],
        _ => vec![],
    }
}

fn footer_line(step: usize) -> Line<'static> {
    let dots: String = (0..TOTAL_STEPS)
        .map(|i| if i == step { "\u{25CF}" } else { "\u{25CB}" })
        .collect::<Vec<_>>()
        .join(" ");

    let nav = if step + 1 >= TOTAL_STEPS {
        "Enter: finish"
    } else {
        "Enter/\u{2192}: next  \u{2190}: back  Esc: skip"
    };

    Line::from(vec![
        Span::styled(dots, Style::default().fg(Color::Cyan)),
        Span::styled("  ", Style::default()),
        Span::styled(nav, Style::default().fg(Color::DarkGray)),
    ])
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
    fn onboarding_advance_steps() {
        let mut overlay = OnboardingOverlay::new();
        assert_eq!(overlay.step, 0);

        assert!(matches!(
            overlay.handle_key(key(KeyCode::Enter)),
            OverlayAction::Consumed
        ));
        assert_eq!(overlay.step, 1);

        overlay.handle_key(key(KeyCode::Right));
        assert_eq!(overlay.step, 2);

        assert!(matches!(
            overlay.handle_key(key(KeyCode::Enter)),
            OverlayAction::Execute(AppEvent::OnboardingCompleted)
        ));
    }

    #[test]
    fn onboarding_back() {
        let mut overlay = OnboardingOverlay::new();
        overlay.step = 2;

        overlay.handle_key(key(KeyCode::Left));
        assert_eq!(overlay.step, 1);

        overlay.handle_key(key(KeyCode::Left));
        assert_eq!(overlay.step, 0);

        overlay.handle_key(key(KeyCode::Left));
        assert_eq!(overlay.step, 0);
    }

    #[test]
    fn onboarding_esc_completes() {
        let mut overlay = OnboardingOverlay::new();
        assert!(matches!(
            overlay.handle_key(key(KeyCode::Esc)),
            OverlayAction::Execute(AppEvent::OnboardingCompleted)
        ));
    }

    #[test]
    fn onboarding_render_no_panic() {
        let overlay = OnboardingOverlay::new();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        overlay.render(area, &mut buf);
    }

    #[test]
    fn onboarding_render_all_steps() {
        let mut overlay = OnboardingOverlay::new();
        let area = Rect::new(0, 0, 80, 24);
        for step in 0..TOTAL_STEPS {
            overlay.step = step;
            let mut buf = Buffer::empty(area);
            overlay.render(area, &mut buf);
        }
    }
}
