//! `.gitignore` rule parsing and path filtering.
//!
//! Thin wrapper around [`ignore::gitignore::Gitignore`] that loads the full
//! chain of ignore files for a directory tree: local `.gitignore`, parent
//! `.gitignore` files, `.git/info/exclude`, and the global gitignore.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Compiled gitignore rules for a directory tree.
///
/// Created from a root directory; automatically discovers and merges all
/// relevant ignore files.
pub struct GitIgnoreFilter {
    /// Root directory the filter was built from.
    root: PathBuf,
    /// The compiled gitignore matcher.
    #[allow(dead_code)]
    gitignore: ignore::gitignore::Gitignore,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl GitIgnoreFilter {
    /// Load all gitignore rules for the directory tree rooted at `root`.
    ///
    /// Discovers:
    /// - `.gitignore` files from `root` upward through parent directories.
    /// - `.git/info/exclude` (if present).
    /// - The global gitignore (e.g. `~/.config/git/ignore`).
    ///
    /// # Errors
    ///
    /// Returns an error if `root` is inaccessible.
    pub fn new(root: &Path) -> crab_common::Result<Self> {
        let _ = root;
        todo!()
    }

    /// Load gitignore rules *plus* an additional custom ignore file
    /// (e.g. `.crabignore`).
    ///
    /// # Errors
    ///
    /// Returns an error if `root` or `ignore_file` is inaccessible.
    pub fn with_custom_ignore(root: &Path, ignore_file: &Path) -> crab_common::Result<Self> {
        let _ = (root, ignore_file);
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

impl GitIgnoreFilter {
    /// Check whether `path` should be ignored.
    ///
    /// Calls [`Path::is_dir`] internally to supply the directory hint
    /// required by gitignore trailing-`/` rules. Use
    /// [`is_ignored_dir`](Self::is_ignored_dir) if you already know whether
    /// the path is a directory (avoids a `stat` syscall).
    #[must_use]
    pub fn is_ignored(&self, path: &Path) -> bool {
        self.is_ignored_dir(path, path.is_dir())
    }

    /// Check whether `path` should be ignored, with an explicit directory hint.
    #[must_use]
    pub fn is_ignored_dir(&self, _path: &Path, _is_dir: bool) -> bool {
        todo!()
    }

    /// The root directory this filter was built for.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}
