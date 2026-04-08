//! Fluent API for constructing skills + MCP skill loading.
//!
//! Provides a builder pattern for creating [`Skill`](super::skill::Skill) instances
//! programmatically, and a helper to convert MCP server tool lists into native skills.
//!
//! Maps to CCB `skills/mcpSkills.ts` + `skills/mcpSkillBuilders.ts`.

use super::skill::Skill;

// ─── Skill builder ─────────────────────────────────────────────────────

/// Fluent builder for constructing [`Skill`] instances.
///
/// # Example
///
/// ```no_run
/// use crab_plugin::skill_builder::SkillBuilder;
///
/// let skill = SkillBuilder::new("commit")
///     .description("Create a git commit with a good message")
///     .content("You are a commit helper. ...")
///     .trigger("/commit")
///     .build();
/// ```
pub struct SkillBuilder {
    name: String,
    description: Option<String>,
    content: Option<String>,
    trigger_patterns: Vec<String>,
}

impl SkillBuilder {
    /// Start building a skill with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            content: None,
            trigger_patterns: Vec::new(),
        }
    }

    /// Set the human-readable description.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the skill's prompt content (markdown body).
    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Add a trigger pattern.
    ///
    /// Patterns starting with `/` become `SkillTrigger::Command`,
    /// other patterns become `SkillTrigger::Pattern`.
    #[must_use]
    pub fn trigger(mut self, pattern: impl Into<String>) -> Self {
        self.trigger_patterns.push(pattern.into());
        self
    }

    /// Consume the builder and produce a [`Skill`].
    ///
    /// # Errors
    ///
    /// Returns `Err` if the skill name is empty or if no content was provided.
    pub fn build(self) -> Result<Skill, String> {
        todo!(
            "SkillBuilder::build: validate fields, resolve trigger from patterns, construct Skill for '{}'",
            self.name
        )
    }
}

// ─── MCP skill loading ────────────────────────────────────────────────

/// Convert an MCP server's tool list into native [`Skill`] instances.
///
/// Each MCP tool becomes a skill named `<server_name>:<tool_name>` with
/// the tool's description as the skill description and a `Manual` trigger.
///
/// # Arguments
///
/// * `server_name` — Name of the MCP server (used as skill name prefix).
/// * `tools` — Array of MCP tool definition JSON objects (each with `name`,
///   `description`, etc.).
pub fn load_mcp_skills(_server_name: &str, _tools: &[serde_json::Value]) -> Vec<Skill> {
    todo!("load_mcp_skills: convert MCP tool definitions to Skill instances")
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_chain_compiles() {
        let _builder = SkillBuilder::new("test")
            .description("A test skill")
            .content("prompt content")
            .trigger("/test")
            .trigger("test.*pattern");
        // build() would panic with todo!(), just verify the chain compiles
    }

    #[test]
    fn builder_new_sets_name() {
        let builder = SkillBuilder::new("my-skill");
        assert_eq!(builder.name, "my-skill");
        assert!(builder.description.is_none());
        assert!(builder.content.is_none());
        assert!(builder.trigger_patterns.is_empty());
    }

    #[test]
    fn builder_accumulates_triggers() {
        let builder = SkillBuilder::new("x").trigger("/cmd").trigger("pattern.*");
        assert_eq!(builder.trigger_patterns.len(), 2);
        assert_eq!(builder.trigger_patterns[0], "/cmd");
        assert_eq!(builder.trigger_patterns[1], "pattern.*");
    }
}
