//! Signal handling and graceful shutdown.

/// Register a handler for Ctrl+C / SIGTERM.
///
/// Spawns a tokio task that awaits the signal and then invokes `on_shutdown`.
pub fn register_shutdown_handler(_on_shutdown: impl Fn() + Send + 'static) {
    todo!()
}
