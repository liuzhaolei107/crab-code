use crate::context::CommandContext;
use crate::types::{CommandEffect, CommandResult, OverlayKind, SlashCommand};

pub struct ConfigCommand;

impl SlashCommand for ConfigCommand {
    fn name(&self) -> &'static str {
        "config"
    }
    fn description(&self) -> &'static str {
        "Show current configuration values"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Config))
    }
}

pub struct PermissionsCommand;

impl SlashCommand for PermissionsCommand {
    fn name(&self) -> &'static str {
        "permissions"
    }
    fn description(&self) -> &'static str {
        "Show current permission mode"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Permissions))
    }
}

pub struct KeybindingsCommand;

impl SlashCommand for KeybindingsCommand {
    fn name(&self) -> &'static str {
        "keybindings"
    }
    fn description(&self) -> &'static str {
        "Show key bindings"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Message(
            "Key bindings:\n  Enter        Send message\n  Ctrl+C       Cancel current operation\n  Ctrl+D       Exit session\n  Tab          Autocomplete\n  Up/Down      Navigate history\n  Esc          Clear input".into(),
        )
    }
}

pub struct ThemeCommand;

impl SlashCommand for ThemeCommand {
    fn name(&self) -> &'static str {
        "theme"
    }
    fn description(&self) -> &'static str {
        "Show current theme"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Message("Current theme: default\nTheme customization coming soon.".into())
    }
}

pub struct PluginCommand;

impl SlashCommand for PluginCommand {
    fn name(&self) -> &'static str {
        "plugin"
    }
    fn description(&self) -> &'static str {
        "List loaded plugins"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Message(
            "Plugins:\n  No plugins loaded.\n  Use `crab plugin list` to manage plugins.".into(),
        )
    }
}

pub struct SkillsCommand;

impl SlashCommand for SkillsCommand {
    fn name(&self) -> &'static str {
        "skills"
    }
    fn description(&self) -> &'static str {
        "List available skills"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Message(
            "Skills:\n  No skills loaded.\n  Use /init to create a AGENTS.md with skill definitions."
                .into(),
        )
    }
}

pub struct McpCommand;

impl SlashCommand for McpCommand {
    fn name(&self) -> &'static str {
        "mcp"
    }
    fn description(&self) -> &'static str {
        "Open MCP server browser"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Mcp))
    }
}

pub struct TeamCommand;

impl SlashCommand for TeamCommand {
    fn name(&self) -> &'static str {
        "team"
    }
    fn description(&self) -> &'static str {
        "Open agent team browser"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Team))
    }
}

pub struct MemoryCommand;

impl SlashCommand for MemoryCommand {
    fn name(&self) -> &'static str {
        "memory"
    }
    fn description(&self) -> &'static str {
        "List memory files"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let Some(dir) = ctx.memory_dir else {
            return CommandResult::Message(
                "No memory directory configured. Set `memory_dir` in ~/.crab/settings.json.".into(),
            );
        };
        if !dir.exists() {
            return CommandResult::Message(format!(
                "Memory directory does not exist: {}",
                dir.display()
            ));
        }
        CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Memory))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};

    #[test]
    fn config_opens_overlay() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ConfigCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Config))
        ));
    }

    #[test]
    fn permissions_opens_overlay() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            PermissionsCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Permissions))
        ));
    }

    #[test]
    fn keybindings_shows_info() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = KeybindingsCommand.execute("", &ctx) {
            assert!(text.contains("Key bindings:"));
            assert!(text.contains("Enter"));
            assert!(text.contains("Ctrl+C"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn theme_shows_info() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = ThemeCommand.execute("", &ctx) {
            assert!(text.contains("theme"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn plugin_shows_info() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = PluginCommand.execute("", &ctx) {
            assert!(text.contains("Plugins:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn skills_shows_info() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = SkillsCommand.execute("", &ctx) {
            assert!(text.contains("Skills:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn mcp_opens_overlay() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            McpCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Mcp))
        ));
    }

    #[test]
    fn team_opens_overlay() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            TeamCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Team))
        ));
    }

    #[test]
    fn memory_no_dir_configured() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = MemoryCommand.execute("", &ctx) {
            assert!(text.contains("No memory directory configured"));
        } else {
            panic!("expected Message");
        }
    }
}
