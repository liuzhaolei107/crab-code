//! Thinking cell in collapsed and expanded states.

use crab_tui::history::HistoryCell;
use crab_tui::history::cells::thinking::ThinkingCell;

use super::helpers::{assert_snapshot, render_lines_to_text};

#[test]
fn s03_thinking_collapsed() {
    let cell = ThinkingCell::new("internal reasoning content".into(), true, None);
    let lines = cell.display_lines(80);
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s03_thinking_collapsed", &text);
}

#[test]
fn s03_thinking_expanded() {
    let cell = ThinkingCell::new("step one\nstep two\nstep three".into(), false, None);
    let lines = cell.display_lines(80);
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s03_thinking_expanded", &text);
}
