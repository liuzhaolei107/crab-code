pub mod audit;
pub mod cancellation;
pub mod capability;
pub mod client;
#[cfg(test)]
mod compliance_tests;
pub mod connection_pool;
pub mod discovery;
pub mod handshake;
pub mod health;
pub mod logging;
pub mod manager;
pub mod negotiation;
pub mod notification;
pub mod progress;
pub mod protocol;
pub mod resource;
pub mod resource_diff;
pub mod resource_subscription;
pub mod resource_watcher;
pub mod roots;
pub mod sampling;
pub mod server;
pub mod sse_server;
pub mod tool_chain;
pub mod tool_group;
pub mod tool_sandbox;
pub mod tool_version;
pub mod transport;
pub mod transport_failover;
pub mod transport_layer;
pub mod transport_monitor;

pub use audit::{AuditEntry, AuditEntryBuilder, AuditOutcome, McpAuditLog};
pub use capability::{
    CapabilityEntry, CapabilityRegistry, McpClientCapabilities, McpServerCapabilities,
    NegotiatedCapabilities,
};
pub use client::McpClient;
pub use connection_pool::{ConnectionPool, ConnectionState, ConnectionSummary, PoolConfig};
pub use discovery::{McpServerConfig, McpTransportConfig, connect_server, parse_mcp_servers};
pub use handshake::{
    HandshakeConfig, HandshakeError, HandshakeProtocol, HandshakeResult, HandshakeState,
};

pub use cancellation::{
    CancellationParams, CancellationReason, CancellationRegistry, CancellationToken,
};
pub use health::{
    AutoReconnect, HealthChecker, HealthCheckerConfig, HealthStatus, Heartbeat, ReconnectConfig,
};
pub use logging::{McpLogEntry, McpLogLevel, McpLogger};
pub use manager::{DiscoveredTool, McpManager};
pub use negotiation::{
    CompatibilityCheck, CompatibilityRegistry, NegotiationResult, ProtocolVersion, VersionRange,
    negotiate_version, negotiate_version_range,
};
pub use notification::{
    McpNotification, NotificationHandler, NotificationQueue, NotificationRouter,
};
pub use progress::{ProgressCallback, ProgressNotification, ProgressToken, ProgressTracker};
pub use protocol::{
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, JsonRpcError,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpPrompt, McpResource, McpToolDef,
    ServerCapabilities, ServerInfo, ToolCallParams, ToolCallResult,
};
pub use resource::ResourceCache;
pub use resource_diff::{DiffChunk, DiffFormat, ResourceDiff, apply_diff, compute_diff};
pub use resource_subscription::{ResourceSubscription, SubscriptionManager};
pub use resource_watcher::{ChangeType, ResourceChangeEvent, ResourceWatcher, WatcherConfig};
pub use roots::{RootInfo, RootRegistry};
pub use sampling::{SamplingHandler, SamplingRequest, SamplingResponse, StopReason};
pub use server::{
    FileResourceHandler, McpServer, PromptHandler, ResourceHandler, SkillPromptHandler,
    ToolHandler, ToolRegistryHandler,
};
pub use sse_server::run_sse;
pub use tool_chain::{
    ChainBuilder, ChainExecutor, ChainResult, ChainStep, ToolChain, ToolChainTemplate,
};
pub use tool_group::{IndexedTool, ToolGroup, ToolIndex};
pub use tool_sandbox::{
    McpPermissionBoundary, McpToolSandbox, PermissionLevel, SandboxPolicy, SandboxVerdict,
};
pub use tool_version::{ToolVersion, ToolVersionRegistry, VersionedTool};
pub use transport::Transport;
pub use transport_failover::{FailoverConfig, FailoverState, TransportFailover};
pub use transport_layer::{MetricsSnapshot, TransportConfig, TransportMetrics, TransportType};
pub use transport_monitor::{
    AlertLevel, HealthSnapshot, MonitorAlert, MonitorThresholds, TransportMonitor,
};
