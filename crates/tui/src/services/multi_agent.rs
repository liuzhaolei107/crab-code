use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentBackendKind {
    InProcess,
    Tmux,
    Iterm2Split,
}

#[derive(Debug, Clone)]
pub struct AgentPane {
    pub agent_id: String,
    pub title: String,
    pub backend: AgentBackendKind,
    pub active: bool,
}

pub trait MultiAgentBackend: Send {
    fn kind(&self) -> AgentBackendKind;

    fn spawn_pane(&mut self, agent_id: &str, title: &str) -> Result<AgentPane, AgentBackendError>;

    fn close_pane(&mut self, agent_id: &str) -> Result<(), AgentBackendError>;

    fn list_panes(&self) -> Vec<&AgentPane>;
}

#[derive(Debug, thiserror::Error)]
pub enum AgentBackendError {
    #[error("backend unavailable: {0}")]
    Unavailable(String),
    #[error("pane not found: {0}")]
    PaneNotFound(String),
    #[error("backend error: {0}")]
    Other(String),
}

pub struct InProcessBackend {
    panes: HashMap<String, AgentPane>,
}

impl InProcessBackend {
    #[must_use]
    pub fn new() -> Self {
        Self {
            panes: HashMap::new(),
        }
    }
}

impl Default for InProcessBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiAgentBackend for InProcessBackend {
    fn kind(&self) -> AgentBackendKind {
        AgentBackendKind::InProcess
    }

    fn spawn_pane(&mut self, agent_id: &str, title: &str) -> Result<AgentPane, AgentBackendError> {
        let pane = AgentPane {
            agent_id: agent_id.to_string(),
            title: title.to_string(),
            backend: AgentBackendKind::InProcess,
            active: true,
        };
        self.panes.insert(agent_id.to_string(), pane.clone());
        Ok(pane)
    }

    fn close_pane(&mut self, agent_id: &str) -> Result<(), AgentBackendError> {
        self.panes
            .remove(agent_id)
            .map(|_| ())
            .ok_or_else(|| AgentBackendError::PaneNotFound(agent_id.to_string()))
    }

    fn list_panes(&self) -> Vec<&AgentPane> {
        self.panes.values().collect()
    }
}

#[allow(dead_code)]
pub struct TmuxBackend {
    session_name: String,
}

#[allow(dead_code)]
impl TmuxBackend {
    pub fn new(session_name: impl Into<String>) -> Self {
        Self {
            session_name: session_name.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_process_spawn_and_list() {
        let mut backend = InProcessBackend::new();
        backend.spawn_pane("agent-1", "Researcher").unwrap();
        assert_eq!(backend.list_panes().len(), 1);
    }

    #[test]
    fn in_process_close() {
        let mut backend = InProcessBackend::new();
        backend.spawn_pane("agent-1", "Researcher").unwrap();
        backend.close_pane("agent-1").unwrap();
        assert!(backend.list_panes().is_empty());
    }

    #[test]
    fn close_nonexistent_errors() {
        let mut backend = InProcessBackend::new();
        assert!(backend.close_pane("nope").is_err());
    }

    #[test]
    fn backend_kind() {
        let backend = InProcessBackend::new();
        assert_eq!(backend.kind(), AgentBackendKind::InProcess);
    }
}
