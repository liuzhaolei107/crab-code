//! Resume browser overlay — picks a prior session to resume.
//!
//! Opened via `/resume` with no args. Lists known sessions (populated by
//! [`SessionSidebar`]) and emits [`AppEvent::SwitchSession`] on selection.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::app_event::AppEvent;
use crate::components::session_sidebar::SessionEntry;
use crate::keybindings::KeyContext;
use crate::overlay::{Overlay, OverlayAction};
use crate::traits::Renderable;

pub struct ResumeBrowserOverlay {
    entries: Vec<SessionEntry>,
    selected: usize,
    current_session_id: String,
}

impl ResumeBrowserOverlay {
    #[must_use]
    pub fn new(entries: Vec<SessionEntry>, current_session_id: String) -> Self {
        // Try to start the cursor on whichever entry isn't the current
        // session — otherwise the user has to press Down before they can do
        // anything meaningful.
        let selected = entries
            .iter()
            .position(|e| e.id != current_session_id)
            .unwrap_or(0);
        Self {
            entries,
            selected,
            current_session_id,
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    fn selected_id(&self) -> Option<&str> {
        self.entries.get(self.selected).map(|e| e.id.as_str())
    }
}

impl Renderable for ResumeBrowserOverlay {
    #[allow(clippy::cast_possible_truncation)]
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 4 || area.width < 30 {
            return;
        }
        Widget::render(Clear, area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                format!(" Resume Session ({}) ", self.entries.len()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        if self.entries.is_empty() {
            let dim = Style::default().fg(Color::DarkGray);
            buf.set_line(
                inner.x,
                inner.y,
                &Line::from(Span::styled("  No prior sessions found.", dim)),
                inner.width,
            );
            return;
        }

        let lines: Vec<Line<'_>> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let is_selected = i == self.selected;
                let is_current = entry.id == self.current_session_id;

                let prefix = if is_selected { "> " } else { "  " };
                let prefix_style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let meta_style = Style::default().fg(Color::DarkGray);

                let mut spans = vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(&entry.name, name_style),
                    Span::styled(
                        format!("  ({} msgs, {})", entry.message_count, entry.last_active),
                        meta_style,
                    ),
                ];
                if is_current {
                    spans.push(Span::styled(
                        "  (current)",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Line::from(spans)
            })
            .collect();

        // Scroll so the selected row stays on screen.
        let visible = inner.height as usize;
        let start = self.selected.saturating_sub(visible.saturating_sub(1));
        for (row, line) in lines.iter().skip(start).take(visible).enumerate() {
            buf.set_line(inner.x, inner.y + row as u16, line, inner.width);
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        (self.entries.len().max(1) as u16).saturating_add(2)
    }
}

impl Overlay for ResumeBrowserOverlay {
    fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => OverlayAction::Dismiss,
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                OverlayAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                OverlayAction::Consumed
            }
            KeyCode::Enter => {
                if let Some(id) = self.selected_id() {
                    OverlayAction::Execute(AppEvent::SwitchSession(id.to_string()))
                } else {
                    OverlayAction::Dismiss
                }
            }
            _ => OverlayAction::Passthrough,
        }
    }

    fn contexts(&self) -> Vec<KeyContext> {
        vec![KeyContext::ScrollBox]
    }

    fn name(&self) -> &'static str {
        "resume_browser"
    }
}
