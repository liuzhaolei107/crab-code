//! Cold-start scenario: only the welcome cell, no user/assistant messages.
//!
//! This fixture covers the welcome cell rendering. The bottom-bar widget
//! is exercised separately at the widget level.

use crab_tui::history::HistoryCell;
use crab_tui::history::cells::welcome::WelcomeCell;

use super::helpers::{assert_snapshot, render_lines_to_text};

#[test]
fn s01_cold_start_welcome_only() {
    let cell = WelcomeCell::new("0.1.0".into(), String::new(), false);
    let lines = cell.display_lines(80);
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s01_cold_start_welcome_only", &text);
}

#[test]
fn s01_cold_start_with_whats_new() {
    let cell = WelcomeCell::new(
        "0.1.0".into(),
        "streaming ratchet now tracks message id".into(),
        true,
    );
    let lines = cell.display_lines(80);
    let text = render_lines_to_text(&lines, 80, 24);
    assert_snapshot("s01_cold_start_with_whats_new", &text);
}
