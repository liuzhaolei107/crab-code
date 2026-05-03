//! Integration test for the inline-viewport drain → scrollback flush flow.
//!
//! Builds a `vt100::Parser`-backed terminal, simulates a couple of finalized
//! cells getting drained via `App::drain_finalized_into_pending`, then runs
//! `insert_history_lines_with_mode` and verifies the rendered text shows up
//! above the inline viewport area in the simulated screen.

#![cfg(test)]

use ratatui::layout::Rect;

use crate::app::{App, AppState, ChatMessage};
use crate::custom_terminal::Terminal;
use crate::insert_history::insert_history_lines_with_mode;
use crate::terminal_detection::InsertHistoryMode;
use crate::test_backend::VT100Backend;

fn build_terminal(width: u16, height: u16) -> Terminal<VT100Backend> {
    Terminal::with_options(VT100Backend::new(width, height)).expect("terminal")
}

#[test]
fn finalized_user_message_lands_in_scrollback() {
    let width: u16 = 80;
    let height: u16 = 24;
    let mut term = build_terminal(width, height);
    // Anchor a small inline viewport at the bottom — chrome lives here,
    // history lands above.
    let viewport = Rect::new(0, height - 4, width, 4);
    term.set_viewport_area(viewport);

    let mut app = App::new("test-model");
    app.state = AppState::Idle;
    app.messages.push(ChatMessage::User {
        text: "hello from the test".into(),
    });

    app.drain_finalized_into_pending(width);
    assert!(
        app.messages.is_empty(),
        "finalized user message should be drained out"
    );
    let lines = app.pending_history.take();
    assert!(!lines.is_empty(), "drain should produce display lines");

    insert_history_lines_with_mode(&mut term, &lines, InsertHistoryMode::Standard)
        .expect("flush succeeds");

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    assert!(
        rows.iter().any(|r| r.contains("hello from the test")),
        "expected drained text in scrollback area, rows: {rows:?}"
    );
}

#[test]
fn streaming_tail_stays_in_viewport() {
    let width: u16 = 80;
    let height: u16 = 24;
    let mut term = build_terminal(width, height);
    term.set_viewport_area(Rect::new(0, height - 6, width, 6));

    let mut app = App::new("test-model");
    // First, a finalized user turn — drainable.
    app.messages.push(ChatMessage::User {
        text: "what's up?".into(),
    });
    // Then an assistant turn that's still streaming. App::Processing keeps
    // the tail anchored to the viewport.
    app.state = AppState::Processing;
    app.messages.push(ChatMessage::Assistant {
        text: "thinking...".into(),
    });

    app.drain_finalized_into_pending(width);
    assert_eq!(
        app.messages.len(),
        1,
        "user message drained, assistant tail kept"
    );
    assert!(
        matches!(&app.messages[0], ChatMessage::Assistant { .. }),
        "remaining cell is the streaming assistant turn"
    );
    let lines = app.pending_history.take();
    assert!(
        !lines.is_empty(),
        "drained user turn should yield pending lines"
    );
    insert_history_lines_with_mode(&mut term, &lines, InsertHistoryMode::Standard).unwrap();

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    assert!(
        rows.iter().any(|r| r.contains("what's up?")),
        "user message reached scrollback"
    );
}

#[test]
fn fallback_mode_emits_history_above_viewport() {
    let width: u16 = 60;
    let height: u16 = 20;
    let mut term = build_terminal(width, height);
    term.set_viewport_area(Rect::new(0, height - 3, width, 3));

    let mut app = App::new("test-model");
    app.messages.push(ChatMessage::User {
        text: "fallback path".into(),
    });

    app.drain_finalized_into_pending(width);
    let lines = app.pending_history.take();
    insert_history_lines_with_mode(&mut term, &lines, InsertHistoryMode::Fallback).unwrap();

    let rows: Vec<String> = term.backend().vt100().screen().rows(0, width).collect();
    assert!(
        rows.iter().any(|r| r.contains("fallback path")),
        "fallback path also writes drained lines into scrollback area"
    );
}
