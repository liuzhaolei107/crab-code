//! TUI REPL runner module.

pub(crate) mod alt_scope;
mod init;
#[cfg(test)]
mod inline_viewport_test;
mod launch;
mod repl;
mod slash;

pub use launch::{ExitInfo, TuiConfig, run};
