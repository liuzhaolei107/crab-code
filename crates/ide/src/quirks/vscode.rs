//! VS Code-specific hints for IDE detection.

#![allow(dead_code)] // R1 scaffolding; wired up in R2

pub const PROCESS_BASENAMES: &[&str] =
    &["code", "code-insiders", "Code.exe", "Code - Insiders.exe"];

/// Value of `TERM_PROGRAM` set by VS Code's integrated terminal.
pub const TERM_PROGRAM_VALUE: &str = "vscode";
