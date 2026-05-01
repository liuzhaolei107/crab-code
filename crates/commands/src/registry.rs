use std::collections::HashMap;
use std::sync::Arc;

use crate::context::CommandContext;
use crate::types::{CommandResult, SlashCommand};

/// Registry of all available slash commands.
///
/// [`CommandRegistry::new`] pre-registers every built-in command.
/// Use [`CommandRegistry::empty`] for an unpopulated registry (useful in tests).
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn SlashCommand>>,
    order: Vec<String>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    #[must_use]
    pub fn new() -> Self {
        let mut reg = Self::empty();
        crate::builtin::register_all(&mut reg);
        reg
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            commands: HashMap::new(),
            order: Vec::new(),
            aliases: HashMap::new(),
        }
    }

    pub fn register(&mut self, cmd: Arc<dyn SlashCommand>) {
        let name = cmd.name().to_string();
        for alias in cmd.aliases() {
            self.aliases.insert((*alias).to_string(), name.clone());
        }
        self.order.push(name.clone());
        self.commands.insert(name, cmd);
    }

    pub fn register_alias(&mut self, alias: &str, target: &str) {
        debug_assert!(
            self.commands.contains_key(target),
            "alias target `{target}` must be registered first"
        );
        self.aliases.insert(alias.to_string(), target.to_string());
    }

    fn resolve(&self, name: &str) -> Option<&Arc<dyn SlashCommand>> {
        self.commands.get(name).or_else(|| {
            self.aliases
                .get(name)
                .and_then(|target| self.commands.get(target))
        })
    }

    pub fn execute(&self, name: &str, args: &str, ctx: &CommandContext) -> Option<CommandResult> {
        self.resolve(name).map(|cmd| cmd.execute(args, ctx))
    }

    pub fn find(&self, name: &str) -> Option<(&str, &str)> {
        self.resolve(name)
            .map(|cmd| (cmd.name(), cmd.description()))
    }

    /// List all commands in registration order as `(name, description)` pairs.
    /// Aliases are not included.
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.order
            .iter()
            .filter_map(|name| self.commands.get(name))
            .map(|cmd| (cmd.name(), cmd.description()))
            .collect()
    }

    /// Return command names (including aliases) matching the given prefix,
    /// sorted alphabetically for deterministic completion menus.
    pub fn completions(&self, prefix: &str) -> Vec<&str> {
        let mut results: Vec<&str> = self
            .order
            .iter()
            .filter(|name| name.starts_with(prefix))
            .map(String::as_str)
            .chain(
                self.aliases
                    .keys()
                    .filter(|a| a.starts_with(prefix))
                    .map(String::as_str),
            )
            .collect();
        results.sort_unstable();
        results
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_test_ctx, test_model_and_dir};
    use crate::types::{CommandEffect, OverlayKind};

    #[test]
    fn registry_has_expected_command_count() {
        let reg = CommandRegistry::new();
        assert_eq!(reg.len(), 34);
        assert!(!reg.is_empty());
    }

    #[test]
    fn find_known_command() {
        let reg = CommandRegistry::new();
        let (name, desc) = reg.find("help").unwrap();
        assert_eq!(name, "help");
        assert!(!desc.is_empty());
    }

    #[test]
    fn find_unknown_returns_none() {
        let reg = CommandRegistry::new();
        assert!(reg.find("nonexistent").is_none());
    }

    #[test]
    fn list_returns_all() {
        let reg = CommandRegistry::new();
        let list = reg.list();
        assert_eq!(list.len(), 34);
        let names: Vec<&str> = list.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"help"));
        assert!(names.contains(&"exit"));
        assert!(names.contains(&"model"));
        assert!(names.contains(&"cost"));
        assert!(names.contains(&"resume"));
        assert!(names.contains(&"history"));
        assert!(names.contains(&"export"));
        assert!(names.contains(&"effort"));
        assert!(names.contains(&"fast"));
        assert!(names.contains(&"add-dir"));
        assert!(names.contains(&"files"));
        assert!(names.contains(&"branch"));
        assert!(names.contains(&"commit"));
        assert!(names.contains(&"copy"));
        assert!(names.contains(&"team"));
    }

    #[test]
    fn execute_unknown_returns_none() {
        let reg = CommandRegistry::new();
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        assert!(reg.execute("nonexistent", "", &ctx).is_none());
    }

    #[test]
    fn quit_is_alias_of_exit() {
        let reg = CommandRegistry::new();
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);

        let result = reg.execute("quit", "", &ctx).unwrap();
        assert!(matches!(result, CommandResult::Effect(CommandEffect::Exit)));

        let (name, _) = reg.find("quit").unwrap();
        assert_eq!(name, "exit");

        let names: Vec<&str> = reg.list().iter().map(|(n, _)| *n).collect();
        assert!(!names.contains(&"quit"));
        assert_eq!(names.iter().filter(|n| **n == "exit").count(), 1);
    }

    #[test]
    fn default_registry_has_commands() {
        let reg = CommandRegistry::default();
        assert_eq!(reg.len(), 34);
    }

    #[test]
    fn completions_prefix() {
        let reg = CommandRegistry::new();
        let matches = reg.completions("co");
        assert!(matches.contains(&"cost"));
        assert!(matches.contains(&"compact"));
        assert!(matches.contains(&"config"));
        assert!(matches.contains(&"commit"));
        assert!(matches.contains(&"copy"));
        assert!(!matches.contains(&"help"));
    }

    #[test]
    fn completions_include_aliases() {
        let reg = CommandRegistry::new();
        let matches = reg.completions("qu");
        assert!(matches.contains(&"quit"));
    }

    #[test]
    fn empty_registry() {
        let reg = CommandRegistry::empty();
        assert_eq!(reg.len(), 0);
        assert!(reg.is_empty());
        assert!(reg.list().is_empty());
    }

    #[test]
    fn help_via_registry() {
        let reg = CommandRegistry::new();
        let (model, dir) = test_model_and_dir();
        let ctx = make_test_ctx(&model, &dir);
        let result = reg.execute("help", "", &ctx).unwrap();
        assert!(matches!(
            result,
            CommandResult::Effect(CommandEffect::OpenOverlay(OverlayKind::Help))
        ));
    }
}
