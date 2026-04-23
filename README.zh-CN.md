<div align="center">

<img src="assets/logo-horizontal.svg" width="360" alt="Crab Code" />

**Claude Code 的开源替代品，完全用 Rust 从零构建。**

[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml/badge.svg)](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](#贡献)

[**English**](README.md) | **中文**

</div>

---

> **积极开发中** — 4500+ 测试 · 24 crate · ~142k LOC

Crab Code 是一个 Rust 原生的 Agentic Coding CLI。它对齐 Claude Code 的工具集、权限模型和交互方式，同时支持任意 LLM 提供商（Anthropic / OpenAI / DeepSeek / Ollama / Bedrock / Vertex 等）。

## 快速开始

```bash
git clone https://github.com/lingcoder/crab-code.git && cd crab-code
cargo build --release
export ANTHROPIC_API_KEY=sk-ant-...

./target/release/crab                   # 交互式 TUI
./target/release/crab "解释这段代码"      # 单次模式
./target/release/crab -p "修复 bug"      # 非交互
```

更多用法见 `crab --help`。配置文件：`~/.crab/settings.json`

## 环境变量

优先级：环境变量覆盖 `settings.json`，后者覆盖 `config.toml` 默认值。

| 分类 | 变量 | 用途 |
|------|------|------|
| Provider | `CRAB_API_PROVIDER` | 覆盖 provider：`anthropic`、`openai`、`deepseek`、`ollama`、`vllm`、`bedrock`、`vertex` |
| Provider | `CRAB_API_KEY` | 统一 API key（优先级高于下方 provider 专用 key） |
| Provider | `CRAB_MODEL` | 覆盖模型名 |
| Provider | `CRAB_API_BASE_URL` | 覆盖 base URL（用于 OpenAI 兼容端点） |
| Provider | `ANTHROPIC_API_KEY` | Anthropic key（`CRAB_API_KEY` 未设置时使用） |
| Provider | `OPENAI_API_KEY` | OpenAI / Ollama / vLLM / DeepSeek 兼容端点 key |
| Provider | `DEEPSEEK_API_KEY` | DeepSeek key |
| Bedrock | `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | 静态凭证 |
| Bedrock | `AWS_SESSION_TOKEN` | 可选 session token（用于临时凭证） |
| Bedrock | `AWS_REGION` / `AWS_DEFAULT_REGION` | AWS 区域 |
| Bedrock | `AWS_ROLE_ARN` | 要扮演的 IAM role ARN |
| Bedrock | `AWS_WEB_IDENTITY_TOKEN_FILE` | OIDC token 文件（web-identity 扮演角色） |
| Bedrock | `AWS_EXTERNAL_ID` | 跨账户扮演角色的 External ID |
| Bedrock | `AWS_ROLE_SESSION_NAME` | 扮演角色时的 session 名 |
| Vertex | `GOOGLE_CLOUD_PROJECT` / `GCLOUD_PROJECT` | GCP project ID |
| Vertex | `GOOGLE_CLOUD_REGION` | GCP 区域（默认 `us-central1`） |
| Vertex | `GOOGLE_APPLICATION_CREDENTIALS` | Service account key JSON 路径 |
| Shell | `CRAB_SHELL` | bash/zsh 路径（覆盖 Bash 工具的自动探测） |
| Shell | `SHELL` | POSIX 标准变量，`CRAB_SHELL` 未设置时作为 fallback |
| Shell | `CRAB_USE_POWERSHELL_TOOL` | 真值在 Windows 上启用 `PowerShell` 工具（默认关闭） |
| Agent | `CRAB_COORDINATOR_MODE` | `1` 启用 Agent Teams 协调模式 |
| Agent | `CRAB_AUTO_DREAM` | `1` 启用会话间后台记忆整理 |
| Agent | `CRAB_AUTO_DREAM_MIN_HOURS` | 两次整理的最小间隔小时数（默认 6） |
| Agent | `CRAB_AUTO_DREAM_MIN_SESSIONS` | 触发整理的最小会话数（默认 2） |
| TLS | `CRAB_CA_BUNDLE` | 自定义 CA 证书 bundle 路径（PEM 格式） |
| TLS | `SSL_CERT_FILE` / `SSL_CERT_DIR` | 标准 OpenSSL CA 覆盖 |

## 对比

| | Crab Code | Claude Code | OpenCode| Codex CLI |
|--|-----------|-------------|----------|-----------|
| 开源 | Apache 2.0 | 闭源 | MIT | Apache 2.0 |
| 语言 | Rust | TypeScript | TypeScript | Rust |
| 模型 | 任意 | Anthropic | 任意 | 仅 OpenAI |
| MCP | 3 传输 | 6 传输 | LSP | 2 传输 |

## 架构

24 个 Rust crate，4 层依赖。详见 [`docs/architecture.md`](docs/architecture.md)。

```
入口    cli · daemon
编排    agent · engine · session · tui · remote
服务    api · tools · mcp · skill · plugin · telemetry · ide · sandbox · job · acp
基础    core · common · config · auth · fs · memory · process
```

## 构建与测试

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## 贡献

欢迎 PR。

## 许可证

[Apache License 2.0](LICENSE)

