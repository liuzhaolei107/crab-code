//! Single Read tool call with a short result.

use crab_core::tool::ToolDisplayStyle;
use crab_tui::app::ToolCallStatus;
use crab_tui::history::HistoryCell;
use crab_tui::history::cells::tool_call::ToolCallCell;
use crab_tui::history::cells::tool_result::ToolResultCell;

use super::helpers::{assert_snapshot, render_lines_to_text};

#[test]
fn s04_read_call_running() {
    let mut cell = ToolCallCell::new(
        "Read",
        Some("Read (Cargo.toml)".into()),
        Some(ToolDisplayStyle::Highlight),
        ToolCallStatus::Running,
    );
    cell.set_frame(2);
    let lines = cell.display_lines(80);
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s04_read_call_running", &text);
}

#[test]
fn s04_read_call_success_with_short_result() {
    let call = ToolCallCell::new(
        "Read",
        Some("Read (Cargo.toml)".into()),
        Some(ToolDisplayStyle::Highlight),
        ToolCallStatus::Success,
    );
    let result = ToolResultCell::new(
        "Read",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]",
        false,
        None,
        true,
    );
    let mut lines = call.display_lines(80);
    lines.extend(result.display_lines(80));
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s04_read_call_success_with_short_result", &text);
}
