//! Probe the host terminal to decide which insert-history strategy to use.
//!
//! Standard mode relies on DECSTBM scroll regions plus Reverse Index (ESC M)
//! to slide existing scrollback content downward. Some terminals (Zellij,
//! legacy Windows console host) silently drop or mishandle those sequences,
//! so the probe falls back to emitting newlines at the screen bottom.

use std::collections::HashMap;

/// Strategy for writing history above the inline viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertHistoryMode {
    /// Use DECSTBM + Reverse Index. Preferred — preserves existing content
    /// without redrawing it.
    Standard,
    /// Emit newlines at the bottom of the screen and write lines at absolute
    /// positions. Compatible with terminals that drop scroll-region escapes.
    Fallback,
}

/// Snapshot of environment variables relevant to the probe. Lifted out so
/// tests can drive each branch deterministically without mutating process env.
#[derive(Debug, Clone, Default)]
pub struct EnvSnapshot {
    vars: HashMap<String, String>,
}

impl EnvSnapshot {
    /// Capture the relevant slice of the current process environment.
    pub fn from_process_env() -> Self {
        let keys = [
            "CRAB_TUI_INSERT_MODE",
            "ZELLIJ",
            "WT_SESSION",
            "ConEmuPID",
            "MSYSTEM",
            "TERM_PROGRAM",
            "TMUX",
            "STY",
            "TERM",
        ];
        let mut vars = HashMap::new();
        for key in keys {
            if let Ok(value) = std::env::var(key)
                && !value.is_empty()
            {
                vars.insert(key.to_string(), value);
            }
        }
        Self { vars }
    }

    /// Empty snapshot — no env signals at all.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Builder helper for tests.
    #[must_use]
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    fn has(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }
}

/// Decide which insert-history mode to use given a snapshot of env vars and
/// whether the current platform is Windows. Pure function, fully testable.
#[must_use]
pub fn probe(env: &EnvSnapshot, platform_is_windows: bool) -> InsertHistoryMode {
    // 1. Explicit override wins.
    if let Some(override_value) = env.get("CRAB_TUI_INSERT_MODE") {
        match override_value.to_ascii_lowercase().as_str() {
            "standard" => return InsertHistoryMode::Standard,
            "fallback" => return InsertHistoryMode::Fallback,
            _ => {}
        }
    }

    // 2. Zellij intercepts terminal escapes — must use newline fallback.
    if env.has("ZELLIJ") {
        return InsertHistoryMode::Fallback;
    }

    // 3-5. Modern terminals on Windows that fully support DECSTBM.
    if env.has("WT_SESSION") || env.has("ConEmuPID") {
        return InsertHistoryMode::Standard;
    }
    if env.has("MSYSTEM") || env.get("TERM_PROGRAM") == Some("mintty") {
        return InsertHistoryMode::Standard;
    }

    // 6. Multiplexers.
    if env.has("TMUX") || env.has("STY") {
        return InsertHistoryMode::Standard;
    }

    // 7. Any TERM_PROGRAM (Apple Terminal, iTerm2, VSCode integrated, etc.).
    if env.has("TERM_PROGRAM") {
        return InsertHistoryMode::Standard;
    }

    // 8. Recognizable TERM values on non-Windows.
    if !platform_is_windows
        && let Some(term) = env.get("TERM")
        && (term.contains("xterm")
            || term.contains("screen")
            || term.contains("tmux")
            || term == "linux")
    {
        return InsertHistoryMode::Standard;
    }

    // 9. On Windows with no recognizable terminal, assume bare conhost and
    // play it safe.
    if platform_is_windows {
        return InsertHistoryMode::Fallback;
    }

    // 10. Catch-all fallback.
    InsertHistoryMode::Fallback
}

/// Convenience wrapper: probe with the live process env and current platform.
#[must_use]
pub fn detect() -> InsertHistoryMode {
    probe(&EnvSnapshot::from_process_env(), cfg!(windows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_standard_wins() {
        let env = EnvSnapshot::empty().with("CRAB_TUI_INSERT_MODE", "standard");
        assert_eq!(probe(&env, true), InsertHistoryMode::Standard);
    }

    #[test]
    fn override_fallback_wins() {
        let env = EnvSnapshot::empty()
            .with("CRAB_TUI_INSERT_MODE", "fallback")
            .with("WT_SESSION", "abc");
        assert_eq!(probe(&env, false), InsertHistoryMode::Fallback);
    }

    #[test]
    fn override_unknown_value_ignored() {
        let env = EnvSnapshot::empty().with("CRAB_TUI_INSERT_MODE", "garbage");
        // Falls through, lands on bare-conhost branch on Windows.
        assert_eq!(probe(&env, true), InsertHistoryMode::Fallback);
    }

    #[test]
    fn zellij_forces_fallback() {
        let env = EnvSnapshot::empty().with("ZELLIJ", "0");
        assert_eq!(probe(&env, false), InsertHistoryMode::Fallback);
    }

    #[test]
    fn windows_terminal_uses_standard() {
        let env = EnvSnapshot::empty().with("WT_SESSION", "deadbeef");
        assert_eq!(probe(&env, true), InsertHistoryMode::Standard);
    }

    #[test]
    fn conemu_uses_standard() {
        let env = EnvSnapshot::empty().with("ConEmuPID", "1234");
        assert_eq!(probe(&env, true), InsertHistoryMode::Standard);
    }

    #[test]
    fn git_bash_uses_standard() {
        let env = EnvSnapshot::empty().with("MSYSTEM", "MINGW64");
        assert_eq!(probe(&env, true), InsertHistoryMode::Standard);
    }

    #[test]
    fn mintty_term_program_uses_standard() {
        let env = EnvSnapshot::empty().with("TERM_PROGRAM", "mintty");
        assert_eq!(probe(&env, true), InsertHistoryMode::Standard);
    }

    #[test]
    fn tmux_uses_standard() {
        let env = EnvSnapshot::empty().with("TMUX", "/tmp/tmux-1000/default,12345,0");
        assert_eq!(probe(&env, false), InsertHistoryMode::Standard);
    }

    #[test]
    fn screen_uses_standard() {
        let env = EnvSnapshot::empty().with("STY", "1234.pts-0.host");
        assert_eq!(probe(&env, false), InsertHistoryMode::Standard);
    }

    #[test]
    fn apple_terminal_uses_standard() {
        let env = EnvSnapshot::empty().with("TERM_PROGRAM", "Apple_Terminal");
        assert_eq!(probe(&env, false), InsertHistoryMode::Standard);
    }

    #[test]
    fn xterm_term_on_unix_uses_standard() {
        let env = EnvSnapshot::empty().with("TERM", "xterm-256color");
        assert_eq!(probe(&env, false), InsertHistoryMode::Standard);
    }

    #[test]
    fn linux_console_uses_standard() {
        let env = EnvSnapshot::empty().with("TERM", "linux");
        assert_eq!(probe(&env, false), InsertHistoryMode::Standard);
    }

    #[test]
    fn unknown_term_on_unix_uses_fallback() {
        let env = EnvSnapshot::empty().with("TERM", "exotic-thing");
        assert_eq!(probe(&env, false), InsertHistoryMode::Fallback);
    }

    #[test]
    fn bare_conhost_uses_fallback() {
        // No env signals + platform is Windows → assume cmd.exe conhost.
        let env = EnvSnapshot::empty();
        assert_eq!(probe(&env, true), InsertHistoryMode::Fallback);
    }

    #[test]
    fn unix_no_term_uses_fallback() {
        let env = EnvSnapshot::empty();
        assert_eq!(probe(&env, false), InsertHistoryMode::Fallback);
    }
}
