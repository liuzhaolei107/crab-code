//! Regression: `permissions.deny` always beats a matching `permissions.allow`,
//! regardless of which source contributed which rule.
//!
//! This is a security-boundary invariant — see `docs/config.md` §8.4 and §10.
//! The merge layer in `crates/config` must surface both lists intact so the
//! runtime resolver in `crab_core::permission::PermissionPolicy` can apply
//! deny-first ordering. If a future refactor accidentally turned `allow`
//! into a "later-wins overrides deny" pattern, the merged config would still
//! look fine but the runtime would silently start running denied tools.

use std::path::PathBuf;

use crab_config::{Config, PermissionsConfig, ResolveContext, resolve};
use crab_core::permission::PermissionPolicy;

/// Build a `PermissionPolicy` view from a resolved `Config`. This is the
/// shape that the actual permission checker (`crab_tools::permission::
/// check_permission`) sees at runtime.
fn policy_from(cfg: &Config) -> PermissionPolicy {
    let perms = cfg.permissions.clone().unwrap_or_default();
    PermissionPolicy {
        mode: crab_core::permission::PermissionMode::Default,
        allowed_tools: perms.allow,
        denied_tools: perms.deny,
    }
}

fn write(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

#[test]
fn deny_wins_when_same_source_contains_both() {
    // The simplest case — a single `config.toml` with the same tool in both
    // lists. The runtime check must return Deny.
    let cfg = Config {
        permissions: Some(PermissionsConfig {
            allow: vec!["Bash".into()],
            deny: vec!["Bash".into()],
            ..Default::default()
        }),
        ..Default::default()
    };
    let policy = policy_from(&cfg);
    let input = serde_json::json!({});
    assert!(
        policy.is_denied_by_filter("Bash", &input),
        "deny must always win over a matching allow"
    );
}

#[test]
fn deny_wins_across_layers_user_allow_project_deny() {
    // User-level layer says "allow Bash", project-level layer says
    // "deny Bash". Per `docs/config.md` §8.4, `deny` wins regardless of
    // which layer contributed it.
    let root = std::env::temp_dir().join("crab-deny-wins-user-allow-project-deny");
    let user_dir = root.join("user");
    let project_dir = root.join("project");
    let project_crab = project_dir.join(".crab");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::create_dir_all(&project_crab).unwrap();

    write(
        &user_dir.join("config.toml"),
        "[permissions]\nallow = [\"Bash\"]\n",
    );
    write(
        &project_crab.join("config.toml"),
        "[permissions]\ndeny = [\"Bash\"]\n",
    );

    let ctx = ResolveContext::new()
        .with_config_dir(user_dir)
        .with_project_dir(Some(project_dir));
    let cfg = resolve(&ctx).unwrap();

    // Both lists must survive the merge — deny must NOT be silently dropped
    // just because a higher-priority allow exists.
    let perms = cfg.permissions.clone().expect("permissions present");
    assert_eq!(perms.allow, vec!["Bash".to_string()]);
    assert_eq!(perms.deny, vec!["Bash".to_string()]);

    let policy = policy_from(&cfg);
    assert!(
        policy.is_denied_by_filter("Bash", &serde_json::json!({})),
        "deny from project layer must beat allow from user layer"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn deny_wins_across_layers_project_allow_user_deny() {
    // Reverse direction — the lower-priority layer (user) holds the deny;
    // the higher-priority layer (project) holds the allow. Deny still wins.
    // Documents that this is about the deny-first *check order*, not about
    // layer precedence. CCB users sometimes assume "more specific layer
    // wins" but that's not how deny works.
    let root = std::env::temp_dir().join("crab-deny-wins-project-allow-user-deny");
    let user_dir = root.join("user");
    let project_dir = root.join("project");
    let project_crab = project_dir.join(".crab");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::create_dir_all(&project_crab).unwrap();

    write(
        &user_dir.join("config.toml"),
        "[permissions]\ndeny = [\"Bash\"]\n",
    );
    write(
        &project_crab.join("config.toml"),
        "[permissions]\nallow = [\"Bash\"]\n",
    );

    let ctx = ResolveContext::new()
        .with_config_dir(user_dir)
        .with_project_dir(Some(project_dir));
    let cfg = resolve(&ctx).unwrap();
    let policy = policy_from(&cfg);
    assert!(
        policy.is_denied_by_filter("Bash", &serde_json::json!({})),
        "deny from any layer must beat allow"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn deny_wins_across_layers_local_allow_user_deny() {
    // The local layer is the highest-priority *file* layer. Even an allow
    // from local must not override a deny from any other layer.
    let root = std::env::temp_dir().join("crab-deny-wins-local-allow");
    let user_dir = root.join("user");
    let project_dir = root.join("project");
    let project_crab = project_dir.join(".crab");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::create_dir_all(&project_crab).unwrap();

    write(
        &user_dir.join("config.toml"),
        "[permissions]\ndeny = [\"Edit\"]\n",
    );
    write(
        &project_crab.join("config.local.toml"),
        "[permissions]\nallow = [\"Edit\"]\n",
    );

    let ctx = ResolveContext::new()
        .with_config_dir(user_dir)
        .with_project_dir(Some(project_dir));
    let cfg = resolve(&ctx).unwrap();
    let policy = policy_from(&cfg);
    assert!(
        policy.is_denied_by_filter("Edit", &serde_json::json!({})),
        "deny in user layer must beat allow in local layer"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn deny_wins_across_layers_cli_config_allow() {
    // `--config <file>` is the highest-priority *file* slot. Even when the
    // user-supplied override file allows Bash, a deny from the user's
    // persistent config still blocks it. (The runtime CLI override
    // `permission-mode dangerously` is a separate mechanism.)
    let root = std::env::temp_dir().join("crab-deny-wins-cli-allow");
    let user_dir = root.join("user");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&user_dir).unwrap();

    write(
        &user_dir.join("config.toml"),
        "[permissions]\ndeny = [\"Bash\"]\n",
    );
    let cli_file: PathBuf = root.join("override.toml");
    write(&cli_file, "[permissions]\nallow = [\"Bash\"]\n");

    let ctx = ResolveContext::new()
        .with_config_dir(user_dir)
        .with_cli_config_file(Some(cli_file));
    let cfg = resolve(&ctx).unwrap();
    let policy = policy_from(&cfg);
    assert!(
        policy.is_denied_by_filter("Bash", &serde_json::json!({})),
        "deny survives even when the highest file-layer allow contradicts"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn deny_glob_pattern_overrides_specific_allow() {
    // The deny entry uses a glob (`mcp__*`); the allow entry names a
    // specific tool inside that glob. Glob deny still wins.
    let cfg = Config {
        permissions: Some(PermissionsConfig {
            allow: vec!["mcp__github_create_issue".into()],
            deny: vec!["mcp__*".into()],
            ..Default::default()
        }),
        ..Default::default()
    };
    let policy = policy_from(&cfg);
    let input = serde_json::json!({});
    assert!(
        policy.is_denied_by_filter("mcp__github_create_issue", &input),
        "glob deny must win over a more specific allow"
    );
}
