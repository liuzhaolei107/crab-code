use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::traits::Renderable;

const MAX_VISIBLE_ITEMS: usize = 10;

#[derive(Debug, Clone)]
pub struct FileSuggestion {
    pub path: String,
    pub is_dir: bool,
}

impl FileSuggestion {
    #[must_use]
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            is_dir: false,
        }
    }

    #[must_use]
    pub fn dir(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            is_dir: true,
        }
    }
}

pub struct AtMentionPopup {
    pub query: String,
    pub suggestions: Vec<FileSuggestion>,
    pub selected: usize,
}

impl AtMentionPopup {
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            suggestions: Vec::new(),
            selected: 0,
        }
    }

    pub fn set_suggestions(&mut self, suggestions: Vec<FileSuggestion>) {
        self.suggestions = suggestions;
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected = (self.selected + 1).min(self.suggestions.len() - 1);
        }
    }

    #[must_use]
    pub fn selected_path(&self) -> Option<&str> {
        self.suggestions.get(self.selected).map(|s| s.path.as_str())
    }

    #[must_use]
    pub fn selected_insertion(&self) -> Option<String> {
        self.selected_path().map(|p| format!("@{p}"))
    }
}

impl Renderable for AtMentionPopup {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if self.suggestions.is_empty() || area.height < 3 {
            return;
        }

        let visible = self.suggestions.len().min(MAX_VISIBLE_ITEMS);
        let height = visible as u16 + 2;
        let width = 50u16.min(area.width);

        let popup_area = Rect {
            x: area.x + 2,
            y: area.bottom().saturating_sub(height + 1),
            width,
            height,
        };

        Widget::render(Clear, popup_area, buf);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" @{} ", self.query));
        let inner = block.inner(popup_area);
        Widget::render(block, popup_area, buf);

        let start = if self.selected >= visible {
            self.selected - visible + 1
        } else {
            0
        };

        for (i, suggestion) in self
            .suggestions
            .iter()
            .skip(start)
            .take(visible)
            .enumerate()
        {
            if i as u16 >= inner.height {
                break;
            }
            let is_selected = start + i == self.selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let icon = if suggestion.is_dir { "📁" } else { "📄" };
            let line = Line::from(vec![
                Span::styled(format!(" {icon} "), style),
                Span::styled(suggestion.path.clone(), style),
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
        if self.suggestions.is_empty() {
            0
        } else {
            self.suggestions.len().min(MAX_VISIBLE_ITEMS) as u16 + 2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation() {
        let mut popup = AtMentionPopup::new("sr");
        popup.set_suggestions(vec![
            FileSuggestion::dir("src"),
            FileSuggestion::file("src/main.rs"),
            FileSuggestion::file("src/lib.rs"),
        ]);
        assert_eq!(popup.selected_path(), Some("src"));
        popup.move_down();
        assert_eq!(popup.selected_path(), Some("src/main.rs"));
        popup.move_down();
        popup.move_down();
        assert_eq!(popup.selected_path(), Some("src/lib.rs"));
    }

    #[test]
    fn insertion_format() {
        let mut popup = AtMentionPopup::new("m");
        popup.set_suggestions(vec![FileSuggestion::file("src/main.rs")]);
        assert_eq!(popup.selected_insertion(), Some("@src/main.rs".into()));
    }

    #[test]
    fn empty_suggestions() {
        let popup = AtMentionPopup::new("xyz");
        assert!(popup.selected_path().is_none());
        assert_eq!(popup.desired_height(80), 0);
    }

    #[test]
    fn render_no_panic() {
        let mut popup = AtMentionPopup::new("s");
        popup.set_suggestions(vec![FileSuggestion::file("src/lib.rs")]);
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        popup.render(area, &mut buf);
    }
}
