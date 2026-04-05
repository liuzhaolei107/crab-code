//! WebSocket transport for MCP servers.
//!
//! Connects to an MCP server over WebSocket (ws:// or wss://), sending and
//! receiving JSON-RPC messages as text frames. A background reader task
//! dispatches incoming responses to pending oneshot channels.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, oneshot};
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::transport::Transport;

/// Connection timeout for the initial WebSocket handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// WebSocket transport for MCP servers.
pub struct WsTransport {
    /// The original URL (for logging/reconnect).
    url: String,
    /// Sink half of the WebSocket stream, shared for concurrent writes.
    writer: Arc<Mutex<WsSink>>,
    /// Pending response senders, keyed by request ID.
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    /// Handle to the background reader task.
    reader_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

/// Type alias for the write half of a tungstenite WebSocket stream over TCP+TLS.
type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

impl std::fmt::Debug for WsTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsTransport")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

impl WsTransport {
    /// Connect to a WebSocket MCP server at the given URL.
    ///
    /// Performs the WebSocket handshake with a timeout, then spawns a
    /// background reader task to dispatch incoming JSON-RPC responses.
    pub async fn connect(url: &str) -> crab_common::Result<Self> {
        use futures::StreamExt as _;

        tracing::debug!(url, "connecting to MCP WebSocket server");

        let (ws_stream, _response) =
            tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url))
                .await
                .map_err(|_| {
                    crab_common::Error::Other(format!(
                        "WebSocket connection to {url} timed out after {CONNECT_TIMEOUT:?}"
                    ))
                })?
                .map_err(|e| {
                    crab_common::Error::Other(format!("WebSocket connection to {url} failed: {e}"))
                })?;

        tracing::debug!(url, "WebSocket connected");

        let (write_half, read_half) = ws_stream.split();
        let writer = Arc::new(Mutex::new(write_half));
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Spawn background reader task.
        let pending_clone = Arc::clone(&pending);
        let url_clone = url.to_string();
        let reader_handle = tokio::spawn(async move {
            use futures::StreamExt as _;
            let mut read_half = read_half;

            while let Some(msg_result) = read_half.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&text) {
                            let mut map = pending_clone.lock().await;
                            if let Some(tx) = map.remove(&resp.id) {
                                let _ = tx.send(resp);
                            }
                        }
                        // Server notifications are silently dropped for now.
                    }
                    Ok(Message::Close(_)) => {
                        tracing::debug!(url = url_clone, "WebSocket server sent close frame");
                        break;
                    }
                    Ok(
                        Message::Ping(_)
                        | Message::Pong(_)
                        | Message::Binary(_)
                        | Message::Frame(_),
                    ) => {
                        // Pings/pongs handled by tungstenite; binary frames ignored.
                    }
                    Err(e) => {
                        tracing::warn!(
                            url = url_clone,
                            error = %e,
                            "WebSocket read error"
                        );
                        break;
                    }
                }
            }

            // Connection closed — cancel all pending requests.
            pending_clone.lock().await.clear();
            tracing::debug!(url = url_clone, "WebSocket reader task exiting");
        });

        Ok(Self {
            url: url.to_string(),
            writer,
            pending,
            reader_handle: Mutex::new(Some(reader_handle)),
        })
    }

    /// Send a text frame over the WebSocket.
    async fn send_text(&self, text: &str) -> crab_common::Result<()> {
        use futures::SinkExt as _;

        self.writer
            .lock()
            .await
            .send(Message::Text(text.to_string().into()))
            .await
            .map_err(|e| {
                crab_common::Error::Other(format!(
                    "failed to send WebSocket message to {}: {e}",
                    self.url
                ))
            })
    }
}

impl Transport for WsTransport {
    fn send(
        &self,
        req: JsonRpcRequest,
    ) -> Pin<Box<dyn Future<Output = crab_common::Result<JsonRpcResponse>> + Send + '_>> {
        Box::pin(async move {
            let id = req.id;

            // Register oneshot channel for the response before sending.
            let (tx, rx) = oneshot::channel();
            {
                self.pending.lock().await.insert(id, tx);
            }

            let json = serde_json::to_string(&req).map_err(|e| {
                crab_common::Error::Other(format!("failed to serialize request: {e}"))
            })?;

            tracing::debug!(method = %req.method, id, url = %self.url, "sending WS request");
            self.send_text(&json).await?;

            // Wait for the response from the reader task.
            rx.await.map_err(|_| {
                crab_common::Error::Other(
                    "WebSocket connection closed before response received".into(),
                )
            })
        })
    }

    fn notify(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>> {
        let notif = JsonRpcNotification::new(
            method.to_string(),
            if params.is_null() { None } else { Some(params) },
        );
        Box::pin(async move {
            let json = serde_json::to_string(&notif).map_err(|e| {
                crab_common::Error::Other(format!("failed to serialize notification: {e}"))
            })?;
            tracing::debug!(method = notif.method, url = %self.url, "sending WS notification");
            self.send_text(&json).await
        })
    }

    fn close(&self) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>> {
        Box::pin(async move {
            use futures::SinkExt as _;

            // Send a close frame.
            let _ = self.writer.lock().await.send(Message::Close(None)).await;

            // Abort the reader task.
            let reader_handle = self.reader_handle.lock().await.take();
            if let Some(handle) = reader_handle {
                handle.abort();
            }

            tracing::debug!(url = %self.url, "WebSocket transport closed");
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_transport_struct_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WsTransport>();
    }

    #[tokio::test]
    async fn connect_invalid_url_fails() {
        let result = WsTransport::connect("ws://127.0.0.1:1/nonexistent").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("failed") || err.contains("timed out"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn connect_nonsense_url_fails() {
        let result = WsTransport::connect("not-a-url").await;
        assert!(result.is_err());
    }

    #[test]
    fn notification_serializes_correctly() {
        let notif = JsonRpcNotification::new("test/notify".to_string(), None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"method\":\"test/notify\""));
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn request_serializes_correctly() {
        let req = JsonRpcRequest::new("test/method", Some(serde_json::json!({"key": "val"})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"test/method\""));
        assert!(json.contains("\"key\":\"val\""));
    }

    #[test]
    fn response_deserializes() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn connect_timeout_constant() {
        assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(30));
    }
}
