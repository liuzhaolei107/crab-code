//! Per-session file-history snapshots — crab's equivalent of CCB's
//! `src/utils/fileHistory.ts` + `/rewind` command.
//!
//! Every time a file is about to be edited (Edit / Write / Notebook tool),
//! the pre-edit contents are saved to
//! `<base_dir>/{session_id}/{hash}@v{version}`. A user `/rewind N` restores
//! the file to its state as of version `N`.
//!
//! Storage uses the on-disk hash of the **file path** (not the content), so
//! repeated edits to the same file accumulate versions `@v1`, `@v2`, … and
//! a stable key for lookup. Each session gets its own subdirectory, and an
//! LRU cap of 100 snapshots per session prevents unbounded growth.
//!
//! This module stands alone — it does not touch [`crate::session`] or the
//! tool registry. Wiring into Edit/Write/Notebook tools lives in a
//! follow-up once [`crab_core::tool::ToolContextExt`] grows a callback slot.

pub mod snapshot;

pub use snapshot::{FileHistory, Snapshot, SnapshotError};
