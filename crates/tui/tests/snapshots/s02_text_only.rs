//! User question followed by a plain-text assistant reply.
//!
//! Covers user and assistant cell rendering only. Bottom bar and status
//! line are App-level surfaces and are exercised separately.

use crab_tui::history::HistoryCell;
use crab_tui::history::cells::assistant::AssistantCell;
use crab_tui::history::cells::user::UserCell;

use super::helpers::{assert_snapshot, render_lines_to_text};

#[test]
fn s02_user_then_assistant_short() {
    let user = UserCell::new("what is 2+2?");
    let asst = AssistantCell::new("Four.");

    let mut all_lines = Vec::new();
    all_lines.extend(user.display_lines(80));
    all_lines.extend(asst.display_lines(80));

    let text = render_lines_to_text(&all_lines, 80, 24);
    assert_snapshot("s02_user_then_assistant_short", &text);
}

#[test]
fn s02_assistant_with_markdown_paragraphs() {
    let asst = AssistantCell::new("First paragraph.\n\nSecond paragraph with **bold** text.");
    let lines = asst.display_lines(80);
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s02_assistant_with_markdown_paragraphs", &text);
}
