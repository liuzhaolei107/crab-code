//! Reusable TUI primitives that compose into higher-level views.
//!
//! The goal is that every modal, palette, or panel in Crab is built from
//! the widgets in this module. Widgets here do not own application state
//! — callers feed in data + a focus flag, and the widget renders a
//! consistent visual language driven by [`crate::theme::Theme`].

pub mod button;
pub mod dialog;
pub mod keyboard_hint;
pub mod pane;
pub mod progress_bar;
pub mod scrollbox;
pub mod status_icon;
pub mod tabs;

pub use button::{Button, ButtonState};
pub use dialog::{Dialog, DialogAction};
pub use keyboard_hint::KeyboardHint;
pub use pane::Pane;
pub use progress_bar::ProgressBar;
pub use scrollbox::{ScrollBox, ScrollBoxState};
pub use status_icon::StatusIcon;
pub use tabs::Tabs;
