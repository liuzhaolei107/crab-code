//! Context-compaction boundary followed by another user/assistant turn.

use crab_tui::history::HistoryCell;
use crab_tui::history::cells::assistant::AssistantCell;
use crab_tui::history::cells::compact_boundary::CompactBoundaryCell;
use crab_tui::history::cells::user::UserCell;

use super::helpers::{assert_snapshot, render_lines_to_text};

#[test]
fn s08_compact_then_user_then_assistant() {
    let boundary = CompactBoundaryCell::new("summary".into(), 50_000, 12);
    let user = UserCell::new("now what?");
    let asst = AssistantCell::new("Continuing from where we left off.");

    let mut lines = Vec::new();
    lines.extend(boundary.display_lines(80));
    lines.extend(user.display_lines(80));
    lines.extend(asst.display_lines(80));

    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s08_compact_then_user_then_assistant", &text);
}
