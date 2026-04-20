use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputStyle {
    #[default]
    Structured,
    Compact,
    Verbose,
    Minimal,
}

impl OutputStyle {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Structured => "Structured",
            Self::Compact => "Compact",
            Self::Verbose => "Verbose",
            Self::Minimal => "Minimal",
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Structured => "Tool calls as collapsible cards with headers",
            Self::Compact => "Minimal tool output, collapsed by default",
            Self::Verbose => "All output expanded, full detail",
            Self::Minimal => "Bare text, no decorations",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Structured,
            Self::Compact,
            Self::Verbose,
            Self::Minimal,
        ]
    }

    #[must_use]
    pub fn show_tool_header(self) -> bool {
        !matches!(self, Self::Minimal)
    }

    #[must_use]
    pub fn auto_collapse_output(self) -> bool {
        matches!(self, Self::Compact | Self::Minimal)
    }

    #[must_use]
    pub fn show_status_icons(self) -> bool {
        matches!(self, Self::Structured | Self::Verbose)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_structured() {
        assert_eq!(OutputStyle::default(), OutputStyle::Structured);
    }

    #[test]
    fn all_styles() {
        assert_eq!(OutputStyle::all().len(), 4);
    }

    #[test]
    fn labels_nonempty() {
        for style in OutputStyle::all() {
            assert!(!style.label().is_empty());
            assert!(!style.description().is_empty());
        }
    }

    #[test]
    fn compact_collapses() {
        assert!(OutputStyle::Compact.auto_collapse_output());
        assert!(!OutputStyle::Verbose.auto_collapse_output());
    }

    #[test]
    fn minimal_hides_headers() {
        assert!(!OutputStyle::Minimal.show_tool_header());
        assert!(OutputStyle::Structured.show_tool_header());
    }

    #[test]
    fn serde_roundtrip() {
        let style = OutputStyle::Compact;
        let json = serde_json::to_string(&style).unwrap();
        assert_eq!(json, "\"compact\"");
        let parsed: OutputStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, style);
    }
}
