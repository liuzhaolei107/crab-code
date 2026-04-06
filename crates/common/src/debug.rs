use std::path::PathBuf;

/// Configuration for the debug/tracing subsystem.
#[derive(Debug, Clone, Default)]
pub struct DebugConfig {
    /// Whether debug output is enabled at all.
    pub enabled: bool,
    /// Optional filter string (e.g. "api", "hooks,tools") passed to `EnvFilter`.
    /// When `None` and `enabled` is true, defaults to "crab=debug".
    pub filter: Option<String>,
    /// Optional file path to write debug logs to (in addition to stderr).
    pub file: Option<PathBuf>,
}

/// Initialise the global tracing subscriber based on `config`.
///
/// Call this once, early in `main`. If `config.enabled` is false, this is a
/// no-op and the default (no-op) subscriber remains active.
pub fn init_debug(config: &DebugConfig) {
    if !config.enabled {
        return;
    }

    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    let filter_str = config
        .filter
        .as_deref()
        .unwrap_or("crab=debug");

    let env_filter = EnvFilter::try_new(filter_str)
        .unwrap_or_else(|_| EnvFilter::new("crab=debug"));

    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    if let Some(ref file_path) = config.file {
        let dir = file_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let filename = file_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("crab-debug.log"));
        let file_appender = tracing_appender::rolling::never(dir, filename);
        let file_layer = fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false)
            .with_target(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_config_default_is_disabled() {
        let cfg = DebugConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.filter.is_none());
        assert!(cfg.file.is_none());
    }

    #[test]
    fn init_debug_noop_when_disabled() {
        // Should not panic
        init_debug(&DebugConfig::default());
    }
}
