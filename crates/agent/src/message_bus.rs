use tokio::sync::mpsc;

/// Messages exchanged between agents via the bus.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    AssignTask { task_id: String, prompt: String },
    TaskComplete { task_id: String, result: String },
    RequestHelp { from: String, message: String },
    Shutdown,
}

/// Inter-agent message bus backed by tokio mpsc channels.
pub struct MessageBus {
    pub tx: mpsc::Sender<AgentMessage>,
    pub rx: mpsc::Receiver<AgentMessage>,
}

impl MessageBus {
    pub fn new(buffer: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer);
        Self { tx, rx }
    }

    pub fn sender(&self) -> mpsc::Sender<AgentMessage> {
        self.tx.clone()
    }
}
