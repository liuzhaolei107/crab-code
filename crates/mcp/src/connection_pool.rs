//! MCP connection pool for managing multiple server connections.
//!
//! Provides [`ConnectionPool`] for tracking, reusing, and monitoring
//! connections to multiple MCP servers with integrated health checking
//! and auto-reconnect.

use crate::health::{
    AutoReconnect, HealthChecker, HealthCheckerConfig, HealthStatus, Heartbeat, ReconnectConfig,
    ReconnectState,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// State of an individual connection in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    /// Connection is active and usable.
    Active,
    /// Connection is being established or re-established.
    Connecting,
    /// Connection is temporarily unavailable (will auto-reconnect).
    Disconnected,
    /// Connection has been permanently closed.
    Closed,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Connecting => write!(f, "connecting"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// A managed connection entry within the pool.
#[derive(Debug)]
pub struct PooledConnection {
    pub server_name: String,
    pub state: ConnectionState,
    pub health: HealthChecker,
    pub reconnect: AutoReconnect,
    pub heartbeat: Heartbeat,
}

impl PooledConnection {
    fn new(
        server_name: impl Into<String>,
        health_config: HealthCheckerConfig,
        reconnect_config: ReconnectConfig,
        heartbeat_interval: Duration,
        heartbeat_timeout: Duration,
    ) -> Self {
        let name = server_name.into();
        Self {
            health: HealthChecker::new(&name, health_config),
            reconnect: AutoReconnect::new(&name, reconnect_config),
            heartbeat: Heartbeat::new(heartbeat_interval, heartbeat_timeout),
            state: ConnectionState::Connecting,
            server_name: name,
        }
    }
}

/// Summary of a pooled connection for external reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSummary {
    pub server_name: String,
    pub state: ConnectionState,
    pub health_status: HealthStatus,
    pub reconnect_attempts: u32,
    pub missed_heartbeats: u32,
}

/// Configuration for the connection pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub health: HealthCheckerConfig,
    pub reconnect: ReconnectConfig,
    pub heartbeat_interval: Duration,
    pub heartbeat_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            health: HealthCheckerConfig::default(),
            reconnect: ReconnectConfig::default(),
            heartbeat_interval: Duration::from_secs(10),
            heartbeat_timeout: Duration::from_secs(30),
        }
    }
}

/// Pool of MCP server connections with integrated health and reconnect.
#[derive(Debug)]
pub struct ConnectionPool {
    connections: HashMap<String, PooledConnection>,
    config: PoolConfig,
}

impl ConnectionPool {
    /// Create a new empty pool.
    #[must_use]
    pub fn new(config: PoolConfig) -> Self {
        Self {
            connections: HashMap::new(),
            config,
        }
    }

    /// Add a server to the pool.
    pub fn add(&mut self, server_name: impl Into<String>) {
        let name = server_name.into();
        if !self.connections.contains_key(&name) {
            self.connections.insert(
                name.clone(),
                PooledConnection::new(
                    name,
                    self.config.health.clone(),
                    self.config.reconnect.clone(),
                    self.config.heartbeat_interval,
                    self.config.heartbeat_timeout,
                ),
            );
        }
    }

    /// Remove a server from the pool.
    pub fn remove(&mut self, server_name: &str) -> bool {
        self.connections.remove(server_name).is_some()
    }

    /// Get a reference to a pooled connection.
    #[must_use]
    pub fn get(&self, server_name: &str) -> Option<&PooledConnection> {
        self.connections.get(server_name)
    }

    /// Get a mutable reference to a pooled connection.
    pub fn get_mut(&mut self, server_name: &str) -> Option<&mut PooledConnection> {
        self.connections.get_mut(server_name)
    }

    /// Number of connections in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// Whether the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }

    /// Mark a server as connected and active.
    pub fn mark_connected(&mut self, server_name: &str) {
        if let Some(conn) = self.connections.get_mut(server_name) {
            conn.state = ConnectionState::Active;
            conn.reconnect.connection_restored();
            conn.health.record_success(Duration::from_millis(0));
        }
    }

    /// Mark a server as disconnected, triggering reconnect logic.
    /// Returns the backoff delay, or `None` if reconnect gave up.
    pub fn mark_disconnected(&mut self, server_name: &str) -> Option<Duration> {
        let conn = self.connections.get_mut(server_name)?;
        conn.state = ConnectionState::Disconnected;
        conn.health.record_failure("disconnected");
        conn.reconnect.connection_lost()
    }

    /// Mark a server as permanently closed.
    pub fn mark_closed(&mut self, server_name: &str) {
        if let Some(conn) = self.connections.get_mut(server_name) {
            conn.state = ConnectionState::Closed;
        }
    }

    /// List all active (usable) server names.
    #[must_use]
    pub fn active_servers(&self) -> Vec<&str> {
        self.connections
            .values()
            .filter(|c| c.state == ConnectionState::Active)
            .map(|c| c.server_name.as_str())
            .collect()
    }

    /// List servers that need a health check.
    #[must_use]
    pub fn servers_needing_check(&self) -> Vec<&str> {
        self.connections
            .values()
            .filter(|c| c.state == ConnectionState::Active && c.health.is_check_due())
            .map(|c| c.server_name.as_str())
            .collect()
    }

    /// List servers that need a heartbeat sent.
    #[must_use]
    pub fn servers_needing_heartbeat(&self) -> Vec<&str> {
        self.connections
            .values()
            .filter(|c| c.state == ConnectionState::Active && c.heartbeat.should_send())
            .map(|c| c.server_name.as_str())
            .collect()
    }

    /// List servers with dead heartbeats.
    #[must_use]
    pub fn dead_servers(&self) -> Vec<&str> {
        self.connections
            .values()
            .filter(|c| c.heartbeat.is_dead())
            .map(|c| c.server_name.as_str())
            .collect()
    }

    /// List servers that have given up reconnecting.
    #[must_use]
    pub fn given_up_servers(&self) -> Vec<&str> {
        self.connections
            .values()
            .filter(|c| c.reconnect.state() == ReconnectState::GivenUp)
            .map(|c| c.server_name.as_str())
            .collect()
    }

    /// Get summary of all connections.
    #[must_use]
    pub fn summaries(&self) -> Vec<ConnectionSummary> {
        self.connections
            .values()
            .map(|c| ConnectionSummary {
                server_name: c.server_name.clone(),
                state: c.state,
                health_status: c.health.status(),
                reconnect_attempts: c.reconnect.attempt(),
                missed_heartbeats: c.heartbeat.missed_count(),
            })
            .collect()
    }

    /// Server names in the pool.
    #[must_use]
    pub fn server_names(&self) -> Vec<&str> {
        self.connections.keys().map(String::as_str).collect()
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new(PoolConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pool() -> ConnectionPool {
        let mut pool = ConnectionPool::default();
        pool.add("server-a");
        pool.add("server-b");
        pool
    }

    #[test]
    fn pool_add_and_len() {
        let pool = make_pool();
        assert_eq!(pool.len(), 2);
        assert!(!pool.is_empty());
    }

    #[test]
    fn pool_add_duplicate_ignored() {
        let mut pool = make_pool();
        pool.add("server-a");
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn pool_remove() {
        let mut pool = make_pool();
        assert!(pool.remove("server-a"));
        assert_eq!(pool.len(), 1);
        assert!(!pool.remove("nonexistent"));
    }

    #[test]
    fn pool_get() {
        let pool = make_pool();
        assert!(pool.get("server-a").is_some());
        assert!(pool.get("missing").is_none());
    }

    #[test]
    fn pool_mark_connected() {
        let mut pool = make_pool();
        pool.mark_connected("server-a");
        let conn = pool.get("server-a").unwrap();
        assert_eq!(conn.state, ConnectionState::Active);
        assert_eq!(conn.health.status(), HealthStatus::Healthy);
    }

    #[test]
    fn pool_mark_disconnected() {
        let mut pool = make_pool();
        pool.mark_connected("server-a");
        let delay = pool.mark_disconnected("server-a");
        assert!(delay.is_some());
        let conn = pool.get("server-a").unwrap();
        assert_eq!(conn.state, ConnectionState::Disconnected);
    }

    #[test]
    fn pool_mark_closed() {
        let mut pool = make_pool();
        pool.mark_closed("server-a");
        assert_eq!(pool.get("server-a").unwrap().state, ConnectionState::Closed);
    }

    #[test]
    fn pool_active_servers() {
        let mut pool = make_pool();
        assert!(pool.active_servers().is_empty());
        pool.mark_connected("server-a");
        let active = pool.active_servers();
        assert_eq!(active.len(), 1);
        assert!(active.contains(&"server-a"));
    }

    #[test]
    fn pool_summaries() {
        let mut pool = make_pool();
        pool.mark_connected("server-a");
        let summaries = pool.summaries();
        assert_eq!(summaries.len(), 2);
        let sa = summaries
            .iter()
            .find(|s| s.server_name == "server-a")
            .unwrap();
        assert_eq!(sa.state, ConnectionState::Active);
        assert_eq!(sa.health_status, HealthStatus::Healthy);
    }

    #[test]
    fn pool_server_names() {
        let pool = make_pool();
        let names = pool.server_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"server-a"));
        assert!(names.contains(&"server-b"));
    }

    #[test]
    fn pool_given_up_servers() {
        let mut pool = ConnectionPool::new(PoolConfig {
            reconnect: ReconnectConfig {
                max_attempts: 1,
                ..Default::default()
            },
            ..Default::default()
        });
        pool.add("fragile");
        pool.mark_connected("fragile");
        pool.mark_disconnected("fragile"); // attempt 1
        pool.mark_disconnected("fragile"); // gives up
        let given_up = pool.given_up_servers();
        assert!(given_up.contains(&"fragile"));
    }

    #[test]
    fn connection_state_display() {
        assert_eq!(ConnectionState::Active.to_string(), "active");
        assert_eq!(ConnectionState::Connecting.to_string(), "connecting");
        assert_eq!(ConnectionState::Disconnected.to_string(), "disconnected");
        assert_eq!(ConnectionState::Closed.to_string(), "closed");
    }

    #[test]
    fn connection_state_serde_roundtrip() {
        for state in [
            ConnectionState::Active,
            ConnectionState::Connecting,
            ConnectionState::Disconnected,
            ConnectionState::Closed,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: ConnectionState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }

    #[test]
    fn connection_summary_serde_roundtrip() {
        let s = ConnectionSummary {
            server_name: "test".into(),
            state: ConnectionState::Active,
            health_status: HealthStatus::Healthy,
            reconnect_attempts: 0,
            missed_heartbeats: 0,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: ConnectionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_name, "test");
        assert_eq!(back.state, ConnectionState::Active);
    }

    #[test]
    fn pool_empty_default() {
        let pool = ConnectionPool::default();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }
}
