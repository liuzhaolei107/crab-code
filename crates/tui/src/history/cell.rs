//! `HistoryCell` — the trait every transcript unit implements.

use std::any::Any;

use ratatui::text::Line;

/// A single unit of the transcript.
pub trait HistoryCell: Send + Sync + std::fmt::Debug {
    /// The lines painted into the main chat viewport at `width`.
    fn display_lines(&self, width: u16) -> Vec<Line<'static>>;

    /// Height this cell occupies at `width`. Implementations typically
    /// return `display_lines(width).len() as u16`; override only when a
    /// cheaper calculation is available.
    fn desired_height(&self, width: u16) -> u16 {
        let lines = self.display_lines(width).len();
        u16::try_from(lines).unwrap_or(u16::MAX)
    }

    /// The lines painted into the transcript overlay (Ctrl+O). Defaults
    /// to the main display, so most cells do not need to override.
    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.display_lines(width)
    }

    /// Return a monotonic counter the cache can use to invalidate. For
    /// static cells return `None`; for animated cells (spinner, shimmer)
    /// return the current frame.
    fn transcript_animation_tick(&self) -> Option<u64> {
        None
    }

    /// Plain-text representation for search / copy / context-collapse.
    /// Defaults to joining `display_lines` at a large width so long
    /// text is not prematurely wrapped.
    fn search_text(&self) -> String {
        self.display_lines(u16::MAX)
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|s| s.content.into_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn as_any(&self) -> &dyn Any;
}
