//! History cells — polymorphic units of the conversation transcript.
//!
//! Each conversational artifact (user input, assistant reply, tool call,
//! tool result, system note) is a cell that owns its own rendering.
//! Cells compose into a `VecDeque<Box<dyn HistoryCell>>` in `App`; the
//! renderer iterates the queue and lays out cell by cell.
//!
//! The trait signature is aligned with Codex's `HistoryCell` so the two
//! crates' learnings can cross-pollinate, with one deliberate Crab
//! addition: [`HistoryCell::search_text`] exposes copy-searchable text
//! (for Ctrl+R history search and the in-buffer find bar) without
//! forcing every caller to re-flatten `display_lines`.

mod adapter;
mod cell;
pub mod cells;
pub mod grouping;

pub use adapter::cell_from_chat_message;
pub use cell::HistoryCell;
pub use cells::{AssistantCell, SystemCell, ToolCallCell, ToolResultCell, UserCell};
pub use grouping::group_messages;
