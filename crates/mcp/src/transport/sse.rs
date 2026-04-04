use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::transport::Transport;
use std::future::Future;
use std::pin::Pin;

/// HTTP Server-Sent Events transport for remote MCP servers.
pub struct SseTransport {
    _url: String,
}

impl SseTransport {
    /// Create a new SSE transport targeting the given endpoint URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self { _url: url.into() }
    }
}

impl Transport for SseTransport {
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
