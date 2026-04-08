//! Modular section architecture for system prompt construction.
//!
//! Each section of the system prompt (environment, tools, memory, git, `crab_md`,
//! skills, tips, custom instructions) is a separate builder function. Sections
//! are tagged as static (cacheable across turns) or dynamic (changes per turn).
//!
//! Maps to CCB `constants/systemPromptSections.ts` + `constants/prompts.ts`.

use std::fmt::Write;
use std::path::Path;

/// A named section of the system prompt.
#[derive(Debug, Clone)]
pub struct PromptSection {
    /// Unique identifier for this section (e.g., "env", "tools", "memory").
    pub name: &'static str,
    /// The assembled text content of this section.
    pub content: String,
    /// Whether this section is static (cacheable) or dynamic (regenerated per turn).
    pub cache_scope: CacheScope,
}

/// Controls whether a section is included in the API prompt cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheScope {
    /// Static section: stable across turns, should be in the cached prefix.
    Static,
    /// Dynamic section: changes per turn, placed after the cache boundary.
    Dynamic,
}

/// Marker inserted into the system prompt to separate static from dynamic sections.
/// The API client uses this to set the `cache_control` breakpoint.
pub const DYNAMIC_BOUNDARY_MARKER: &str = "<!-- SYSTEM_PROMPT_DYNAMIC_BOUNDARY -->";

/// Registry of section builders.
pub struct SectionRegistry {
    builders: Vec<Box<dyn SectionBuilder + Send + Sync>>,
}

/// Trait for building a prompt section.
pub trait SectionBuilder: Send + Sync {
    /// Section name for identification and caching.
    fn name(&self) -> &'static str;
    /// Whether this section is static or dynamic.
    fn cache_scope(&self) -> CacheScope;
    /// Build the section content. May return `None` to skip.
    fn build(&self, ctx: &SectionContext) -> Option<String>;
}

/// Context passed to section builders.
pub struct SectionContext<'a> {
    /// The project root directory.
    pub project_dir: &'a Path,
    /// Pre-rendered tool descriptions.
    pub tool_descriptions: &'a str,
    /// Current git status summary, if available.
    pub git_status: Option<&'a str>,
    /// CRAB.md content, if available.
    pub crab_md_content: Option<&'a str>,
    /// Memory content, if available.
    pub memory_content: Option<&'a str>,
    /// Custom user instructions, if available.
    pub custom_instructions: Option<&'a str>,
}

impl SectionRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            builders: Vec::new(),
        }
    }

    /// Register a section builder.
    pub fn register(&mut self, builder: Box<dyn SectionBuilder + Send + Sync>) {
        self.builders.push(builder);
    }

    /// Create a registry with all default section builders.
    pub fn default_sections() -> Self {
        todo!("Register: env, tools, memory, git, crab_md, skills, tips, custom")
    }

    /// Build all sections and assemble them, inserting the dynamic boundary marker.
    ///
    /// Static sections are placed first, followed by the boundary marker,
    /// then dynamic sections.
    pub fn assemble(&self, ctx: &SectionContext) -> String {
        let mut statics: Vec<PromptSection> = Vec::new();
        let mut dynamics: Vec<PromptSection> = Vec::new();

        for builder in &self.builders {
            if let Some(content) = builder.build(ctx) {
                let section = PromptSection {
                    name: builder.name(),
                    content,
                    cache_scope: builder.cache_scope(),
                };
                match section.cache_scope {
                    CacheScope::Static => statics.push(section),
                    CacheScope::Dynamic => dynamics.push(section),
                }
            }
        }

        let mut output = String::with_capacity(4096);

        // Static (cacheable) sections first
        for section in &statics {
            let _ = writeln!(output, "{}\n", section.content);
        }

        // Boundary marker
        if !dynamics.is_empty() {
            let _ = writeln!(output, "{DYNAMIC_BOUNDARY_MARKER}\n");
        }

        // Dynamic sections after the boundary
        for section in &dynamics {
            let _ = writeln!(output, "{}\n", section.content);
        }

        output
    }
}

impl Default for SectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSection {
        name: &'static str,
        scope: CacheScope,
        content: Option<String>,
    }

    impl SectionBuilder for StubSection {
        fn name(&self) -> &'static str {
            self.name
        }

        fn cache_scope(&self) -> CacheScope {
            self.scope
        }

        fn build(&self, _ctx: &SectionContext) -> Option<String> {
            self.content.clone()
        }
    }

    fn make_ctx() -> SectionContext<'static> {
        SectionContext {
            project_dir: Path::new("."),
            tool_descriptions: "",
            git_status: None,
            crab_md_content: None,
            memory_content: None,
            custom_instructions: None,
        }
    }

    #[test]
    fn assemble_empty_registry() {
        let registry = SectionRegistry::new();
        let result = registry.assemble(&make_ctx());
        assert!(result.is_empty());
    }

    #[test]
    fn assemble_static_only() {
        let mut registry = SectionRegistry::new();
        registry.register(Box::new(StubSection {
            name: "env",
            scope: CacheScope::Static,
            content: Some("Environment info".into()),
        }));
        let result = registry.assemble(&make_ctx());
        assert!(result.contains("Environment info"));
        assert!(!result.contains(DYNAMIC_BOUNDARY_MARKER));
    }

    #[test]
    fn assemble_static_and_dynamic() {
        let mut registry = SectionRegistry::new();
        registry.register(Box::new(StubSection {
            name: "env",
            scope: CacheScope::Static,
            content: Some("Static part".into()),
        }));
        registry.register(Box::new(StubSection {
            name: "git",
            scope: CacheScope::Dynamic,
            content: Some("Dynamic part".into()),
        }));
        let result = registry.assemble(&make_ctx());
        let boundary_pos = result.find(DYNAMIC_BOUNDARY_MARKER).unwrap();
        let static_pos = result.find("Static part").unwrap();
        let dynamic_pos = result.find("Dynamic part").unwrap();
        assert!(static_pos < boundary_pos);
        assert!(boundary_pos < dynamic_pos);
    }

    #[test]
    fn assemble_skips_none_sections() {
        let mut registry = SectionRegistry::new();
        registry.register(Box::new(StubSection {
            name: "skills",
            scope: CacheScope::Static,
            content: None,
        }));
        registry.register(Box::new(StubSection {
            name: "env",
            scope: CacheScope::Static,
            content: Some("Present".into()),
        }));
        let result = registry.assemble(&make_ctx());
        assert!(result.contains("Present"));
        assert!(!result.contains("skills"));
    }

    #[test]
    fn cache_scope_equality() {
        assert_eq!(CacheScope::Static, CacheScope::Static);
        assert_ne!(CacheScope::Static, CacheScope::Dynamic);
    }
}
