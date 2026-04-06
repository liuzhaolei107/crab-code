use std::path::{Path, PathBuf};

/// A single diagnostic check result.
#[derive(Debug)]
struct Check {
    name: &'static str,
    passed: bool,
    detail: String,
}

/// Run all diagnostic checks and print results.
pub fn run() -> anyhow::Result<()> {
    let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let global_dir = crab_config::settings::global_config_dir();
    let settings = crab_config::settings::load_merged_settings(Some(&working_dir))
        .unwrap_or_default();

    let checks = vec![
        check_api_key(&settings),
        check_settings_file(&global_dir),
        check_git(),
        check_working_dir(&working_dir),
        check_sessions_dir(&global_dir),
        check_memory_dir(&global_dir),
        check_mcp_config(&settings),
    ];

    let mut pass_count = 0;
    let mut fail_count = 0;

    eprintln!("Crab Code Doctor");
    eprintln!("================");
    eprintln!();

    for check in &checks {
        let icon = if check.passed { "ok" } else { "FAIL" };
        eprintln!("  [{}] {} — {}", icon, check.name, check.detail);
        if check.passed {
            pass_count += 1;
        } else {
            fail_count += 1;
        }
    }

    eprintln!();
    eprintln!(
        "{} checks passed, {} failed out of {} total.",
        pass_count,
        fail_count,
        checks.len()
    );

    if fail_count > 0 {
        eprintln!();
        eprintln!("Run 'crab auth login' for help configuring API keys.");
    }

    Ok(())
}

fn check_api_key(settings: &crab_config::Settings) -> Check {
    let has_settings_key = settings
        .api_key
        .as_ref()
        .is_some_and(|k| !k.is_empty());
    let has_anthropic_env = std::env::var("ANTHROPIC_API_KEY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let has_openai_env = std::env::var("OPENAI_API_KEY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let passed = has_settings_key || has_anthropic_env || has_openai_env;

    let detail = if has_settings_key {
        "configured in settings".into()
    } else if has_anthropic_env {
        "ANTHROPIC_API_KEY set".into()
    } else if has_openai_env {
        "OPENAI_API_KEY set".into()
    } else {
        "no API key found (set ANTHROPIC_API_KEY or run 'crab auth setup-token')".into()
    };

    Check {
        name: "API key",
        passed,
        detail,
    }
}

fn check_settings_file(global_dir: &Path) -> Check {
    let path = global_dir.join("settings.json");
    if !path.exists() {
        return Check {
            name: "Settings file",
            passed: true,
            detail: format!("{} (not created yet, using defaults)", path.display()),
        };
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => Check {
                name: "Settings file",
                passed: true,
                detail: format!("{} (valid JSON)", path.display()),
            },
            Err(e) => Check {
                name: "Settings file",
                passed: false,
                detail: format!("{} (invalid JSON: {})", path.display(), e),
            },
        },
        Err(e) => Check {
            name: "Settings file",
            passed: false,
            detail: format!("{} (read error: {})", path.display(), e),
        },
    }
}

fn check_git() -> Check {
    match std::process::Command::new("git")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            Check {
                name: "Git",
                passed: true,
                detail: version,
            }
        }
        Ok(_) => Check {
            name: "Git",
            passed: false,
            detail: "git command failed".into(),
        },
        Err(_) => Check {
            name: "Git",
            passed: false,
            detail: "git not found in PATH".into(),
        },
    }
}

fn check_working_dir(working_dir: &Path) -> Check {
    let readable = working_dir.exists() && working_dir.is_dir();
    let writable = if readable {
        // Try creating a temp file to test write access
        let test_path = working_dir.join(".crab_doctor_test");
        let w = std::fs::write(&test_path, "").is_ok();
        let _ = std::fs::remove_file(&test_path);
        w
    } else {
        false
    };

    Check {
        name: "Working directory",
        passed: readable && writable,
        detail: if readable && writable {
            format!("{} (readable, writable)", working_dir.display())
        } else if readable {
            format!("{} (readable, NOT writable)", working_dir.display())
        } else {
            format!("{} (not accessible)", working_dir.display())
        },
    }
}

fn check_sessions_dir(global_dir: &Path) -> Check {
    check_dir_exists_writable(global_dir, "sessions", "Sessions directory")
}

fn check_memory_dir(global_dir: &Path) -> Check {
    check_dir_exists_writable(global_dir, "memory", "Memory directory")
}

fn check_dir_exists_writable(global_dir: &Path, subdir: &str, name: &'static str) -> Check {
    let dir = global_dir.join(subdir);
    if !dir.exists() {
        return Check {
            name,
            passed: true,
            detail: format!("{} (will be created on first use)", dir.display()),
        };
    }

    let writable = {
        let test_path = dir.join(".crab_doctor_test");
        let w = std::fs::write(&test_path, "").is_ok();
        let _ = std::fs::remove_file(&test_path);
        w
    };

    Check {
        name,
        passed: writable,
        detail: if writable {
            format!("{} (exists, writable)", dir.display())
        } else {
            format!("{} (exists, NOT writable)", dir.display())
        },
    }
}

fn check_mcp_config(settings: &crab_config::Settings) -> Check {
    match &settings.mcp_servers {
        None => Check {
            name: "MCP servers",
            passed: true,
            detail: "no MCP servers configured".into(),
        },
        Some(value) => {
            if let Some(obj) = value.as_object() {
                Check {
                    name: "MCP servers",
                    passed: true,
                    detail: format!("{} server(s) configured", obj.len()),
                }
            } else {
                Check {
                    name: "MCP servers",
                    passed: false,
                    detail: "mcpServers must be a JSON object".into(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_api_key_with_settings() {
        let settings = crab_config::Settings {
            api_key: Some("sk-test".into()),
            ..Default::default()
        };
        let result = check_api_key(&settings);
        assert!(result.passed);
        assert!(result.detail.contains("settings"));
    }

    #[test]
    fn check_api_key_without_any() {
        let settings = crab_config::Settings::default();
        // May or may not pass depending on env vars in CI
        let result = check_api_key(&settings);
        // Just verify it runs without panic
        assert!(!result.name.is_empty());
    }

    #[test]
    fn check_git_runs() {
        let result = check_git();
        // Git should be available in most dev environments
        assert!(!result.name.is_empty());
        if result.passed {
            assert!(result.detail.contains("git"));
        }
    }

    #[test]
    fn check_working_dir_current() {
        let wd = std::env::current_dir().unwrap();
        let result = check_working_dir(&wd);
        assert!(result.passed);
        assert!(result.detail.contains("writable"));
    }

    #[test]
    fn check_working_dir_nonexistent() {
        let result = check_working_dir(Path::new("/nonexistent/path"));
        assert!(!result.passed);
    }

    #[test]
    fn check_settings_file_nonexistent_global() {
        let result = check_settings_file(Path::new("/nonexistent/crab"));
        assert!(result.passed); // Not created yet is fine
        assert!(result.detail.contains("defaults"));
    }

    #[test]
    fn check_mcp_no_config() {
        let settings = crab_config::Settings::default();
        let result = check_mcp_config(&settings);
        assert!(result.passed);
    }

    #[test]
    fn check_mcp_with_servers() {
        let settings = crab_config::Settings {
            mcp_servers: Some(serde_json::json!({
                "playwright": { "command": "npx", "args": ["playwright"] }
            })),
            ..Default::default()
        };
        let result = check_mcp_config(&settings);
        assert!(result.passed);
        assert!(result.detail.contains("1 server"));
    }

    #[test]
    fn check_mcp_invalid_format() {
        let settings = crab_config::Settings {
            mcp_servers: Some(serde_json::json!("not an object")),
            ..Default::default()
        };
        let result = check_mcp_config(&settings);
        assert!(!result.passed);
    }

    #[test]
    fn check_dir_exists_writable_nonexistent() {
        let result = check_dir_exists_writable(
            Path::new("/nonexistent/crab"),
            "sessions",
            "Sessions directory",
        );
        assert!(result.passed); // Will be created on first use
    }
}
