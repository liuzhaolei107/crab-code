//! Per-source settings change detection.
//!
//! Tracks which settings layer changed and emits diff events so that
//! consumers (hot-reload, UI) can react to specific configuration changes
//! without re-reading the entire merged settings tree.
//!
//! Each settings source (global, project, env, MDM) is fingerprinted
//! and compared against the last-known state.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

// ── Change event ──────────────────────────────────────────────────────

/// Describes a detected change in a single settings source.
#[derive(Debug, Clone)]
pub struct SettingsChange {
    /// The source that changed (e.g. "global", "project", "env", "mdm").
    pub source: String,
    /// Keys within that source whose values differ from the last snapshot.
    pub changed_keys: Vec<String>,
}

// ── Detector ──────────────────────────────────────────────────────────

/// Detects changes in individual settings layers by comparing content
/// fingerprints between successive checks.
pub struct ChangeDetector {
    /// Last-known fingerprint per source name.
    known_hashes: HashMap<String, u64>,
}

impl ChangeDetector {
    /// Create a new detector with no known state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            known_hashes: HashMap::new(),
        }
    }

    /// Check all settings sources and return a list of changes since the
    /// last call. Sources that have not changed are omitted.
    ///
    /// Reads known config paths and computes a hash fingerprint for each.
    /// On first call, all readable sources are reported as changed (to
    /// trigger initial loading).
    pub fn check_for_changes(&mut self) -> Vec<SettingsChange> {
        let mut changes = Vec::new();

        // Check each known source path
        let sources = [
            (
                "global",
                crate::settings::global_config_dir().join("settings.json"),
            ),
            // Project source is checked if we have a project dir
            // (callers can use check_source() for project-specific paths)
        ];

        for (name, path) in &sources {
            if let Some(change) = self.check_source(name, path) {
                changes.push(change);
            }
        }

        changes
    }

    /// Check a single source file for changes.
    ///
    /// Returns `Some(SettingsChange)` if the file's fingerprint differs
    /// from the last known state.
    pub fn check_source(&mut self, source: &str, path: &Path) -> Option<SettingsChange> {
        let current_hash = fingerprint_file(path);
        let prev = self.known_hashes.get(source).copied();

        if prev == Some(current_hash) {
            return None;
        }

        self.known_hashes.insert(source.to_string(), current_hash);

        Some(SettingsChange {
            source: source.to_string(),
            changed_keys: Vec::new(), // Key-level diff is expensive; report source-level changes
        })
    }

    /// Mark a source as known at its current state, suppressing change
    /// notifications until it actually changes again.
    pub fn mark_known(&mut self, source: &str, path: &Path) {
        let hash = fingerprint_file(path);
        self.known_hashes.insert(source.to_string(), hash);
    }

    /// Mark a source with an explicit hash value (for non-file sources).
    pub fn mark_known_hash(&mut self, source: &str, hash: u64) {
        self.known_hashes.insert(source.to_string(), hash);
    }
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute a fingerprint for a file's contents. Returns 0 if unreadable.
fn fingerprint_file(path: &Path) -> u64 {
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            hasher.finish()
        }
        Err(_) => 0, // File doesn't exist or can't be read
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_detector_has_empty_state() {
        let detector = ChangeDetector::new();
        assert!(detector.known_hashes.is_empty());
    }

    #[test]
    fn default_is_same_as_new() {
        let d = ChangeDetector::default();
        assert!(d.known_hashes.is_empty());
    }

    #[test]
    fn settings_change_fields() {
        let change = SettingsChange {
            source: "global".into(),
            changed_keys: vec!["theme".into(), "model".into()],
        };
        assert_eq!(change.source, "global");
        assert_eq!(change.changed_keys.len(), 2);
    }

    #[test]
    fn check_source_nonexistent_file() {
        let mut detector = ChangeDetector::new();
        // First check should report change (new source)
        let change = detector.check_source("test", Path::new("/nonexistent/path.json"));
        assert!(change.is_some());

        // Second check with same (non-existent) path should not report change
        let change = detector.check_source("test", Path::new("/nonexistent/path.json"));
        assert!(change.is_none());
    }

    #[test]
    fn mark_known_suppresses_change() {
        let mut detector = ChangeDetector::new();
        let path = Path::new("/nonexistent/path.json");
        detector.mark_known("test", path);

        // Should not report change since we just marked it
        let change = detector.check_source("test", path);
        assert!(change.is_none());
    }

    #[test]
    fn fingerprint_nonexistent_returns_zero() {
        assert_eq!(fingerprint_file(Path::new("/no/such/file")), 0);
    }
}
