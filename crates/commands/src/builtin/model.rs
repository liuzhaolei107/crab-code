use crate::context::CommandContext;
use crate::types::{CommandEffect, CommandResult, OverlayKind, SlashCommand};

pub struct ModelCommand;

impl SlashCommand for ModelCommand {
    fn name(&self) -> &'static str {
        "model"
    }
    fn description(&self) -> &'static str {
        "Switch model (/model <name-or-alias>)"
    }
    fn execute(&self, args: &str, _ctx: &CommandContext) -> CommandResult {
        let name = args.trim();
        if name.is_empty() {
            return CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Model));
        }
        let resolved = resolve_model_alias(name);
        CommandResult::Effect(CommandEffect::SwitchModel(resolved))
    }
}

pub struct EffortCommand;

impl SlashCommand for EffortCommand {
    fn name(&self) -> &'static str {
        "effort"
    }
    fn description(&self) -> &'static str {
        "Set effort level (/effort low|medium|high|max)"
    }
    fn execute(&self, args: &str, _ctx: &CommandContext) -> CommandResult {
        let level = args.trim().to_lowercase();
        match level.as_str() {
            "low" | "medium" | "high" | "max" => {
                CommandResult::Effect(CommandEffect::SetEffort(level))
            }
            "" => CommandResult::Message(
                "Usage: /effort <low|medium|high|max>\nControls reasoning depth and token budget."
                    .into(),
            ),
            _ => CommandResult::Message(format!(
                "Unknown effort level: '{level}'. Valid: low, medium, high, max"
            )),
        }
    }
}

pub struct FastCommand;

impl SlashCommand for FastCommand {
    fn name(&self) -> &'static str {
        "fast"
    }
    fn description(&self) -> &'static str {
        "Toggle fast mode"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::ToggleFast)
    }
}

pub struct PlanCommand;

impl SlashCommand for PlanCommand {
    fn name(&self) -> &'static str {
        "plan"
    }
    fn description(&self) -> &'static str {
        "Toggle plan mode"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::TogglePlanMode)
    }
}

pub struct VimCommand;

impl SlashCommand for VimCommand {
    fn name(&self) -> &'static str {
        "vim"
    }
    fn description(&self) -> &'static str {
        "Toggle vim input mode"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::ToggleVim)
    }
}

pub struct SandboxCommand;

impl SlashCommand for SandboxCommand {
    fn name(&self) -> &'static str {
        "sandbox"
    }
    fn description(&self) -> &'static str {
        "Toggle sandbox mode"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::ToggleSandbox)
    }
}

fn resolve_model_alias(alias: &str) -> String {
    match alias {
        "sonnet" => "claude-sonnet-4-6".to_string(),
        "opus" => "claude-opus-4-6".to_string(),
        "haiku" => "claude-haiku-4-5-20251001".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};

    #[test]
    fn model_alias_sonnet() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ModelCommand.execute("sonnet", &ctx),
            CommandResult::Effect(CommandEffect::SwitchModel(ref m)) if m == "claude-sonnet-4-6"
        ));
    }

    #[test]
    fn model_alias_opus() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ModelCommand.execute("opus", &ctx),
            CommandResult::Effect(CommandEffect::SwitchModel(ref m)) if m == "claude-opus-4-6"
        ));
    }

    #[test]
    fn model_alias_haiku() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ModelCommand.execute("haiku", &ctx),
            CommandResult::Effect(CommandEffect::SwitchModel(ref m)) if m == "claude-haiku-4-5-20251001"
        ));
    }

    #[test]
    fn model_passthrough_full_id() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ModelCommand.execute("gpt-4o", &ctx),
            CommandResult::Effect(CommandEffect::SwitchModel(ref m)) if m == "gpt-4o"
        ));
    }

    #[test]
    fn model_no_args_opens_picker() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ModelCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Model))
        ));
    }

    #[test]
    fn effort_valid_levels() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        for level in &["low", "medium", "high", "max"] {
            let result = EffortCommand.execute(level, &ctx);
            assert!(
                matches!(&result, CommandResult::Effect(CommandEffect::SetEffort(l)) if l == level),
                "failed for level: {level}"
            );
        }
    }

    #[test]
    fn effort_no_args_shows_usage() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = EffortCommand.execute("", &ctx) {
            assert!(text.contains("Usage:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn effort_invalid_shows_error() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = EffortCommand.execute("turbo", &ctx) {
            assert!(text.contains("Unknown effort level"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn fast_returns_toggle() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            FastCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::ToggleFast)
        ));
    }

    #[test]
    fn plan_returns_toggle() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            PlanCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::TogglePlanMode)
        ));
    }

    #[test]
    fn vim_returns_toggle() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            VimCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::ToggleVim)
        ));
    }

    #[test]
    fn sandbox_returns_toggle() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            SandboxCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::ToggleSandbox)
        ));
    }
}
