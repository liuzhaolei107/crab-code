//! Generic selection list component with keyboard navigation.
//!
//! Keyboard model (consumed by `handle_key`):
//!
//! - `↑` / `k` — move selection up
//! - `↓` / `j` — move selection down
//! - `PageUp` / `PageDown` — page by visible height
//! - `Home` / `End` — jump to first/last
//! - `1`..`9` — jump to that 1-based index (if within range)
//! - `Enter` / `Space` — select current
//! - `Esc` — cancel
//!
//! Rendering automatically scrolls to keep the selected row visible when
//! items outnumber the drawable area.

use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

/// A selectable list item.
#[derive(Debug, Clone)]
pub struct SelectItem {
    pub label: String,
    /// Short hint suffix shown in parentheses after the label (e.g. key binding "y").
    pub key_hint: Option<String>,
    /// Optional secondary description rendered dim to the right of the label
    /// (compact layout — truncated to fit). Good for "what this option does".
    pub description: Option<String>,
}

impl SelectItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            key_hint: None,
            description: None,
        }
    }

    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.key_hint = Some(hint.into());
        self
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Outcome of a key press on the select list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectAction {
    /// The user confirmed a selection (Enter).
    Selected(usize),
    /// The user cancelled (Esc).
    Cancelled,
    /// Key was consumed but no final action yet (navigation).
    Consumed,
    /// Key was not handled.
    Ignored,
}

/// Generic selection list with arrow-key navigation.
pub struct SelectList {
    items: Vec<SelectItem>,
    selected: usize,
    /// Index of the first visible item — updated by render-time auto-scroll
    /// to keep `selected` in view. Persisted across renders so scroll
    /// position is stable under key navigation.
    scroll_top: usize,
}

impl SelectList {
    #[must_use]
    pub fn new(items: Vec<SelectItem>) -> Self {
        Self {
            items,
            selected: 0,
            scroll_top: 0,
        }
    }

    /// Current selected index.
    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// Number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Replace items and reset selection / scroll.
    pub fn set_items(&mut self, items: Vec<SelectItem>) {
        self.items = items;
        self.selected = 0;
        self.scroll_top = 0;
    }

    /// Force the selection to a specific index (clamped to range).
    pub fn set_selected(&mut self, idx: usize) {
        if self.items.is_empty() {
            self.selected = 0;
        } else {
            self.selected = idx.min(self.items.len() - 1);
        }
    }

    /// Handle a key event. `page_size` is the current viewport height in
    /// rows (used for `PageUp`/`PageDown`). Pass `1` if unknown.
    pub fn handle_key_with_page(&mut self, code: KeyCode, page_size: usize) -> SelectAction {
        if self.items.is_empty() {
            return match code {
                KeyCode::Esc => SelectAction::Cancelled,
                _ => SelectAction::Ignored,
            };
        }
        let last = self.items.len() - 1;
        let page = page_size.max(1);
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                SelectAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < last {
                    self.selected += 1;
                }
                SelectAction::Consumed
            }
            KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(page);
                SelectAction::Consumed
            }
            KeyCode::PageDown => {
                self.selected = (self.selected + page).min(last);
                SelectAction::Consumed
            }
            KeyCode::Home => {
                self.selected = 0;
                SelectAction::Consumed
            }
            KeyCode::End => {
                self.selected = last;
                SelectAction::Consumed
            }
            // Digit shortcuts: 1-based jump to index.
            KeyCode::Char(c @ '1'..='9') => {
                let idx = (c as usize) - ('1' as usize);
                if idx <= last {
                    self.selected = idx;
                    SelectAction::Consumed
                } else {
                    SelectAction::Ignored
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => SelectAction::Selected(self.selected),
            KeyCode::Esc => SelectAction::Cancelled,
            _ => SelectAction::Ignored,
        }
    }

    /// Convenience wrapper for callers that don't track page size.
    pub fn handle_key(&mut self, code: KeyCode) -> SelectAction {
        self.handle_key_with_page(code, 1)
    }

    /// Compute how many items would fit in `height` rows.
    #[must_use]
    const fn visible_count(height: u16) -> usize {
        height as usize
    }

    /// Adjust `scroll_top` so `selected` is visible given the draw height.
    fn ensure_selected_visible(&mut self, height: u16) {
        let visible = Self::visible_count(height);
        if visible == 0 {
            return;
        }
        if self.selected < self.scroll_top {
            self.scroll_top = self.selected;
        } else if self.selected >= self.scroll_top + visible {
            self.scroll_top = self.selected + 1 - visible;
        }
        // Clamp in case items shrank.
        let max_top = self.items.len().saturating_sub(visible);
        if self.scroll_top > max_top {
            self.scroll_top = max_top;
        }
    }
}

impl Widget for &SelectList {
    #[allow(clippy::cast_possible_truncation)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 || self.items.is_empty() {
            return;
        }

        // Auto-scroll on render: widget trait takes &self, so we use a
        // local copy for scroll math. `scroll_top` being &self-read only
        // means callers can persist by keeping their own SelectList
        // mut-borrowed between key events — which is what overlays do.
        let visible = SelectList::visible_count(area.height);
        let total = self.items.len();
        let mut scroll_top = self.scroll_top;
        if self.selected < scroll_top {
            scroll_top = self.selected;
        } else if self.selected >= scroll_top + visible {
            scroll_top = self.selected + 1 - visible;
        }
        let max_top = total.saturating_sub(visible);
        if scroll_top > max_top {
            scroll_top = max_top;
        }

        for row in 0..visible {
            let item_idx = scroll_top + row;
            if item_idx >= total {
                break;
            }
            let item = &self.items[item_idx];
            let y = area.y + row as u16;
            let is_selected = item_idx == self.selected;

            let prefix = if is_selected { "▸ " } else { "  " };
            let label_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let mut spans = vec![
                Span::styled(prefix, label_style),
                Span::styled(&item.label, label_style),
            ];

            if let Some(hint) = &item.key_hint {
                spans.push(Span::styled(
                    format!("  ({hint})"),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            if let Some(desc) = &item.description {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    desc.as_str(),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ));
            }

            let line = Line::from(spans);
            let line_area = Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            };
            Widget::render(line, line_area, buf);
        }

        // Render a compact scroll indicator in the top-right if content
        // exceeds the viewport: "↑" when more above, "↓" when more below.
        if total > visible && area.width > 2 {
            let has_above = scroll_top > 0;
            let has_below = scroll_top + visible < total;
            let marker = match (has_above, has_below) {
                (true, true) => '↕',
                (true, false) => '↑',
                (false, true) => '↓',
                (false, false) => ' ',
            };
            if marker != ' ' {
                let x = area.x + area.width - 1;
                if let Some(cell) = buf.cell_mut((x, area.y)) {
                    cell.set_char(marker);
                    cell.set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }
    }
}

impl SelectList {
    /// Stateful render — updates `scroll_top` so the selected row stays
    /// visible. Prefer this over the `Widget` impl when you have `&mut`.
    pub fn render_stateful(&mut self, area: Rect, buf: &mut Buffer) {
        self.ensure_selected_visible(area.height);
        Widget::render(&*self, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<SelectItem> {
        vec![
            SelectItem::new("Alpha"),
            SelectItem::new("Beta"),
            SelectItem::new("Gamma"),
        ]
    }

    #[test]
    fn new_selects_first() {
        let list = SelectList::new(items());
        assert_eq!(list.selected(), 0);
        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());
    }

    #[test]
    fn down_moves_selection() {
        let mut list = SelectList::new(items());
        assert_eq!(list.handle_key(KeyCode::Down), SelectAction::Consumed);
        assert_eq!(list.selected(), 1);
        list.handle_key(KeyCode::Down);
        assert_eq!(list.selected(), 2);
    }

    #[test]
    fn down_stops_at_end() {
        let mut list = SelectList::new(items());
        list.handle_key(KeyCode::Down);
        list.handle_key(KeyCode::Down);
        list.handle_key(KeyCode::Down);
        assert_eq!(list.selected(), 2);
    }

    #[test]
    fn up_moves_selection() {
        let mut list = SelectList::new(items());
        list.handle_key(KeyCode::Down);
        list.handle_key(KeyCode::Down);
        list.handle_key(KeyCode::Up);
        assert_eq!(list.selected(), 1);
    }

    #[test]
    fn up_stops_at_zero() {
        let mut list = SelectList::new(items());
        list.handle_key(KeyCode::Up);
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn enter_selects() {
        let mut list = SelectList::new(items());
        list.handle_key(KeyCode::Down);
        assert_eq!(list.handle_key(KeyCode::Enter), SelectAction::Selected(1));
    }

    #[test]
    fn esc_cancels() {
        let mut list = SelectList::new(items());
        assert_eq!(list.handle_key(KeyCode::Esc), SelectAction::Cancelled);
    }

    #[test]
    fn vim_keys_work() {
        let mut list = SelectList::new(items());
        list.handle_key(KeyCode::Char('j'));
        assert_eq!(list.selected(), 1);
        list.handle_key(KeyCode::Char('k'));
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn space_selects() {
        let mut list = SelectList::new(items());
        assert_eq!(
            list.handle_key(KeyCode::Char(' ')),
            SelectAction::Selected(0)
        );
    }

    #[test]
    fn empty_list() {
        let mut list = SelectList::new(vec![]);
        assert!(list.is_empty());
        assert_eq!(list.handle_key(KeyCode::Enter), SelectAction::Ignored);
        assert_eq!(list.handle_key(KeyCode::Esc), SelectAction::Cancelled);
    }

    #[test]
    fn home_end_jumps() {
        let mut list = SelectList::new(items());
        list.handle_key(KeyCode::End);
        assert_eq!(list.selected(), 2);
        list.handle_key(KeyCode::Home);
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn page_up_down_by_page_size() {
        let many: Vec<SelectItem> = (0..20).map(|i| SelectItem::new(format!("#{i}"))).collect();
        let mut list = SelectList::new(many);
        list.handle_key_with_page(KeyCode::PageDown, 5);
        assert_eq!(list.selected(), 5);
        list.handle_key_with_page(KeyCode::PageDown, 5);
        assert_eq!(list.selected(), 10);
        list.handle_key_with_page(KeyCode::PageUp, 3);
        assert_eq!(list.selected(), 7);
    }

    #[test]
    fn page_down_clamps_to_last() {
        let many: Vec<SelectItem> = (0..5).map(|i| SelectItem::new(format!("#{i}"))).collect();
        let mut list = SelectList::new(many);
        list.handle_key_with_page(KeyCode::PageDown, 20);
        assert_eq!(list.selected(), 4);
    }

    #[test]
    fn digit_jump_within_range() {
        let many: Vec<SelectItem> = (0..7).map(|i| SelectItem::new(format!("#{i}"))).collect();
        let mut list = SelectList::new(many);
        assert_eq!(list.handle_key(KeyCode::Char('3')), SelectAction::Consumed);
        assert_eq!(list.selected(), 2); // 1-based '3' = index 2
        assert_eq!(list.handle_key(KeyCode::Char('1')), SelectAction::Consumed);
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn digit_jump_out_of_range_ignored() {
        let mut list = SelectList::new(items()); // 3 items
        assert_eq!(list.handle_key(KeyCode::Char('7')), SelectAction::Ignored);
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn set_selected_clamps() {
        let mut list = SelectList::new(items());
        list.set_selected(99);
        assert_eq!(list.selected(), 2);
    }

    #[test]
    fn set_items_resets_selection() {
        let mut list = SelectList::new(items());
        list.handle_key(KeyCode::Down);
        list.set_items(vec![SelectItem::new("Only")]);
        assert_eq!(list.selected(), 0);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn renders_items() {
        let list = SelectList::new(items());
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        Widget::render(&list, area, &mut buf);

        let row0: String = (0..area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(row0.contains("Alpha"));

        let row1: String = (0..area.width)
            .map(|x| buf.cell((x, 1)).unwrap().symbol().to_string())
            .collect();
        assert!(row1.contains("Beta"));
    }

    #[test]
    fn item_with_hint() {
        let item = SelectItem::new("Option A").with_hint("y");
        assert_eq!(item.label, "Option A");
        assert_eq!(item.key_hint.as_deref(), Some("y"));
    }

    #[test]
    fn item_with_description() {
        let item = SelectItem::new("Allow").with_description("Run this once");
        assert_eq!(item.description.as_deref(), Some("Run this once"));
    }

    #[test]
    fn renders_description_dim() {
        let list = SelectList::new(vec![
            SelectItem::new("Allow").with_description("once"),
            SelectItem::new("Deny"),
        ]);
        let area = Rect::new(0, 0, 40, 3);
        let mut buf = Buffer::empty(area);
        Widget::render(&list, area, &mut buf);
        let row0: String = (0..area.width)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(row0.contains("Allow"));
        assert!(row0.contains("once"));
    }

    #[test]
    fn stateful_scroll_keeps_selected_visible() {
        let many: Vec<SelectItem> = (0..20).map(|i| SelectItem::new(format!("#{i}"))).collect();
        let mut list = SelectList::new(many);
        // Jump to end — viewport is only 5 rows.
        list.set_selected(19);
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        list.render_stateful(area, &mut buf);

        // Last visible row should contain "#19".
        let last_row: String = (0..area.width)
            .map(|x| buf.cell((x, area.height - 1)).unwrap().symbol().to_string())
            .collect();
        assert!(
            last_row.contains("#19"),
            "expected #19 in last row, got: {last_row}"
        );
    }

    #[test]
    fn scroll_indicator_rendered_when_overflow() {
        let many: Vec<SelectItem> = (0..10).map(|i| SelectItem::new(format!("#{i}"))).collect();
        let mut list = SelectList::new(many);
        list.set_selected(0); // top — expect ↓
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        list.render_stateful(area, &mut buf);
        let corner = buf
            .cell((area.x + area.width - 1, area.y))
            .unwrap()
            .symbol()
            .to_string();
        assert_eq!(corner, "↓");

        list.set_selected(9); // bottom — expect ↑
        let mut buf = Buffer::empty(area);
        list.render_stateful(area, &mut buf);
        let corner = buf
            .cell((area.x + area.width - 1, area.y))
            .unwrap()
            .symbol()
            .to_string();
        assert_eq!(corner, "↑");

        list.set_selected(5); // middle — expect ↕
        let mut buf = Buffer::empty(area);
        list.render_stateful(area, &mut buf);
        let corner = buf
            .cell((area.x + area.width - 1, area.y))
            .unwrap()
            .symbol()
            .to_string();
        assert_eq!(corner, "↕");
    }
}
