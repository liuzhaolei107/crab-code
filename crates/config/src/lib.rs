pub mod config;
pub mod gitignore;
pub mod hooks;
pub mod loader;
pub mod merge;
pub mod migration;
pub mod permissions;
pub mod plugin_loader;
pub mod runtime;
pub mod validation;
pub mod writer;

pub use config::{Config, ConfigLayer, EnabledPluginValue, GitContextConfig, PermissionsConfig};
pub use loader::{CliFlags, ResolveContext, overlay_config, resolve};
pub use merge::{dedup_preserving_order, merge_toml_values};
pub use migration::migrate_settings;
pub use permissions::PermissionRuleSet;
pub use validation::{
    ValidationError, validate_all_config_files, validate_config, validate_raw_file,
};
pub use writer::{WriteTarget, set_value};
