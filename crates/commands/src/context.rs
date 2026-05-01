use std::path::Path;

use crab_core::model::ModelId;
use crab_core::permission::PermissionMode;

/// Flat snapshot of session cost data, passed into commands by value.
///
/// Callers build this from their own cost accumulator so that `crab-commands`
/// does not depend on `crab-session`.
#[derive(Debug, Clone, Default)]
pub struct CostSnapshot {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_cost_usd: f64,
    pub api_calls: u64,
}

/// Read-only context passed to every slash command.
pub struct CommandContext<'a> {
    pub model: &'a ModelId,
    pub session_id: &'a str,
    pub working_dir: &'a Path,
    pub permission_mode: PermissionMode,
    pub cost: CostSnapshot,
    pub estimated_tokens: u64,
    pub message_count: usize,
    pub memory_dir: Option<&'a Path>,
}
