use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::transport::Transport;
use std::future::Future;
use std::pin::Pin;

/// Stdin/stdout transport for MCP servers launched as child processes.
pub struct StdioTransport {
    _private: (),
}

impl StdioTransport {
    /// Create a new stdio transport connected to a child process.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Transport for StdioTransport {
    fn send(
        &self,
        _req: JsonRpcRequest,
    ) -> Pin<Box<dyn Future<Output = crab_common::Result<JsonRpcResponse>> + Send + '_>> {
        Box::pin(async move { todo!() })
    }

    fn notify(
        &self,
        _method: &str,
        _params: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>> {
        Box::pin(async move { todo!() })
    }

    fn close(&self) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>> {
        Box::pin(async move { todo!() })
    }
}
