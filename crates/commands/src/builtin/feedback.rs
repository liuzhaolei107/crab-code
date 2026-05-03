use crate::context::CommandContext;
use crate::types::{CommandResult, SlashCommand};

pub struct FeedbackCommand;

impl SlashCommand for FeedbackCommand {
    fn name(&self) -> &'static str {
        "feedback"
    }
    fn description(&self) -> &'static str {
        "Submit feedback via GitHub Issue"
    }
    fn execute(&self, args: &str, ctx: &CommandContext) -> CommandResult {
        let title = args.trim();
        if title.is_empty() {
            return CommandResult::Message(
                "Usage: /feedback <title>\nCreates a GitHub issue with environment info.".into(),
            );
        }
        let body = format!(
            "**Submitted via /feedback**\n\nModel: {}\nSession: {}\nWorking dir: {}",
            ctx.model,
            ctx.session_id,
            ctx.working_dir.display()
        );
        match std::process::Command::new("gh")
            .args([
                "issue",
                "create",
                "--repo",
                "crabforge/crab-code",
                "--title",
                title,
                "--body",
                &body,
            ])
            .output()
        {
            Ok(output) if output.status.success() => {
                let url = String::from_utf8_lossy(&output.stdout);
                CommandResult::Message(format!("Issue created: {}", url.trim()))
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                CommandResult::Message(format!("Failed to create issue: {}", err.trim()))
            }
            Err(_) => CommandResult::Message(
                "GitHub CLI (gh) not found. Install it from https://cli.github.com/".into(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};

    #[test]
    fn feedback_no_args_shows_usage() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = FeedbackCommand.execute("", &ctx) {
            assert!(text.contains("Usage:"));
            assert!(text.contains("/feedback"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn feedback_with_args_returns_message() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        let result = FeedbackCommand.execute("My bug report", &ctx);
        assert!(matches!(result, CommandResult::Message(_)));
    }
}
