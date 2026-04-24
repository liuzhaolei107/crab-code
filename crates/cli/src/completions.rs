//! Shell completion script generation.
//!
//! Provides a standalone utility to generate shell completion scripts for
//! the `crab` CLI using `clap_complete`. This module complements the
//! `Completion` subcommand in `main.rs` by exposing a reusable function
//! and a crate-local [`Shell`] enum.

use std::io::Write;

use clap::ValueEnum;

/// Supported shells for completion generation.
///
/// This is the canonical argument type for the `crab completion <shell>`
/// subcommand. It wraps [`clap_complete::Shell`] with accepting-aliases
/// (pwsh → PowerShell) and a stable lowercase [`Shell::name`] for logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    /// GNU Bash.
    Bash,
    /// Z shell.
    Zsh,
    /// Fish shell.
    Fish,
    /// PowerShell (cross-platform).
    #[clap(name = "powershell", alias = "pwsh")]
    PowerShell,
}

impl Shell {
    /// Convert to the `clap_complete` [`Shell`](clap_complete::Shell) variant.
    fn to_clap(self) -> clap_complete::Shell {
        match self {
            Self::Bash => clap_complete::Shell::Bash,
            Self::Zsh => clap_complete::Shell::Zsh,
            Self::Fish => clap_complete::Shell::Fish,
            Self::PowerShell => clap_complete::Shell::PowerShell,
        }
    }

    /// The canonical lowercase name of this shell.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::PowerShell => "powershell",
        }
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Generate shell completion scripts for the `crab` binary.
///
/// The completions are written to the provided writer. Pass a clap
/// [`Command`](clap::Command) that describes the full CLI so that all
/// subcommands and flags are included.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn generate_completions<W: Write>(
    shell: Shell,
    cmd: &mut clap::Command,
    writer: &mut W,
) -> std::io::Result<()> {
    clap_complete::generate(shell.to_clap(), cmd, "crab", writer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_name_uses_lowercase() {
        assert_eq!(Shell::Bash.name(), "bash");
        assert_eq!(Shell::Zsh.name(), "zsh");
        assert_eq!(Shell::Fish.name(), "fish");
        assert_eq!(Shell::PowerShell.name(), "powershell");
    }

    #[test]
    fn shell_value_enum_parses_pwsh_alias() {
        use clap::ValueEnum;
        let matched = Shell::from_str("pwsh", true).unwrap();
        assert_eq!(matched, Shell::PowerShell);
    }

    #[test]
    fn shell_display() {
        assert_eq!(Shell::Bash.to_string(), "bash");
        assert_eq!(Shell::PowerShell.to_string(), "powershell");
    }

    #[test]
    fn generate_completions_produces_output() {
        let mut cmd = clap::Command::new("crab")
            .subcommand(clap::Command::new("doctor"))
            .subcommand(clap::Command::new("config"))
            .subcommand(clap::Command::new("session"));

        let mut buf = Vec::new();
        generate_completions(Shell::Bash, &mut cmd, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(!output.is_empty());
        assert!(output.contains("crab"));
    }

    #[test]
    fn all_shells_generate_without_error() {
        use clap::ValueEnum;
        for shell in Shell::value_variants() {
            let mut cmd = clap::Command::new("crab").subcommand(clap::Command::new("test"));
            let mut buf = Vec::new();
            generate_completions(*shell, &mut cmd, &mut buf).unwrap();
            assert!(!buf.is_empty(), "shell {shell} produced empty output");
        }
    }
}
