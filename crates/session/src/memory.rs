use std::path::PathBuf;

/// File-based memory system (persists across sessions).
pub struct MemoryStore {
    pub path: PathBuf,
}

impl MemoryStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn save(&self, _session_id: &str, _content: &str) -> crab_common::Result<()> {
        todo!()
    }

    pub fn load(&self, _session_id: &str) -> crab_common::Result<Option<String>> {
        todo!()
    }
}
