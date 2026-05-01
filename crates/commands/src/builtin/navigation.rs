use crate::context::CommandContext;
use crate::types::{CommandEffect, CommandResult, OverlayKind, SlashCommand};

pub struct HelpCommand;

impl SlashCommand for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }
    fn description(&self) -> &'static str {
        "List all available commands"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Help))
    }
}

pub struct ClearCommand;

impl SlashCommand for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }
    fn description(&self) -> &'static str {
        "Clear conversation history"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::Clear)
    }
}

pub struct CompactCommand;

impl SlashCommand for CompactCommand {
    fn name(&self) -> &'static str {
        "compact"
    }
    fn description(&self) -> &'static str {
        "Trigger context compaction"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::Compact)
    }
}

pub struct ExitCommand;

impl SlashCommand for ExitCommand {
    fn name(&self) -> &'static str {
        "exit"
    }
    fn description(&self) -> &'static str {
        "Exit the session"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["quit"]
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::Exit)
    }
}

pub struct CopyCommand;

impl SlashCommand for CopyCommand {
    fn name(&self) -> &'static str {
        "copy"
    }
    fn description(&self) -> &'static str {
        "Copy last assistant message"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::CopyLast)
    }
}

pub struct RewindCommand;

impl SlashCommand for RewindCommand {
    fn name(&self) -> &'static str {
        "rewind"
    }
    fn description(&self) -> &'static str {
        "Rewind the latest file edit (/rewind [path])"
    }
    fn execute(&self, args: &str, _ctx: &CommandContext) -> CommandResult {
        let trimmed = args.trim();
        let target = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
        CommandResult::Effect(CommandEffect::Rewind(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};

    #[test]
    fn help_opens_overlay() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        let result = HelpCommand.execute("", &ctx);
        assert!(matches!(
            result,
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Help))
        ));
    }

    #[test]
    fn clear_returns_effect() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ClearCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::Clear)
        ));
    }

    #[test]
    fn compact_returns_effect() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            CompactCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::Compact)
        ));
    }

    #[test]
    fn exit_returns_effect() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ExitCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::Exit)
        ));
    }

    #[test]
    fn exit_has_quit_alias() {
        assert_eq!(ExitCommand.aliases(), &["quit"]);
    }

    #[test]
    fn copy_returns_effect() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            CopyCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::CopyLast)
        ));
    }

    #[test]
    fn rewind_without_arg_targets_all() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            RewindCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::Rewind(None))
        ));
    }

    #[test]
    fn rewind_with_path_targets_path() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            RewindCommand.execute("foo.rs", &ctx),
            CommandResult::Effect(CommandEffect::Rewind(Some(ref p))) if p == "foo.rs"
        ));
    }
}
