use crab_core::permission::PermissionMode;
use crab_core::tool::ToolSource;

/// Result of a permission check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Tool execution is allowed.
    Allow,
    /// User confirmation is required.
    Prompt,
    /// Tool execution is denied by policy.
    Deny,
}

/// Check permission for a tool invocation.
#[must_use]
pub fn check_permission(
    mode: PermissionMode,
    source: ToolSource,
    read_only: bool,
    _in_project: bool,
) -> PermissionDecision {
    match mode {
        PermissionMode::Dangerously => PermissionDecision::Allow,
        PermissionMode::TrustProject => {
            if read_only || source == ToolSource::BuiltIn {
                PermissionDecision::Allow
            } else {
                PermissionDecision::Prompt
            }
        }
        PermissionMode::Default => {
            if read_only {
                PermissionDecision::Allow
            } else {
                PermissionDecision::Prompt
            }
        }
    }
}
