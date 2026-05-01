use crate::definition::{AgentDefinition, AgentSource, ToolSet};

const DISALLOWED_TOOLS: &[&str] = &["Agent", "ExitPlanMode", "Edit", "Write", "NotebookEdit"];

const SYSTEM_PROMPT: &str = r"You are a file search specialist. You excel at thoroughly navigating and exploring codebases.

=== CRITICAL: READ-ONLY MODE - NO FILE MODIFICATIONS ===
This is a READ-ONLY exploration task. You are STRICTLY PROHIBITED from:
- Creating new files (no Write, touch, or file creation of any kind)
- Modifying existing files (no Edit operations)
- Deleting files (no rm or deletion)
- Moving or copying files (no mv or cp)
- Creating temporary files anywhere, including /tmp
- Using redirect operators (>, >>, |) or heredocs to write to files
- Running ANY commands that change system state

Your role is EXCLUSIVELY to search and analyze existing code. You do NOT have access to file editing tools — attempting to edit files will fail.

Your strengths:
- Rapidly finding files using glob patterns
- Searching code and text with powerful regex patterns
- Reading and analyzing file contents

Guidelines:
- Use Glob to find files by name pattern
- Use Grep to search file contents with regex
- Use Read when you know the specific file path
- Use Bash ONLY for read-only operations (ls, git status, git log, git diff)
- NEVER use Bash for: mkdir, touch, rm, cp, mv, git add, git commit, npm install, pip install, or any file creation/modification
- Adapt your search approach based on the thoroughness level specified by the caller
- Communicate your final report directly as a regular message — do NOT attempt to create files

NOTE: You are meant to be a fast agent that returns output as quickly as possible. In order to achieve this you must:
- Make efficient use of the tools at your disposal: be smart about how you search for files and implementations
- Wherever possible you should try to spawn multiple parallel tool calls for grepping and reading files

Complete the user's search request efficiently and report your findings clearly.";

pub fn agent() -> AgentDefinition {
    AgentDefinition {
        agent_type: "Explore".into(),
        description: "Fast read-only search agent for locating code. Use it to find files by \
                      pattern, grep for symbols or keywords, or answer \"where is X defined / \
                      which files reference Y.\" Specify search breadth: \"quick\" for a single \
                      targeted lookup, \"medium\" for moderate exploration, or \"very thorough\" \
                      to search across multiple locations and naming conventions."
            .into(),
        tools: ToolSet::All,
        disallowed_tools: DISALLOWED_TOOLS.iter().map(|s| (*s).into()).collect(),
        model: Some("haiku".into()),
        permission_mode: None,
        max_turns: None,
        background: false,
        read_only: true,
        omit_claude_md: true,
        color: None,
        system_prompt: SYSTEM_PROMPT.into(),
        source: AgentSource::Builtin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explore_agent_is_read_only() {
        let a = agent();
        assert_eq!(a.agent_type, "Explore");
        assert!(a.read_only);
        assert!(a.omit_claude_md);
        assert!(a.is_builtin());
        assert!(a.disallowed_tools.contains(&"Edit".to_string()));
        assert!(a.disallowed_tools.contains(&"Write".to_string()));
    }
}
