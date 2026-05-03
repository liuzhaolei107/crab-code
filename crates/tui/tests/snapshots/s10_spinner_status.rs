//! Spinner widget rendering with a stable verb to keep the snapshot
//! deterministic (avoids the random-verb start path).
//!
//! Note: `Spinner` is `impl Widget for &Spinner`, not a `HistoryCell`, so
//! it renders directly into a buffer instead of going through
//! `helpers::render_lines_to_text`.
//!
//! Status line snapshots are deferred — building one requires significant
//! `App` state that is awkward to construct at the cell level. Their
//! coverage will be decided based on findings during the alignment work.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crab_tui::components::spinner::Spinner;

use super::helpers::assert_snapshot;

fn buf_to_text(buf: &Buffer, w: u16, h: u16) -> String {
    let mut out = String::new();
    for y in 0..h {
        let mut row = String::new();
        for x in 0..w {
            row.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        out.push_str(row.trim_end());
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[test]
fn s10_spinner_with_stable_verb() {
    let mut spinner = Spinner::new();
    spinner.start("Working");
    let area = Rect::new(0, 0, 60, 1);
    let mut buf = Buffer::empty(area);
    Widget::render(&spinner, area, &mut buf);
    assert_snapshot("s10_spinner_with_stable_verb", &buf_to_text(&buf, 60, 1));
}
