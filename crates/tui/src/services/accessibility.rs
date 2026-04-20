use std::sync::atomic::{AtomicBool, Ordering};

static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

pub fn init_accessibility() {
    let reduce =
        std::env::var("NO_MOTION").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    REDUCE_MOTION.store(reduce, Ordering::Relaxed);
}

pub fn set_reduce_motion(enabled: bool) {
    REDUCE_MOTION.store(enabled, Ordering::Relaxed);
}

#[must_use]
pub fn reduce_motion() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed)
}

#[must_use]
pub fn spinner_text() -> &'static str {
    if reduce_motion() { "..." } else { "⠋" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_no_reduce() {
        REDUCE_MOTION.store(false, Ordering::Relaxed);
        assert!(!reduce_motion());
        assert_eq!(spinner_text(), "⠋");
    }

    #[test]
    fn reduce_motion_enabled() {
        set_reduce_motion(true);
        assert!(reduce_motion());
        assert_eq!(spinner_text(), "...");
        set_reduce_motion(false);
    }
}
