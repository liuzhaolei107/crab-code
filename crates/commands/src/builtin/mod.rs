pub mod auth;
pub mod feedback;
pub mod git;
pub mod meta;
pub mod model;
pub mod navigation;
pub mod project;
pub mod session;
pub mod status;

use std::sync::Arc;

use crate::registry::CommandRegistry;

pub fn register_all(registry: &mut CommandRegistry) {
    // Navigation
    registry.register(Arc::new(navigation::HelpCommand));
    registry.register(Arc::new(navigation::ClearCommand));
    registry.register(Arc::new(navigation::CompactCommand));

    // Status / diagnostics
    registry.register(Arc::new(status::CostCommand));
    registry.register(Arc::new(status::StatusCommand));

    // Meta — memory
    registry.register(Arc::new(meta::MemoryCommand));

    // Project
    registry.register(Arc::new(project::InitCommand));

    // Model
    registry.register(Arc::new(model::ModelCommand));

    // Meta — config / permissions
    registry.register(Arc::new(meta::ConfigCommand));
    registry.register(Arc::new(meta::PermissionsCommand));

    // Navigation — exit
    registry.register(Arc::new(navigation::ExitCommand));

    // Model — plan
    registry.register(Arc::new(model::PlanCommand));

    // Session
    registry.register(Arc::new(session::ResumeCommand));
    registry.register(Arc::new(session::HistoryCommand));
    registry.register(Arc::new(session::ExportCommand));

    // Status — doctor
    registry.register(Arc::new(status::DoctorCommand));

    // Git
    registry.register(Arc::new(git::DiffCommand));
    registry.register(Arc::new(git::ReviewCommand));

    // Model — effort / fast
    registry.register(Arc::new(model::EffortCommand));
    registry.register(Arc::new(model::FastCommand));

    // Status — thinking
    registry.register(Arc::new(status::ThinkingCommand));

    // Meta — skills
    registry.register(Arc::new(meta::SkillsCommand));

    // Project — add-dir / files
    registry.register(Arc::new(project::AddDirCommand));
    registry.register(Arc::new(project::FilesCommand));

    // Meta — plugin / mcp / team
    registry.register(Arc::new(meta::PluginCommand));
    registry.register(Arc::new(meta::McpCommand));
    registry.register(Arc::new(meta::TeamCommand));

    // Git — branch / commit
    registry.register(Arc::new(git::BranchCommand));
    registry.register(Arc::new(git::CommitCommand));

    // Meta — theme / keybindings
    registry.register(Arc::new(meta::ThemeCommand));
    registry.register(Arc::new(meta::KeybindingsCommand));

    // Session — rename
    registry.register(Arc::new(session::RenameCommand));

    // Navigation — copy / rewind
    registry.register(Arc::new(navigation::CopyCommand));
    registry.register(Arc::new(navigation::RewindCommand));

    // Auth
    registry.register(Arc::new(auth::LoginCommand));
    registry.register(Arc::new(auth::LogoutCommand));

    // Feedback
    registry.register(Arc::new(feedback::FeedbackCommand));

    // New meta commands
    registry.register(Arc::new(meta::AgentsCommand));
    registry.register(Arc::new(meta::HooksCommand));
    registry.register(Arc::new(meta::TasksCommand));
    registry.register(Arc::new(meta::ColorCommand));
    registry.register(Arc::new(meta::IdeCommand));
    registry.register(Arc::new(meta::ReloadPluginsCommand));

    // New status commands
    registry.register(Arc::new(status::ContextCommand));
    registry.register(Arc::new(status::ReleaseNotesCommand));

    // New model commands
    registry.register(Arc::new(model::VimCommand));
    registry.register(Arc::new(model::SandboxCommand));

    // New navigation command
    registry.register(Arc::new(navigation::BtwCommand));
}
