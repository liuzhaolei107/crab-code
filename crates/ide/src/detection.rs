//! Detection of the hosting IDE (if any) via parent-process inspection
//! and environment variables.
//!
//! Complements `lockfile::discover()` — the lockfile tells us a plugin
//! is running, this tells us whether *our* process is running inside
//! that plugin's terminal.

#![allow(dead_code)] // R1 scaffolding; wired up in R2

/// Rough classification of the surrounding terminal / IDE host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    JetBrains,
    VsCode,
    WindowsTerminal,
    Unknown,
}

/// Best-effort detection via environment variables first, then parent
/// process name as a fallback.
pub fn detect() -> HostKind {
    // Cheap env-based hints first.
    if std::env::var_os("TERMINAL_EMULATOR")
        .is_some_and(|v| v.to_string_lossy().contains("JetBrains"))
    {
        return HostKind::JetBrains;
    }
    if std::env::var_os("TERM_PROGRAM").is_some_and(|v| v.to_string_lossy() == "vscode") {
        return HostKind::VsCode;
    }
    // R2: fall back to sysinfo parent-pid walk for cases where env
    // vars are stripped (SSH, nested shells).
    HostKind::Unknown
}
