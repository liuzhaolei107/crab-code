use tokio::sync::mpsc;

use crate::message_bus::AgentMessage;

/// Sub-agent worker lifecycle.
pub struct Worker {
    pub id: String,
    pub name: String,
    pub tx: mpsc::Sender<AgentMessage>,
}

impl Worker {
    pub fn new(id: String, name: String, tx: mpsc::Sender<AgentMessage>) -> Self {
        Self { id, name, tx }
    }
}
