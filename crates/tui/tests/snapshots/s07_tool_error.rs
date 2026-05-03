//! Tool failure (`is_error = true`) styling for both call and result cells.

use crab_core::tool::ToolDisplayStyle;
use crab_tui::app::ToolCallStatus;
use crab_tui::history::HistoryCell;
use crab_tui::history::cells::tool_call::ToolCallCell;
use crab_tui::history::cells::tool_result::ToolResultCell;

use super::helpers::{assert_snapshot, render_lines_to_text};

#[test]
fn s07_bash_command_failed() {
    let call = ToolCallCell::new(
        "Bash",
        Some("Run (cargo nope)".into()),
        Some(ToolDisplayStyle::Highlight),
        ToolCallStatus::Error,
    );
    let result = ToolResultCell::new("Bash", "error: no such command: `nope`", true, None, true);

    let mut lines = call.display_lines(80);
    lines.extend(result.display_lines(80));
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s07_bash_command_failed", &text);
}
