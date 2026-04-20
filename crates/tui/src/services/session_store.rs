use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub turn_count: u32,
    pub path: PathBuf,
}

impl SessionMeta {
    #[must_use]
    pub fn new(id: impl Into<String>, path: PathBuf) -> Self {
        let id = id.into();
        Self {
            title: id.clone(),
            id,
            model: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            turn_count: 0,
            path,
        }
    }

    #[must_use]
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.id
        } else {
            &self.title
        }
    }
}

#[derive(Debug)]
pub struct SessionStore {
    sessions: Vec<SessionMeta>,
    current_session_id: Option<String>,
    sessions_dir: PathBuf,
}

impl SessionStore {
    #[must_use]
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            sessions: Vec::new(),
            current_session_id: None,
            sessions_dir,
        }
    }

    pub fn set_current(&mut self, session_id: impl Into<String>) {
        self.current_session_id = Some(session_id.into());
    }

    #[must_use]
    pub fn current_session_id(&self) -> Option<&str> {
        self.current_session_id.as_deref()
    }

    #[must_use]
    pub fn current_session(&self) -> Option<&SessionMeta> {
        let id = self.current_session_id.as_deref()?;
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn add_session(&mut self, meta: SessionMeta) {
        if let Some(existing) = self.sessions.iter_mut().find(|s| s.id == meta.id) {
            *existing = meta;
        } else {
            self.sessions.push(meta);
        }
    }

    pub fn remove_session(&mut self, session_id: &str) {
        self.sessions.retain(|s| s.id != session_id);
        if self.current_session_id.as_deref() == Some(session_id) {
            self.current_session_id = None;
        }
    }

    #[must_use]
    pub fn sessions(&self) -> &[SessionMeta] {
        &self.sessions
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    #[must_use]
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    pub fn sort_by_updated(&mut self) {
        self.sessions
            .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    }

    #[must_use]
    pub fn find_by_title(&self, query: &str) -> Vec<&SessionMeta> {
        let query_lower = query.to_lowercase();
        self.sessions
            .iter()
            .filter(|s| s.title.to_lowercase().contains(&query_lower))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        PathBuf::from("/tmp/test-sessions")
    }

    fn sample_session(id: &str) -> SessionMeta {
        let mut meta = SessionMeta::new(id, test_dir().join(id));
        meta.title = format!("Session {id}");
        meta.updated_at = id.to_string();
        meta
    }

    #[test]
    fn empty_store() {
        let store = SessionStore::new(test_dir());
        assert_eq!(store.session_count(), 0);
        assert!(store.current_session_id().is_none());
        assert!(store.current_session().is_none());
    }

    #[test]
    fn add_and_retrieve() {
        let mut store = SessionStore::new(test_dir());
        store.add_session(sample_session("abc"));
        assert_eq!(store.session_count(), 1);
        assert_eq!(store.sessions()[0].id, "abc");
    }

    #[test]
    fn upsert_existing() {
        let mut store = SessionStore::new(test_dir());
        store.add_session(sample_session("abc"));
        let mut updated = sample_session("abc");
        updated.title = "Updated".into();
        store.add_session(updated);
        assert_eq!(store.session_count(), 1);
        assert_eq!(store.sessions()[0].title, "Updated");
    }

    #[test]
    fn set_current() {
        let mut store = SessionStore::new(test_dir());
        store.add_session(sample_session("abc"));
        store.set_current("abc");
        assert_eq!(store.current_session_id(), Some("abc"));
        assert_eq!(store.current_session().unwrap().id, "abc");
    }

    #[test]
    fn remove_session() {
        let mut store = SessionStore::new(test_dir());
        store.add_session(sample_session("abc"));
        store.set_current("abc");
        store.remove_session("abc");
        assert_eq!(store.session_count(), 0);
        assert!(store.current_session_id().is_none());
    }

    #[test]
    fn find_by_title() {
        let mut store = SessionStore::new(test_dir());
        store.add_session(sample_session("abc"));
        store.add_session(sample_session("def"));
        let results = store.find_by_title("abc");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "abc");
    }

    #[test]
    fn sort_by_updated() {
        let mut store = SessionStore::new(test_dir());
        store.add_session(sample_session("a"));
        store.add_session(sample_session("c"));
        store.add_session(sample_session("b"));
        store.sort_by_updated();
        assert_eq!(store.sessions()[0].id, "c");
        assert_eq!(store.sessions()[1].id, "b");
        assert_eq!(store.sessions()[2].id, "a");
    }

    #[test]
    fn display_title_fallback() {
        let meta = SessionMeta::new("test-id", test_dir().join("test-id"));
        assert_eq!(meta.display_title(), "test-id");

        let mut meta2 = meta.clone();
        meta2.title = "My Chat".into();
        assert_eq!(meta2.display_title(), "My Chat");
    }
}
