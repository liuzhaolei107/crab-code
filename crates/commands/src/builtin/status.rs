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
}
