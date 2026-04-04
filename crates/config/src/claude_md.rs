pub struct ClaudeMd {
    pub content: String,
    pub source: ClaudeMdSource,
}

pub enum ClaudeMdSource {
    Global,
    User,
    Project,
}

pub fn collect_claude_md(_project_dir: &std::path::Path) -> Vec<ClaudeMd> {
    // TODO: collect global → user → project CLAUDE.md files
    Vec::new()
}
