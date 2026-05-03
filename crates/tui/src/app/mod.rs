//! App state machine and main event loop.

mod commands;
mod instance;
mod state;
mod update;

pub use instance::App;
pub use state::{
    ActiveToolInfo, AppAction, AppState, ChatMessage, ExitKey, PromptInputMode, ThinkingState,
    ToolCallStatus,
};
