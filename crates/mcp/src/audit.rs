//! MCP tool execution audit log.
//!
//! Provides [`McpAuditLog`] for recording all MCP tool invocations including
//! tool name, parameters, result summary, duration, caller, and outcome.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Outcome of a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Error,
    Denied,
    Timeout,
}

impl std::fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Error => write!(f, "error"),
            Self::Denied => write!(f, "denied"),
            Self::Timeout => write!(f, "timeout"),
        }
    }
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Monotonically increasing sequence number.
    pub seq: u64,
    /// Wall-clock time of the invocation.
    pub timestamp: SystemTime,
    /// Name of the tool that was called.
    pub tool_name: String,
    /// JSON-encoded input parameters (truncated for large payloads).
    pub parameters: String,
    /// Short summary of the result (first 256 chars).
    pub result_summary: String,
    /// How long the tool execution took.
    pub duration: Duration,
    /// Who initiated the call (e.g. session id, agent name).
    pub caller: String,
    /// Whether the call succeeded, failed, was denied, or timed out.
    pub outcome: AuditOutcome,
}

/// Builder for constructing an [`AuditEntry`] incrementally.
pub struct AuditEntryBuilder {
    tool_name: String,
    parameters: String,
    caller: String,
    start: std::time::Instant,
}

impl AuditEntryBuilder {
    /// Start timing a tool call.
    #[must_use]
    pub fn start(tool_name: impl Into<String>, parameters: impl Into<String>, caller: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            parameters: truncate_string(parameters.into(), 1024),
            caller: caller.into(),
            start: std::time::Instant::now(),
        }
    }

    /// Finish the entry with a result.
    #[must_use]
    pub fn finish(self, outcome: AuditOutcome, result_summary: impl Into<String>) -> PartialAuditEntry {
        PartialAuditEntry {
            tool_name: self.tool_name,
            parameters: self.parameters,
            result_summary: truncate_string(result_summary.into(), 256),
            duration: self.start.elapsed(),
            caller: self.caller,
            outcome,
        }
    }
}

/// An entry ready to be logged (seq and timestamp assigned by the log).
pub struct PartialAuditEntry {
    pub tool_name: String,
    pub parameters: String,
    pub result_summary: String,
    pub duration: Duration,
    pub caller: String,
    pub outcome: AuditOutcome,
}

/// Thread-safe audit log that records MCP tool invocations.
#[derive(Debug, Clone)]
pub struct McpAuditLog {
    inner: Arc<Mutex<AuditLogInner>>,
}

#[derive(Debug)]
struct AuditLogInner {
    entries: Vec<AuditEntry>,
    next_seq: u64,
    max_entries: usize,
}

impl McpAuditLog {
    /// Create a new audit log with a maximum capacity.
    /// When the limit is reached, oldest entries are dropped.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AuditLogInner {
                entries: Vec::new(),
                next_seq: 1,
                max_entries,
            })),
        }
    }

    /// Record a completed tool call from a builder.
    pub fn record(&self, partial: PartialAuditEntry) -> u64 {
        let mut inner = self.inner.lock().unwrap();
        let seq = inner.next_seq;
        inner.next_seq += 1;

        let entry = AuditEntry {
            seq,
            timestamp: SystemTime::now(),
            tool_name: partial.tool_name,
            parameters: partial.parameters,
            result_summary: partial.result_summary,
            duration: partial.duration,
            caller: partial.caller,
            outcome: partial.outcome,
        };

        inner.entries.push(entry);

        // Evict oldest if over capacity
        if inner.entries.len() > inner.max_entries {
            let excess = inner.entries.len() - inner.max_entries;
            inner.entries.drain(..excess);
        }

        seq
    }

    /// Record a tool call directly (convenience method).
    pub fn record_call(
        &self,
        tool_name: impl Into<String>,
        parameters: impl Into<String>,
        result_summary: impl Into<String>,
        duration: Duration,
        caller: impl Into<String>,
        outcome: AuditOutcome,
    ) -> u64 {
        let partial = PartialAuditEntry {
            tool_name: tool_name.into(),
            parameters: truncate_string(parameters.into(), 1024),
            result_summary: truncate_string(result_summary.into(), 256),
            duration,
            caller: caller.into(),
            outcome,
        };
        self.record(partial)
    }

    /// Retrieve all entries (newest last).
    #[must_use]
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.inner.lock().unwrap().entries.clone()
    }

    /// Number of entries currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().entries.len()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().entries.is_empty()
    }

    /// Get entries for a specific tool.
    #[must_use]
    pub fn entries_for_tool(&self, tool_name: &str) -> Vec<AuditEntry> {
        self.inner
            .lock()
            .unwrap()
            .entries
            .iter()
            .filter(|e| e.tool_name == tool_name)
            .cloned()
            .collect()
    }

    /// Get entries for a specific caller.
    #[must_use]
    pub fn entries_for_caller(&self, caller: &str) -> Vec<AuditEntry> {
        self.inner
            .lock()
            .unwrap()
            .entries
            .iter()
            .filter(|e| e.caller == caller)
            .cloned()
            .collect()
    }

    /// Count entries by outcome.
    #[must_use]
    pub fn count_by_outcome(&self, outcome: AuditOutcome) -> usize {
        self.inner
            .lock()
            .unwrap()
            .entries
            .iter()
            .filter(|e| e.outcome == outcome)
            .count()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.entries.clear();
    }
}

impl Default for McpAuditLog {
    fn default() -> Self {
        Self::new(10_000)
    }
}

/// Truncate a string to at most `max_len` characters, appending "..." if truncated.
fn truncate_string(s: String, max_len: usize) -> String {
    if s.len() <= max_len {
        s
    } else {
        let mut truncated = s[..max_len.saturating_sub(3)].to_string();
        truncated.push_str("...");
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_outcome_display() {
        assert_eq!(AuditOutcome::Success.to_string(), "success");
        assert_eq!(AuditOutcome::Error.to_string(), "error");
        assert_eq!(AuditOutcome::Denied.to_string(), "denied");
        assert_eq!(AuditOutcome::Timeout.to_string(), "timeout");
    }

    #[test]
    fn audit_outcome_serde_roundtrip() {
        for outcome in [
            AuditOutcome::Success,
            AuditOutcome::Error,
            AuditOutcome::Denied,
            AuditOutcome::Timeout,
        ] {
            let json = serde_json::to_string(&outcome).unwrap();
            let back: AuditOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(outcome, back);
        }
    }

    #[test]
    fn record_and_retrieve() {
        let log = McpAuditLog::new(100);
        let seq = log.record_call(
            "read_file",
            r#"{"path":"/tmp/x"}"#,
            "file contents...",
            Duration::from_millis(42),
            "session-1",
            AuditOutcome::Success,
        );
        assert_eq!(seq, 1);
        assert_eq!(log.len(), 1);

        let entries = log.entries();
        assert_eq!(entries[0].tool_name, "read_file");
        assert_eq!(entries[0].caller, "session-1");
        assert_eq!(entries[0].outcome, AuditOutcome::Success);
        assert_eq!(entries[0].duration, Duration::from_millis(42));
    }

    #[test]
    fn sequential_ids() {
        let log = McpAuditLog::new(100);
        let s1 = log.record_call("a", "", "", Duration::ZERO, "c", AuditOutcome::Success);
        let s2 = log.record_call("b", "", "", Duration::ZERO, "c", AuditOutcome::Error);
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
    }

    #[test]
    fn eviction_on_capacity() {
        let log = McpAuditLog::new(3);
        for i in 0..5 {
            log.record_call(
                format!("tool_{i}"),
                "",
                "",
                Duration::ZERO,
                "c",
                AuditOutcome::Success,
            );
        }
        assert_eq!(log.len(), 3);
        let entries = log.entries();
        // Oldest two should be evicted
        assert_eq!(entries[0].tool_name, "tool_2");
        assert_eq!(entries[1].tool_name, "tool_3");
        assert_eq!(entries[2].tool_name, "tool_4");
    }

    #[test]
    fn entries_for_tool() {
        let log = McpAuditLog::new(100);
        log.record_call("read", "", "", Duration::ZERO, "c", AuditOutcome::Success);
        log.record_call("write", "", "", Duration::ZERO, "c", AuditOutcome::Success);
        log.record_call("read", "", "", Duration::ZERO, "c", AuditOutcome::Error);
        assert_eq!(log.entries_for_tool("read").len(), 2);
        assert_eq!(log.entries_for_tool("write").len(), 1);
        assert_eq!(log.entries_for_tool("missing").len(), 0);
    }

    #[test]
    fn entries_for_caller() {
        let log = McpAuditLog::new(100);
        log.record_call("t", "", "", Duration::ZERO, "alice", AuditOutcome::Success);
        log.record_call("t", "", "", Duration::ZERO, "bob", AuditOutcome::Success);
        log.record_call("t", "", "", Duration::ZERO, "alice", AuditOutcome::Success);
        assert_eq!(log.entries_for_caller("alice").len(), 2);
        assert_eq!(log.entries_for_caller("bob").len(), 1);
    }

    #[test]
    fn count_by_outcome() {
        let log = McpAuditLog::new(100);
        log.record_call("t", "", "", Duration::ZERO, "c", AuditOutcome::Success);
        log.record_call("t", "", "", Duration::ZERO, "c", AuditOutcome::Success);
        log.record_call("t", "", "", Duration::ZERO, "c", AuditOutcome::Error);
        log.record_call("t", "", "", Duration::ZERO, "c", AuditOutcome::Denied);
        assert_eq!(log.count_by_outcome(AuditOutcome::Success), 2);
        assert_eq!(log.count_by_outcome(AuditOutcome::Error), 1);
        assert_eq!(log.count_by_outcome(AuditOutcome::Denied), 1);
        assert_eq!(log.count_by_outcome(AuditOutcome::Timeout), 0);
    }

    #[test]
    fn clear_log() {
        let log = McpAuditLog::new(100);
        log.record_call("t", "", "", Duration::ZERO, "c", AuditOutcome::Success);
        assert!(!log.is_empty());
        log.clear();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn builder_pattern() {
        let builder = AuditEntryBuilder::start("read_file", r#"{"path":"x"}"#, "agent-1");
        // Simulate some work
        std::thread::sleep(Duration::from_millis(1));
        let partial = builder.finish(AuditOutcome::Success, "ok");

        let log = McpAuditLog::new(100);
        let seq = log.record(partial);
        assert_eq!(seq, 1);

        let entries = log.entries();
        assert_eq!(entries[0].tool_name, "read_file");
        assert!(entries[0].duration >= Duration::from_millis(1));
    }

    #[test]
    fn truncation() {
        let long_params = "x".repeat(2000);
        let log = McpAuditLog::new(100);
        log.record_call("t", &long_params, "", Duration::ZERO, "c", AuditOutcome::Success);
        let entry = &log.entries()[0];
        assert!(entry.parameters.len() <= 1024);
        assert!(entry.parameters.ends_with("..."));
    }

    #[test]
    fn thread_safety() {
        let log = McpAuditLog::new(1000);
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let log = log.clone();
                std::thread::spawn(move || {
                    for j in 0..25 {
                        log.record_call(
                            format!("tool_{i}_{j}"),
                            "",
                            "",
                            Duration::ZERO,
                            "c",
                            AuditOutcome::Success,
                        );
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(log.len(), 100);
    }

    #[test]
    fn default_log_capacity() {
        let log = McpAuditLog::default();
        // Just check it's usable
        log.record_call("t", "", "", Duration::ZERO, "c", AuditOutcome::Success);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn audit_entry_serde_roundtrip() {
        let entry = AuditEntry {
            seq: 1,
            timestamp: SystemTime::now(),
            tool_name: "test".into(),
            parameters: "{}".into(),
            result_summary: "ok".into(),
            duration: Duration::from_millis(100),
            caller: "agent".into(),
            outcome: AuditOutcome::Success,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.seq, 1);
        assert_eq!(back.tool_name, "test");
        assert_eq!(back.outcome, AuditOutcome::Success);
    }
}
