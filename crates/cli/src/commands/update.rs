use clap::Subcommand;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/crabcode/crab-code/releases";

/// Update management subcommands.
#[derive(Subcommand)]
pub enum UpdateAction {
    /// Check for a newer version (default if no subcommand given)
    Check {
        /// List the most recent releases
        #[arg(long)]
        list: bool,
    },
    /// Install a specific version (or latest)
    Install {
        /// Target version to install (e.g. "0.3.0"). Defaults to latest.
        #[arg()]
        target: Option<String>,

        /// Only show what would be done, don't actually install
        #[arg(long)]
        dry_run: bool,

        /// Force install even if already on this version
        #[arg(long)]
        force: bool,
    },
    /// Roll back to a previous version
    Rollback {
        /// Version to roll back to
        #[arg()]
        target: Option<String>,
    },
}

/// Current binary version from Cargo.toml.
fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run(action: &UpdateAction) -> anyhow::Result<()> {
    match action {
        UpdateAction::Check { list } => run_check(*list),
        UpdateAction::Install {
            target,
            dry_run,
            force,
        } => run_install(target.as_deref(), *dry_run, *force),
        UpdateAction::Rollback { target } => run_rollback(target.as_deref()),
    }
}

/// Default entry point when `crab update` is invoked without a subcommand.
pub fn run_default() -> anyhow::Result<()> {
    run_check(false)
}

fn run_check(list: bool) -> anyhow::Result<()> {
    let current = current_version();
    eprintln!("crab-code v{current}");
    eprintln!("Releases URL: {GITHUB_RELEASES_URL}");
    eprintln!();

    if list {
        eprintln!("Recent releases:");
        eprintln!("  (network fetch not yet implemented — check GitHub Releases page)");
    } else {
        eprintln!("To check for updates, visit:");
        eprintln!("  https://github.com/crabcode/crab-code/releases");
        eprintln!();
        eprintln!("Automatic update checking will be implemented in a future release.");
    }

    Ok(())
}

fn run_install(target: Option<&str>, dry_run: bool, force: bool) -> anyhow::Result<()> {
    let current = current_version();
    let version_label = target.unwrap_or("latest");

    eprintln!("Current version: v{current}");
    eprintln!("Target version:  {version_label}");

    if dry_run {
        eprintln!("[dry-run] Would download and install v{version_label}");
        return Ok(());
    }

    if force {
        eprintln!("Force flag set — will install even if versions match.");
    }

    eprintln!();
    eprintln!("Binary download and replacement is not yet implemented.");
    eprintln!("Install manually: cargo install crab-code");

    Ok(())
}

fn run_rollback(target: Option<&str>) -> anyhow::Result<()> {
    let current = current_version();
    eprintln!("Current version: v{current}");

    match target {
        Some(v) => {
            eprintln!("Requested rollback to: v{v}");
            eprintln!();
            eprintln!("Rollback is not yet implemented.");
            eprintln!("Install a specific version: cargo install crab-code@{v}");
        }
        None => {
            eprintln!();
            eprintln!("Available rollback targets:");
            eprintln!("  (version history not yet available — check GitHub Releases page)");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_version_is_semver() {
        let v = current_version();
        assert!(!v.is_empty());
        // Should have at least major.minor.patch
        let parts: Vec<&str> = v.split('.').collect();
        assert!(parts.len() >= 2, "version should be semver: {v}");
    }

    #[test]
    fn run_check_default() {
        assert!(run_check(false).is_ok());
    }

    #[test]
    fn run_check_list() {
        assert!(run_check(true).is_ok());
    }

    #[test]
    fn run_install_dry_run() {
        assert!(run_install(Some("1.0.0"), true, false).is_ok());
    }

    #[test]
    fn run_install_force() {
        assert!(run_install(None, false, true).is_ok());
    }

    #[test]
    fn run_rollback_with_target() {
        assert!(run_rollback(Some("0.1.0")).is_ok());
    }

    #[test]
    fn run_rollback_without_target() {
        assert!(run_rollback(None).is_ok());
    }
}
