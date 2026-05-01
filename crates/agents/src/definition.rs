use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AgentDefinition {
    pub agent_type: String,
    pub description: String,
    pub tools: ToolSet,
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    pub max_turns: Option<u32>,
    pub background: bool,
    pub read_only: bool,
    pub omit_claude_md: bool,
    pub color: Option<AgentColor>,
    pub system_prompt: String,
    pub source: AgentSource,
}

#[derive(Debug, Clone)]
pub enum ToolSet {
    All,
    Specific(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum AgentSource {
    Builtin,
    Custom { path: PathBuf },
    Plugin { plugin_name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentColor {
    Red,
    Blue,
    Green,
    Yellow,
    Purple,
    Pink,
    Cyan,
}

impl std::fmt::Display for AgentColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Red => write!(f, "red"),
            Self::Blue => write!(f, "blue"),
            Self::Green => write!(f, "green"),
            Self::Yellow => write!(f, "yellow"),
            Self::Purple => write!(f, "purple"),
            Self::Pink => write!(f, "pink"),
            Self::Cyan => write!(f, "cyan"),
        }
    }
}

impl std::fmt::Display for AgentSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin => write!(f, "builtin"),
            Self::Custom { path } => write!(f, "custom({})", path.display()),
            Self::Plugin { plugin_name } => write!(f, "plugin({plugin_name})"),
        }
    }
}

impl AgentDefinition {
    pub fn is_builtin(&self) -> bool {
        matches!(self.source, AgentSource::Builtin)
    }

    pub fn is_custom(&self) -> bool {
        matches!(self.source, AgentSource::Custom { .. })
    }

    pub fn is_plugin(&self) -> bool {
        matches!(self.source, AgentSource::Plugin { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_source_display() {
        assert_eq!(AgentSource::Builtin.to_string(), "builtin");
        assert_eq!(
            AgentSource::Custom {
                path: PathBuf::from(".crab/agents/review.md")
            }
            .to_string(),
            "custom(.crab/agents/review.md)"
        );
        assert_eq!(
            AgentSource::Plugin {
                plugin_name: "my-plugin".into()
            }
            .to_string(),
            "plugin(my-plugin)"
        );
    }

    #[test]
    fn agent_color_display() {
        assert_eq!(AgentColor::Red.to_string(), "red");
        assert_eq!(AgentColor::Cyan.to_string(), "cyan");
    }

    #[test]
    fn source_predicates() {
        let def = AgentDefinition {
            agent_type: "test".into(),
            description: "test agent".into(),
            tools: ToolSet::All,
            disallowed_tools: Vec::new(),
            model: None,
            permission_mode: None,
            max_turns: None,
            background: false,
            read_only: false,
            omit_claude_md: false,
            color: None,
            system_prompt: String::new(),
            source: AgentSource::Builtin,
        };
        assert!(def.is_builtin());
        assert!(!def.is_custom());
        assert!(!def.is_plugin());
    }
}
