//! JetBrains-specific hints for IDE detection.
//!
//! The MCP protocol itself is IDE-agnostic; this module only holds
//! data used by `detection.rs` to classify the host environment.

#![allow(dead_code)] // R1 scaffolding; wired up in R2

/// Executable basenames that identify `JetBrains` IDEs in the process
/// ancestry. Used by parent-pid walks when `TERMINAL_EMULATOR` is
/// absent (e.g. over SSH).
pub const PROCESS_BASENAMES: &[&str] = &[
    "idea",
    "idea64",
    "pycharm",
    "pycharm64",
    "webstorm",
    "webstorm64",
    "phpstorm",
    "phpstorm64",
    "goland",
    "goland64",
    "rustrover",
    "rustrover64",
    "clion",
    "clion64",
    "rubymine",
    "rubymine64",
    "datagrip",
    "datagrip64",
];

/// Value of `TERMINAL_EMULATOR` set by `JetBrains`' built-in terminal.
pub const TERMINAL_EMULATOR_VALUE: &str = "JetBrains-JediTerm";
