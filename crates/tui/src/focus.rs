#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FocusTarget {
    #[default]
    Composer,
    MessageList,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_composer() {
        assert_eq!(FocusTarget::default(), FocusTarget::Composer);
    }
}
