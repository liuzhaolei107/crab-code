use tokio::sync::mpsc;

use crate::message_bus::AgentMessage;

/// Multi-agent orchestrator. Manages the main agent and worker pool.
pub struct AgentCoordinator {
    pub main_agent: AgentHandle,
    pub workers: Vec<AgentHandle>,
    pub bus: mpsc::Sender<AgentMessage>,
}

/// Handle to a running agent (main or sub-agent).
pub struct AgentHandle {
    pub id: String,
    pub name: String,
    pub tx: mpsc::Sender<AgentMessage>,
}
