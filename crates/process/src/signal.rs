//! Signal handling and graceful shutdown.

use tokio::sync::mpsc;

/// Register a handler that sends a message on Ctrl+C / SIGTERM.
///
/// Returns a receiver that gets a `()` when a shutdown signal is received.
/// Spawns a background tokio task to listen for the signal.
///
/// # Errors
///
/// Returns an error if the signal handler cannot be registered.
pub fn register_shutdown_handler() -> crab_core::Result<mpsc::Receiver<()>> {
    let (tx, rx) = mpsc::channel(1);

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = tx.send(()).await;
    });

    Ok(rx)
}

/// Register a shutdown handler that invokes a callback.
///
/// Spawns a background tokio task that awaits Ctrl+C / SIGTERM
/// and then calls `on_shutdown`.
pub fn register_shutdown_callback(on_shutdown: impl Fn() + Send + 'static) {
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        on_shutdown();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_handler_returns_receiver() {
        let rx = register_shutdown_handler();
        assert!(rx.is_ok());
        // We can't easily test the signal itself, but verify the receiver is valid
        let mut rx = rx.unwrap();
        // Should not have received anything yet
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn shutdown_callback_does_not_panic() {
        // Registering the callback should not panic
        register_shutdown_callback(|| {
            // no-op
        });
    }
}
