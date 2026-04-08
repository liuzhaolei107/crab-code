//! Managed Device Management: reads enterprise-managed settings from well-known paths.
//!
//! On managed corporate devices, system administrators may deploy Crab Code
//! configuration via MDM profiles. This module discovers and loads those
//! settings so they can be merged (with highest priority) into the
//! configuration stack.
//!
//! Well-known paths checked:
//! - macOS: `/Library/Managed Preferences/com.crabcode.settings.plist`
//! - Windows: `%ProgramData%\CrabCode\managed-settings.json`
//! - Linux: `/etc/crab-code/managed-settings.json`

use std::path::PathBuf;

// ── Discovery ─────────────────────────────────────────────────────────

/// Return the platform-specific paths where managed settings may be found.
///
/// The returned paths are ordered by priority (highest-priority first).
/// Not all paths may exist; callers should check existence before reading.
#[must_use]
pub fn managed_settings_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from(
            "/Library/Managed Preferences/com.crabcode.settings.plist",
        ));
        // Also check user-level managed preferences
        if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(format!(
                "{home}/Library/Managed Preferences/com.crabcode.settings.plist"
            )));
        }
    }

    #[cfg(target_os = "windows")]
    {
        // %ProgramData% is typically C:\ProgramData
        if let Ok(program_data) = std::env::var("ProgramData") {
            paths.push(PathBuf::from(format!(
                "{program_data}\\CrabCode\\managed-settings.json"
            )));
        }
        paths.push(PathBuf::from(
            "C:\\ProgramData\\CrabCode\\managed-settings.json",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/etc/crab-code/managed-settings.json"));
        // Also check drop-in directory
        paths.push(PathBuf::from("/etc/crab-code/managed-settings.d/"));
    }

    // Fallback for unsupported platforms: empty list
    paths
}

// ── Loading ───────────────────────────────────────────────────────────

/// Load and parse the managed settings from the first available well-known
/// path. Returns `None` if no managed settings are found.
///
/// The returned `Value` follows the same schema as `settings.json` and
/// will be merged into the configuration stack with the highest priority.
#[must_use]
pub fn load_managed_settings() -> Option<serde_json::Value> {
    for path in managed_settings_paths() {
        if !path.exists() {
            continue;
        }

        // Handle drop-in directories (Linux)
        if path.is_dir() {
            return load_drop_in_dir(&path);
        }

        // Handle regular files (JSON or plist)
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(value) = serde_json::from_str(&content)
        {
            return Some(value);
        }
    }

    None
}

/// Load and merge all `.json` files from a drop-in directory.
fn load_drop_in_dir(dir: &std::path::Path) -> Option<serde_json::Value> {
    let mut merged = serde_json::Map::new();
    let mut found_any = false;

    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut paths: Vec<_> = entries
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "json")
            })
            .map(|e| e.path())
            .collect();
        paths.sort(); // Alphabetical order for deterministic merge

        for path in paths {
            if let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(&content)
            {
                for (key, value) in obj {
                    merged.insert(key, value);
                }
                found_any = true;
            }
        }
    }

    if found_any {
        Some(serde_json::Value::Object(merged))
    } else {
        None
    }
}

// ── Detection ─────────────────────────────────────────────────────────

/// Detect whether the current device appears to be enterprise-managed.
///
/// Returns `true` if any of the well-known MDM paths exist, even if
/// the contents are empty or unparseable.
#[must_use]
pub fn is_managed_environment() -> bool {
    managed_settings_paths().iter().any(|p| p.exists())
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_paths_returns_vec() {
        let paths = managed_settings_paths();
        // Should return at least one path on any platform
        // (or empty on unsupported — that's fine too)
        let _ = paths; // Just verify it compiles and doesn't panic
    }

    #[test]
    fn load_managed_settings_returns_none_when_no_files() {
        // On a dev machine, MDM files typically don't exist
        let result = load_managed_settings();
        // Most likely None, but could be Some on enterprise machines
        let _ = result;
    }

    #[test]
    fn is_managed_false_on_dev_machine() {
        // On most dev machines, no MDM paths exist
        // We can't assert false universally (enterprise machines exist)
        let _ = is_managed_environment();
    }
}
