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

> **Active Development** — 3600+ tests · 24 crates · ~144k LOC

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

| Category | Variable | Purpose |
|----------|----------|---------|
| Provider | `CRAB_API_PROVIDER` | Override provider: `anthropic`, `openai`, `deepseek`, `ollama`, `vllm`, `bedrock`, `vertex` |
| Provider | `CRAB_API_KEY` | Unified API key (takes priority over provider-specific keys) |
| Provider | `CRAB_MODEL` | Override model name |
| Provider | `CRAB_API_BASE_URL` | Override base URL (for OpenAI-compatible endpoints) |
| Provider | `ANTHROPIC_API_KEY` | Anthropic key (used when `CRAB_API_KEY` is unset) |
| Provider | `OPENAI_API_KEY` | OpenAI / Ollama / vLLM / DeepSeek-compat key |
| Provider | `DEEPSEEK_API_KEY` | DeepSeek key |
| Bedrock | `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | Static credentials |
| Bedrock | `AWS_SESSION_TOKEN` | Optional session token (temporary credentials) |
| Bedrock | `AWS_REGION` / `AWS_DEFAULT_REGION` | AWS region |
| Bedrock | `AWS_ROLE_ARN` | IAM role ARN to assume |
| Bedrock | `AWS_WEB_IDENTITY_TOKEN_FILE` | OIDC token file (web-identity role assumption) |
| Bedrock | `AWS_EXTERNAL_ID` | External ID for cross-account role assumption |
| Bedrock | `AWS_ROLE_SESSION_NAME` | Session name for assumed role |
| Vertex | `GOOGLE_CLOUD_PROJECT` / `GCLOUD_PROJECT` | GCP project ID |
| Vertex | `GOOGLE_CLOUD_REGION` | GCP region (default: `us-central1`) |
| Vertex | `GOOGLE_APPLICATION_CREDENTIALS` | Path to service account key JSON |
| Shell | `CRAB_SHELL` | Path to bash/zsh binary (overrides auto-detection for the Bash tool) |
| Shell | `SHELL` | POSIX fallback when `CRAB_SHELL` is unset |
| Shell | `CRAB_USE_POWERSHELL_TOOL` | Truthy value exposes the `PowerShell` tool on Windows (default off) |
| Agent | `CRAB_COORDINATOR_MODE` | `1` enables Agent Teams coordinator mode |
| Agent | `CRAB_AUTO_DREAM` | `1` enables background memory consolidation between sessions |
| Agent | `CRAB_AUTO_DREAM_MIN_HOURS` | Minimum hours between consolidations (default: 6) |
| Agent | `CRAB_AUTO_DREAM_MIN_SESSIONS` | Minimum sessions before consolidation triggers (default: 2) |
| TLS | `CRAB_CA_BUNDLE` | Path to custom CA certificate bundle (PEM) |
| TLS | `SSL_CERT_FILE` / `SSL_CERT_DIR` | Standard OpenSSL CA overrides |

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

