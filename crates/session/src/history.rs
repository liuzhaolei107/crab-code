use std::path::PathBuf;

/// Persists and recovers session transcripts from disk.
pub struct SessionHistory {
    pub base_dir: PathBuf,
}

impl SessionHistory {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn save(
        &self,
        _session_id: &str,
        _messages: &[crab_core::message::Message],
    ) -> crab_common::Result<()> {
        todo!()
    }

    pub fn load(
        &self,
        _session_id: &str,
    ) -> crab_common::Result<Option<Vec<crab_core::message::Message>>> {
        todo!()
    }

    pub fn list_sessions(&self) -> crab_common::Result<Vec<String>> {
        todo!()
    }
}
