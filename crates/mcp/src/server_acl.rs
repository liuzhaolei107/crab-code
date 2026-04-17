//! Per-server access-control list (ACL) for MCP sub-surfaces.
//!
//! Governs **which** tools, resources, prompts, and notifications a
//! given MCP server is allowed to expose to the crab session. Rules
//! are configured per-server and evaluated before any forwarded
//! operation reaches the session.
//!
//! ## Name disambiguation
//!
//! Not to be confused with:
//! - `core::permission::*` — tool-execution permission system (can this
//!   bash command run? does the user trust this path?). That applies to
//!   all tools regardless of origin.
//! - The future `crab-bridge::permission_relay` module (not built yet) —
//!   CCB's remote permission relay over Telegram / iMessage / Discord,
//!   where the user approves or denies via an external chat message.
//!
//! This module is about **MCP server exposure**: "server `github` should
//! only expose `get_*` tools, never `delete_*`".
//!
//! The term "channel" here refers strictly to the four MCP protocol
//! sub-surfaces (tools / resources / prompts / notifications) per the
//! MCP spec, not to Slack/Discord chat channels.
//!
//! Matching is glob-based via the `globset` crate, supporting:
//! - `*` — match any run of characters except `/`
//! - `**` — match any characters including `/`
//! - `?` — match a single character
//! - `[abc]` / `[a-z]` — character classes
//! - `{a,b}` — alternation
//!
//! Falls back to plain string equality if the pattern isn't a valid glob.

use std::collections::HashMap;

use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};

// ─── Config shape ──────────────────────────────────────────────────────

/// Per-server permission rules, serde-friendly for loading from
/// `settings.json` / `~/.crab/settings.json`.
///
/// Empty allow-list = allow all (as long as nothing in deny matches).
/// Non-empty allow-list = the name must match at least one allow pattern.
/// Deny always beats allow.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AclRules {
    /// Tool name patterns that are allowed (e.g., `"read_*"`).
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Tool name patterns that are explicitly denied.
    #[serde(default)]
    pub denied_tools: Vec<String>,
    /// Resource URI patterns that are allowed.
    #[serde(default)]
    pub allowed_resources: Vec<String>,
    /// Resource URI patterns that are denied.
    #[serde(default)]
    pub denied_resources: Vec<String>,
    /// Prompt name patterns that are allowed.
    #[serde(default)]
    pub allowed_prompts: Vec<String>,
    /// Prompt name patterns that are denied.
    #[serde(default)]
    pub denied_prompts: Vec<String>,
    /// Notification method patterns (e.g., `"notifications/message"`)
    /// that are allowed to be forwarded.
    #[serde(default)]
    pub allowed_notifications: Vec<String>,
    /// Notification method patterns that are silently dropped.
    #[serde(default)]
    pub denied_notifications: Vec<String>,
}

// ─── Channel enum (for uniform dispatch) ───────────────────────────────

/// Which MCP sub-surface an operation targets. Lets callers use the
/// single `is_allowed(channel, server, name)` entry point instead of
/// four `is_tool_allowed` / `is_resource_allowed` / … methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclChannel {
    /// `tools/call`
    Tool,
    /// `resources/read`
    Resource,
    /// `prompts/get`
    Prompt,
    /// `notifications/*`
    Notification,
}

// ─── Manager ───────────────────────────────────────────────────────────

/// Manages channel-level permissions across all connected MCP servers.
///
/// Holds pre-compiled `GlobMatcher`s so per-call matching is O(patterns)
/// with no re-parsing.
pub struct ServerAclRegistry {
    /// Raw config per server (kept so we can round-trip + mutate).
    servers: HashMap<String, AclRules>,
    /// Compiled matchers per server, built lazily on insert.
    compiled: HashMap<String, CompiledAclRules>,
}

/// Compiled form of `AclRules` ready for fast evaluation.
struct CompiledAclRules {
    allowed_tools: Vec<GlobMatcher>,
    denied_tools: Vec<GlobMatcher>,
    allowed_resources: Vec<GlobMatcher>,
    denied_resources: Vec<GlobMatcher>,
    allowed_prompts: Vec<GlobMatcher>,
    denied_prompts: Vec<GlobMatcher>,
    allowed_notifications: Vec<GlobMatcher>,
    denied_notifications: Vec<GlobMatcher>,
}

impl CompiledAclRules {
    fn from_raw(raw: &AclRules) -> Self {
        Self {
            allowed_tools: compile_list(&raw.allowed_tools),
            denied_tools: compile_list(&raw.denied_tools),
            allowed_resources: compile_list(&raw.allowed_resources),
            denied_resources: compile_list(&raw.denied_resources),
            allowed_prompts: compile_list(&raw.allowed_prompts),
            denied_prompts: compile_list(&raw.denied_prompts),
            allowed_notifications: compile_list(&raw.allowed_notifications),
            denied_notifications: compile_list(&raw.denied_notifications),
        }
    }

    fn lists_for(&self, channel: AclChannel) -> (&[GlobMatcher], &[GlobMatcher]) {
        match channel {
            AclChannel::Tool => (&self.allowed_tools, &self.denied_tools),
            AclChannel::Resource => (&self.allowed_resources, &self.denied_resources),
            AclChannel::Prompt => (&self.allowed_prompts, &self.denied_prompts),
            AclChannel::Notification => (&self.allowed_notifications, &self.denied_notifications),
        }
    }
}

/// Compile each pattern with `globset`; if the pattern isn't a valid
/// glob, fall back to a literal equality matcher by escaping the
/// pattern. Missing/empty strings are skipped.
fn compile_list(patterns: &[String]) -> Vec<GlobMatcher> {
    patterns
        .iter()
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            Glob::new(p)
                .or_else(|_| Glob::new(&globset::escape(p)))
                .ok()
        })
        .map(|g| g.compile_matcher())
        .collect()
}

fn matches_any(matchers: &[GlobMatcher], name: &str) -> bool {
    matchers.iter().any(|m| m.is_match(name))
}

impl ServerAclRegistry {
    /// Empty permission set — allows everything on every server.
    #[must_use]
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            compiled: HashMap::new(),
        }
    }

    /// Bulk-construct from a config map.
    #[must_use]
    pub fn from_config(servers: HashMap<String, AclRules>) -> Self {
        let compiled = servers
            .iter()
            .map(|(name, raw)| (name.clone(), CompiledAclRules::from_raw(raw)))
            .collect();
        Self { servers, compiled }
    }

    /// Unified channel-check. Servers with no configured rules default
    /// to allow.
    #[must_use]
    pub fn is_allowed(&self, channel: AclChannel, server: &str, name: &str) -> bool {
        let Some(compiled) = self.compiled.get(server) else {
            return true;
        };
        let (allowed, denied) = compiled.lists_for(channel);

        // Deny wins.
        if matches_any(denied, name) {
            return false;
        }
        // Empty allow list = allow by default.
        if allowed.is_empty() {
            return true;
        }
        matches_any(allowed, name)
    }

    /// Check whether a tool call is allowed for the given server.
    #[must_use]
    pub fn is_tool_allowed(&self, server: &str, tool: &str) -> bool {
        self.is_allowed(AclChannel::Tool, server, tool)
    }

    /// Check whether a resource access is allowed for the given server.
    #[must_use]
    pub fn is_resource_allowed(&self, server: &str, resource: &str) -> bool {
        self.is_allowed(AclChannel::Resource, server, resource)
    }

    /// Check whether a prompt is allowed for the given server.
    #[must_use]
    pub fn is_prompt_allowed(&self, server: &str, prompt: &str) -> bool {
        self.is_allowed(AclChannel::Prompt, server, prompt)
    }

    /// Check whether a notification method is allowed to be forwarded
    /// from the given server.
    #[must_use]
    pub fn is_notification_allowed(&self, server: &str, method: &str) -> bool {
        self.is_allowed(AclChannel::Notification, server, method)
    }

    /// Replace (or insert) a server's permission config. Recompiles
    /// matchers immediately so subsequent `is_*_allowed` calls are fast.
    pub fn set_server_permissions(&mut self, server: String, perms: AclRules) {
        let compiled = CompiledAclRules::from_raw(&perms);
        self.servers.insert(server.clone(), perms);
        self.compiled.insert(server, compiled);
    }

    /// Drop a server's permission config.
    pub fn remove_server(&mut self, server: &str) {
        self.servers.remove(server);
        self.compiled.remove(server);
    }

    /// Read back the raw config for a server.
    #[must_use]
    pub fn get_server_permissions(&self, server: &str) -> Option<&AclRules> {
        self.servers.get(server)
    }

    /// Iterate over all configured server names.
    pub fn server_names(&self) -> impl Iterator<Item = &str> {
        self.servers.keys().map(String::as_str)
    }

    /// Number of configured servers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.servers.len()
    }

    /// Whether any server has rules configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }
}

impl Default for ServerAclRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ServerAclRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Intentionally hide the compiled-matchers field; it's an
        // internal derived cache and dumping raw Glob matchers is noisy.
        f.debug_struct("ServerAclRegistry")
            .field("server_count", &self.servers.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_rules(allowed: &[&str], denied: &[&str]) -> AclRules {
        AclRules {
            allowed_tools: allowed.iter().map(|s| (*s).to_string()).collect(),
            denied_tools: denied.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_permissions_allows_everything() {
        let perms = ServerAclRegistry::new();
        assert!(perms.is_tool_allowed("any-server", "any-tool"));
        assert!(perms.is_resource_allowed("any-server", "any-resource"));
        assert!(perms.is_prompt_allowed("any-server", "any-prompt"));
        assert!(perms.is_notification_allowed("any-server", "any/method"));
    }

    #[test]
    fn unconfigured_server_defaults_to_allow() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("strict".into(), tool_rules(&[], &["dangerous"]));
        // "other" has no config → allow
        assert!(perms.is_tool_allowed("other", "dangerous"));
        // "strict" → deny dangerous
        assert!(!perms.is_tool_allowed("strict", "dangerous"));
    }

    #[test]
    fn deny_beats_allow() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("srv".into(), tool_rules(&["*"], &["danger"]));
        assert!(perms.is_tool_allowed("srv", "safe"));
        assert!(!perms.is_tool_allowed("srv", "danger"));
    }

    #[test]
    fn empty_allow_list_means_allow_all() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions(
            "srv".into(),
            AclRules {
                denied_tools: vec!["bad".into()],
                ..Default::default()
            },
        );
        assert!(perms.is_tool_allowed("srv", "good"));
    }

    #[test]
    fn non_empty_allow_list_restricts() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("srv".into(), tool_rules(&["read_*"], &[]));
        assert!(perms.is_tool_allowed("srv", "read_file"));
        assert!(!perms.is_tool_allowed("srv", "write_file"));
    }

    #[test]
    fn middle_wildcard_glob_works() {
        // Prior hand-rolled glob couldn't match this; globset can.
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("srv".into(), tool_rules(&["read_*_config"], &[]));
        assert!(perms.is_tool_allowed("srv", "read_project_config"));
        assert!(perms.is_tool_allowed("srv", "read_user_config"));
        assert!(!perms.is_tool_allowed("srv", "read_config"));
        assert!(!perms.is_tool_allowed("srv", "write_user_config"));
    }

    #[test]
    fn char_class_and_alternation() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("srv".into(), tool_rules(&["get_{user,admin}"], &[]));
        assert!(perms.is_tool_allowed("srv", "get_user"));
        assert!(perms.is_tool_allowed("srv", "get_admin"));
        assert!(!perms.is_tool_allowed("srv", "get_guest"));
    }

    #[test]
    fn resource_uri_glob() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions(
            "srv".into(),
            AclRules {
                denied_resources: vec!["secret://**".into()],
                ..Default::default()
            },
        );
        assert!(perms.is_resource_allowed("srv", "file://readme"));
        assert!(!perms.is_resource_allowed("srv", "secret://key"));
        assert!(!perms.is_resource_allowed("srv", "secret://nested/path"));
    }

    #[test]
    fn prompts_channel() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions(
            "srv".into(),
            AclRules {
                allowed_prompts: vec!["code_review".into(), "commit_msg".into()],
                ..Default::default()
            },
        );
        assert!(perms.is_prompt_allowed("srv", "code_review"));
        assert!(perms.is_prompt_allowed("srv", "commit_msg"));
        assert!(!perms.is_prompt_allowed("srv", "translate"));
    }

    #[test]
    fn notifications_channel() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions(
            "srv".into(),
            AclRules {
                denied_notifications: vec!["notifications/progress".into()],
                ..Default::default()
            },
        );
        assert!(perms.is_notification_allowed("srv", "notifications/message"));
        assert!(!perms.is_notification_allowed("srv", "notifications/progress"));
    }

    #[test]
    fn unified_is_allowed_matches_specific_methods() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions(
            "srv".into(),
            AclRules {
                allowed_tools: vec!["get_*".into()],
                denied_resources: vec!["secret".into()],
                ..Default::default()
            },
        );

        assert_eq!(
            perms.is_allowed(AclChannel::Tool, "srv", "get_users"),
            perms.is_tool_allowed("srv", "get_users"),
        );
        assert_eq!(
            perms.is_allowed(AclChannel::Resource, "srv", "secret"),
            perms.is_resource_allowed("srv", "secret"),
        );
    }

    #[test]
    fn invalid_glob_falls_back_to_literal() {
        // Unbalanced bracket: globset rejects, we fall back to escape-
        // then-recompile so it matches the literal string.
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions(
            "srv".into(),
            AclRules {
                allowed_tools: vec!["weird[name".into()],
                ..Default::default()
            },
        );
        // Literal match succeeds.
        assert!(perms.is_tool_allowed("srv", "weird[name"));
        // Non-match fails.
        assert!(!perms.is_tool_allowed("srv", "other"));
    }

    #[test]
    fn empty_pattern_is_ignored() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions(
            "srv".into(),
            AclRules {
                allowed_tools: vec![String::new(), "good".into()],
                ..Default::default()
            },
        );
        assert!(perms.is_tool_allowed("srv", "good"));
        assert!(!perms.is_tool_allowed("srv", "anything_else"));
        // Empty string pattern must not match everything.
        assert!(!perms.is_tool_allowed("srv", ""));
    }

    #[test]
    fn set_then_remove_reverts_to_allow() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("srv".into(), tool_rules(&[], &["bad"]));
        assert!(!perms.is_tool_allowed("srv", "bad"));
        perms.remove_server("srv");
        assert!(perms.is_tool_allowed("srv", "bad"));
        assert_eq!(perms.len(), 0);
    }

    #[test]
    fn overwrite_replaces_matchers() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("srv".into(), tool_rules(&["read_*"], &[]));
        assert!(!perms.is_tool_allowed("srv", "write_file"));
        perms.set_server_permissions("srv".into(), tool_rules(&["write_*"], &[]));
        // `write_*` matches "write_file" but not bare "write" — globset
        // needs at least one char where `*` sits.
        assert!(perms.is_tool_allowed("srv", "write_file"));
        assert!(!perms.is_tool_allowed("srv", "read_file"));
    }

    #[test]
    fn from_config_bulk_load() {
        let mut cfg = HashMap::new();
        cfg.insert("github".into(), tool_rules(&["get_*"], &[]));
        cfg.insert("slack".into(), tool_rules(&[], &["delete_channel"]));
        let perms = ServerAclRegistry::from_config(cfg);
        assert_eq!(perms.len(), 2);
        assert!(perms.is_tool_allowed("github", "get_repos"));
        assert!(!perms.is_tool_allowed("github", "fork_repo"));
        assert!(perms.is_tool_allowed("slack", "post_message"));
        assert!(!perms.is_tool_allowed("slack", "delete_channel"));
    }

    #[test]
    fn server_permissions_serde_full_shape() {
        let sp = AclRules {
            allowed_tools: vec!["read_*".into()],
            denied_tools: vec!["write_secret".into()],
            allowed_resources: vec!["file://**".into()],
            denied_resources: vec!["secret://**".into()],
            allowed_prompts: vec!["code_review".into()],
            denied_prompts: vec!["internal_*".into()],
            allowed_notifications: vec!["notifications/*".into()],
            denied_notifications: vec!["notifications/progress".into()],
        };
        let json = serde_json::to_string(&sp).unwrap();
        let back: AclRules = serde_json::from_str(&json).unwrap();
        assert_eq!(sp, back);
    }

    #[test]
    fn server_names_lists_configured() {
        let mut perms = ServerAclRegistry::new();
        perms.set_server_permissions("a".into(), AclRules::default());
        perms.set_server_permissions("b".into(), AclRules::default());
        let mut names: Vec<_> = perms.server_names().collect();
        names.sort_unstable();
        assert_eq!(names, vec!["a", "b"]);
    }
}
