//! Stdio transport helper for ACP agents.
//!
//! Provides [`serve_stdio`] which takes a fully-configured
//! [`Builder`](agent_client_protocol::Builder) and connects it to the
//! process's stdin/stdout via [`ByteStreams`].

use agent_client_protocol::{Agent, ByteStreams, Client, HandleDispatchFrom, RunWithConnectionTo};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// Errors raised while serving the ACP connection.
#[derive(Debug, thiserror::Error)]
pub enum AcpServeError {
    #[error("ACP connection error: {0}")]
    Connection(#[from] agent_client_protocol::Error),
}

/// Connect a fully-configured agent builder to stdio and run until the
/// connection closes.
///
/// The caller constructs the builder via
/// `Agent.builder().on_receive_request(...)` etc., then passes it here.
/// This function only provides the transport.
pub async fn serve_stdio<H, R>(
    builder: agent_client_protocol::Builder<Agent, H, R>,
) -> Result<(), AcpServeError>
where
    H: HandleDispatchFrom<Client> + Send + 'static,
    R: RunWithConnectionTo<Client> + Send + 'static,
{
    let transport = ByteStreams::new(
        tokio::io::stdout().compat_write(),
        tokio::io::stdin().compat(),
    );
    builder.connect_to(transport).await.map_err(Into::into)
}
