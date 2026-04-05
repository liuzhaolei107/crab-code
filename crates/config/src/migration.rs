//! Settings migration system — versioned config with ordered upgrades.
//!
//! Tracks a `configVersion` field inside `settings.json`. When the running
//! application expects a newer version, [`MigrationRunner`] applies each
//! intermediate migration in order (v0→v1→v2→…) and writes the result back.
//!
//! A [`BackupManager`] automatically snapshots the old file before any
//! migration runs, so the user can roll back manually if needed.

use serde_json::Value;
use std::path::{Path, PathBuf};

// ── ConfigVersion ───────────────────────────────────────────────────────

/// The current config schema version that the application expects.
pub const CURRENT_VERSION: u32 = 1;

/// Read the `configVersion` field from a JSON value. Returns `0` when absent.
#[must_use]
pub fn read_version(root: &Value) -> u32 {
    root.get("configVersion")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}

/// Set the `configVersion` field on a JSON object.
fn set_version(root: &mut Value, version: u32) {
    if let Some(obj) = root.as_object_mut() {
        obj.insert("configVersion".to_string(), Value::Number(version.into()));
    }
}

// ── Migration trait ─────────────────────────────────────────────────────

/// A single migration step that transforms the config JSON from one version
/// to the next.
pub trait Migration: Send + Sync {
    /// The version this migration produces (e.g. `1` for the v0→v1 migration).
    fn target_version(&self) -> u32;

    /// Human-readable description shown in logs / dry-run output.
    fn description(&self) -> &str;

    /// Apply the migration in-place. Must NOT update `configVersion` — the
    /// runner handles that.
    fn apply(&self, root: &mut Value) -> crab_common::Result<()>;
}

// ── Built-in migrations ─────────────────────────────────────────────────

/// v0 → v1: add default fields that were introduced after the initial release.
struct MigrateV0ToV1;

impl Migration for MigrateV0ToV1 {
    fn target_version(&self) -> u32 {
        1
    }

    fn description(&self) -> &'static str {
        "add default permissionMode and theme fields"
    }

    fn apply(&self, root: &mut Value) -> crab_common::Result<()> {
        let Some(obj) = root.as_object_mut() else {
            return Ok(());
        };
        // Add permissionMode if missing
        obj.entry("permissionMode")
            .or_insert_with(|| Value::String("default".to_string()));
        // Add theme if missing
        obj.entry("theme")
            .or_insert_with(|| Value::String("auto".to_string()));
        Ok(())
    }
}

/// Return all built-in migrations in order.
fn builtin_migrations() -> Vec<Box<dyn Migration>> {
    vec![Box::new(MigrateV0ToV1)]
}

// ── BackupManager ───────────────────────────────────────────────────────

/// Creates timestamped backups of settings files before migration.
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a manager that stores backups under `backup_dir`.
    #[must_use]
    pub fn new(backup_dir: PathBuf) -> Self {
        Self { backup_dir }
    }

    /// Default backup directory: `~/.crab/backups/`.
    #[must_use]
    pub fn default_dir() -> PathBuf {
        crate::settings::global_config_dir().join("backups")
    }

    /// Back up `source` before a migration. Returns the backup path.
    ///
    /// The backup filename encodes the original version:
    /// `settings.v{version}.{timestamp}.json`
    pub fn backup(&self, source: &Path, version: u32) -> crab_common::Result<PathBuf> {
        std::fs::create_dir_all(&self.backup_dir).map_err(|e| {
            crab_common::Error::Config(format!(
                "cannot create backup dir {}: {e}",
                self.backup_dir.display()
            ))
        })?;

        let timestamp = timestamp_compact();
        let filename = format!("settings.v{version}.{timestamp}.json");
        let dest = self.backup_dir.join(filename);

        std::fs::copy(source, &dest).map_err(|e| {
            crab_common::Error::Config(format!(
                "backup failed {} → {}: {e}",
                source.display(),
                dest.display()
            ))
        })?;

        Ok(dest)
    }

    /// List existing backups sorted by name (oldest first).
    pub fn list_backups(&self) -> crab_common::Result<Vec<PathBuf>> {
        if !self.backup_dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&self.backup_dir)
            .map_err(|e| {
                crab_common::Error::Config(format!(
                    "cannot read backup dir {}: {e}",
                    self.backup_dir.display()
                ))
            })?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("settings.v")
                    && std::path::Path::new(&name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .collect();
        entries.sort();
        Ok(entries)
    }
}

// ── MigrationRunner ─────────────────────────────────────────────────────

/// Orchestrates ordered migration of a settings file from its current version
/// to [`CURRENT_VERSION`].
pub struct MigrationRunner {
    migrations: Vec<Box<dyn Migration>>,
    backup_manager: Option<BackupManager>,
}

impl MigrationRunner {
    /// Create a runner with built-in migrations and automatic backup.
    #[must_use]
    pub fn new() -> Self {
        Self {
            migrations: builtin_migrations(),
            backup_manager: Some(BackupManager::new(BackupManager::default_dir())),
        }
    }

    /// Create a runner with a custom backup directory.
    #[must_use]
    pub fn with_backup_dir(backup_dir: PathBuf) -> Self {
        Self {
            migrations: builtin_migrations(),
            backup_manager: Some(BackupManager::new(backup_dir)),
        }
    }

    /// Create a runner that skips backup (useful for tests).
    #[must_use]
    pub fn without_backup() -> Self {
        Self {
            migrations: builtin_migrations(),
            backup_manager: None,
        }
    }

    /// Register an additional migration (for plugins / extensions).
    pub fn add_migration(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
        self.migrations.sort_by_key(|m| m.target_version());
    }

    /// Check whether `path` needs migration.
    pub fn needs_migration(&self, path: &Path) -> crab_common::Result<bool> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => {
                return Err(crab_common::Error::Config(format!(
                    "cannot read {}: {e}",
                    path.display()
                )));
            }
        };
        let root = parse_jsonc_value(&content)?;
        let version = read_version(&root);
        Ok(version < CURRENT_VERSION)
    }

    /// Migrate the settings file at `path` in-place.
    ///
    /// Returns `Ok(MigrationResult)` describing what happened. If the file
    /// is already at `CURRENT_VERSION` (or newer), no writes occur.
    pub fn migrate(&self, path: &Path) -> crab_common::Result<MigrationResult> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(MigrationResult {
                    from_version: 0,
                    to_version: 0,
                    applied: Vec::new(),
                    backup_path: None,
                });
            }
            Err(e) => {
                return Err(crab_common::Error::Config(format!(
                    "cannot read {}: {e}",
                    path.display()
                )));
            }
        };

        let mut root = parse_jsonc_value(&content)?;
        let from_version = read_version(&root);

        if from_version >= CURRENT_VERSION {
            return Ok(MigrationResult {
                from_version,
                to_version: from_version,
                applied: Vec::new(),
                backup_path: None,
            });
        }

        // Backup before mutating
        let backup_path = if let Some(ref bm) = self.backup_manager {
            Some(bm.backup(path, from_version)?)
        } else {
            None
        };

        // Apply each migration whose target_version is in (from_version, CURRENT_VERSION]
        let mut applied = Vec::new();
        for migration in &self.migrations {
            let tv = migration.target_version();
            if tv > from_version && tv <= CURRENT_VERSION {
                migration.apply(&mut root)?;
                set_version(&mut root, tv);
                applied.push(tv);
            }
        }

        // Write back
        let pretty = serde_json::to_string_pretty(&root).map_err(|e| {
            crab_common::Error::Config(format!("failed to serialize migrated config: {e}"))
        })?;
        std::fs::write(path, pretty).map_err(|e| {
            crab_common::Error::Config(format!(
                "failed to write migrated config to {}: {e}",
                path.display()
            ))
        })?;

        Ok(MigrationResult {
            from_version,
            to_version: CURRENT_VERSION,
            applied,
            backup_path,
        })
    }

    /// Dry-run: return which migrations would be applied without writing.
    pub fn plan(&self, path: &Path) -> crab_common::Result<Vec<MigrationPlan>> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(crab_common::Error::Config(format!(
                    "cannot read {}: {e}",
                    path.display()
                )));
            }
        };
        let root = parse_jsonc_value(&content)?;
        let from_version = read_version(&root);

        Ok(self
            .migrations
            .iter()
            .filter(|m| {
                let tv = m.target_version();
                tv > from_version && tv <= CURRENT_VERSION
            })
            .map(|m| MigrationPlan {
                target_version: m.target_version(),
                description: m.description().to_string(),
            })
            .collect())
    }
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a migration run.
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// Config version before migration.
    pub from_version: u32,
    /// Config version after migration.
    pub to_version: u32,
    /// Target versions of each migration that was applied.
    pub applied: Vec<u32>,
    /// Path to the backup file, if one was created.
    pub backup_path: Option<PathBuf>,
}

impl MigrationResult {
    /// True if any migrations were actually applied.
    #[must_use]
    pub fn was_migrated(&self) -> bool {
        !self.applied.is_empty()
    }
}

/// A planned migration step (dry-run output).
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    pub target_version: u32,
    pub description: String,
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Parse JSONC into a `serde_json::Value`.
fn parse_jsonc_value(content: &str) -> crab_common::Result<Value> {
    jsonc_parser::parse_to_serde_value::<Value>(content, &jsonc_parser::ParseOptions::default())
        .map_err(|e| crab_common::Error::Config(format!("JSONC parse error: {e}")))
}

/// Compact timestamp for backup filenames: `YYYYMMDD_HHMMSS`.
///
/// Uses Howard Hinnant's `civil_from_days` algorithm.
fn timestamp_compact() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    let days = (secs / 86400).cast_signed();
    let day_secs = secs % 86400;

    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let doe = (shifted - era * 146_097).cast_unsigned();
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let year_raw = yoe.cast_signed() + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let month_offset = (5 * doy + 2) / 153;
    let day = doy - (153 * month_offset + 2) / 5 + 1;
    let month = if month_offset < 10 {
        month_offset + 3
    } else {
        month_offset - 9
    };
    let year = if month <= 2 { year_raw + 1 } else { year_raw };

    let hour = day_secs / 3600;
    let min = (day_secs % 3600) / 60;
    let sec = day_secs % 60;

    format!("{year:04}{month:02}{day:02}_{hour:02}{min:02}{sec:02}")
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ConfigVersion ───────────────────────────────────────────────────

    #[test]
    fn read_version_missing_returns_zero() {
        let root = serde_json::json!({"model": "test"});
        assert_eq!(read_version(&root), 0);
    }

    #[test]
    fn read_version_present() {
        let root = serde_json::json!({"configVersion": 3});
        assert_eq!(read_version(&root), 3);
    }

    #[test]
    fn read_version_non_numeric_returns_zero() {
        let root = serde_json::json!({"configVersion": "abc"});
        assert_eq!(read_version(&root), 0);
    }

    #[test]
    fn set_version_on_object() {
        let mut root = serde_json::json!({"model": "test"});
        set_version(&mut root, 5);
        assert_eq!(read_version(&root), 5);
    }

    #[test]
    fn set_version_overwrites_existing() {
        let mut root = serde_json::json!({"configVersion": 1});
        set_version(&mut root, 2);
        assert_eq!(read_version(&root), 2);
    }

    #[test]
    fn set_version_noop_on_non_object() {
        let mut root = serde_json::json!("not an object");
        set_version(&mut root, 1);
        // Should not panic, just no-op
        assert_eq!(read_version(&root), 0);
    }

    // ── MigrateV0ToV1 ──────────────────────────────────────────────────

    #[test]
    fn migrate_v0_to_v1_adds_defaults() {
        let mut root = serde_json::json!({"model": "gpt-4o"});
        let m = MigrateV0ToV1;
        m.apply(&mut root).unwrap();

        assert_eq!(root["permissionMode"], "default");
        assert_eq!(root["theme"], "auto");
        assert_eq!(root["model"], "gpt-4o"); // preserved
    }

    #[test]
    fn migrate_v0_to_v1_preserves_existing_fields() {
        let mut root = serde_json::json!({"permissionMode": "dangerously", "theme": "dark"});
        let m = MigrateV0ToV1;
        m.apply(&mut root).unwrap();

        assert_eq!(root["permissionMode"], "dangerously"); // not overwritten
        assert_eq!(root["theme"], "dark"); // not overwritten
    }

    #[test]
    fn migrate_v0_to_v1_noop_on_non_object() {
        let mut root = serde_json::json!([1, 2, 3]);
        let m = MigrateV0ToV1;
        assert!(m.apply(&mut root).is_ok());
    }

    #[test]
    fn migrate_v0_to_v1_metadata() {
        let m = MigrateV0ToV1;
        assert_eq!(m.target_version(), 1);
        assert!(!m.description().is_empty());
    }

    // ── BackupManager ───────────────────────────────────────────────────

    #[test]
    fn backup_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("settings.json");
        std::fs::write(&source, r#"{"model": "test"}"#).unwrap();

        let backup_dir = tmp.path().join("backups");
        let bm = BackupManager::new(backup_dir.clone());
        let backup_path = bm.backup(&source, 0).unwrap();

        assert!(backup_path.exists());
        assert!(backup_path.starts_with(&backup_dir));

        let name = backup_path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("settings.v0."));
        assert!(name.ends_with(".json"));

        let content = std::fs::read_to_string(&backup_path).unwrap();
        assert_eq!(content, r#"{"model": "test"}"#);
    }

    #[test]
    fn backup_creates_dir_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("settings.json");
        std::fs::write(&source, "{}").unwrap();

        let backup_dir = tmp.path().join("deep").join("nested").join("backups");
        let bm = BackupManager::new(backup_dir.clone());
        let backup_path = bm.backup(&source, 0).unwrap();
        assert!(backup_path.exists());
    }

    #[test]
    fn list_backups_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bm = BackupManager::new(tmp.path().to_path_buf());
        let list = bm.list_backups().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_backups_nonexistent_dir() {
        let bm = BackupManager::new(PathBuf::from("/nonexistent/backup/dir"));
        let list = bm.list_backups().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_backups_filters_non_matching() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("settings.v0.20260101_000000.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("random.txt"), "nope").unwrap();
        std::fs::write(tmp.path().join("settings.v1.20260102_000000.json"), "{}").unwrap();

        let bm = BackupManager::new(tmp.path().to_path_buf());
        let list = bm.list_backups().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn list_backups_sorted() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("settings.v0.20260201_120000.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("settings.v0.20260101_120000.json"), "{}").unwrap();

        let bm = BackupManager::new(tmp.path().to_path_buf());
        let list = bm.list_backups().unwrap();
        let names: Vec<String> = list
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names[0] < names[1]); // sorted ascending
    }

    // ── MigrationRunner ─────────────────────────────────────────────────

    #[test]
    fn runner_no_migration_needed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({"configVersion": 1})).unwrap(),
        )
        .unwrap();

        let runner = MigrationRunner::without_backup();
        let result = runner.migrate(&path).unwrap();
        assert!(!result.was_migrated());
        assert_eq!(result.from_version, 1);
        assert_eq!(result.to_version, 1);
    }

    #[test]
    fn runner_migrates_v0_to_current() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"model": "gpt-4o"}"#).unwrap();

        let runner = MigrationRunner::without_backup();
        let result = runner.migrate(&path).unwrap();
        assert!(result.was_migrated());
        assert_eq!(result.from_version, 0);
        assert_eq!(result.to_version, CURRENT_VERSION);
        assert!(result.applied.contains(&1));

        // Verify file was rewritten
        let content = std::fs::read_to_string(&path).unwrap();
        let root: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(read_version(&root), CURRENT_VERSION);
        assert_eq!(root["permissionMode"], "default");
        assert_eq!(root["theme"], "auto");
        assert_eq!(root["model"], "gpt-4o"); // preserved
    }

    #[test]
    fn runner_creates_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"model": "test"}"#).unwrap();

        let backup_dir = tmp.path().join("backups");
        let runner = MigrationRunner::with_backup_dir(backup_dir.clone());
        let result = runner.migrate(&path).unwrap();
        assert!(result.was_migrated());
        assert!(result.backup_path.is_some());
        assert!(result.backup_path.unwrap().exists());
    }

    #[test]
    fn runner_without_backup_has_no_backup_path() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"model": "test"}"#).unwrap();

        let runner = MigrationRunner::without_backup();
        let result = runner.migrate(&path).unwrap();
        assert!(result.backup_path.is_none());
    }

    #[test]
    fn runner_missing_file_returns_empty_result() {
        let runner = MigrationRunner::without_backup();
        let result = runner
            .migrate(Path::new("/nonexistent/settings.json"))
            .unwrap();
        assert!(!result.was_migrated());
        assert_eq!(result.from_version, 0);
        assert_eq!(result.to_version, 0);
    }

    #[test]
    fn runner_needs_migration_true() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"model": "test"}"#).unwrap();

        let runner = MigrationRunner::without_backup();
        assert!(runner.needs_migration(&path).unwrap());
    }

    #[test]
    fn runner_needs_migration_false_when_current() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({"configVersion": 1})).unwrap(),
        )
        .unwrap();

        let runner = MigrationRunner::without_backup();
        assert!(!runner.needs_migration(&path).unwrap());
    }

    #[test]
    fn runner_needs_migration_false_when_missing() {
        let runner = MigrationRunner::without_backup();
        assert!(!runner.needs_migration(Path::new("/nonexistent")).unwrap());
    }

    #[test]
    fn runner_plan_shows_pending_migrations() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"model": "test"}"#).unwrap();

        let runner = MigrationRunner::without_backup();
        let plan = runner.plan(&path).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].target_version, 1);
        assert!(!plan[0].description.is_empty());
    }

    #[test]
    fn runner_plan_empty_when_current() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({"configVersion": 1})).unwrap(),
        )
        .unwrap();

        let runner = MigrationRunner::without_backup();
        let plan = runner.plan(&path).unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn runner_plan_empty_for_missing_file() {
        let runner = MigrationRunner::without_backup();
        let plan = runner.plan(Path::new("/nonexistent")).unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn runner_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"model": "gpt-4o"}"#).unwrap();

        let runner = MigrationRunner::without_backup();

        // First run should migrate
        let r1 = runner.migrate(&path).unwrap();
        assert!(r1.was_migrated());

        // Second run should be a no-op
        let r2 = runner.migrate(&path).unwrap();
        assert!(!r2.was_migrated());
        assert_eq!(r2.from_version, CURRENT_VERSION);
    }

    #[test]
    fn runner_handles_jsonc_input() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
                // user comment
                "model": "test"
            }"#,
        )
        .unwrap();

        let runner = MigrationRunner::without_backup();
        let result = runner.migrate(&path).unwrap();
        assert!(result.was_migrated());

        // File should now be valid JSON (comments stripped during migration)
        let content = std::fs::read_to_string(&path).unwrap();
        let root: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(read_version(&root), CURRENT_VERSION);
    }

    // ── Custom migration ────────────────────────────────────────────────

    struct MockMigrationV2;

    impl Migration for MockMigrationV2 {
        fn target_version(&self) -> u32 {
            2
        }

        fn description(&self) -> &str {
            "mock v1->v2 migration"
        }

        fn apply(&self, root: &mut Value) -> crab_common::Result<()> {
            if let Some(obj) = root.as_object_mut() {
                obj.insert("migratedV2".to_string(), Value::Bool(true));
            }
            Ok(())
        }
    }

    #[test]
    fn runner_add_custom_migration() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({"configVersion": 1})).unwrap(),
        )
        .unwrap();

        let mut runner = MigrationRunner::without_backup();
        runner.add_migration(Box::new(MockMigrationV2));

        // With CURRENT_VERSION=1, the v2 migration should NOT apply
        // (it targets version > CURRENT_VERSION)
        let result = runner.migrate(&path).unwrap();
        assert!(!result.was_migrated());
    }

    // ── Timestamp helper ────────────────────────────────────────────────

    #[test]
    fn timestamp_compact_format() {
        let ts = timestamp_compact();
        // Should be 15 characters: YYYYMMDD_HHMMSS
        assert_eq!(ts.len(), 15);
        assert_eq!(ts.as_bytes()[8], b'_');
        // All other chars should be digits
        for (i, ch) in ts.chars().enumerate() {
            if i == 8 {
                assert_eq!(ch, '_');
            } else {
                assert!(ch.is_ascii_digit(), "char at {i} is not a digit: {ch}");
            }
        }
    }

    // ── Builtin migrations ordering ─────────────────────────────────────

    #[test]
    fn builtin_migrations_are_ordered() {
        let migrations = builtin_migrations();
        for window in migrations.windows(2) {
            assert!(
                window[0].target_version() < window[1].target_version(),
                "migrations not ordered: v{} >= v{}",
                window[0].target_version(),
                window[1].target_version()
            );
        }
    }

    #[test]
    fn builtin_migrations_target_up_to_current() {
        let migrations = builtin_migrations();
        if let Some(last) = migrations.last() {
            assert_eq!(last.target_version(), CURRENT_VERSION);
        }
    }
}
