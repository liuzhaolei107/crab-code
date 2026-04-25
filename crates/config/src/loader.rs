//! Unified `Config` resolution pipeline.
//!
//! [`resolve`] is the single entry point that all callers use to obtain a
//! merged [`crate::config::Config`]. It assembles the file layer (defaults,
//! plugin contributions, user, project, local, `--config <path>`) and the
//! runtime layer (env, CLI flags) at the [`toml::Value`] level using
//! [`crate::merge::merge_toml_values`], then deserializes the result once.
//!
//! Phases later than 3 fill in the currently-empty extension points:
//! - Phase 5 fills [`env_to_value`] and [`cli_flags_to_value`].
//! - The plugin slot is filled by [`crate::plugin_loader`].
//! - Phase 7 inserts schema validation between merge and deserialization.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use toml::Value;
use toml::value::Table;

use crate::config::{
    Config, ConfigSource, config_file_name, global_config_dir, local_config_file_name,
    project_config_dir,
};
use crate::merge::merge_toml_values;

/// Inputs that drive [`resolve`]. Construct via [`ResolveContext::new`] and
/// then chain the `with_*` setters before passing to [`resolve`].
#[derive(Debug, Clone)]
pub struct ResolveContext {
    /// User-level config root (defaults to `~/.crab/`). Phase 5 will route
    /// `CRAB_CONFIG_DIR` and `--config-dir` through this field.
    pub config_dir: PathBuf,
    /// Optional project root. When set, `<project_dir>/.crab/config.toml`
    /// and `<project_dir>/.crab/config.local.toml` join the file layer.
    pub project_dir: Option<PathBuf>,
    /// Whole-file CLI override (`--config <path>`). Highest file-layer slot.
    pub cli_config_file: Option<PathBuf>,
    /// Captured environment for the runtime layer. Phase 5 reads it; Phase 3
    /// keeps it empty by default. Tests inject a fake map.
    pub env: HashMap<String, String>,
    /// CLI flag overrides for the runtime layer. Phase 5 introduces a real
    /// `CliFlags` struct; Phase 3 carries the raw `toml::Value` produced by
    /// the future flag parser, defaulting to an empty table.
    pub flags: CliFlags,
    /// Restrict which file-layer sources participate. `None` means all.
    pub sources_filter: Option<Vec<ConfigSource>>,
}

/// Placeholder for parsed CLI flag overrides.
///
/// Phase 5 expands this into a typed struct (model, `permission_mode`,
/// `-c key.path=value` accumulator, etc.); Phase 3 needs only an opaque
/// "no overrides yet" carrier.
#[derive(Debug, Clone, Default)]
pub struct CliFlags;

impl ResolveContext {
    /// Build a context with a default config dir (`~/.crab/`) and no
    /// runtime overrides.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config_dir: global_config_dir(),
            project_dir: None,
            cli_config_file: None,
            env: HashMap::new(),
            flags: CliFlags,
            sources_filter: None,
        }
    }

    #[must_use]
    pub fn with_config_dir(mut self, dir: PathBuf) -> Self {
        self.config_dir = dir;
        self
    }

    #[must_use]
    pub fn with_project_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.project_dir = dir;
        self
    }

    #[must_use]
    pub fn with_cli_config_file(mut self, path: Option<PathBuf>) -> Self {
        self.cli_config_file = path;
        self
    }

    #[must_use]
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Capture the full process environment. Convenience wrapper for the
    /// most common runtime usage.
    #[must_use]
    pub fn with_process_env(mut self) -> Self {
        self.env = std::env::vars().collect();
        self
    }

    #[must_use]
    pub fn with_flags(mut self, flags: CliFlags) -> Self {
        self.flags = flags;
        self
    }

    #[must_use]
    pub fn with_sources_filter(mut self, sources: Option<Vec<ConfigSource>>) -> Self {
        self.sources_filter = sources;
        self
    }
}

impl Default for ResolveContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve a [`Config`] by merging every layer in priority order.
///
/// Priority (low → high, aligned with `docs/config.md` §9):
///
/// ```text
///   defaults
///     < plugin
///     < user (config_dir/config.toml)
///     < project (<project_dir>/.crab/config.toml)
///     < local (<project_dir>/.crab/config.local.toml)
///     < cli_config_file (--config <path>)
///     < env runtime
///     < CLI flag runtime
/// ```
///
/// Each file source is graceful: a missing file is skipped, a malformed
/// file currently surfaces as an error (Phase 8 will downgrade parse
/// failures to a warning per `docs/config.md` §10.1).
pub fn resolve(ctx: &ResolveContext) -> crab_core::Result<Config> {
    let mut value = defaults_as_value()?;

    // Plugin layer.
    for plugin_cfg in load_enabled_plugin_configs(ctx)? {
        merge_toml_values(&mut value, plugin_cfg);
    }

    // File layer.
    for path in file_layer_paths(ctx) {
        if let Some(layer) = load_toml_file(&path)? {
            merge_toml_values(&mut value, layer);
        }
    }

    // Runtime layer: env first, then CLI flags. Both stubbed in Phase 3.
    let mut runtime = Value::Table(Table::new());
    merge_toml_values(&mut runtime, env_to_value(&ctx.env)?);
    merge_toml_values(&mut runtime, cli_flags_to_value(&ctx.flags)?);
    merge_toml_values(&mut value, runtime);

    value.try_into().map_err(|e: toml::de::Error| {
        crab_core::Error::Config(format!("config deserialization error: {e}"))
    })
}

/// Compute the file-layer paths that participate in `resolve`, honoring
/// `sources_filter`. Order is low → high priority.
fn file_layer_paths(ctx: &ResolveContext) -> Vec<PathBuf> {
    let include = |source: ConfigSource| {
        ctx.sources_filter
            .as_ref()
            .is_none_or(|list| list.contains(&source))
    };

    let mut paths = Vec::new();

    if include(ConfigSource::User) {
        paths.push(ctx.config_dir.join(config_file_name()));
    }
    if include(ConfigSource::Project)
        && let Some(dir) = ctx.project_dir.as_deref()
    {
        paths.push(project_config_dir(dir).join(config_file_name()));
    }
    if include(ConfigSource::Local)
        && let Some(dir) = ctx.project_dir.as_deref()
    {
        paths.push(project_config_dir(dir).join(local_config_file_name()));
    }

    // `--config <path>` is always honored regardless of `sources_filter` —
    // the user is explicitly requesting it for this invocation.
    if let Some(path) = ctx.cli_config_file.as_deref() {
        paths.push(path.to_path_buf());
    }

    paths
}

/// Return the compiled-in defaults serialized to a `toml::Value` table so
/// they participate in the merge chain like any other source.
fn defaults_as_value() -> crab_core::Result<Value> {
    let defaults = Config::default();
    Value::try_from(&defaults)
        .map_err(|e| crab_core::Error::Config(format!("default config not serializable: {e}")))
}

/// Read a TOML file into a `toml::Value`. Returns `Ok(None)` when the file
/// does not exist; surfaces parse and IO errors otherwise.
fn load_toml_file(path: &Path) -> crab_core::Result<Option<Value>> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let value: Value = toml::from_str(&text).map_err(|e| {
                crab_core::Error::Config(format!("failed to parse {}: {e}", path.display()))
            })?;
            Ok(Some(value))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(crab_core::Error::Config(format!(
            "failed to read {}: {e}",
            path.display()
        ))),
    }
}

/// Scan and load all enabled plugin contributions, alphabetically ordered.
///
/// Delegates to [`crate::plugin_loader::load_enabled_plugin_configs`], which
/// peeks the user-level `enabledPlugins` map, filters and orders the
/// `plugins/<name>/config.json` files, and converts each to `toml::Value`.
fn load_enabled_plugin_configs(ctx: &ResolveContext) -> crab_core::Result<Vec<Value>> {
    crate::plugin_loader::load_enabled_plugin_configs(ctx)
}

/// Stub: project process environment into a partial `toml::Value`.
///
/// Phase 5 maps `CRAB_MODEL`, `CRAB_API_PROVIDER`, etc. Phase 3 returns an
/// empty table so callers can already pass through the runtime slot.
#[allow(clippy::unnecessary_wraps)]
fn env_to_value(_env: &HashMap<String, String>) -> crab_core::Result<Value> {
    Ok(Value::Table(Table::new()))
}

/// Stub: project parsed CLI flags into a partial `toml::Value`.
///
/// Phase 5 introduces `--model`, `--permission-mode`, and the
/// `-c key.path=value` accumulator. Phase 3 returns an empty table.
#[allow(clippy::unnecessary_wraps)]
fn cli_flags_to_value(_flags: &CliFlags) -> crab_core::Result<Value> {
    Ok(Value::Table(Table::new()))
}

/// Merge `overlay` into `base` at the value layer.
///
/// Used by call sites that need to layer an extra source on top of an
/// already-resolved config (e.g. legacy `--settings <inline-json>`). Both
/// inputs round-trip through `toml::Value`, so the same array
/// concat+dedup and table deep-merge semantics apply as in [`resolve`].
pub fn overlay_config(base: &Config, overlay: &Config) -> crab_core::Result<Config> {
    let mut base_value = Value::try_from(base)
        .map_err(|e| crab_core::Error::Config(format!("base config not serializable: {e}")))?;
    let overlay_value = Value::try_from(overlay)
        .map_err(|e| crab_core::Error::Config(format!("overlay config not serializable: {e}")))?;
    merge_toml_values(&mut base_value, overlay_value);
    base_value.try_into().map_err(|e: toml::de::Error| {
        crab_core::Error::Config(format!("merge result invalid: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn resolve_with_no_files_returns_defaults() {
        let dir = std::env::temp_dir().join("crab-loader-test-defaults");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let ctx = ResolveContext::new()
            .with_config_dir(dir.clone())
            .with_project_dir(None);
        let cfg = resolve(&ctx).unwrap();
        assert_eq!(cfg, Config::default());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_user_then_project_then_local_priority() {
        let root = std::env::temp_dir().join("crab-loader-test-chain");
        let user_dir = root.join("user");
        let project_dir = root.join("project");
        let project_crab = project_dir.join(".crab");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_crab).unwrap();

        write(
            &user_dir.join("config.toml"),
            "model = \"user\"\ntheme = \"light\"\n",
        );
        write(
            &project_crab.join("config.toml"),
            "model = \"project\"\nlanguage = \"en\"\n",
        );
        write(
            &project_crab.join("config.local.toml"),
            "theme = \"dark\"\n",
        );

        let ctx = ResolveContext::new()
            .with_config_dir(user_dir)
            .with_project_dir(Some(project_dir));
        let cfg = resolve(&ctx).unwrap();
        assert_eq!(cfg.model.as_deref(), Some("project"));
        assert_eq!(cfg.theme.as_deref(), Some("dark"));
        assert_eq!(cfg.language.as_deref(), Some("en"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_cli_config_file_overrides_local() {
        let root = std::env::temp_dir().join("crab-loader-test-cli-file");
        let user_dir = root.join("user");
        let project_dir = root.join("project");
        let project_crab = project_dir.join(".crab");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_crab).unwrap();

        write(
            &project_crab.join("config.local.toml"),
            "model = \"local\"\n",
        );
        let cli_file = root.join("override.toml");
        write(&cli_file, "model = \"cli-file\"\n");

        let ctx = ResolveContext::new()
            .with_config_dir(user_dir)
            .with_project_dir(Some(project_dir))
            .with_cli_config_file(Some(cli_file));
        let cfg = resolve(&ctx).unwrap();
        assert_eq!(cfg.model.as_deref(), Some("cli-file"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sources_filter_excludes_project() {
        let root = std::env::temp_dir().join("crab-loader-test-filter");
        let user_dir = root.join("user");
        let project_dir = root.join("project");
        let project_crab = project_dir.join(".crab");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_crab).unwrap();

        write(&user_dir.join("config.toml"), "model = \"user\"\n");
        write(&project_crab.join("config.toml"), "model = \"project\"\n");

        let ctx = ResolveContext::new()
            .with_config_dir(user_dir)
            .with_project_dir(Some(project_dir))
            .with_sources_filter(Some(vec![ConfigSource::User]));
        let cfg = resolve(&ctx).unwrap();
        assert_eq!(cfg.model.as_deref(), Some("user"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn malformed_toml_surfaces_error() {
        let root = std::env::temp_dir().join("crab-loader-test-malformed");
        let user_dir = root.join("user");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&user_dir).unwrap();
        write(&user_dir.join("config.toml"), "not = valid = toml");

        let ctx = ResolveContext::new().with_config_dir(user_dir);
        let result = resolve(&ctx);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn overlay_config_field_wins_and_arrays_concat() {
        use crate::config::PermissionsConfig;
        let base = Config {
            model: Some("base".into()),
            permissions: Some(PermissionsConfig {
                allow: vec!["Bash".into()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let overlay = Config {
            model: Some("over".into()),
            permissions: Some(PermissionsConfig {
                allow: vec!["Edit".into()],
                ..Default::default()
            }),
            ..Default::default()
        };
        let merged = overlay_config(&base, &overlay).unwrap();
        assert_eq!(merged.model.as_deref(), Some("over"));
        let allow = merged.permissions.unwrap().allow;
        assert_eq!(allow, vec!["Bash".to_string(), "Edit".to_string()]);
    }
}
