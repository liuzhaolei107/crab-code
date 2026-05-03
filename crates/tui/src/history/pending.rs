//! Queue of finalized history lines waiting to be flushed into the
//! terminal's native scrollback.
//!
//! Once a `HistoryCell` reports `is_finalized() == true`, the inline
//! viewport drains its rendered lines into this queue. The next render
//! pass passes them to `insert_history_lines`, which writes them above
//! the viewport using DECSTBM scroll regions (or the newline fallback).

use ratatui::text::Line;

#[derive(Debug, Default)]
pub struct PendingHistory {
    lines: Vec<Line<'static>>,
}

impl PendingHistory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extend<I: IntoIterator<Item = Line<'static>>>(&mut self, iter: I) {
        self.lines.extend(iter);
    }

    pub fn take(&mut self) -> Vec<Line<'static>> {
        std::mem::take(&mut self.lines)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_and_take_round_trip() {
        let mut p = PendingHistory::new();
        assert!(p.is_empty());
        p.extend(vec![Line::from("a"), Line::from("b")]);
        assert_eq!(p.len(), 2);
        let taken = p.take();
        assert_eq!(taken.len(), 2);
        assert!(p.is_empty());
    }
}
