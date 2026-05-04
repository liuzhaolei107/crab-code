//! Enter / leave the terminal alternate screen for a full-screen overlay.
//!
//! Inline-viewport rendering keeps the TUI in the bottom few rows of the
//! terminal so native scrollback works. When an overlay (diff viewer,
//! full-screen picker, transcript dump) needs the whole screen, it must
//! switch into the alt-screen first and restore the previous viewport on
//! exit.
//!
//! Intentionally narrow API: callers wrap their overlay loop in
//! `with_alt_screen(term, |term| { ... })` and the helper handles enter,
//! viewport bookkeeping, and exit even on early return.

use std::io;
use std::io::Write;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::Backend;
use ratatui::layout::Rect;

use crate::custom_terminal::Terminal;

/// Run `f` with the terminal switched to the alternate screen. The previous
/// inline viewport area is restored before this returns, regardless of
/// whether `f` returned `Ok` or `Err`.
#[allow(dead_code)]
pub fn with_alt_screen<B, F, T>(term: &mut Terminal<B>, f: F) -> io::Result<T>
where
    B: Backend<Error = io::Error> + Write,
    F: FnOnce(&mut Terminal<B>) -> io::Result<T>,
{
    let saved_viewport = term.viewport_area;
    enter(term)?;
    let result = f(term);
    leave(term, saved_viewport)?;
    result
}

#[allow(dead_code)]
fn enter<B>(term: &mut Terminal<B>) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    execute!(term.backend_mut(), EnterAlternateScreen)?;
    let size = term.size()?;
    let full = Rect::new(0, 0, size.width, size.height);
    term.set_viewport_area(full);
    term.clear()?;
    Ok(())
}

#[allow(dead_code)]
fn leave<B>(term: &mut Terminal<B>, saved: Rect) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.set_viewport_area(saved);
    term.invalidate_viewport();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_backend::VT100Backend;

    #[test]
    fn restores_viewport_on_success() {
        let mut term = Terminal::with_options(VT100Backend::new(40, 12)).unwrap();
        let original = Rect::new(0, 8, 40, 4);
        term.set_viewport_area(original);
        let result = with_alt_screen(&mut term, |term| {
            let area = term.viewport_area;
            assert_eq!(area, Rect::new(0, 0, 40, 12));
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(term.viewport_area, original);
    }

    #[test]
    fn restores_viewport_on_error() {
        let mut term = Terminal::with_options(VT100Backend::new(40, 12)).unwrap();
        let original = Rect::new(0, 9, 40, 3);
        term.set_viewport_area(original);
        let result: io::Result<()> = with_alt_screen(&mut term, |_| Err(io::Error::other("boom")));
        assert!(result.is_err());
        assert_eq!(term.viewport_area, original);
    }
}
