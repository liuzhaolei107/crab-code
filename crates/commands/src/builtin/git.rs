use crate::context::CommandContext;
use crate::types::{CommandEffect, CommandResult, OverlayKind, SlashCommand};

pub struct BranchCommand;

impl SlashCommand for BranchCommand {
    fn name(&self) -> &'static str {
        "branch"
    }
    fn description(&self) -> &'static str {
        "Show current git branch"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let output = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(ctx.working_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if branch.is_empty() {
                    CommandResult::Message("Detached HEAD (no branch).".into())
                } else {
                    CommandResult::Message(format!("Current branch: {branch}"))
                }
            }
            Ok(_) | Err(_) => {
                CommandResult::Message("Not a git repository or git not available.".into())
            }
        }
    }
}

pub struct CommitCommand;

impl SlashCommand for CommitCommand {
    fn name(&self) -> &'static str {
        "commit"
    }
    fn description(&self) -> &'static str {
        "Show recent git commits"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "-10"])
            .current_dir(ctx.working_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.trim().is_empty() {
                    CommandResult::Message("No commits yet.".into())
                } else {
                    CommandResult::Message(format!("Recent commits:\n{text}"))
                }
            }
            Ok(_) | Err(_) => {
                CommandResult::Message("Not a git repository or git not available.".into())
            }
        }
    }
}

pub struct ReviewCommand;

impl SlashCommand for ReviewCommand {
    fn name(&self) -> &'static str {
        "review"
    }
    fn description(&self) -> &'static str {
        "Show pending review items"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let output = std::process::Command::new("git")
            .args(["diff", "--cached", "--stat"])
            .current_dir(ctx.working_dir)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.trim().is_empty() {
                    CommandResult::Message("No staged changes to review.".into())
                } else {
                    CommandResult::Message(format!("Staged changes for review:\n{text}"))
                }
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                CommandResult::Message(format!("git diff --cached failed: {err}"))
            }
            Err(e) => CommandResult::Message(format!("Failed to run git: {e}")),
        }
    }
}

pub struct DiffCommand;

impl SlashCommand for DiffCommand {
    fn name(&self) -> &'static str {
        "diff"
    }
    fn description(&self) -> &'static str {
        "Show git diff summary"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Diff))
    }
}
