//! Child process spawning, environment inheritance.

use std::path::PathBuf;
use std::time::Duration;

/// Options for spawning a child process.
pub struct SpawnOptions {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub timeout: Option<Duration>,
    pub stdin_data: Option<String>,
}

/// Captured output from a completed child process.
pub struct SpawnOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Execute a command and wait for completion.
///
/// # Errors
///
/// Returns an error if the command cannot be spawned or output cannot be captured.
#[allow(clippy::unused_async)]
pub async fn run(_opts: SpawnOptions) -> crab_common::Result<SpawnOutput> {
    todo!()
}

/// Execute a command and stream stdout/stderr via callbacks.
///
/// # Errors
///
/// Returns an error if the command cannot be spawned.
#[allow(clippy::unused_async)]
pub async fn run_streaming(
    _opts: SpawnOptions,
    _on_stdout: impl Fn(&str) + Send,
    _on_stderr: impl Fn(&str) + Send,
) -> crab_common::Result<i32> {
    todo!()
}
