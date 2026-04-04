//! File system watching via `notify`.

use std::path::{Path, PathBuf};

/// Events emitted by the file watcher.
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

/// Watches a directory tree for file changes.
pub struct FileWatcher {
    _private: (),
}

impl FileWatcher {
    /// Start watching `path` for changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or cannot be watched.
    pub fn new(_path: &Path) -> crab_common::Result<Self> {
        todo!()
    }

    /// Stop watching.
    pub fn stop(&mut self) {
        todo!()
    }
}
