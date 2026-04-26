//! Path safety validation for memory file keys and team memory paths.
//!
//! Guards against path-traversal, absolute paths, URL-encoded traversal,
//! null-byte injection, and symlink escapes out of a team directory.

use std::path::{Path, PathBuf};

use crab_core::Error;

/// Validate a memory file key for safety.
///
/// Rejects:
/// - null bytes
/// - backslash (disallowed separator)
/// - absolute paths (Unix `/foo`, Windows `C:\foo`)
/// - `..` path-traversal substrings
/// - URL-encoded traversal (`%2e%2e`, `%2f`, case-insensitive)
pub fn validate_memory_key(key: &str) -> crab_core::Result<String> {
    if key.contains('\0') {
        return Err(Error::Permission("memory key contains null byte".into()));
    }
    if key.contains('\\') {
        return Err(Error::Permission("memory key contains backslash".into()));
    }
    if key.starts_with('/') {
        return Err(Error::Permission(
            "memory key must not be an absolute path".into(),
        ));
    }
    // Windows drive prefix: e.g. "C:" at the start.
    if key.len() >= 2 && key.as_bytes()[1] == b':' {
        return Err(Error::Permission(
            "memory key must not contain a drive prefix".into(),
        ));
    }
    if key.contains("..") {
        return Err(Error::Permission("memory key must not contain '..'".into()));
    }
    let lower = key.to_ascii_lowercase();
    if lower.contains("%2e%2e") || lower.contains("%2f") {
        return Err(Error::Permission(
            "memory key contains URL-encoded traversal".into(),
        ));
    }
    Ok(key.to_string())
}

/// Validate that `path` resolves inside `team_dir` after symlink resolution.
///
/// Uses [`dunce::canonicalize`] on both inputs and verifies the prefix
/// relationship on the canonical paths. Because canonical paths retain
/// separators, a sibling like `team-evil` cannot match a `team` prefix.
pub fn validate_team_mem_path(path: &Path, team_dir: &Path) -> crab_core::Result<PathBuf> {
    let canonical_path = dunce::canonicalize(path).map_err(|e| {
        Error::Permission(format!(
            "failed to canonicalize memory path {}: {e}",
            path.display()
        ))
    })?;
    let canonical_dir = dunce::canonicalize(team_dir).map_err(|e| {
        Error::Permission(format!(
            "failed to canonicalize team dir {}: {e}",
            team_dir.display()
        ))
    })?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err(Error::Permission(format!(
            "path {} escapes team dir {}",
            canonical_path.display(),
            canonical_dir.display()
        )));
    }
    Ok(canonical_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ── validate_memory_key ──────────────────────────────────────

    #[test]
    fn validate_key_normal() {
        assert!(validate_memory_key("user_role.md").is_ok());
        assert!(validate_memory_key("feedback-testing.md").is_ok());
        assert!(validate_memory_key("notes/sub.md").is_ok());
    }

    #[test]
    fn validate_key_traversal() {
        assert!(validate_memory_key("../etc/passwd").is_err());
        assert!(validate_memory_key("..\\windows\\system32").is_err());
        assert!(validate_memory_key("a/../b").is_err());
    }

    #[test]
    fn validate_key_null_byte() {
        assert!(validate_memory_key("file\0.md").is_err());
    }

    #[test]
    fn validate_key_url_encoded_traversal() {
        assert!(validate_memory_key("%2e%2e%2fetc").is_err());
        assert!(validate_memory_key("%2E%2E%2Fetc").is_err());
        assert!(validate_memory_key("foo%2fbar").is_err());
    }

    #[test]
    fn validate_key_absolute_path() {
        assert!(validate_memory_key("/etc/passwd").is_err());
        assert!(validate_memory_key(r"C:\Windows\System32").is_err());
    }

    #[test]
    fn validate_key_backslash() {
        assert!(validate_memory_key(r"sub\file.md").is_err());
    }

    // ── validate_team_mem_path ───────────────────────────────────

    #[test]
    fn validate_team_path_within_dir() {
        let tmp = tempdir().unwrap();
        let team_dir = tmp.path().join("team");
        fs::create_dir_all(&team_dir).unwrap();
        let file = team_dir.join("note.md");
        fs::write(&file, b"hi").unwrap();

        let got = validate_team_mem_path(&file, &team_dir).unwrap();
        let canonical_team = dunce::canonicalize(&team_dir).unwrap();
        assert!(got.starts_with(&canonical_team));
    }

    #[test]
    fn validate_team_path_outside_dir() {
        let tmp = tempdir().unwrap();
        let team_dir = tmp.path().join("team");
        fs::create_dir_all(&team_dir).unwrap();
        let outside = tmp.path().join("other.md");
        fs::write(&outside, b"hi").unwrap();

        assert!(validate_team_mem_path(&outside, &team_dir).is_err());
    }

    #[test]
    fn validate_team_path_prefix_attack() {
        // Two sibling dirs "team" and "team-evil". A file in team-evil
        // must not validate against team_dir="team" — canonical-path
        // prefix check catches this because it matches on full components.
        let tmp = tempdir().unwrap();
        let team_dir = tmp.path().join("team");
        let evil_dir = tmp.path().join("team-evil");
        fs::create_dir_all(&team_dir).unwrap();
        fs::create_dir_all(&evil_dir).unwrap();
        let evil_file = evil_dir.join("leak.md");
        fs::write(&evil_file, b"leak").unwrap();

        assert!(validate_team_mem_path(&evil_file, &team_dir).is_err());
    }

    #[test]
    fn validate_team_path_nonexistent_fails() {
        let tmp = tempdir().unwrap();
        let team_dir = tmp.path().join("team");
        fs::create_dir_all(&team_dir).unwrap();
        let missing = team_dir.join("does-not-exist.md");
        assert!(validate_team_mem_path(&missing, &team_dir).is_err());
    }
}
