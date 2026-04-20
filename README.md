<div align="center">

<img src="assets/logo-horizontal.svg" width="360" alt="Crab Code" />

**Open-source alternative to Claude Code, built from scratch in Rust.**

*Inspired by Claude Code's agentic workflow -- open source, Rust-native, works with any LLM.*

[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml/badge.svg)](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](#contributing)

**English** | [**中文**](README.zh-CN.md)

</div>

---

> **Status: Active Development** -- 46 built-in tools · 33 slash commands · 6 permission modes · three-layer multi-agent architecture · file-history rewinding · extended thinking · TUI with Markdown tables, toast notifications, OSC 8 hyperlinks, and Vim editing · 4500+ tests · 24 crates · ~140k LOC.

## What is Crab Code?

[Claude Code](https://docs.anthropic.com/en/docs/claude-code) pioneered the agentic coding CLI -- an AI that doesn't just suggest code, but thinks, plans, and executes autonomously in your terminal.

**Crab Code** brings that experience to the open-source world, independently built from the ground up in Rust:

- **Fully open source** -- Apache 2.0, no feature-gating, no black box
- **Rust-native performance** -- instant startup, minimal memory, no Node.js overhead
- **Model agnostic** -- Claude, GPT, DeepSeek, Qwen, Ollama, or any OpenAI-compatible API
- **Secure** -- 6 permission modes + tool-level allow/deny lists
- **MCP compatible** -- stdio, SSE, and WebSocket transports
- **Claude Code aligned** -- CLI flags, slash commands, tools, and workflows match Claude Code behavior

## Quick Start

```bash
git clone https://github.com/lingcoder/crab-code.git
cd crab-code
cargo build --release

# Set your API key
export ANTHROPIC_API_KEY=sk-ant-...

# Interactive TUI mode
./target/release/crab

# Single-shot mode
./target/release/crab "explain this codebase"

# Print mode (non-interactive, reads from stdin if no prompt given)
./target/release/crab -p "fix the bug in main.rs"

# With a specific provider
./target/release/crab --provider openai --model gpt-4o "refactor auth module"
```

### Claude Code Compatible Configuration

Crab Code reads Claude Code's `settings.json` format, including the `env` field:

```json
// ~/.crab/settings.json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "cr_...",
    "ANTHROPIC_BASE_URL": "http://your-proxy/api"
  },
  "model": "claude-opus-4-6"
}
```

## Features

### Core Agent Loop

- **Streaming SSE** -- real-time token-by-token output from LLM APIs
- **Tool execution cycle** -- model reasoning → tool call → permission check → execute → result → next turn
- **Extended thinking** -- `budget_tokens` support for deep reasoning (Anthropic thinking blocks)
- **Retry + fallback** -- automatic retry on transient errors, model fallback on overload
- **Effort levels** -- low / medium / high / max mapped to API parameters
- **Auto-compaction** -- heuristic summarizer triggers at 80% context-window watermark; `/compact` to invoke manually

### Built-in Tools (46)

| Category | Tools |
|----------|-------|
| File I/O | `Read`, `Write`, `Edit`, `Glob`, `Grep`, `ImageRead` |
| Execution | `Bash`, `PowerShell` (Windows) |
| Sub-agents | `Agent`, `TeamCreate`, `TeamDelete`, `SendMessage` |
| Tasks | `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, `TaskStop`, `TaskOutput`, `TodoWrite` |
| Scheduling | `CronCreate`, `CronDelete`, `CronList` |
| Planning | `EnterPlanMode`, `ExitPlanMode`, `VerifyPlanExecution` |
| Notebook | `NotebookEdit`, `NotebookRead` |
| Web | `WebFetch`, `WebSearch`, `WebBrowser` |
| LSP | `LSP` (definition, references, hover, symbols) |
| Worktree | `EnterWorktree`, `ExitWorktree` |
| MCP | `ListMcpResources`, `ReadMcpResource`, `McpAuth` |
| Files | `SendUserFile` |
| Other | `AskUserQuestion`, `Skill`, `Sleep`, `Snip`, `Brief`, `Config`, `ToolSearch`, `Monitor`, `RemoteTrigger` |

### Multi-Agent Architecture

Three conceptually distinct layers in `crates/agent/`:

- **Teams** — base infrastructure: `WorkerPool`, per-agent mailbox router, shared `TaskList` (`fd-lock` file-locked `claim_task` for cross-process teammates), spawner backends (in-process, tmux).
- **Swarm** — the default flat topology (`TeamMode::PeerToPeer`). No extra module — it's just how teammates cooperate when no overlay is active.
- **Coordinator Mode** — a star-topology overlay gated on `CRAB_COORDINATOR_MODE=1`. The coordinator's tool registry is reduced to `{Agent, SendMessage, TaskStop}`; workers it spawns lose `{TeamCreate, TeamDelete, SendMessage}`; the coordinator's system prompt carries an anti-pattern guardrail.

### File History & Rewind

Every session snapshots file edits under `~/.crab/file-history/{session_id}/` (at most 100 per session, LRU-evicted). `/rewind [path]` restores a file to an earlier version. Tool-level `track_edit` hooks on `Edit` / `Write` / `Notebook` are a planned follow-up; the `file_history` primitive is already unit-tested.

### Permission System

6 modes aligned with Claude Code behaviour:

- **default** — prompt for potentially dangerous operations
- **acceptEdits** — auto-approve file edits, still prompt for Bash
- **trust-project** — auto-approve in-project writes; out-of-project and dangerous ops still prompt
- **dontAsk** — auto-approve everything (no prompts)
- **dangerously** — auto-approve everything except `denied_tools` (use with care)
- **plan** — read-only planning mode; mutations require explicit plan approval

Plus tool-level filtering via `--allowedTools` / `--disallowedTools` with glob patterns (`Bash(git:*)`, `mcp__*`, `Edit`).

### Slash Commands (33)

The REPL intercepts input matching `/<letter>…` and dispatches through `SlashCommandRegistry` before the prompt reaches the LLM, so paths like `/tmp/foo` still pass through as prompts.

`/help` `/clear` `/compact` `/cost` `/status` `/memory` `/init` `/model` `/config` `/permissions` `/resume` `/history` `/export` `/doctor` `/diff` `/review` `/plan` `/exit` `/fast` `/effort` `/add-dir` `/files` `/thinking` `/rewind` `/skills` `/plugin` `/mcp` `/branch` `/commit` `/theme` `/keybindings` `/copy` `/rename`

### LLM Providers

- **Anthropic** — Messages API with SSE streaming (default `claude-sonnet-4-6`)
- **OpenAI-compatible** — Chat Completions API (GPT, DeepSeek, Qwen, Ollama, vLLM, …)
- **AWS Bedrock** — SigV4 signing with inference profile discovery
- **GCP Vertex AI** — Application Default Credentials

### MCP (Model Context Protocol)

- stdio, SSE, and WebSocket transports
- `McpToolAdapter` bridges MCP tools to the native `Tool` trait
- Configure via `~/.crab/settings.json` or `--mcp-config`

### Session Management

- Auto-save conversation history to `~/.crab/sessions/`
- `--continue` / `-c` resumes the last session
- `--resume <id>` resumes a specific session
- `--fork-session` branches on resume instead of continuing in-place
- `--name` friendly session names
- Per-session file-history snapshots for `/rewind`

### Hook System

- `PreToolUse` / `PostToolUse` / `UserPromptSubmit` triggers
- Shell command execution with `Allow` / `Deny` / `Modify` responses
- Configure in `settings.json`

### Interactive TUI

- `ratatui` + `crossterm` terminal UI, immediate-mode rendering
- Markdown rendering with syntax highlighting and GFM table support (responsive horizontal/vertical layout)
- OSC 8 clickable hyperlinks in Markdown links and images (with plain-text fallback)
- Toast notification queue with key-based dedup and auto-expiry
- OS-level terminal notifications via OSC 9/99/777 (iTerm2, Kitty, WezTerm, Ghostty)
- Vim-mode editing (normal, insert, visual, with motions, operators, and registers)
- Autocomplete for slash commands and file paths
- Session sidebar with tabbed Recent/Saved views
- Permission dialogs with allow/deny/always actions
- Cost tracking status bar
- Dark/light theme auto-detection via OSC 11

## Architecture

24 Rust crates organised in 4 layers:

```
Layer 4 (Entry)     cli          daemon
                      |             |
Layer 3 (Orch)     agent      engine      session
                      |          |           |
Layer 2 (Service)  api  tools  mcp  tui  skill  plugin  telemetry  acp  ide  job  remote  sandbox
                      |    |     |    |     |       |         |
Layer 1 (Found)    common   core   config   auth   fs   memory   process
```

Key design decisions:
- **Async runtime** — tokio (multi-threaded)
- **LLM dispatch** — `enum LlmBackend`; zero dynamic dispatch, exhaustive match
- **Tool system** — `trait Tool` + `ToolRegistry` + `ToolExecutor`, discovered via JSON Schema
- **TUI** — `ratatui` + `crossterm`, immediate-mode
- **Error handling** — `thiserror` for libraries, `anyhow` for the application

> Full architecture details: [`docs/architecture.md`](docs/architecture.md)

## Configuration

```bash
# Global
~/.crab/settings.json        # API keys, provider settings, MCP servers
~/.crab/memory/              # Persistent memory files
~/.crab/sessions/            # Saved conversation sessions
~/.crab/file-history/        # Per-session pre-edit snapshots
~/.crab/skills/              # Global skill definitions

# Project
your-project/CRAB.md                # Project instructions (like CLAUDE.md)
your-project/.crab/settings.json    # Project-level overrides
your-project/.crab/skills/          # Project-specific skills
```

## CLI Usage

```bash
crab                              # Interactive TUI mode
crab "your prompt"                # Single-shot mode
crab -p "your prompt"             # Print mode (non-interactive)
crab -c                           # Continue last session
crab --provider openai            # Use an OpenAI-compatible provider
crab --model opus                 # Model alias (sonnet / opus / haiku)
crab --permission-mode plan       # Plan mode
crab --effort high                # Set reasoning effort
crab --resume <session-id>        # Resume a saved session
crab doctor                       # Run diagnostics
crab auth login                   # Configure authentication
```

Insider toggles (no CLI flag, env only — keeps the surface hidden from `--help`):
- `CRAB_COORDINATOR_MODE=1` — enables Coordinator Mode overlay
- `CRAB_AUTO_DREAM=1` — arms the `auto-dream` memory-consolidation gate (requires the `auto-dream` cargo feature)

## Build & Development

```bash
cargo build --workspace                    # Build all
cargo test --workspace                     # Run all tests (4500+)
cargo clippy --workspace -- -D warnings    # Lint (CI treats warnings as errors)
cargo fmt --all --check                    # Check formatting
cargo run --bin crab                       # Run CLI
```

Experimental cargo features (all default off):
- `auto-dream` on `crab-agent` — CCB-aligned memory-consolidation scheduler (runner still stubbed)
- `proactive` on `crab-agent` — placeholder matching CCB's `feature('PROACTIVE')` posture
- `mem-ranker` on `crab-agent` — re-exports `crab-memory/mem-ranker`

## Comparison

| | Crab Code | Claude Code | [OpenCode](https://github.com/anomalyco/opencode) | Codex CLI |
|--|-----------|-------------|----------|-----------|
| Open Source | Apache 2.0 | Proprietary | MIT | Apache 2.0 |
| Language | Rust | TypeScript (Bun) | TypeScript | Rust |
| Model Agnostic | Any provider | Anthropic + AWS/GCP | Any provider | OpenAI only |
| Self-hosted | Yes | No | Yes | Yes |
| MCP Support | stdio + SSE + WS | 6 transports | LSP | 2 transports |
| TUI | ratatui | Ink (React) | Custom | ratatui |
| Built-in Tools | 46 | 52 | ~10 | ~10 |
| Permission Modes | 6 | 6 | 2 | 3 |

## Contributing

Areas where we'd love help:

- Claude Code feature alignment — remaining gaps include the `auto-dream` forked-agent runner, tool-level `track_edit` hooks on Edit / Write / Notebook, and the `proactive` mini-agent
- OS-level sandboxing (Landlock / Seatbelt / Windows Job Object)
- End-to-end integration testing
- Additional LLM provider testing
- Documentation & i18n

## License

[Apache License 2.0](LICENSE)

---

<div align="center">

**Built with Rust by [lingcoder](https://github.com/lingcoder)**

*Claude Code showed us the future of agentic coding. Crab Code makes it open for everyone.*

</div>
