use crate::context::CommandContext;
use crate::types::{CommandResult, SlashCommand};

pub struct CostCommand;

impl SlashCommand for CostCommand {
    fn name(&self) -> &'static str {
        "cost"
    }
    fn description(&self) -> &'static str {
        "Show token usage and cost"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let c = &ctx.cost;
        let text = format!(
            "Session cost:\n  Input tokens:    {}\n  Output tokens:   {}\n  Cache read:      {}\n  Cache creation:  {}\n  API calls:       {}\n  Total cost:      ${:.4}",
            c.input_tokens,
            c.output_tokens,
            c.cache_read_tokens,
            c.cache_creation_tokens,
            c.api_calls,
            c.total_cost_usd,
        );
        CommandResult::Message(text)
    }
}

pub struct StatusCommand;

impl SlashCommand for StatusCommand {
    fn name(&self) -> &'static str {
        "status"
    }
    fn description(&self) -> &'static str {
        "Show session status (model, tokens, dir)"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let text = format!(
            "Session status:\n  Model:           {}\n  Session ID:      {}\n  Working dir:     {}\n  Permission mode: {}\n  Messages:        {}\n  Est. tokens:     {}",
            ctx.model,
            ctx.session_id,
            ctx.working_dir.display(),
            ctx.permission_mode,
            ctx.message_count,
            ctx.estimated_tokens,
        );
        CommandResult::Message(text)
    }
}

pub struct ThinkingCommand;

impl SlashCommand for ThinkingCommand {
    fn name(&self) -> &'static str {
        "thinking"
    }
    fn description(&self) -> &'static str {
        "Show current thinking/effort settings"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let text = format!(
            "Thinking settings:\n  Model: {}\n  Permission mode: {}",
            ctx.model, ctx.permission_mode,
        );
        CommandResult::Message(text)
    }
}

pub struct DoctorCommand;

impl SlashCommand for DoctorCommand {
    fn name(&self) -> &'static str {
        "doctor"
    }
    fn description(&self) -> &'static str {
        "Run health diagnostics"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let mut checks = vec!["Health check:".to_string()];

        if ctx.working_dir.exists() {
            checks.push(format!(
                "  [ok] Working directory: {}",
                ctx.working_dir.display()
            ));
        } else {
            checks.push(format!(
                "  [!!] Working directory missing: {}",
                ctx.working_dir.display()
            ));
        }

        let git_ok = std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok();
        checks.push(if git_ok {
            "  [ok] git: available".into()
        } else {
            "  [!!] git: not found".into()
        });

        checks.push(format!("  [ok] Model: {}", ctx.model));
        checks.push(format!("  [ok] Permission mode: {}", ctx.permission_mode));

        match ctx.memory_dir {
            Some(d) if d.exists() => {
                checks.push(format!("  [ok] Memory dir: {}", d.display()));
            }
            Some(d) => checks.push(format!("  [!!] Memory dir missing: {}", d.display())),
            None => checks.push("  [--] Memory dir: not configured".into()),
        }

        CommandResult::Message(checks.join("\n"))
    }
}

pub struct ContextCommand;

impl SlashCommand for ContextCommand {
    fn name(&self) -> &'static str {
        "context"
    }
    fn description(&self) -> &'static str {
        "Show context window usage"
    }
    #[allow(clippy::cast_sign_loss)]
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let used = ctx.estimated_tokens;
        let total = ctx.context_window;
        let pct = if total > 0 {
            (used as f64 / total as f64 * 100.0) as u64
        } else {
            0
        };
        let filled = (pct as usize) / 5;
        let empty = 20_usize.saturating_sub(filled);
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
        let c = &ctx.cost;
        CommandResult::Message(format!(
            "Context usage: {pct}% ({used} / {total} tokens)\n{bar} {pct}%\nInput: {} · Output: {}\nMessages: {} · API calls: {}",
            c.input_tokens, c.output_tokens, ctx.message_count, c.api_calls,
        ))
    }
}

pub struct ReleaseNotesCommand;

impl SlashCommand for ReleaseNotesCommand {
    fn name(&self) -> &'static str {
        "release-notes"
    }
    fn description(&self) -> &'static str {
        "Show release notes"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["changelog"]
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Message(
            "Crab Code v0.1.0\n\n\
             - Agentic coding CLI with multi-provider LLM support\n\
             - Full TUI with syntax highlighting and diff viewer\n\
             - 44+ built-in tools (Bash, Read, Write, Edit, Glob, Grep, ...)\n\
             - Agent teams with in-process coordination\n\
             - MCP server integration\n\
             - Session persistence and resume\n\n\
             For full changelog, see CHANGELOG.md"
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};

    #[test]
    fn cost_shows_summary() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = CostCommand.execute("", &ctx) {
            assert!(text.contains("Session cost:"));
            assert!(text.contains("Input tokens:"));
            assert!(text.contains("Total cost:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn status_shows_session_info() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = StatusCommand.execute("", &ctx) {
            assert!(text.contains("claude-sonnet-4-20250514"));
            assert!(text.contains("sess_test"));
            assert!(text.contains("Messages:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn thinking_shows_settings() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = ThinkingCommand.execute("", &ctx) {
            assert!(text.contains("Thinking settings:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn doctor_shows_health_check() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = DoctorCommand.execute("", &ctx) {
            assert!(text.contains("Health check:"));
            assert!(text.contains("Model:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn context_shows_usage() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = ContextCommand.execute("", &ctx) {
            assert!(text.contains("Context usage:"));
            assert!(text.contains("tokens"));
            assert!(text.contains("Messages:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn release_notes_shows_info() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = ReleaseNotesCommand.execute("", &ctx) {
            assert!(text.contains("Crab Code"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn release_notes_has_changelog_alias() {
        assert_eq!(ReleaseNotesCommand.aliases(), &["changelog"]);
    }
}
