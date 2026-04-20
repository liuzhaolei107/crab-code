use std::collections::VecDeque;
use std::time::{Duration, Instant};

const MAX_NOTIFICATIONS: usize = 50;
const DEFAULT_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub created_at: Instant,
    pub ttl: Duration,
}

impl Notification {
    #[must_use]
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: NotificationLevel::Info,
            created_at: Instant::now(),
            ttl: DEFAULT_TTL,
        }
    }

    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: NotificationLevel::Warning,
            created_at: Instant::now(),
            ttl: DEFAULT_TTL,
        }
    }

    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: NotificationLevel::Error,
            created_at: Instant::now(),
            ttl: Duration::from_secs(10),
        }
    }

    #[must_use]
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: NotificationLevel::Success,
            created_at: Instant::now(),
            ttl: DEFAULT_TTL,
        }
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }

    #[must_use]
    pub fn remaining_fraction(&self) -> f64 {
        let elapsed = self.created_at.elapsed().as_secs_f64();
        let total = self.ttl.as_secs_f64();
        (1.0 - elapsed / total).max(0.0)
    }
}

#[derive(Debug)]
pub struct NotificationService {
    queue: VecDeque<Notification>,
}

impl NotificationService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, notification: Notification) {
        if self.queue.len() >= MAX_NOTIFICATIONS {
            self.queue.pop_front();
        }
        self.queue.push_back(notification);
    }

    pub fn push_info(&mut self, message: impl Into<String>) {
        self.push(Notification::info(message));
    }

    pub fn push_warning(&mut self, message: impl Into<String>) {
        self.push(Notification::warning(message));
    }

    pub fn push_error(&mut self, message: impl Into<String>) {
        self.push(Notification::error(message));
    }

    pub fn push_success(&mut self, message: impl Into<String>) {
        self.push(Notification::success(message));
    }

    pub fn gc(&mut self) {
        self.queue.retain(|n| !n.is_expired());
    }

    #[must_use]
    pub fn active(&self) -> Vec<&Notification> {
        self.queue.iter().filter(|n| !n.is_expired()).collect()
    }

    #[must_use]
    pub fn latest(&self) -> Option<&Notification> {
        self.queue.back().filter(|n| !n.is_expired())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active().is_empty()
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_retrieve() {
        let mut svc = NotificationService::new();
        svc.push_info("hello");
        assert!(!svc.is_empty());
        assert_eq!(svc.latest().unwrap().message, "hello");
    }

    #[test]
    fn levels() {
        let info = Notification::info("a");
        let warn = Notification::warning("b");
        let err = Notification::error("c");
        let ok = Notification::success("d");
        assert_eq!(info.level, NotificationLevel::Info);
        assert_eq!(warn.level, NotificationLevel::Warning);
        assert_eq!(err.level, NotificationLevel::Error);
        assert_eq!(ok.level, NotificationLevel::Success);
    }

    #[test]
    fn not_expired_immediately() {
        let n = Notification::info("test");
        assert!(!n.is_expired());
        assert!(n.remaining_fraction() > 0.9);
    }

    #[test]
    fn max_capacity() {
        let mut svc = NotificationService::new();
        for i in 0..(MAX_NOTIFICATIONS + 5) {
            svc.push_info(format!("msg {i}"));
        }
        assert!(svc.queue.len() <= MAX_NOTIFICATIONS);
    }

    #[test]
    fn gc_removes_expired() {
        let mut svc = NotificationService::new();
        svc.queue.push_back(Notification {
            message: "old".into(),
            level: NotificationLevel::Info,
            created_at: Instant::now() - Duration::from_secs(60),
            ttl: Duration::from_secs(1),
        });
        svc.push_info("fresh");
        svc.gc();
        assert_eq!(svc.queue.len(), 1);
        assert_eq!(svc.queue[0].message, "fresh");
    }
}
