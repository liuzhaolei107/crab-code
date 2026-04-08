//! Managed Device Management: reads enterprise-managed settings from well-known paths.
//!
//! On managed corporate devices, system administrators may deploy Crab Code
//! configuration via MDM profiles. This module discovers and loads those
//! settings so they can be merged (with highest priority) into the
//! configuration stack.
//!
//! Well-known paths checked:
//! - macOS: `/Library/Managed Preferences/com.crabcode.settings.plist`
//! - Windows: `HKLM\SOFTWARE\Policies\CrabCode` (via registry, read as JSON)
//! - Linux: `/etc/crab-code/managed.json`

use std::path::PathBuf;

// ── Discovery ─────────────────────────────────────────────────────────

/// Return the platform-specific paths where managed settings may be found.
///
/// The returned paths are ordered by priority (highest-priority first).
/// Not all paths may exist; callers should check existence before reading.
#[must_use]
pub fn managed_settings_paths() -> Vec<PathBuf> {
    todo!("managed_settings_paths: return platform-specific MDM config paths")
}

// ── Loading ───────────────────────────────────────────────────────────

/// Load and parse the managed settings from the first available well-known
/// path. Returns `None` if no managed settings are found.
///
/// The returned `Value` follows the same schema as `settings.json` and
/// will be merged into the configuration stack with the highest priority.
#[must_use]
pub fn load_managed_settings() -> Option<serde_json::Value> {
    todo!("load_managed_settings: try each managed_settings_path, parse first found")
}

// ── Detection ─────────────────────────────────────────────────────────

/// Detect whether the current device appears to be enterprise-managed.
///
/// Returns `true` if any of the well-known MDM paths exist, even if
/// the contents are empty or unparseable. This is used to adjust the
/// default permission mode (managed environments default to stricter
/// permissions).
#[must_use]
pub fn is_managed_environment() -> bool {
    todo!("is_managed_environment: check if any managed settings paths exist")
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // Smoke tests — the real functions are `todo!()` so we just verify
    // the module compiles and types are correct.

    #[test]
    fn module_compiles() {
        // Intentionally empty — existence of this test proves the module
        // is syntactically valid and all imports resolve.
    }
}
