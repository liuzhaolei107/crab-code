<div align="center">

<img src="assets/logo-horizontal.svg" width="360" alt="Crab Code" />

**Open-source alternative to Claude Code, built from scratch in Rust.**

[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml/badge.svg)](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](#contributing)

**English** | [**中文**](README.zh-CN.md)

</div>

---

> **Active Development** — 4500+ tests · 24 crates · ~142k LOC

Crab Code is a Rust-native agentic coding CLI. It aligns with Claude Code's toolset, permission model, and interaction patterns while supporting any LLM provider (Anthropic / OpenAI / DeepSeek / Ollama / Bedrock / Vertex, etc.).

## Quick Start

```bash
git clone https://github.com/lingcoder/crab-code.git && cd crab-code
cargo build --release
export ANTHROPIC_API_KEY=sk-ant-...

./target/release/crab                   # Interactive TUI
./target/release/crab "explain this codebase"   # Single-shot
./target/release/crab -p "fix the bug"  # Non-interactive
```

See `crab --help` for more. Config: `~/.crab/settings.json`

## Environment Variables

Priority: env vars override `settings.json`, which overrides `config.toml` defaults.

**Provider / API**
| Variable | Purpose |
|----------|---------|
| `CRAB_API_PROVIDER` | Override LLM provider (`anthropic`, `openai`, `deepseek`, …) |
| `CRAB_API_KEY` | Override API key (takes priority over provider-specific keys) |
| `CRAB_MODEL` | Override model name |
| `CRAB_API_BASE_URL` | Override base URL (for OpenAI-compatible endpoints) |
| `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` / `DEEPSEEK_API_KEY` | Provider-specific fallbacks when `CRAB_API_KEY` is unset |

**Shell / Tools**
| Variable | Purpose |
|----------|---------|
| `CRAB_CODE_SHELL` | Path to bash/zsh binary (overrides auto-detection for the Bash tool) |
| `CRAB_CODE_USE_POWERSHELL_TOOL` | Truthy value exposes the `PowerShell` tool on Windows (default: off) |

**Agent Behavior**
| Variable | Purpose |
|----------|---------|
| `CRAB_COORDINATOR_MODE` | `1` enables Agent Teams coordinator mode (multi-agent orchestration) |
| `CRAB_AUTO_DREAM` | `1` enables background memory consolidation between sessions |
| `CRAB_AUTO_DREAM_MIN_HOURS` | Minimum hours between consolidations (default: 6) |
| `CRAB_AUTO_DREAM_MIN_SESSIONS` | Minimum sessions before consolidation triggers (default: 2) |

**TLS / Networking**
| Variable | Purpose |
|----------|---------|
| `CRAB_CA_BUNDLE` | Path to a custom CA certificate bundle (PEM) |
| `SSL_CERT_FILE` / `SSL_CERT_DIR` | Standard OpenSSL CA overrides |

**Vertex AI (when provider=vertex)**
| Variable | Purpose |
|----------|---------|
| `GOOGLE_CLOUD_PROJECT` / `GCLOUD_PROJECT` | GCP project ID |
| `GOOGLE_CLOUD_REGION` | GCP region (default: `us-central1`) |
| `GOOGLE_APPLICATION_CREDENTIALS` | Path to service account key JSON |

## Comparison

| | Crab Code | Claude Code | OpenCode| Codex CLI |
|--|-----------|-------------|---------|-----------|
| Open Source | Apache 2.0 | Proprietary | MIT | Apache 2.0 |
| Language | Rust | TypeScript | TypeScript | Rust |
| Models | Any provider | Anthropic | Any provider | OpenAI only |
| MCP | 3 transports | 6 transports | LSP | 2 transports |

## Architecture

24 Rust crates in 4 layers. See [`docs/architecture.md`](docs/architecture.md) for details.

```
Entry     cli · daemon
Engine    agent · engine · session · tui · remote
Service   api · tools · mcp · skill · plugin · telemetry · ide · sandbox · job · acp
Foundation core · common · config · auth · fs · memory · process
```

## Build & Test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Contributing

PRs welcome.

## License

[Apache License 2.0](LICENSE)

