//! Bottom-anchored painter for the inline-viewport content area.
//!
//! Iterates [`ChatMessage`]s, renders each via the cell adapter, and paints
//! the resulting lines starting at the bottom of `area` and growing
//! upward. Excess content (more lines than `area.height`) is clipped from
//! the top — those rows belong in scrollback and the drain pipeline is
//! expected to push them out before this is called.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::app::ChatMessage;
use crate::history::cell_from_chat_message;

pub fn paint_messages_bottom_up(messages: &[ChatMessage], area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 || messages.is_empty() {
        return;
    }
    let mut all_lines = Vec::new();
    for msg in messages {
        let cell = cell_from_chat_message(msg);
        all_lines.extend(cell.display_lines(area.width));
    }
    let visible = area.height as usize;
    let total = all_lines.len();
    let start = total.saturating_sub(visible);
    let to_paint = total - start;
    let top_pad = visible - to_paint;
    for (i, line) in all_lines[start..].iter().enumerate() {
        let y = area.y + (top_pad + i) as u16;
        if y >= area.y + area.height {
            break;
        }
        Widget::render(
            line.clone(),
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
            buf,
        );
    }
}

pub fn messages_total_lines(messages: &[ChatMessage], width: u16) -> usize {
    if width == 0 {
        return 0;
    }
    messages
        .iter()
        .map(|msg| cell_from_chat_message(msg).desired_height(width) as usize)
        .sum()
}
