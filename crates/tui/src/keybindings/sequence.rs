use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;

use crate::action::Action;

pub struct KeySequenceParser {
    bindings: Vec<(Vec<KeyEvent>, Action)>,
    pending: Vec<KeyEvent>,
    timeout: Duration,
    last_feed: Option<Instant>,
}

impl KeySequenceParser {
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        Self {
            bindings: Vec::new(),
            pending: Vec::new(),
            timeout,
            last_feed: None,
        }
    }

    pub fn register(&mut self, seq: Vec<KeyEvent>, action: Action) {
        self.bindings.push((seq, action));
    }

    pub fn feed(&mut self, key: KeyEvent) -> SequenceResult {
        if let Some(last) = self.last_feed
            && last.elapsed() > self.timeout
        {
            self.pending.clear();
        }

        self.pending.push(key);
        self.last_feed = Some(Instant::now());

        let mut exact_match = None;
        let mut has_prefix = false;

        for (seq, action) in &self.bindings {
            if *seq == self.pending {
                exact_match = Some(*action);
            } else if seq.len() > self.pending.len()
                && seq[..self.pending.len()] == self.pending[..]
            {
                has_prefix = true;
            }
        }

        if let Some(action) = exact_match {
            if has_prefix {
                SequenceResult::PendingOrMatch(action)
            } else {
                self.pending.clear();
                self.last_feed = None;
                SequenceResult::Complete(action)
            }
        } else if has_prefix {
            SequenceResult::Pending
        } else {
            let single = self.pending.len() == 1;
            self.pending.clear();
            self.last_feed = None;
            if single {
                SequenceResult::PassThrough(key)
            } else {
                SequenceResult::NoMatch
            }
        }
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.last_feed = None;
    }

    #[must_use]
    pub fn pending(&self) -> &[KeyEvent] {
        &self.pending
    }

    pub fn tick(&mut self, now: Instant) -> Option<SequenceResult> {
        if let Some(last) = self.last_feed
            && !self.pending.is_empty()
            && now.duration_since(last) > self.timeout
        {
            let pending = self.pending.clone();
            self.pending.clear();
            self.last_feed = None;
            if pending.len() == 1 {
                return Some(SequenceResult::PassThrough(pending[0]));
            }
            return Some(SequenceResult::NoMatch);
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceResult {
    Complete(Action),
    PendingOrMatch(Action),
    Pending,
    PassThrough(KeyEvent),
    NoMatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    fn ctrl(c: char) -> KeyEvent {
        key(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn plain(c: char) -> KeyEvent {
        key(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn single_key_pass_through() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(500));
        let result = parser.feed(plain('a'));
        assert_eq!(result, SequenceResult::PassThrough(plain('a')));
    }

    #[test]
    fn single_key_match() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(500));
        parser.register(vec![ctrl('c')], Action::Quit);
        let result = parser.feed(ctrl('c'));
        assert_eq!(result, SequenceResult::Complete(Action::Quit));
    }

    #[test]
    fn two_key_sequence() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(500));
        parser.register(vec![ctrl('x'), ctrl('e')], Action::ExternalEditor);

        let first = parser.feed(ctrl('x'));
        assert_eq!(first, SequenceResult::Pending);

        let second = parser.feed(ctrl('e'));
        assert_eq!(second, SequenceResult::Complete(Action::ExternalEditor));
    }

    #[test]
    fn two_key_no_match() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(500));
        parser.register(vec![ctrl('x'), ctrl('e')], Action::ExternalEditor);

        let first = parser.feed(ctrl('x'));
        assert_eq!(first, SequenceResult::Pending);

        let second = parser.feed(ctrl('z'));
        assert_eq!(second, SequenceResult::NoMatch);
    }

    #[test]
    fn timeout_clears_pending() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(100));
        parser.register(vec![ctrl('x'), ctrl('e')], Action::ExternalEditor);

        parser.feed(ctrl('x'));
        assert!(!parser.pending().is_empty());

        std::thread::sleep(Duration::from_millis(150));

        let result = parser.feed(plain('a'));
        assert_eq!(result, SequenceResult::PassThrough(plain('a')));
    }

    #[test]
    fn clear_resets_state() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(500));
        parser.register(vec![ctrl('x'), ctrl('e')], Action::ExternalEditor);

        parser.feed(ctrl('x'));
        assert!(!parser.pending().is_empty());

        parser.clear();
        assert!(parser.pending().is_empty());
    }

    #[test]
    fn tick_expires_pending() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(100));
        parser.register(vec![ctrl('x'), ctrl('e')], Action::ExternalEditor);

        parser.feed(ctrl('x'));
        let now = Instant::now() + Duration::from_millis(200);
        let result = parser.tick(now);
        assert!(matches!(result, Some(SequenceResult::PassThrough(_))));
        assert!(parser.pending().is_empty());
    }

    #[test]
    fn tick_no_expire_within_timeout() {
        let mut parser = KeySequenceParser::new(Duration::from_millis(500));
        parser.register(vec![ctrl('x'), ctrl('e')], Action::ExternalEditor);

        parser.feed(ctrl('x'));
        let now = Instant::now();
        let result = parser.tick(now);
        assert!(result.is_none());
    }
}
