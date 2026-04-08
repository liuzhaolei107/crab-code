//! Watch skill/config files for changes and trigger hook re-registration.
//!
//! Monitors filesystem paths for modifications and invokes a callback when
//! changes are detected. This allows the hook and skill systems to
//! automatically reload when skill files or configuration are edited.
//!
//! Maps to CCB `hooks/fileChangedWatcher.ts`.

use std::path::{Path, PathBuf};

// ─── File watcher ──────────────────────────────────────────────────────

/// Watches a set of filesystem paths and triggers a callback on changes.
///
/// # Example
///
/// ```ignore
/// use crab_plugin::hook_watchers::HookFileWatcher;
///
/// let mut watcher = HookFileWatcher::new();
/// watcher.watch("/home/user/.crab/skills".into());
/// watcher.watch("/project/.crab/skills".into());
///
/// watcher.run(|path| {
///     println!("File changed: {}", path.display());
///     // Re-register hooks/skills from the changed file
/// }).await;
/// ```
pub struct HookFileWatcher {
    /// Paths being monitored for changes.
    watched_paths: Vec<PathBuf>,
}

impl HookFileWatcher {
    /// Create a new watcher with no paths.
    #[must_use]
    pub fn new() -> Self {
        Self {
            watched_paths: Vec::new(),
        }
    }

    /// Add a path to the watch list.
    ///
    /// The path can be a file or directory. Directories are watched
    /// recursively for any `.md` or `.json` file changes.
    pub fn watch(&mut self, path: PathBuf) {
        if !self.watched_paths.contains(&path) {
            self.watched_paths.push(path);
        }
    }

    /// Remove a path from the watch list.
    ///
    /// Returns `true` if the path was found and removed.
    pub fn unwatch(&mut self, path: &Path) -> bool {
        let before = self.watched_paths.len();
        self.watched_paths.retain(|p| p != path);
        self.watched_paths.len() < before
    }

    /// Get the list of currently watched paths.
    #[must_use]
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }

    /// Start watching and call `on_change` when a watched file is modified.
    ///
    /// This is a long-running async task that blocks until cancelled.
    /// Uses filesystem notification APIs where available, falling back
    /// to periodic polling.
    pub async fn run<F: Fn(&Path) + Send + 'static>(self, _on_change: F) {
        todo!("HookFileWatcher::run: set up filesystem watcher and dispatch on_change callbacks")
    }
}

impl Default for HookFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_watcher_is_empty() {
        let watcher = HookFileWatcher::new();
        assert!(watcher.watched_paths().is_empty());
    }

    #[test]
    fn watch_adds_path() {
        let mut watcher = HookFileWatcher::new();
        watcher.watch(PathBuf::from("/tmp/skills"));
        assert_eq!(watcher.watched_paths().len(), 1);
        assert_eq!(watcher.watched_paths()[0], Path::new("/tmp/skills"));
    }

    #[test]
    fn watch_deduplicates() {
        let mut watcher = HookFileWatcher::new();
        watcher.watch(PathBuf::from("/tmp/skills"));
        watcher.watch(PathBuf::from("/tmp/skills"));
        assert_eq!(watcher.watched_paths().len(), 1);
    }

    #[test]
    fn unwatch_removes_path() {
        let mut watcher = HookFileWatcher::new();
        watcher.watch(PathBuf::from("/tmp/a"));
        watcher.watch(PathBuf::from("/tmp/b"));
        assert!(watcher.unwatch(Path::new("/tmp/a")));
        assert_eq!(watcher.watched_paths().len(), 1);
        assert_eq!(watcher.watched_paths()[0], Path::new("/tmp/b"));
    }

    #[test]
    fn unwatch_nonexistent_returns_false() {
        let mut watcher = HookFileWatcher::new();
        assert!(!watcher.unwatch(Path::new("/tmp/nope")));
    }

    #[test]
    fn default_watcher_is_empty() {
        let watcher = HookFileWatcher::default();
        assert!(watcher.watched_paths().is_empty());
    }
}
