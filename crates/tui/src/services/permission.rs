use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionGrant {
    AllowOnce,
    AllowSession,
    AllowAlways,
    Denied,
}

#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub request_id: String,
    pub tool_name: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct PermissionRule {
    pub tool_pattern: String,
    pub grant: PermissionGrant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    AutoAllow,
    AutoDeny,
    NeedsUserDecision,
}

#[derive(Debug)]
pub struct PermissionService {
    session_grants: HashMap<String, PermissionGrant>,
    pending: Option<PermissionRequest>,
    persistent_rules: Vec<PermissionRule>,
}

impl PermissionService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            session_grants: HashMap::new(),
            pending: None,
            persistent_rules: Vec::new(),
        }
    }

    pub fn check(&self, tool_name: &str) -> PermissionDecision {
        if let Some(grant) = self.session_grants.get(tool_name) {
            return match grant {
                PermissionGrant::AllowOnce
                | PermissionGrant::AllowSession
                | PermissionGrant::AllowAlways => PermissionDecision::AutoAllow,
                PermissionGrant::Denied => PermissionDecision::AutoDeny,
            };
        }

        for rule in &self.persistent_rules {
            if tool_name_matches(&rule.tool_pattern, tool_name) {
                return match rule.grant {
                    PermissionGrant::AllowAlways => PermissionDecision::AutoAllow,
                    PermissionGrant::Denied => PermissionDecision::AutoDeny,
                    _ => PermissionDecision::NeedsUserDecision,
                };
            }
        }

        PermissionDecision::NeedsUserDecision
    }

    pub fn record_user_choice(&mut self, tool_name: String, grant: PermissionGrant) {
        match grant {
            PermissionGrant::AllowOnce => {}
            _ => {
                self.session_grants.insert(tool_name, grant);
            }
        }
    }

    pub fn set_pending(&mut self, req: PermissionRequest) {
        self.pending = Some(req);
    }

    #[must_use]
    pub fn take_pending(&mut self) -> Option<PermissionRequest> {
        self.pending.take()
    }

    #[must_use]
    pub fn pending(&self) -> Option<&PermissionRequest> {
        self.pending.as_ref()
    }

    pub fn add_persistent_rule(&mut self, rule: PermissionRule) {
        self.persistent_rules.push(rule);
    }

    #[must_use]
    pub fn persistent_rules(&self) -> &[PermissionRule] {
        &self.persistent_rules
    }
}

impl Default for PermissionService {
    fn default() -> Self {
        Self::new()
    }
}

fn tool_name_matches(pattern: &str, tool_name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return tool_name.starts_with(prefix);
    }
    pattern == tool_name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncached_tool_needs_user_decision() {
        let svc = PermissionService::new();
        assert_eq!(svc.check("Bash"), PermissionDecision::NeedsUserDecision);
    }

    #[test]
    fn allow_session_auto_allows() {
        let mut svc = PermissionService::new();
        svc.record_user_choice("Bash".into(), PermissionGrant::AllowSession);
        assert_eq!(svc.check("Bash"), PermissionDecision::AutoAllow);
    }

    #[test]
    fn denied_auto_denies() {
        let mut svc = PermissionService::new();
        svc.record_user_choice("Bash".into(), PermissionGrant::Denied);
        assert_eq!(svc.check("Bash"), PermissionDecision::AutoDeny);
    }

    #[test]
    fn allow_once_does_not_cache() {
        let mut svc = PermissionService::new();
        svc.record_user_choice("Bash".into(), PermissionGrant::AllowOnce);
        assert_eq!(svc.check("Bash"), PermissionDecision::NeedsUserDecision);
    }

    #[test]
    fn persistent_rule_wildcard() {
        let mut svc = PermissionService::new();
        svc.add_persistent_rule(PermissionRule {
            tool_pattern: "*".into(),
            grant: PermissionGrant::AllowAlways,
        });
        assert_eq!(svc.check("AnyTool"), PermissionDecision::AutoAllow);
    }

    #[test]
    fn persistent_rule_prefix() {
        let mut svc = PermissionService::new();
        svc.add_persistent_rule(PermissionRule {
            tool_pattern: "Bash*".into(),
            grant: PermissionGrant::AllowAlways,
        });
        assert_eq!(svc.check("BashExec"), PermissionDecision::AutoAllow);
        assert_eq!(svc.check("Read"), PermissionDecision::NeedsUserDecision);
    }

    #[test]
    fn session_grant_overrides_persistent() {
        let mut svc = PermissionService::new();
        svc.add_persistent_rule(PermissionRule {
            tool_pattern: "Bash".into(),
            grant: PermissionGrant::Denied,
        });
        svc.record_user_choice("Bash".into(), PermissionGrant::AllowSession);
        assert_eq!(svc.check("Bash"), PermissionDecision::AutoAllow);
    }

    #[test]
    fn pending_set_take() {
        let mut svc = PermissionService::new();
        assert!(svc.pending().is_none());
        svc.set_pending(PermissionRequest {
            request_id: "1".into(),
            tool_name: "Bash".into(),
            summary: "ls".into(),
        });
        assert!(svc.pending().is_some());
        let req = svc.take_pending().unwrap();
        assert_eq!(req.tool_name, "Bash");
        assert!(svc.pending().is_none());
    }
}
