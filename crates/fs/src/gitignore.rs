//! `.gitignore` rule parsing and path filtering.

use std::path::Path;

/// Parsed gitignore rules for a directory tree.
pub struct GitIgnoreFilter {
    _private: (),
}

impl GitIgnoreFilter {
    /// Load gitignore rules starting from `root`, walking up to find parent rules.
    ///
    /// # Errors
    ///
    /// Returns an error if the root path is inaccessible.
    pub fn new(_root: &Path) -> crab_common::Result<Self> {
        todo!()
    }

    /// Check whether a path should be ignored.
    #[must_use]
    pub fn is_ignored(&self, _path: &Path) -> bool {
        todo!()
    }
}
