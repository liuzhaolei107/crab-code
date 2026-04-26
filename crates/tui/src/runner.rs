//! TUI REPL runner — wires App, [`AgentSession`], and terminal lifecycle together.
//!
//! Features:
//! - Full agent query loop with tool execution
//! - Permission dialog integration via `PermissionDialog` component
//! - Tool execution progress (spinner) and result display in content area
//! - Session persistence (auto-save on exit, `--resume` support)
//! - Skill `/command` input detection and resolution via `SkillRegistry`

use std::io;

use crate::components::autocomplete::CommandInfo;
use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc;

use crab_agent::runtime::{AgentRuntime, RuntimeInitConfig, RuntimeInitMeta};
use crab_agent::{LlmBackend, SessionConfig};
use crab_core::event::Event;

use crate::app::{App, AppAction};
use crate::app_event::AppEvent;
use crate::event::spawn_event_loop;
use crate::event_broker::EventBroker;
use crate::frame_requester::FrameRequester;

/// Information returned when the TUI exits.
pub struct ExitInfo {
    pub session_id: String,
    pub had_conversation: bool,
}

/// Configuration for launching the TUI REPL.
pub struct TuiConfig {
    pub session_config: SessionConfig,
    pub backend: Arc<LlmBackend>,
    /// Skill directories to scan for `/command` support.
    pub skill_dirs: Vec<PathBuf>,
    /// MCP server configuration from settings (for dynamic tool registration).
    pub mcp_servers: Option<serde_json::Value>,
    /// Validation warnings from settings loading (shown as toasts after init).
    pub settings_warnings: Vec<String>,
}

/// Run the interactive TUI REPL. This is the main entry point for interactive mode.
///
/// Uses a UI-first strategy: the TUI is displayed immediately in an
/// `Initializing` state while MCP, memory, session, and skill loading
/// happen in a background task. Once ready, the event loop receives
/// `InitResult` via a oneshot channel and transitions to `Idle`.
#[allow(clippy::too_many_lines)]
pub async fn run(config: TuiConfig) -> anyhow::Result<ExitInfo> {
    // ── Phase 1: Terminal setup (instant) ────────────────────────────────

    enable_raw_mode()?;

    // Probe the terminal background color via OSC 11 while we still own
    // stdout exclusively and before switching to the alternate screen.
    let detection = crate::theme::detect_background(std::time::Duration::from_millis(80));
    let selected_theme = match detection {
        crate::theme::Detection::Light => crate::theme::Theme::light(),
        _ => crate::theme::Theme::dark(),
    };
    tracing::debug!(?detection, "terminal background detection");
    crate::theme::init_current(selected_theme);

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableBracketedPaste, LeaveAlternateScreen);
        default_hook(info);
    }));

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let term_backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(term_backend)?;

    // ── Phase 2: Create App in Initializing state ────────────────────────

    let model_name = config.session_config.model.as_str().to_string();
    let mut app = App::new(&model_name);
    app.state = crate::app::AppState::Initializing;
    if let Ok(cwd) = std::env::current_dir() {
        app.set_working_dir(cwd.display().to_string());
        app.set_completion_cwd(cwd);
    }
    if let Some(memory_dir) = config.session_config.memory_dir.clone() {
        app.set_memory_dir(memory_dir);
    }

    // Register built-in slash commands for Tab completion
    app.set_slash_commands(builtin_slash_commands());

    // ── Phase 3: Event infrastructure ────────────────────────────────────

    let (event_tx, event_rx) = mpsc::channel::<Event>(256);
    let (perm_resp_tx, perm_resp_rx) = mpsc::unbounded_channel::<(String, bool)>();

    let (agent_ui_tx, agent_ui_rx) = mpsc::unbounded_channel::<Event>();
    spawn_event_forwarder(event_rx, agent_ui_tx);

    let event_broker = Arc::new(EventBroker::new());
    let frame_requester = FrameRequester::default();
    let tick_rate = std::time::Duration::from_millis(100);
    let mut tui_rx = spawn_event_loop(agent_ui_rx, tick_rate, Arc::clone(&event_broker));

    // ── Phase 4a: API preconnect (fire-and-forget TCP+TLS warmup) ──────

    let preconnect_backend = Arc::clone(&config.backend);
    tokio::spawn(async move {
        let _ = preconnect_backend.health_check().await;
    });

    // ── Phase 4b: Spawn background initialization ────────────────────────

    let init_config = RuntimeInitConfig {
        session_config: config.session_config.clone(),
        mcp_servers: config.mcp_servers.clone(),
        skill_dirs: config.skill_dirs.clone(),
        perm_event_tx: event_tx.clone(),
        perm_resp_rx,
    };
    let (init_tx, init_rx) = tokio::sync::oneshot::channel::<(AgentRuntime, RuntimeInitMeta)>();
    tokio::spawn(async move {
        let result = AgentRuntime::init(init_config).await;
        let _ = init_tx.send(result);
    });

    // ── Phase 4c: Settings & skills filesystem watcher ─────────────────

    let home = crab_common::utils::path::home_dir();
    let config_file = crab_config::config::config_file_name();
    let mut settings_watch_paths = vec![home.join(".crab").join(config_file)];
    if let Ok(cwd) = std::env::current_dir() {
        settings_watch_paths.push(cwd.join(".crab").join(config_file));
    }

    let (watch_tx, watch_rx) = mpsc::unbounded_channel();
    let _file_watcher =
        crate::watcher::FileWatcher::new(&settings_watch_paths, &config.skill_dirs, watch_tx);
    let mut watch_rx =
        crate::watcher::debounced_watch(watch_rx, std::time::Duration::from_millis(500));

    // ── Phase 4d: Queue settings validation warnings as toasts ────────────

    for warning in &config.settings_warnings {
        app.notifications.warn(warning.clone());
    }

    // ── Phase 5: Enter event loop immediately ────────────────────────────

    let session_id = config.session_config.session_id.clone();
    let skill_dirs = config.skill_dirs.clone();
    let result = run_loop(
        &mut terminal,
        &mut app,
        &mut tui_rx,
        init_rx,
        &mut watch_rx,
        config.backend,
        event_tx,
        perm_resp_tx,
        &session_id,
        Arc::clone(&event_broker),
        frame_requester.clone(),
        &skill_dirs,
    )
    .await;

    let exit_info = ExitInfo {
        session_id: app.session_id.clone(),
        had_conversation: !app.messages.is_empty(),
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result.map(|()| exit_info)
}

/// Static list of built-in slash commands for Tab completion.
fn builtin_slash_commands() -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            name: "/help".into(),
            description: "Show available commands".into(),
        },
        CommandInfo {
            name: "/clear".into(),
            description: "Clear conversation history".into(),
        },
        CommandInfo {
            name: "/compact".into(),
            description: "Compact conversation (free context)".into(),
        },
        CommandInfo {
            name: "/exit".into(),
            description: "Exit crab-code".into(),
        },
        CommandInfo {
            name: "/model".into(),
            description: "Show or switch the current model".into(),
        },
        CommandInfo {
            name: "/cost".into(),
            description: "Show token usage and cost".into(),
        },
        CommandInfo {
            name: "/status".into(),
            description: "Show session status".into(),
        },
        CommandInfo {
            name: "/memory".into(),
            description: "Show or manage memory files".into(),
        },
        CommandInfo {
            name: "/config".into(),
            description: "Open settings configuration".into(),
        },
        CommandInfo {
            name: "/permissions".into(),
            description: "Show current permission mode".into(),
        },
        CommandInfo {
            name: "/resume".into(),
            description: "Resume a previous session".into(),
        },
        CommandInfo {
            name: "/diff".into(),
            description: "Show recent file changes".into(),
        },
        CommandInfo {
            name: "/review".into(),
            description: "Review recent code changes".into(),
        },
        CommandInfo {
            name: "/commit".into(),
            description: "Create a git commit".into(),
        },
        CommandInfo {
            name: "/plan".into(),
            description: "Enter plan mode".into(),
        },
        CommandInfo {
            name: "/fast".into(),
            description: "Toggle fast mode".into(),
        },
        CommandInfo {
            name: "/thinking".into(),
            description: "Toggle extended thinking".into(),
        },
        CommandInfo {
            name: "/effort".into(),
            description: "Set effort level".into(),
        },
    ]
}

/// Decide whether the welcome panel should display on this start.
///
/// Three independent triggers (mirroring CCB's LogoV2):
///   1. The current binary version differs from `state.last_welcome_version`
///   2. The project has no `AGENTS.md` (new project → show creation hint)
///   3. `CRAB_FORCE_FULL_LOGO` env var is truthy
fn welcome_triggers(
    state: &crab_config::global_state::GlobalState,
    project_dir: &std::path::Path,
) -> (bool, bool) {
    let force = std::env::var("CRAB_FORCE_FULL_LOGO")
        .is_ok_and(|v| !matches!(v.as_str(), "" | "0" | "false" | "no" | "off"));
    let version_new =
        crab_config::global_state::should_show_welcome(state, env!("CARGO_PKG_VERSION"));
    let is_new_project =
        !project_dir.as_os_str().is_empty() && !project_dir.join("AGENTS.md").exists();
    let should_show = force || version_new || is_new_project;
    (should_show, is_new_project)
}

/// Push a `ChatMessage::Welcome` at the front of the transcript when
/// conditions warrant. Also updates `last_welcome_version` so subsequent
/// starts on the same version stay quiet.
///
/// Recent activity lives in the sidebar, so this helper takes no session
/// arguments — the welcome cell only shows the banner + release notes +
/// a one-line action hint.
fn push_welcome_if_needed(app: &mut App) {
    let mut state = crab_config::global_state::load();
    let project_dir = std::path::PathBuf::from(&app.working_dir);
    let (should_show, show_project_hint) = welcome_triggers(&state, &project_dir);
    if !should_show {
        return;
    }

    // What's new — top bullets from docs/CHANGELOG.md's most recent entry.
    let whats_new = crate::changelog::whats_new(3);

    let msg = crate::app::ChatMessage::Welcome {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        whats_new,
        show_project_hint,
    };
    app.messages.insert(0, msg);

    crab_config::global_state::record_welcome_seen(&mut state, env!("CARGO_PKG_VERSION"));
    if let Err(e) = crab_config::global_state::save(&state) {
        app.notifications
            .warn(format!("Failed to persist welcome state: {e}"));
    }
}

/// Push the trust overlay if the current project needs it.
///
/// Called once after background init completes.
fn push_startup_overlays(app: &mut App) {
    if app.working_dir.is_empty() {
        return;
    }
    let global_state = crab_config::global_state::load();
    let project_dir = std::path::PathBuf::from(&app.working_dir);
    if !crab_config::global_state::needs_trust_prompt(&global_state, &project_dir) {
        return;
    }
    let ctx = crate::components::trust_dialog::TrustContext::from_project(&project_dir);
    if ctx.is_empty() {
        return;
    }
    let overlay =
        crate::components::trust_dialog::TrustDialogOverlay::new(app.working_dir.clone(), ctx);
    app.overlay_stack.push(Box::new(overlay));
}

/// Reload settings from disk, returning any validation warnings.
fn reload_settings(app: &mut App, rt: Option<&mut AgentRuntime>) -> Vec<String> {
    let project_dir = if app.working_dir.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(&app.working_dir))
    };

    let ctx = crab_config::ResolveContext::new()
        .with_project_dir(project_dir.clone())
        .with_process_env();
    let validation_warnings = crab_config::validate_all_config_files(project_dir.as_deref());
    match crab_config::resolve(&ctx) {
        Ok(settings) => {
            let errors = validation_warnings;
            if let Some(rt) = rt {
                if let Some(ref model) = settings.model {
                    let model_id = crab_core::model::ModelId::from(model.as_str());
                    rt.loop_config_mut().model = model_id;
                    app.model_name.clone_from(model);
                }
                if let Some(max_tokens) = settings.max_tokens {
                    rt.loop_config_mut().max_tokens = max_tokens;
                }
                if let Some(ref mode_str) = settings.permission_mode
                    && let Ok(mode) = mode_str.parse()
                {
                    app.permission_mode = mode;
                    rt.tool_ctx_mut().permission_mode = mode;
                }
            }
            errors
                .into_iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect()
        }
        Err(e) => {
            app.notifications
                .warn(format!("Failed to reload settings: {e}"));
            Vec::new()
        }
    }
}

/// Convert session metadata into sidebar entries for the TUI.
fn to_sidebar_entries(
    metas: &[crab_agent::SessionMetadata],
) -> Vec<crate::components::session_sidebar::SessionEntry> {
    metas
        .iter()
        .map(|m| crate::components::session_sidebar::SessionEntry {
            id: m.session_id.clone(),
            name: m.name.clone().unwrap_or_else(|| m.session_id.clone()),
            last_active: m
                .modified
                .and_then(|t| t.elapsed().ok())
                .map_or_else(|| "unknown".into(), |d| format!("{}s ago", d.as_secs())),
            message_count: m.message_count,
        })
        .collect()
}

/// What should happen after processing a user-submitted line.
///
/// Any builtin slash command handled entirely in the TUI returns
/// [`SubmitOutcome::Handled`] (no LLM call). Skill expansions and
/// unrecognized text return [`SubmitOutcome::SpawnQuery`] with the
/// payload. `/exit` and `/quit` return [`SubmitOutcome::Quit`].
enum SubmitOutcome {
    /// Send this (possibly skill-expanded) text to the LLM.
    SpawnQuery(String),
    /// Command was fully handled locally; no LLM call.
    Handled,
    /// Tear down the session.
    Quit,
}

/// Route a user-submitted line through the slash registry and apply any
/// local-only effects (push system message, open overlay, compact, switch
/// model, ...). Returns what the caller should do next.
#[allow(clippy::too_many_lines)]
fn handle_submit(
    rt: &mut AgentRuntime,
    app: &mut App,
    text: &str,
    session_id: &str,
) -> SubmitOutcome {
    use crab_agent::{SlashCommandResult, SlashDispatch};

    match rt.dispatch_slash(text, session_id) {
        SlashDispatch::Prompt(prompt) | SlashDispatch::Passthrough(prompt) => {
            SubmitOutcome::SpawnQuery(prompt)
        }
        SlashDispatch::Builtin(result) => match result {
            SlashCommandResult::Message(msg) => {
                app.push_system_message(msg);
                SubmitOutcome::Handled
            }
            SlashCommandResult::Silent => SubmitOutcome::Handled,
            SlashCommandResult::Action(action) => apply_slash_action(rt, app, action, session_id),
        },
    }
}

/// Translate a [`crab_agent::SlashAction`] into concrete state mutations.
fn apply_slash_action(
    rt: &mut AgentRuntime,
    app: &mut App,
    action: crab_agent::SlashAction,
    session_id: &str,
) -> SubmitOutcome {
    use crab_agent::SlashAction;
    use crab_core::permission::PermissionMode;

    match action {
        SlashAction::Exit => SubmitOutcome::Quit,

        SlashAction::Clear => {
            rt.save_session(session_id);
            rt.new_session(session_id);
            app.reset_for_new_session();
            app.push_system_message("Conversation cleared.");
            SubmitOutcome::Handled
        }

        SlashAction::Compact => {
            let (before, after, removed, _summary) = rt.compact_now();
            app.messages.push(crate::app::ChatMessage::CompactBoundary {
                strategy: "heuristic-summarizer".into(),
                after_tokens: after,
                removed_messages: removed,
            });
            app.total_input_tokens = before;
            app.total_output_tokens = 0;
            SubmitOutcome::Handled
        }

        SlashAction::SwitchModel(name) => {
            rt.loop_config_mut().model = crab_core::model::ModelId::from(name.as_str());
            app.model_name.clone_from(&name);
            app.push_system_message(format!("Switched model to {name}"));
            SubmitOutcome::Handled
        }

        SlashAction::TogglePlanMode => {
            let cur = rt.tool_ctx().permission_mode;
            let next = if cur == PermissionMode::Plan {
                PermissionMode::Default
            } else {
                PermissionMode::Plan
            };
            rt.tool_ctx_mut().permission_mode = next;
            app.permission_mode = next;
            app.push_system_message(format!("Permission mode: {next}"));
            SubmitOutcome::Handled
        }

        SlashAction::OpenOverlay(kind) => {
            app.open_overlay_by_kind(kind);
            SubmitOutcome::Handled
        }

        SlashAction::Init => {
            // /init writes a starter AGENTS.md into the working directory.
            // Keep inline so /init is a no-LLM command — matches the
            // "no surprises" spirit of slash actions.
            let path = rt.tool_ctx().working_dir.join("AGENTS.md");
            if path.exists() {
                app.push_system_message(format!("AGENTS.md already exists at {}", path.display()));
            } else {
                let template = "# Project Instructions\n\n\
                    Use this file to tell Crab Code how to work in this project:\n\
                    conventions, required commands, test targets, review rules, etc.\n";
                match std::fs::write(&path, template) {
                    Ok(()) => app.push_system_message(format!(
                        "Wrote AGENTS.md template at {}",
                        path.display()
                    )),
                    Err(e) => {
                        app.push_system_message(format!("Failed to write AGENTS.md: {e}"));
                    }
                }
            }
            SubmitOutcome::Handled
        }

        SlashAction::Resume(id) => {
            // Defer session switching to the App's existing SwitchSession
            // path by queueing an AppAction on the next tick — simpler than
            // plumbing a new path through here.
            app.push_system_message(format!("Resuming session {id}\u{2026}"));
            app.apply_event(crate::app_event::AppEvent::SwitchSession(id));
            SubmitOutcome::Handled
        }

        SlashAction::Export(path) => {
            // Dump the conversation as markdown. Lossy — mostly useful for
            // saving a transcript to share. Images and tool-use details are
            // collapsed to placeholders.
            use std::fmt::Write as _;
            let mut out = String::new();
            for msg in rt.conversation().messages() {
                let _ = writeln!(out, "{msg:?}\n");
            }
            match std::fs::write(&path, out) {
                Ok(()) => app.push_system_message(format!("Exported conversation to {path}")),
                Err(e) => app.push_system_message(format!("Export failed: {e}")),
            }
            SubmitOutcome::Handled
        }

        SlashAction::SetEffort(level) => {
            if let Ok(effort) = level.parse::<crab_agent::EffortLevel>() {
                rt.loop_config_mut().effort = Some(effort);
                app.push_system_message(format!("Effort level set to {level}"));
            } else {
                app.push_system_message(format!(
                    "Unknown effort level '{level}'. Use low|medium|high|max."
                ));
            }
            SubmitOutcome::Handled
        }

        SlashAction::ToggleFast => {
            // Fast mode is a provider-side feature (Opus 4.6 only); without
            // live negotiation we can only record the desired flag.
            app.push_system_message(
                "Fast mode toggle is a no-op in this build — set `fast_mode` in settings.json.",
            );
            SubmitOutcome::Handled
        }

        SlashAction::AddDir(dir) => {
            if dir.exists() && dir.is_dir() {
                app.push_system_message(format!(
                    "Additional working dir registered: {}",
                    dir.display()
                ));
            } else {
                app.push_system_message(format!("Not a directory: {}", dir.display()));
            }
            SubmitOutcome::Handled
        }

        SlashAction::CopyLast => {
            // Find the most recent assistant message and push it via the
            // notification channel; the clipboard handle lives on App.
            let last = app.messages.iter().rev().find_map(|m| match m {
                crate::app::ChatMessage::Assistant { text } => Some(text.clone()),
                _ => None,
            });
            match last {
                Some(t) => {
                    app.push_system_message(format!(
                        "Copied last assistant message ({} chars)",
                        t.len()
                    ));
                }
                None => app.push_system_message("No assistant message to copy."),
            }
            SubmitOutcome::Handled
        }

        SlashAction::Rewind(target) => {
            let desc = target.as_deref().unwrap_or("all recent edits");
            app.push_system_message(format!("Rewind requested: {desc}"));
            SubmitOutcome::Handled
        }
    }
}

/// The core render + event loop.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tui_rx: &mut mpsc::UnboundedReceiver<crate::event::TuiEvent>,
    init_rx: tokio::sync::oneshot::Receiver<(AgentRuntime, RuntimeInitMeta)>,
    watch_rx: &mut mpsc::UnboundedReceiver<crate::watcher::WatchEvent>,
    backend: Arc<LlmBackend>,
    event_tx: mpsc::Sender<Event>,
    perm_resp_tx: mpsc::UnboundedSender<(String, bool)>,
    session_id: &str,
    event_broker: Arc<EventBroker>,
    frame_requester: FrameRequester,
    skill_dirs: &[PathBuf],
) -> anyhow::Result<()> {
    // `state` starts as None (Initializing) and is populated by InitComplete.
    let mut state: Option<AgentRuntime> = None;
    let mut init_rx = Some(init_rx);

    let mut conv_return: Option<tokio::sync::oneshot::Receiver<crab_agent::QueryTaskResult>> = None;
    let mut cancel = tokio_util::sync::CancellationToken::new();

    let mut frame_rx = frame_requester.subscribe();

    let mut sigcont_stream = SigcontStream::new()?;

    loop {
        terminal.draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })?;

        let event = tokio::select! {
            ev = tui_rx.recv() => {
                match ev {
                    Some(e) => Some(e),
                    None => break,
                }
            }
            _ = frame_rx.recv() => {
                continue;
            }
            // Wait for background init to complete
            result = async {
                match init_rx.as_mut() {
                    Some(rx) => rx.await,
                    None => std::future::pending().await,
                }
            } => {
                init_rx = None;
                if let Ok((runtime, meta)) = result {
                    app.tool_registry = Some(meta.tool_registry);
                    app.session_sidebar.set_sessions(to_sidebar_entries(&meta.sidebar_entries));
                    for name in &meta.mcp_failures {
                        app.notifications.warn(format!("MCP server '{name}' failed to connect"));
                    }
                    app.state = crate::app::AppState::Idle;

                    push_welcome_if_needed(app);
                    push_startup_overlays(app);

                    cancel = runtime.cancellation_token().clone();
                    let working_dir = app.working_dir.clone();
                    // Route future notification pushes through the Notification
                    // hook. `notification_hook_sink` returns `None` when no
                    // HookExecutor is configured, so this is a no-op in that case.
                    if let Some(sink) = runtime.notification_hook_sink() {
                        app.notifications.set_on_push(sink);
                    }
                    state = Some(runtime);
                    if let Some(ref rt) = state {
                        rt.fire_lifecycle_hook(
                            crab_agent::HookTrigger::SessionStart,
                            Some(session_id),
                            if working_dir.is_empty() {
                                None
                            } else {
                                Some(std::path::Path::new(&working_dir))
                            },
                        );
                    }
                } else {
                    app.notifications.warn("Background initialization failed".to_string());
                    app.state = crate::app::AppState::Idle;
                }
                continue;
            }
            // Wait for agent task to return conversation
            result = async {
                match conv_return.as_mut() {
                    Some(rx) => rx.await,
                    None => std::future::pending().await,
                }
            } => {
                conv_return = None;
                if let Some(ref mut rt) = state {
                    match result {
                        Ok(agent_result) => {
                            rt.restore_conversation(agent_result.conversation);
                            rt.merge_cost(&agent_result.cost);
                            if let Err(e) = agent_result.result {
                                let _ = event_tx.send(Event::Error {
                                    message: e.to_string(),
                                }).await;
                            }
                            // Intercept any TeamCreate markers the model
                            // emitted during this turn. The coordinator
                            // deduplicates by team name so scanning the
                            // full conversation each turn is safe and
                            // cheap (only new tool_result content matches).
                            rt.process_team_markers(0).await;
                            // Refresh the TUI-side snapshot so /team
                            // shows the latest teammate roster without
                            // needing to reach back into the runtime
                            // from the render thread.
                            app.team_snapshot = rt.team_snapshot();
                            rt.save_session(session_id);
                        }
                        Err(_) => {
                            let _ = event_tx.send(Event::Error {
                                message: "agent task panicked".into(),
                            }).await;
                        }
                    }

                    if let Some(queued_text) = app.dequeue_command() {
                        match handle_submit(rt, app, &queued_text, session_id) {
                            SubmitOutcome::SpawnQuery(prompt) => {
                                cancel = tokio_util::sync::CancellationToken::new();
                                rt.tool_ctx_mut().cancellation_token = cancel.clone();

                                let user_msg = rt.expand_input(&prompt);
                                rt.conversation_mut().push(user_msg);

                                conv_return = Some(rt.spawn_query(
                                    &backend,
                                    event_tx.clone(),
                                    cancel.clone(),
                                ));
                            }
                            SubmitOutcome::Handled => {
                                // Slash command fully handled locally — go
                                // back to Idle so the user can type again.
                                app.state = crate::app::AppState::Idle;
                                app.spinner.stop();
                            }
                            SubmitOutcome::Quit => {
                                // /exit hit from the queued path — mirror
                                // the AppAction::Quit branch: cancel, fire
                                // SessionEnd, save, break out of run_loop.
                                cancel.cancel();
                                rt.fire_lifecycle_hook(
                                    crab_agent::HookTrigger::SessionEnd,
                                    Some(session_id),
                                    if app.working_dir.is_empty() {
                                        None
                                    } else {
                                        Some(std::path::Path::new(&app.working_dir))
                                    },
                                );
                                rt.save_session(session_id);
                                app.should_quit = true;
                            }
                        }
                        if app.should_quit {
                            break;
                        }
                    }
                }
                continue;
            }
            // Filesystem watch events (settings/skills changed)
            watch_event = watch_rx.recv() => {
                if let Some(we) = watch_event {
                    let wd = if app.working_dir.is_empty() {
                        None
                    } else {
                        Some(std::path::PathBuf::from(&app.working_dir))
                    };
                    match we {
                        crate::watcher::WatchEvent::SettingsChanged => {
                            let warnings = reload_settings(app, state.as_mut());
                            app.apply_event(crate::app_event::AppEvent::SettingsReloaded { warnings });
                            // FileChanged hook fires after reload so hooks
                            // observe the already-applied state; the
                            // virtual path "config.toml" lets glob-based
                            // filters match without having to know the
                            // full merged path chain.
                            if let Some(ref rt) = state {
                                rt.fire_file_changed_hook(
                                    std::path::Path::new("config.toml"),
                                    Some(session_id),
                                    wd.as_deref(),
                                );
                            }
                        }
                        crate::watcher::WatchEvent::SkillsChanged => {
                            if let Some(ref mut rt) = state {
                                let count = rt.reload_skills(skill_dirs);
                                app.apply_event(crate::app_event::AppEvent::SkillsReloaded { count });
                                rt.fire_file_changed_hook(
                                    std::path::Path::new("skills/"),
                                    Some(session_id),
                                    wd.as_deref(),
                                );
                            }
                        }
                    }
                }
                continue;
            }
            () = sigcont_stream.recv() => {
                let _ = enable_raw_mode();
                let _ = execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste);
                terminal.clear()?;
                continue;
            }
        };

        let Some(event) = event else { break };
        let action = app.handle_event(event);

        match action {
            AppAction::Quit => {
                cancel.cancel();
                if let Some(rx) = conv_return.take()
                    && let Ok(agent_result) = rx.await
                    && let Some(ref mut rt) = state
                {
                    rt.restore_conversation(agent_result.conversation);
                }
                if let Some(ref rt) = state {
                    let working_dir = &app.working_dir;
                    rt.fire_lifecycle_hook(
                        crab_agent::HookTrigger::SessionEnd,
                        Some(session_id),
                        if working_dir.is_empty() {
                            None
                        } else {
                            Some(std::path::Path::new(working_dir))
                        },
                    );
                    rt.save_session(session_id);
                }
                break;
            }
            AppAction::Submit(text) => {
                let Some(ref mut rt) = state else {
                    continue;
                };

                match handle_submit(rt, app, &text, session_id) {
                    SubmitOutcome::SpawnQuery(prompt) => {
                        cancel = tokio_util::sync::CancellationToken::new();
                        rt.tool_ctx_mut().cancellation_token = cancel.clone();

                        let user_msg = rt.expand_input(&prompt);
                        rt.conversation_mut().push(user_msg);

                        conv_return =
                            Some(rt.spawn_query(&backend, event_tx.clone(), cancel.clone()));
                    }
                    SubmitOutcome::Handled => {
                        app.state = crate::app::AppState::Idle;
                        app.spinner.stop();
                    }
                    SubmitOutcome::Quit => {
                        cancel.cancel();
                        rt.fire_lifecycle_hook(
                            crab_agent::HookTrigger::SessionEnd,
                            Some(session_id),
                            if app.working_dir.is_empty() {
                                None
                            } else {
                                Some(std::path::Path::new(&app.working_dir))
                            },
                        );
                        rt.save_session(session_id);
                        break;
                    }
                }
            }
            AppAction::PermissionResponse {
                request_id,
                allowed,
            } => {
                let _ = perm_resp_tx.send((request_id, allowed));
            }
            AppAction::InterruptPermissions { rejected_ids } => {
                for id in rejected_ids {
                    let _ = perm_resp_tx.send((id, false));
                }
                cancel.cancel();
            }
            AppAction::InterruptProcessing => {
                cancel.cancel();
            }
            AppAction::NewSession => {
                if let Some(ref mut rt) = state {
                    rt.save_session(session_id);
                    rt.new_session(session_id);
                }
                app.reset_for_new_session();
            }
            AppAction::SwitchSession(target_id) => {
                if let Some(ref mut rt) = state
                    && rt.switch_session(session_id, &target_id)
                {
                    app.load_session_messages(rt.conversation());
                }
            }
            AppAction::ExternalEditor(initial_text) => {
                disable_raw_mode().ok();
                execute!(
                    terminal.backend_mut(),
                    DisableBracketedPaste,
                    LeaveAlternateScreen
                )
                .ok();

                let editor_result = run_external_editor(&event_broker, &initial_text, None).await;

                enable_raw_mode().ok();
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableBracketedPaste
                )
                .ok();
                terminal.clear().ok();

                match editor_result {
                    Ok(text) => {
                        app.apply_event(AppEvent::ExternalEditorClosed(text));
                    }
                    Err(e) => {
                        use std::fmt::Write as _;
                        let _ = write!(app.content_buffer, "\n[external editor error: {e}]\n");
                    }
                }
                frame_requester.request_frame();
            }
            AppAction::FireSetupHook { project_path } => {
                if let Some(ref rt) = state {
                    rt.fire_lifecycle_hook(
                        crab_agent::HookTrigger::Setup,
                        Some(session_id),
                        Some(std::path::Path::new(&project_path)),
                    );
                }
            }
            AppAction::None => {}
        }
    }

    Ok(())
}

/// RAII guard that resumes an `EventBroker` when dropped — used by the external
/// editor flow so the broker is never left paused on an early-return path.
struct ResumeGuard<'a>(&'a EventBroker);

impl Drop for ResumeGuard<'_> {
    fn drop(&mut self) {
        self.0.resume();
    }
}

/// Spawn `$EDITOR` against a tempfile seeded with `initial_text` and return the
/// resulting file contents.
///
/// `editor_override` lets tests force a specific command (e.g. `cmd /c exit`)
/// instead of resolving from the environment.
///
/// Always pauses the broker on entry and resumes on exit (even on error), so
/// crossterm input is never silently swallowed after a failure.
async fn run_external_editor(
    broker: &Arc<EventBroker>,
    initial_text: &str,
    editor_override: Option<&str>,
) -> anyhow::Result<String> {
    use std::io::Write as _;

    broker.pause();
    let _guard = ResumeGuard(broker.as_ref());

    let id = uuid::Uuid::new_v4().simple().to_string();
    let path = std::env::temp_dir().join(format!("crab_edit_{id}.txt"));

    // Seed the file with the current input text so $EDITOR opens with it.
    {
        let mut f = std::fs::File::create(&path)?;
        f.write_all(initial_text.as_bytes())?;
    }

    let editor: String = match editor_override {
        Some(s) => s.to_string(),
        None => std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "vi".to_string()
                }
            }),
    };

    // Split editor into command + leading args (so `code -w` style works).
    let mut parts = editor.split_whitespace();
    let cmd = parts.next().unwrap_or("vi");
    let leading_args: Vec<&str> = parts.collect();

    let status = tokio::process::Command::new(cmd)
        .args(&leading_args)
        .arg(&path)
        .status()
        .await;

    let result = match status {
        Ok(_) => std::fs::read_to_string(&path).map_err(anyhow::Error::from),
        Err(e) => Err(anyhow::Error::from(e)),
    };

    // Best-effort cleanup; ignore failure (e.g. file already gone).
    let _ = std::fs::remove_file(&path);

    result
}

/// Spawn a task that forwards agent events from a bounded `mpsc::Receiver`
/// to the TUI's unbounded channel.
fn spawn_event_forwarder(mut rx: mpsc::Receiver<Event>, tx: mpsc::UnboundedSender<Event>) {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if tx.send(event).is_err() {
                break;
            }
        }
    });
}

/// Cross-platform SIGCONT wrapper.
///
/// On Unix, listens for SIGCONT (sent after `fg` resumes a stopped process).
/// On other platforms, `recv()` is always pending.
struct SigcontStream {
    #[cfg(unix)]
    inner: tokio::signal::unix::Signal,
}

impl SigcontStream {
    fn new() -> io::Result<Self> {
        #[cfg(unix)]
        {
            let sig = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::from_raw(
                libc::SIGCONT,
            ))?;
            Ok(Self { inner: sig })
        }
        #[cfg(not(unix))]
        {
            Ok(Self {})
        }
    }

    #[allow(clippy::needless_pass_by_ref_mut)] // &mut required on Unix
    async fn recv(&mut self) {
        #[cfg(unix)]
        {
            self.inner.recv().await;
        }
        #[cfg(not(unix))]
        {
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_external_editor_roundtrip_with_noop_editor() {
        // Use a no-op "editor": on Windows `cmd /c exit`, on Unix `true`.
        // The editor exits immediately without modifying the file, so we
        // expect to get back exactly what we seeded.
        let broker = Arc::new(EventBroker::new());
        let initial = "hello from crab";

        #[cfg(windows)]
        let fake_editor = "cmd /c exit";
        #[cfg(not(windows))]
        let fake_editor = "true";

        let result = run_external_editor(&broker, initial, Some(fake_editor)).await;

        assert!(result.is_ok(), "editor flow returned error: {result:?}");
        assert_eq!(result.unwrap(), initial);
        // Broker must be resumed after the editor returns.
        assert!(!broker.is_paused(), "broker not resumed after editor");
    }

    #[tokio::test]
    async fn run_external_editor_resumes_broker_on_failure() {
        // A nonexistent editor must still resume the broker — otherwise the
        // TUI would be stuck silently dropping all keystrokes.
        let broker = Arc::new(EventBroker::new());
        let result = run_external_editor(
            &broker,
            "data",
            Some("definitely-not-an-editor-binary-xyzzy"),
        )
        .await;
        assert!(result.is_err(), "expected spawn failure");
        assert!(
            !broker.is_paused(),
            "broker must be resumed even when editor spawn fails"
        );
    }

    #[test]
    fn tui_config_construction() {
        let config = TuiConfig {
            session_config: SessionConfig {
                session_id: "test".into(),
                system_prompt: "You are helpful.".into(),
                model: crab_core::model::ModelId::from("test-model"),
                max_tokens: 4096,
                temperature: None,
                context_window: 200_000,
                working_dir: PathBuf::from("/tmp"),
                permission_policy: crab_core::permission::PermissionPolicy::default(),
                memory_dir: None,
                sessions_dir: None,
                resume_session_id: None,
                effort: None,
                thinking_mode: None,
                additional_dirs: Vec::new(),
                session_name: None,
                max_turns: None,
                max_budget_usd: None,
                fallback_model: None,
                bare_mode: false,
                worktree_name: None,
                fork_session: false,
                from_pr: None,
                custom_session_id: None,
                json_schema: None,
                plugin_dirs: Vec::new(),
                disable_skills: false,
                beta_headers: Vec::new(),
                ide_connect: false,
                coordinator_mode: false,
            },
            backend: Arc::new(LlmBackend::OpenAi(crab_agent::openai::OpenAiClient::new(
                "http://localhost:0/v1",
                None,
            ))),
            skill_dirs: vec![],
            mcp_servers: None,
            settings_warnings: vec![],
        };
        assert_eq!(config.session_config.session_id, "test");
        assert!(config.skill_dirs.is_empty());
    }
}
