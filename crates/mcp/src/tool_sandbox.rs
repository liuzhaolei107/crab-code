//! MCP tool execution sandbox and permission boundary.
//!
//! Provides [`McpToolSandbox`] for restricting file-system and network access
//! of MCP-discovered tools, and [`McpPermissionBoundary`] for assigning
//! permission levels based on tool origin (builtin / mcp / plugin / user).

use crate::tool_group::ToolGroup;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ─── Sandbox policy ────────────────────────────────────────────────────

/// Describes the file-system and network constraints imposed on a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Directories the tool is allowed to read from.
    pub allowed_read_paths: Vec<PathBuf>,
    /// Directories the tool is allowed to write to.
    pub allowed_write_paths: Vec<PathBuf>,
    /// Network hosts the tool may contact (empty = no network).
    pub allowed_hosts: HashSet<String>,
    /// Whether the tool may execute sub-processes.
    pub allow_subprocess: bool,
    /// Maximum execution time in seconds (0 = unlimited).
    pub max_execution_secs: u64,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            allowed_read_paths: Vec::new(),
            allowed_write_paths: Vec::new(),
            allowed_hosts: HashSet::new(),
            allow_subprocess: false,
            max_execution_secs: 30,
        }
    }
}

impl SandboxPolicy {
    /// A fully permissive policy (for builtin tools).
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            allowed_read_paths: vec![PathBuf::from("/")],
            allowed_write_paths: vec![PathBuf::from("/")],
            allowed_hosts: {
                let mut s = HashSet::new();
                s.insert("*".to_string());
                s
            },
            allow_subprocess: true,
            max_execution_secs: 0,
        }
    }

    /// A restricted policy scoped to a project directory (for MCP / plugin tools).
    #[must_use]
    pub fn project_scoped(project_dir: impl Into<PathBuf>) -> Self {
        let dir = project_dir.into();
        Self {
            allowed_read_paths: vec![dir.clone()],
            allowed_write_paths: vec![dir],
            allowed_hosts: HashSet::new(),
            allow_subprocess: false,
            max_execution_secs: 30,
        }
    }

    /// Check if reading `path` is allowed under this policy.
    #[must_use]
    pub fn can_read(&self, path: &Path) -> bool {
        self.allowed_read_paths
            .iter()
            .any(|allowed| path.starts_with(allowed))
    }

    /// Check if writing `path` is allowed under this policy.
    #[must_use]
    pub fn can_write(&self, path: &Path) -> bool {
        self.allowed_write_paths
            .iter()
            .any(|allowed| path.starts_with(allowed))
    }

    /// Check if connecting to `host` is allowed.
    #[must_use]
    pub fn can_connect(&self, host: &str) -> bool {
        self.allowed_hosts.contains("*") || self.allowed_hosts.contains(host)
    }
}

// ─── Tool sandbox ──────────────────────────────────────────────────────

/// Result of a sandbox check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxVerdict {
    /// The operation is allowed.
    Allow,
    /// The operation is denied with a reason.
    Deny(String),
}

impl SandboxVerdict {
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    #[must_use]
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Deny(_))
    }
}

/// Sandbox that evaluates tool operations against a [`SandboxPolicy`].
#[derive(Debug, Clone)]
pub struct McpToolSandbox {
    policy: SandboxPolicy,
}

impl McpToolSandbox {
    #[must_use]
    pub fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }

    /// Access the underlying policy.
    #[must_use]
    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    /// Check whether a file read is permitted.
    #[must_use]
    pub fn check_read(&self, path: &Path) -> SandboxVerdict {
        if self.policy.can_read(path) {
            SandboxVerdict::Allow
        } else {
            SandboxVerdict::Deny(format!(
                "read access denied: {} is outside allowed paths",
                path.display()
            ))
        }
    }

    /// Check whether a file write is permitted.
    #[must_use]
    pub fn check_write(&self, path: &Path) -> SandboxVerdict {
        if self.policy.can_write(path) {
            SandboxVerdict::Allow
        } else {
            SandboxVerdict::Deny(format!(
                "write access denied: {} is outside allowed paths",
                path.display()
            ))
        }
    }

    /// Check whether a network connection is permitted.
    #[must_use]
    pub fn check_network(&self, host: &str) -> SandboxVerdict {
        if self.policy.can_connect(host) {
            SandboxVerdict::Allow
        } else {
            SandboxVerdict::Deny(format!("network access denied: host {host} is not allowed"))
        }
    }

    /// Check whether subprocess execution is permitted.
    #[must_use]
    pub fn check_subprocess(&self) -> SandboxVerdict {
        if self.policy.allow_subprocess {
            SandboxVerdict::Allow
        } else {
            SandboxVerdict::Deny("subprocess execution is not allowed".to_string())
        }
    }
}

// ─── Permission boundary ───────────────────────────────────────────────

/// Permission level assigned to tools based on their trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// No access — tool is blocked.
    None,
    /// Read-only access.
    ReadOnly,
    /// Read and write within project scope.
    ProjectScoped,
    /// Full access (trusted).
    Full,
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::ReadOnly => write!(f, "read_only"),
            Self::ProjectScoped => write!(f, "project_scoped"),
            Self::Full => write!(f, "full"),
        }
    }
}

/// Maps [`ToolGroup`] origins to [`PermissionLevel`]s and resolves sandbox
/// policies accordingly.
#[derive(Debug, Clone)]
pub struct McpPermissionBoundary {
    builtin_level: PermissionLevel,
    mcp_level: PermissionLevel,
    plugin_level: PermissionLevel,
    user_level: PermissionLevel,
    project_dir: PathBuf,
}

impl McpPermissionBoundary {
    /// Create a boundary with sensible defaults:
    /// - Builtin → Full
    /// - MCP → `ProjectScoped`
    /// - Plugin → `ProjectScoped`
    /// - User → `ProjectScoped`
    #[must_use]
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            builtin_level: PermissionLevel::Full,
            mcp_level: PermissionLevel::ProjectScoped,
            plugin_level: PermissionLevel::ProjectScoped,
            user_level: PermissionLevel::ProjectScoped,
            project_dir: project_dir.into(),
        }
    }

    /// Override the permission level for a specific group.
    #[must_use]
    pub fn with_level(mut self, group: ToolGroup, level: PermissionLevel) -> Self {
        match group {
            ToolGroup::Builtin => self.builtin_level = level,
            ToolGroup::Mcp => self.mcp_level = level,
            ToolGroup::Plugin => self.plugin_level = level,
            ToolGroup::User => self.user_level = level,
        }
        self
    }

    /// Get the permission level for a tool group.
    #[must_use]
    pub fn level_for(&self, group: ToolGroup) -> PermissionLevel {
        match group {
            ToolGroup::Builtin => self.builtin_level,
            ToolGroup::Mcp => self.mcp_level,
            ToolGroup::Plugin => self.plugin_level,
            ToolGroup::User => self.user_level,
        }
    }

    /// Build a [`SandboxPolicy`] appropriate for the given tool group.
    #[must_use]
    pub fn sandbox_policy_for(&self, group: ToolGroup) -> SandboxPolicy {
        match self.level_for(group) {
            PermissionLevel::None => SandboxPolicy {
                max_execution_secs: 0,
                ..SandboxPolicy::default()
            },
            PermissionLevel::ReadOnly => SandboxPolicy {
                allowed_read_paths: vec![self.project_dir.clone()],
                allowed_write_paths: Vec::new(),
                allowed_hosts: HashSet::new(),
                allow_subprocess: false,
                max_execution_secs: 30,
            },
            PermissionLevel::ProjectScoped => SandboxPolicy::project_scoped(&self.project_dir),
            PermissionLevel::Full => SandboxPolicy::permissive(),
        }
    }

    /// Create a [`McpToolSandbox`] for a tool from the given group.
    #[must_use]
    pub fn sandbox_for(&self, group: ToolGroup) -> McpToolSandbox {
        McpToolSandbox::new(self.sandbox_policy_for(group))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SandboxPolicy tests ──

    #[test]
    fn default_policy_denies_everything() {
        let p = SandboxPolicy::default();
        assert!(!p.can_read(Path::new("/tmp/file")));
        assert!(!p.can_write(Path::new("/tmp/file")));
        assert!(!p.can_connect("example.com"));
        assert!(!p.allow_subprocess);
        assert_eq!(p.max_execution_secs, 30);
    }

    #[test]
    fn permissive_policy_allows_everything() {
        let p = SandboxPolicy::permissive();
        assert!(p.can_read(Path::new("/any/path")));
        assert!(p.can_write(Path::new("/any/path")));
        assert!(p.can_connect("anything.example.com"));
        assert!(p.allow_subprocess);
        assert_eq!(p.max_execution_secs, 0);
    }

    #[test]
    fn project_scoped_policy() {
        let p = SandboxPolicy::project_scoped("/home/user/project");
        assert!(p.can_read(Path::new("/home/user/project/src/main.rs")));
        assert!(p.can_write(Path::new("/home/user/project/out.txt")));
        assert!(!p.can_read(Path::new("/etc/passwd")));
        assert!(!p.can_write(Path::new("/tmp/evil")));
        assert!(!p.can_connect("example.com"));
        assert!(!p.allow_subprocess);
    }

    #[test]
    fn policy_serde_roundtrip() {
        let p = SandboxPolicy::project_scoped("/proj");
        let json = serde_json::to_string(&p).unwrap();
        let back: SandboxPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back.allowed_read_paths, p.allowed_read_paths);
        assert_eq!(back.allow_subprocess, p.allow_subprocess);
    }

    // ── McpToolSandbox tests ──

    #[test]
    fn sandbox_check_read_allowed() {
        let sb = McpToolSandbox::new(SandboxPolicy::project_scoped("/proj"));
        assert!(sb.check_read(Path::new("/proj/src/lib.rs")).is_allowed());
    }

    #[test]
    fn sandbox_check_read_denied() {
        let sb = McpToolSandbox::new(SandboxPolicy::project_scoped("/proj"));
        let v = sb.check_read(Path::new("/etc/shadow"));
        assert!(v.is_denied());
        if let SandboxVerdict::Deny(msg) = v {
            assert!(msg.contains("read access denied"));
        }
    }

    #[test]
    fn sandbox_check_write_allowed_and_denied() {
        let sb = McpToolSandbox::new(SandboxPolicy::project_scoped("/proj"));
        assert!(sb.check_write(Path::new("/proj/out.txt")).is_allowed());
        assert!(sb.check_write(Path::new("/tmp/hack")).is_denied());
    }

    #[test]
    fn sandbox_check_network() {
        let sb = McpToolSandbox::new(SandboxPolicy::permissive());
        assert!(sb.check_network("api.example.com").is_allowed());

        let sb2 = McpToolSandbox::new(SandboxPolicy::default());
        assert!(sb2.check_network("api.example.com").is_denied());
    }

    #[test]
    fn sandbox_check_subprocess() {
        let sb_allow = McpToolSandbox::new(SandboxPolicy::permissive());
        assert!(sb_allow.check_subprocess().is_allowed());

        let sb_deny = McpToolSandbox::new(SandboxPolicy::default());
        assert!(sb_deny.check_subprocess().is_denied());
    }

    // ── PermissionLevel tests ──

    #[test]
    fn permission_level_ordering() {
        assert!(PermissionLevel::None < PermissionLevel::ReadOnly);
        assert!(PermissionLevel::ReadOnly < PermissionLevel::ProjectScoped);
        assert!(PermissionLevel::ProjectScoped < PermissionLevel::Full);
    }

    #[test]
    fn permission_level_display() {
        assert_eq!(PermissionLevel::None.to_string(), "none");
        assert_eq!(PermissionLevel::ReadOnly.to_string(), "read_only");
        assert_eq!(PermissionLevel::ProjectScoped.to_string(), "project_scoped");
        assert_eq!(PermissionLevel::Full.to_string(), "full");
    }

    #[test]
    fn permission_level_serde_roundtrip() {
        for level in [
            PermissionLevel::None,
            PermissionLevel::ReadOnly,
            PermissionLevel::ProjectScoped,
            PermissionLevel::Full,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let back: PermissionLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    // ── McpPermissionBoundary tests ──

    #[test]
    fn boundary_default_levels() {
        let b = McpPermissionBoundary::new("/proj");
        assert_eq!(b.level_for(ToolGroup::Builtin), PermissionLevel::Full);
        assert_eq!(b.level_for(ToolGroup::Mcp), PermissionLevel::ProjectScoped);
        assert_eq!(
            b.level_for(ToolGroup::Plugin),
            PermissionLevel::ProjectScoped
        );
        assert_eq!(b.level_for(ToolGroup::User), PermissionLevel::ProjectScoped);
    }

    #[test]
    fn boundary_custom_level() {
        let b = McpPermissionBoundary::new("/proj")
            .with_level(ToolGroup::Mcp, PermissionLevel::ReadOnly)
            .with_level(ToolGroup::Plugin, PermissionLevel::None);
        assert_eq!(b.level_for(ToolGroup::Mcp), PermissionLevel::ReadOnly);
        assert_eq!(b.level_for(ToolGroup::Plugin), PermissionLevel::None);
    }

    #[test]
    fn boundary_sandbox_for_builtin_is_permissive() {
        let b = McpPermissionBoundary::new("/proj");
        let sb = b.sandbox_for(ToolGroup::Builtin);
        assert!(sb.check_read(Path::new("/any/file")).is_allowed());
        assert!(sb.check_write(Path::new("/any/file")).is_allowed());
        assert!(sb.check_subprocess().is_allowed());
    }

    #[test]
    fn boundary_sandbox_for_mcp_is_project_scoped() {
        let b = McpPermissionBoundary::new("/proj");
        let sb = b.sandbox_for(ToolGroup::Mcp);
        assert!(sb.check_read(Path::new("/proj/src/lib.rs")).is_allowed());
        assert!(sb.check_write(Path::new("/proj/out.txt")).is_allowed());
        assert!(sb.check_read(Path::new("/etc/passwd")).is_denied());
        assert!(sb.check_subprocess().is_denied());
    }

    #[test]
    fn boundary_sandbox_for_none_level() {
        let b = McpPermissionBoundary::new("/proj")
            .with_level(ToolGroup::Plugin, PermissionLevel::None);
        let sb = b.sandbox_for(ToolGroup::Plugin);
        assert!(sb.check_read(Path::new("/proj/file")).is_denied());
        assert!(sb.check_write(Path::new("/proj/file")).is_denied());
    }

    #[test]
    fn boundary_sandbox_for_read_only() {
        let b = McpPermissionBoundary::new("/proj")
            .with_level(ToolGroup::Mcp, PermissionLevel::ReadOnly);
        let sb = b.sandbox_for(ToolGroup::Mcp);
        assert!(sb.check_read(Path::new("/proj/src/main.rs")).is_allowed());
        assert!(sb.check_write(Path::new("/proj/src/main.rs")).is_denied());
        assert!(sb.check_network("api.example.com").is_denied());
    }

    #[test]
    fn verdict_is_allowed_and_denied() {
        assert!(SandboxVerdict::Allow.is_allowed());
        assert!(!SandboxVerdict::Allow.is_denied());
        let d = SandboxVerdict::Deny("reason".into());
        assert!(d.is_denied());
        assert!(!d.is_allowed());
    }
}
