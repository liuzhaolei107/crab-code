pub mod builtin;
pub mod context;
pub mod registry;
pub mod types;

pub use context::{CommandContext, CostSnapshot};
pub use registry::CommandRegistry;
pub use types::{CommandEffect, CommandResult, OverlayKind, SlashCommand};

#[cfg(test)]
pub(crate) mod test_helpers {
    use std::path::{Path, PathBuf};

    use crab_core::model::ModelId;
    use crab_core::permission::PermissionMode;

    use crate::{CommandContext, CostSnapshot};

    pub fn test_model_and_dir() -> (ModelId, PathBuf) {
        (
            ModelId::from("claude-sonnet-4-20250514"),
            PathBuf::from("/tmp/project"),
        )
    }

    pub fn make_test_ctx<'a>(model: &'a ModelId, dir: &'a Path) -> CommandContext<'a> {
        CommandContext {
            model,
            session_id: "sess_test",
            working_dir: dir,
            permission_mode: PermissionMode::Default,
            cost: CostSnapshot::default(),
            estimated_tokens: 5000,
            message_count: 10,
            memory_dir: None,
        }
    }
}
