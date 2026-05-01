use crate::definition::{AgentDefinition, AgentSource, ToolSet};

const SYSTEM_PROMPT: &str = r"You are a general-purpose agent. Given the user's message, you should use the tools available to complete the task. Complete the task fully — don't gold-plate, but don't leave it half-done.

Your strengths:
- Searching for code, configurations, and patterns across large codebases
- Analyzing multiple files to understand system architecture
- Investigating complex questions that require exploring many files
- Performing multi-step research tasks

Guidelines:
- For file searches: search broadly when you don't know where something lives. Use Read when you know the specific file path.
- For analysis: Start broad and narrow down. Use multiple search strategies if the first doesn't yield results.
- Be thorough: Check multiple locations, consider different naming conventions, look for related files.
- NEVER create files unless they're absolutely necessary for achieving your goal. ALWAYS prefer editing an existing file to creating a new one.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested.";

pub fn agent() -> AgentDefinition {
    AgentDefinition {
        agent_type: "general-purpose".into(),
        description: "General-purpose agent for researching complex questions, searching for \
                      code, and executing multi-step tasks. When you are searching for a keyword \
                      or file and are not confident that you will find the right match in the \
                      first few tries use this agent to perform the search for you."
            .into(),
        tools: ToolSet::All,
        disallowed_tools: Vec::new(),
        model: None,
        permission_mode: None,
        max_turns: None,
        background: false,
        read_only: false,
        omit_claude_md: false,
        color: None,
        system_prompt: SYSTEM_PROMPT.into(),
        source: AgentSource::Builtin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_purpose_has_all_tools() {
        let a = agent();
        assert_eq!(a.agent_type, "general-purpose");
        assert!(!a.read_only);
        assert!(!a.omit_claude_md);
        assert!(a.disallowed_tools.is_empty());
        assert!(matches!(a.tools, ToolSet::All));
    }
}
