#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    Header,
    MessageList,
    Composer,
    StatusBar,
    ShortcutHint,

    SlashPopup,
    FileAutocompletePopup,

    TranscriptOverlay,
    DiffOverlay,
    HelpOverlay,
    ModelPicker,
    SessionPicker,
    HistorySearch,
    GlobalSearch,
    PermissionDialog,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_id_eq() {
        assert_eq!(ComponentId::Header, ComponentId::Header);
        assert_ne!(ComponentId::Header, ComponentId::Composer);
    }

    #[test]
    fn component_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ComponentId::Composer);
        set.insert(ComponentId::Composer);
        assert_eq!(set.len(), 1);
    }
}
