use std::path::PathBuf;

use crate::context::CommandContext;

/// Trait implemented by all slash commands.
///
/// Each command is a zero-sized struct whose [`execute`](SlashCommand::execute)
/// method receives a read-only [`CommandContext`] and returns a [`CommandResult`]
/// telling the caller what to display or which side-effect to perform.
pub trait SlashCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }
    fn execute(&self, args: &str, ctx: &CommandContext) -> CommandResult;
}

/// Result of executing a slash command.
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Display a message to the user.
    Message(String),
    /// Request a side-effect from the caller (TUI / CLI).
    Effect(CommandEffect),
    /// Command executed silently — no output, no effect.
    Silent,
}

/// Side effects that a command requests the caller to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEffect {
    Clear,
    Compact,
    Exit,
    SwitchModel(String),
    TogglePlanMode,
    Init,
    Export(String),
    SetEffort(String),
    ToggleFast,
    AddDir(PathBuf),
    Resume(String),
    CopyLast,
    Rewind(Option<String>),
    OpenOverlay(OverlayKind),
}

/// Which overlay a command requests the TUI to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Help,
    Model,
    Memory,
    Mcp,
    Team,
    Diff,
    Permissions,
    Config,
    Resume,
}
