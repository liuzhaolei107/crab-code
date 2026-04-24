//! Config browser overlay — read-only snapshot of the current runtime config.
//!
//! Opened via `/config` or the corresponding keybinding. Shows the values
//! that `/status` and `/config` historically dumped as text, but in a
//! proper overlay with a titled border and scrolling.
//!
//! Intentionally read-only: editing configuration happens in
//! `~/.crab/settings.json` and takes effect on next hot-reload.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crab_core::permission::PermissionMode;

use crate::keybindings::KeyContext;
use crate::overlay::{Overlay, OverlayAction};
use crate::traits::Renderable;

pub struct ConfigBrowserOverlay {
    model_name: String,
    permission_mode: PermissionMode,
    working_dir: String,
    memory_dir: Option<PathBuf>,
    scroll: usize,
}

impl ConfigBrowserOverlay {
    #[must_use]
    pub fn new(
        model_name: String,
        permission_mode: PermissionMode,
        working_dir: String,
        memory_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            model_name,
            permission_mode,
            working_dir,
            memory_dir,
            scroll: 0,
        }
    }

    fn lines(&self) -> Vec<Line<'_>> {
        let key_style = Style::default().fg(Color::Gray);
        let val_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let dim_style = Style::default().fg(Color::DarkGray);

        let row = |label: &'static str, value: String| -> Line<'_> {
            Line::from(vec![
                Span::styled(format!("  {label:<16}"), key_style),
                Span::styled(value, val_style),
            ])
        };

        let mut lines = vec![
            Line::from(Span::styled(
                " Configuration ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            row("Model", self.model_name.clone()),
            row("Permission mode", self.permission_mode.to_string()),
            row("Working dir", self.working_dir.clone()),
            row(
                "Memory dir",
                self.memory_dir
                    .as_ref()
                    .map_or_else(|| "(none)".into(), |d| d.display().to_string()),
            ),
            Line::from(""),
            Line::from(Span::styled(
                "  Edit ~/.crab/settings.json to change values.",
                dim_style,
            )),
            Line::from(Span::styled(
                "  Changes are picked up on next save (hot-reload).",
                dim_style,
            )),
            Line::from(""),
            Line::from(Span::styled("  Press Esc or q to close.", dim_style)),
        ];

        if self.scroll > 0 {
            lines.drain(0..self.scroll.min(lines.len()));
        }
        lines
    }
}

impl Renderable for ConfigBrowserOverlay {
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
                " Config ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let lines = self.lines();
        for (i, line) in lines.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            buf.set_line(inner.x, inner.y + i as u16, line, inner.width);
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        self.lines().len().saturating_add(2).min(u16::MAX as usize) as u16
    }
}

impl Overlay for ConfigBrowserOverlay {
    fn handle_key(&mut self, key: KeyEvent) -> OverlayAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => OverlayAction::Dismiss,
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                OverlayAction::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                OverlayAction::Consumed
            }
            _ => OverlayAction::Passthrough,
        }
    }

    fn contexts(&self) -> Vec<KeyContext> {
        vec![KeyContext::ScrollBox]
    }

    fn name(&self) -> &'static str {
        "config_browser"
    }
}
