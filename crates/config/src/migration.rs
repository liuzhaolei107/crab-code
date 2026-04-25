//! Config schema migration — versioned upgrade chain.
//!
//! Each time config is loaded from disk, `migrate_settings()` checks
//! the `schemaVersion` field and applies any pending migrations in order.
//! Migrations are pure `fn(&mut Value)` transforms on the raw JSON before
//! it is deserialized into `Config`.

use serde_json::Value;

/// The current schema version. Bump this and add a migration entry to
/// `MIGRATIONS` when the settings format changes.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

type MigrateFn = fn(&mut Value);

/// Ordered list of `(target_version, migration_fn)` pairs.
/// Each function upgrades from `target_version - 1` to `target_version`.
const MIGRATIONS: &[(u32, MigrateFn)] = &[
    // Version 1 is the initial schema — no migration needed.
    // Future example: (2, migrate_v1_to_v2),
];

/// Apply pending migrations to raw settings JSON.
///
/// Reads `schemaVersion` (defaulting to 0 if absent), runs each
/// applicable migration in order, and stamps `schemaVersion` to
/// [`CURRENT_SCHEMA_VERSION`].
///
/// Returns the version the settings were at *before* migration
/// (useful for logging).
pub fn migrate_settings(raw: &mut Value) -> u32 {
    let current_version = raw
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    if current_version >= CURRENT_SCHEMA_VERSION {
        return current_version;
    }

    for &(target, migrate_fn) in MIGRATIONS {
        if current_version < target {
            migrate_fn(raw);
        }
    }

    raw["schemaVersion"] = Value::from(CURRENT_SCHEMA_VERSION);
    current_version
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrate_no_version_field() {
        let mut raw = json!({"model": "test"});
        let before = migrate_settings(&mut raw);
        assert_eq!(before, 0);
        assert_eq!(raw["schemaVersion"], CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrate_already_current() {
        let mut raw = json!({"schemaVersion": CURRENT_SCHEMA_VERSION});
        let before = migrate_settings(&mut raw);
        assert_eq!(before, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrate_preserves_existing_fields() {
        let mut raw = json!({"model": "test-model", "maxTokens": 4096});
        migrate_settings(&mut raw);
        assert_eq!(raw["model"], "test-model");
        assert_eq!(raw["maxTokens"], 4096);
        assert_eq!(raw["schemaVersion"], CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrate_future_version_untouched() {
        let mut raw = json!({"schemaVersion": 999});
        let before = migrate_settings(&mut raw);
        assert_eq!(before, 999);
        assert_eq!(raw["schemaVersion"], 999);
    }
}
