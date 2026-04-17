pub mod auto_dream;
pub mod coordinator;
pub mod error_recovery;
pub mod file_history;
pub mod proactive;
pub mod repl_commands;
pub mod session;
pub mod slash_commands;
pub mod summarizer;
pub mod system_prompt;
pub mod teams;

pub use coordinator::{PermissionDecisionEvent, PermissionSyncManager};
pub use error_recovery::{ErrorCategory, ErrorClassifier, RecoveryAction, RecoveryStrategy};
pub use file_history::{FileHistory, Snapshot, SnapshotError};
pub use repl_commands::{CommandResult, ReplCommand};
pub use session::{AgentSession, SessionConfig};
pub use slash_commands::{
    SlashAction, SlashCommandContext, SlashCommandRegistry, SlashCommandResult,
};
pub use summarizer::{
    ConversationSummary, SummarizerConfig, SummaryItem, SummaryItemKind, summarize_conversation,
};
pub use system_prompt::{build_system_prompt, build_system_prompt_with_memories};
pub use teams::{
    AgentHandle, AgentMessage, AgentStatus, AgentWorker, Capability, Envelope, InProcessBackend,
    MessageRouter, PaneInfo, PaneManager, RetryDecision, RetryPolicy, RetryTracker, SharedTaskList,
    SwarmBackend, Task, TaskList, TaskStatus, Team, TeamMember, TeamMode, Teammate, TeammateConfig,
    TeammateState, TmuxBackend, Worker, WorkerConfig, WorkerPool, WorkerResult, event_channel,
    generate_init_script, shared_task_list,
};
