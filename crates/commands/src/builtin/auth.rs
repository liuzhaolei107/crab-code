use crate::context::CommandContext;
use crate::types::{CommandEffect, CommandResult, SlashCommand};

pub struct LoginCommand;

impl SlashCommand for LoginCommand {
    fn name(&self) -> &'static str {
        "login"
    }
    fn description(&self) -> &'static str {
        "Log in to your account"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::Login)
    }
}

pub struct LogoutCommand;

impl SlashCommand for LogoutCommand {
    fn name(&self) -> &'static str {
        "logout"
    }
    fn description(&self) -> &'static str {
        "Log out of your account"
    }
    fn execute(&self, _args: &str, _ctx: &CommandContext) -> CommandResult {
        CommandResult::Effect(CommandEffect::Logout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};

    #[test]
    fn login_returns_effect() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            LoginCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::Login)
        ));
    }

    #[test]
    fn logout_returns_effect() {
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(matches!(
            LogoutCommand.execute("", &ctx),
            CommandResult::Effect(CommandEffect::Logout)
        ));
    }
}
