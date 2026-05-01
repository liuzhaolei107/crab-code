use crate::context::CommandContext;
use crate::types::{CommandEffect, CommandResult, OverlayKind, SlashCommand};

pub struct HistoryCommand;

impl SlashCommand for HistoryCommand {
    fn name(&self) -> &'static str {
        "history"
    }
    fn description(&self) -> &'static str {
        "List recent sessions"
    }
    fn execute(&self, _args: &str, ctx: &CommandContext) -> CommandResult {
        let sessions_dir = ctx
            .working_dir
            .parent()
            .unwrap_or(ctx.working_dir)
            .join(".crab")
            .join("sessions");

        if !sessions_dir.exists() {
            return CommandResult::Message("No session history found.".into());
        }

        let mut entries: Vec<(String, std::time::SystemTime)> = std::fs::read_dir(&sessions_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|e| {
                let mtime = e.metadata().ok()?.modified().ok()?;
                let name = e.path().file_stem()?.to_string_lossy().to_string();
                Some((name, mtime))
            })
            .collect();

        entries.sort_by_key(|e| std::cmp::Reverse(e.1));

        if entries.is_empty() {
            return CommandResult::Message("No sessions found.".into());
        }

        let max = 10.min(entries.len());
        let mut lines = vec![format!(
            "Recent sessions (showing {max} of {}):",
            entries.len()
        )];
        for (id, _mtime) in &entries[..max] {
            lines.push(format!("  {id}"));
        }
        lines.push("\nUse /resume <id> to continue a session.".into());
        CommandResult::Message(lines.join("\n"))
    }
}

pub struct ExportCommand;

impl SlashCommand for ExportCommand {
    fn name(&self) -> &'static str {
        "export"
    }
    fn description(&self) -> &'static str {
        "Export conversation (/export [path])"
    }
    fn execute(&self, args: &str, ctx: &CommandContext) -> CommandResult {
        let path = args.trim();
        if path.is_empty() {
            let default = format!("session_{}.md", ctx.session_id);
            return CommandResult::Effect(CommandEffect::Export(default));
        }
        CommandResult::Effect(CommandEffect::Export(path.to_string()))
    }
}

pub struct ResumeCommand;

impl SlashCommand for ResumeCommand {
    fn name(&self) -> &'static str {
        "resume"
    }
    fn description(&self) -> &'static str {
        "Resume a previous session (/resume [id])"
    }
    fn execute(&self, args: &str, _ctx: &CommandContext) -> CommandResult {
        let id = args.trim();
        if id.is_empty() {
            return CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Resume));
        }
        CommandResult::Effect(CommandEffect::Resume(id.to_string()))
    }
}

pub struct RenameCommand;

impl SlashCommand for RenameCommand {
    fn name(&self) -> &'static str {
        "rename"
    }
    fn description(&self) -> &'static str {
        "Rename current session (/rename <name>)"
    }
    fn execute(&self, args: &str, _ctx: &CommandContext) -> CommandResult {
        let name = args.trim();
        if name.is_empty() {
            return CommandResult::Message("Usage: /rename <name>".into());
        }
        CommandResult::Message(format!("Session renamed to: {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};
    use crate::types::CommandEffect;

    #[test]
    fn resume_no_args_opens_picker() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ResumeCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Resume))
        ));
    }

    #[test]
    fn resume_with_id_returns_action() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ResumeCommand.execute("abc123", &ctx),
            CommandResult::Effect(CommandEffect::Resume(ref id)) if id == "abc123"
        ));
    }

    #[test]
    fn export_default_path() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Effect(CommandEffect::Export(path)) = ExportCommand.execute("", &ctx)
        {
            assert!(path.starts_with("session_"));
            assert!(path.to_ascii_lowercase().ends_with(".md"));
        } else {
            panic!("expected Export effect");
        }
    }

    #[test]
    fn export_custom_path() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            ExportCommand.execute("output.md", &ctx),
            CommandResult::Effect(CommandEffect::Export(ref p)) if p == "output.md"
        ));
    }

    #[test]
    fn rename_no_args_shows_usage() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = RenameCommand.execute("", &ctx) {
            assert!(text.contains("Usage:"));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn rename_with_name_confirms() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        if let CommandResult::Message(text) = RenameCommand.execute("my-session", &ctx) {
            assert!(text.contains("my-session"));
        } else {
            panic!("expected Message");
        }
    }
}
