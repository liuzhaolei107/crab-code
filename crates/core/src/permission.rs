use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// All non-read-only tools require user confirmation.
    Default,
    /// Trust in-project file operations; out-of-project and dangerous still prompt.
    TrustProject,
    /// Auto-approve everything (except `denied_tools`). Use with caution.
    Dangerously,
}

impl fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => f.write_str("default"),
            Self::TrustProject => f.write_str("trust-project"),
            Self::Dangerously => f.write_str("dangerously"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub mode: PermissionMode,
    pub allowed_tools: Vec<String>,
    /// Supports glob pattern matching (e.g. "mcp__*", "bash").
    pub denied_tools: Vec<String>,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Default,
            allowed_tools: Vec::new(),
            denied_tools: Vec::new(),
        }
    }
}

impl PermissionPolicy {
    /// Check whether a tool name matches any `denied_tools` glob pattern.
    pub fn is_denied(&self, tool_name: &str) -> bool {
        self.denied_tools
            .iter()
            .any(|pattern| glob_match(pattern, tool_name))
    }

    /// Check whether a tool name is in the `allowed_tools` list.
    pub fn is_explicitly_allowed(&self, tool_name: &str) -> bool {
        self.allowed_tools.iter().any(|a| a == tool_name)
    }
}

/// Result of a permission check for a tool invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Tool execution is allowed without user interaction.
    Allow,
    /// Tool execution is denied; includes the reason.
    Deny(String),
    /// Tool execution requires user confirmation; includes a prompt message.
    AskUser(String),
}

/// Simple glob matching supporting `*` (any chars), `?` (single char),
/// and `[abc]` (character class). This avoids pulling in globset for
/// a small pattern set used only in permission checks.
fn glob_match(pattern: &str, input: &str) -> bool {
    let pat_chars: Vec<char> = pattern.chars().collect();
    let input_chars: Vec<char> = input.chars().collect();
    glob_match_inner(&pat_chars, &input_chars)
}

fn glob_match_inner(pat: &[char], input: &[char]) -> bool {
    let (mut pi, mut ii) = (0, 0);
    let (mut star_pat, mut star_input) = (usize::MAX, usize::MAX);

    while ii < input.len() {
        if pi < pat.len() && pat[pi] == '[' {
            // Character class
            if let Some((matched, end)) = match_char_class(&pat[pi..], input[ii])
                && matched
            {
                pi += end;
                ii += 1;
                continue;
            }
            // No match in class -- fall through to star backtrack
            if star_pat != usize::MAX {
                pi = star_pat + 1;
                star_input += 1;
                ii = star_input;
                continue;
            }
            return false;
        } else if pi < pat.len() && (pat[pi] == '?' || pat[pi] == input[ii]) {
            pi += 1;
            ii += 1;
        } else if pi < pat.len() && pat[pi] == '*' {
            star_pat = pi;
            star_input = ii;
            pi += 1;
        } else if star_pat != usize::MAX {
            pi = star_pat + 1;
            star_input += 1;
            ii = star_input;
        } else {
            return false;
        }
    }

    while pi < pat.len() && pat[pi] == '*' {
        pi += 1;
    }
    pi == pat.len()
}

/// Match a `[abc]` or `[a-z]` character class. Returns (matched, consumed count) or `None` if malformed.
fn match_char_class(pat: &[char], ch: char) -> Option<(bool, usize)> {
    if pat.is_empty() || pat[0] != '[' {
        return None;
    }
    let mut i = 1;
    let mut matched = false;
    while i < pat.len() && pat[i] != ']' {
        if i + 2 < pat.len() && pat[i + 1] == '-' && pat[i + 2] != ']' {
            if ch >= pat[i] && ch <= pat[i + 2] {
                matched = true;
            }
            i += 3;
        } else {
            if ch == pat[i] {
                matched = true;
            }
            i += 1;
        }
    }
    if i < pat.len() && pat[i] == ']' {
        Some((matched, i + 1))
    } else {
        None // Malformed: no closing bracket
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_mode_display() {
        assert_eq!(PermissionMode::Default.to_string(), "default");
        assert_eq!(PermissionMode::TrustProject.to_string(), "trust-project");
        assert_eq!(PermissionMode::Dangerously.to_string(), "dangerously");
    }

    #[test]
    fn permission_mode_serde_roundtrip() {
        let mode = PermissionMode::TrustProject;
        let json = serde_json::to_string(&mode).unwrap();
        let parsed: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, parsed);
    }

    #[test]
    fn policy_default() {
        let policy = PermissionPolicy::default();
        assert_eq!(policy.mode, PermissionMode::Default);
        assert!(policy.allowed_tools.is_empty());
        assert!(policy.denied_tools.is_empty());
    }

    #[test]
    fn policy_is_denied_exact() {
        let policy = PermissionPolicy {
            mode: PermissionMode::Default,
            allowed_tools: vec![],
            denied_tools: vec!["bash".to_string()],
        };
        assert!(policy.is_denied("bash"));
        assert!(!policy.is_denied("read"));
    }

    #[test]
    fn policy_is_denied_glob_star() {
        let policy = PermissionPolicy {
            mode: PermissionMode::Default,
            allowed_tools: vec![],
            denied_tools: vec!["mcp__*".to_string()],
        };
        assert!(policy.is_denied("mcp__playwright_click"));
        assert!(policy.is_denied("mcp__"));
        assert!(!policy.is_denied("bash"));
    }

    #[test]
    fn policy_is_denied_glob_question() {
        let policy = PermissionPolicy {
            mode: PermissionMode::Default,
            allowed_tools: vec![],
            denied_tools: vec!["tool_?".to_string()],
        };
        assert!(policy.is_denied("tool_a"));
        assert!(policy.is_denied("tool_1"));
        assert!(!policy.is_denied("tool_ab"));
        assert!(!policy.is_denied("tool_"));
    }

    #[test]
    fn policy_is_denied_glob_char_class() {
        let policy = PermissionPolicy {
            mode: PermissionMode::Default,
            allowed_tools: vec![],
            denied_tools: vec!["tool_[abc]".to_string()],
        };
        assert!(policy.is_denied("tool_a"));
        assert!(policy.is_denied("tool_b"));
        assert!(policy.is_denied("tool_c"));
        assert!(!policy.is_denied("tool_d"));
    }

    #[test]
    fn policy_is_denied_glob_char_range() {
        let policy = PermissionPolicy {
            mode: PermissionMode::Default,
            allowed_tools: vec![],
            denied_tools: vec!["v[0-9]".to_string()],
        };
        assert!(policy.is_denied("v0"));
        assert!(policy.is_denied("v9"));
        assert!(!policy.is_denied("va"));
    }

    #[test]
    fn policy_is_explicitly_allowed() {
        let policy = PermissionPolicy {
            mode: PermissionMode::Default,
            allowed_tools: vec!["read".to_string(), "glob".to_string()],
            denied_tools: vec![],
        };
        assert!(policy.is_explicitly_allowed("read"));
        assert!(policy.is_explicitly_allowed("glob"));
        assert!(!policy.is_explicitly_allowed("bash"));
    }

    #[test]
    fn permission_decision_variants() {
        let allow = PermissionDecision::Allow;
        let deny = PermissionDecision::Deny("denied by policy".into());
        let ask = PermissionDecision::AskUser("confirm bash execution?".into());

        assert_eq!(allow, PermissionDecision::Allow);
        assert_eq!(deny, PermissionDecision::Deny("denied by policy".into()));
        assert_eq!(
            ask,
            PermissionDecision::AskUser("confirm bash execution?".into())
        );
    }

    #[test]
    fn policy_serde_roundtrip() {
        let policy = PermissionPolicy {
            mode: PermissionMode::TrustProject,
            allowed_tools: vec!["read".into(), "write".into()],
            denied_tools: vec!["mcp__*".into()],
        };
        let json = serde_json::to_string(&policy).unwrap();
        let parsed: PermissionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mode, PermissionMode::TrustProject);
        assert_eq!(parsed.allowed_tools, vec!["read", "write"]);
        assert_eq!(parsed.denied_tools, vec!["mcp__*"]);
    }

    #[test]
    fn glob_match_wildcard_patterns() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", ""));
        assert!(glob_match("foo*", "foobar"));
        assert!(glob_match("*bar", "foobar"));
        assert!(glob_match("f*r", "foobar"));
        assert!(!glob_match("foo*", "barfoo"));
    }

    #[test]
    fn glob_match_exact() {
        assert!(glob_match("bash", "bash"));
        assert!(!glob_match("bash", "Bash"));
        assert!(!glob_match("bash", "bash_exec"));
    }

    #[test]
    fn glob_match_empty_pattern() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "nonempty"));
    }

    #[test]
    fn glob_match_multiple_stars() {
        assert!(glob_match("*foo*bar*", "xxxfooYYYbarZZZ"));
        assert!(!glob_match("*foo*bar*", "xxxbarYYYfooZZZ"));
    }

    #[test]
    fn policy_denied_takes_priority_concept() {
        // If a tool is both allowed and denied, is_denied should still return true.
        let policy = PermissionPolicy {
            mode: PermissionMode::Default,
            allowed_tools: vec!["bash".into()],
            denied_tools: vec!["bash".into()],
        };
        assert!(policy.is_denied("bash"));
        assert!(policy.is_explicitly_allowed("bash"));
    }

    #[test]
    fn permission_mode_all_variants_serde() {
        for mode in [
            PermissionMode::Default,
            PermissionMode::TrustProject,
            PermissionMode::Dangerously,
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: PermissionMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, parsed);
        }
    }

    // ─── Additional glob_match edge cases ───

    #[test]
    fn glob_match_char_class_range() {
        assert!(glob_match("[a-z]_tool", "m_tool"));
        assert!(!glob_match("[a-z]_tool", "M_tool"));
        assert!(!glob_match("[a-z]_tool", "1_tool"));
    }

    #[test]
    fn glob_match_malformed_char_class() {
        // Missing closing bracket — should not match
        assert!(!glob_match("[abc", "a"));
    }

    #[test]
    fn glob_match_star_at_start_and_end() {
        assert!(glob_match("*bash*", "bash"));
        assert!(glob_match("*bash*", "my_bash_tool"));
        assert!(!glob_match("*bash*", "ba_sh"));
    }

    #[test]
    fn glob_match_question_mark_only() {
        assert!(glob_match("?", "a"));
        assert!(!glob_match("?", "ab"));
        assert!(!glob_match("?", ""));
    }

    #[test]
    fn glob_match_complex_pattern() {
        assert!(glob_match("mcp__*__[a-z]*", "mcp__server__tool_name"));
        assert!(!glob_match("mcp__*__[a-z]*", "mcp__server__9tool"));
    }

    #[test]
    fn policy_empty_denied_allows_everything() {
        let policy = PermissionPolicy::default();
        assert!(!policy.is_denied("bash"));
        assert!(!policy.is_denied("read"));
        assert!(!policy.is_denied("mcp__anything"));
    }

    #[test]
    fn policy_multiple_denied_patterns() {
        let policy = PermissionPolicy {
            mode: PermissionMode::TrustProject,
            allowed_tools: vec![],
            denied_tools: vec!["bash".into(), "mcp__*".into(), "dangerous_[a-z]".into()],
        };
        assert!(policy.is_denied("bash"));
        assert!(policy.is_denied("mcp__server__tool"));
        assert!(policy.is_denied("dangerous_x"));
        assert!(!policy.is_denied("read"));
        assert!(!policy.is_denied("dangerous_1"));
    }

    #[test]
    fn permission_decision_deny_message() {
        let decision = PermissionDecision::Deny("tool is in denied list".into());
        if let PermissionDecision::Deny(msg) = &decision {
            assert!(msg.contains("denied"));
        } else {
            panic!("expected Deny");
        }
    }

    #[test]
    fn permission_decision_ask_message() {
        let decision = PermissionDecision::AskUser("Allow bash to run 'rm -rf /'?".into());
        if let PermissionDecision::AskUser(msg) = &decision {
            assert!(msg.contains("bash"));
        } else {
            panic!("expected AskUser");
        }
    }

    #[test]
    fn glob_match_consecutive_stars() {
        // Multiple consecutive stars should behave like one
        assert!(glob_match("**", "anything"));
        assert!(glob_match("a**b", "aXXXb"));
    }

    #[test]
    fn glob_match_char_class_single_char() {
        assert!(glob_match("[x]", "x"));
        assert!(!glob_match("[x]", "y"));
    }
}
