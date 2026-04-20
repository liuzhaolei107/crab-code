<div align="center">

<img src="assets/logo-horizontal.svg" width="360" alt="Crab Code" />

**Claude Code 的开源替代品，完全用 Rust 从零构建。**

*灵感源自 Claude Code 的 Agentic 工作流 -- 开源、Rust 原生、任意 LLM 即可。*

[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml/badge.svg)](https://github.com/lingcoder/crab-code/actions/workflows/ci.yml)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](#贡献)

[**English**](README.md) | **中文**

</div>

---

> **状态：积极开发中** -- 46 个内置工具 · 33 条斜杠命令 · 6 种权限模式 · 三层多 Agent 架构 · 文件历史回滚 · 扩展思维 · 支持 Markdown 表格、Toast 通知、OSC 8 超链接和 Vim 编辑的 TUI · 4500+ 测试 · 24 个 crate · ~140k LOC。

## 什么是 Crab Code？

[Claude Code](https://docs.anthropic.com/en/docs/claude-code) 开创了 Agentic Coding CLI —— 不仅能建议代码，还能在终端中自主思考、规划和执行的 AI。

**Crab Code** 将这种体验带到开源世界，完全用 Rust 从零独立构建：

- **完全开源** —— Apache 2.0 许可，无功能限制，无黑盒
- **Rust 原生性能** —— 瞬时启动、极低内存、无 Node.js 开销
- **模型无关** —— Claude、GPT、DeepSeek、Qwen、Ollama 或任何 OpenAI 兼容 API
- **安全可控** —— 6 种权限模式 + 工具级 allow/deny 清单
- **MCP 兼容** —— stdio、SSE、WebSocket 三种传输
- **对齐 Claude Code** —— CLI flag、斜杠命令、工具、工作流均与 Claude Code 一致

## 快速开始

```bash
git clone https://github.com/lingcoder/crab-code.git
cd crab-code
cargo build --release

# 设置 API 密钥
export ANTHROPIC_API_KEY=sk-ant-...

# 交互式 TUI 模式
./target/release/crab

# 单次模式
./target/release/crab "解释这个代码库"

# Print 模式（非交互；无 prompt 时从 stdin 读）
./target/release/crab -p "修复 main.rs 中的 bug"

# 指定提供商
./target/release/crab --provider openai --model gpt-4o "重构 auth 模块"
```

### 兼容 Claude Code 的配置

Crab Code 读取 Claude Code 的 `settings.json` 格式，包括 `env` 字段：

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

## 功能

### 核心 Agent 循环

- **SSE 流式** —— LLM API token-by-token 实时输出
- **工具执行循环** —— 模型推理 → 工具调用 → 权限检查 → 执行 → 结果 → 下一轮
- **扩展思维** —— 支持 `budget_tokens` 深度推理（Anthropic thinking blocks）
- **重试 + fallback** —— 瞬时错误自动重试，主模型过载时切备用
- **Effort 等级** —— low / medium / high / max 映射到 API 参数
- **自动压缩** —— 对话达到上下文窗口 80% 时启发式摘要自动触发；也可 `/compact` 手动触发

### 内置工具（46 个）

| 类别 | 工具 |
|------|------|
| 文件 I/O | `Read`, `Write`, `Edit`, `Glob`, `Grep`, `ImageRead` |
| 执行 | `Bash`, `PowerShell`（Windows） |
| 子 Agent | `Agent`, `TeamCreate`, `TeamDelete`, `SendMessage` |
| 任务 | `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, `TaskStop`, `TaskOutput`, `TodoWrite` |
| 调度 | `CronCreate`, `CronDelete`, `CronList` |
| 规划 | `EnterPlanMode`, `ExitPlanMode`, `VerifyPlanExecution` |
| Notebook | `NotebookEdit`, `NotebookRead` |
| Web | `WebFetch`, `WebSearch`, `WebBrowser` |
| LSP | `LSP`（定义 / 引用 / hover / symbols） |
| Worktree | `EnterWorktree`, `ExitWorktree` |
| MCP | `ListMcpResources`, `ReadMcpResource`, `McpAuth` |
| 文件传输 | `SendUserFile` |
| 其他 | `AskUserQuestion`, `Skill`, `Sleep`, `Snip`, `Brief`, `Config`, `ToolSearch`, `Monitor`, `RemoteTrigger` |

### 多 Agent 架构

`crates/agent/` 下按三个语义层拆分：

- **Teams** —— 基础设施：`WorkerPool`、每 Agent 一个邮箱 router、共享 `TaskList`（通过 `fd-lock` 支持跨进程 `claim_task`）、Spawner 后端（in-process、tmux）。
- **Swarm** —— 默认扁平拓扑（`TeamMode::PeerToPeer`）。不是独立模块，就是"未启用叠加层时 teammate 的默认协作方式"。
- **Coordinator Mode** —— `CRAB_COORDINATOR_MODE=1` 门控的星型叠加层：Coordinator 的工具仅剩 `{Agent, SendMessage, TaskStop}`；它派出的 Worker 失去 `{TeamCreate, TeamDelete, SendMessage}`；Coordinator 的 system prompt 追加反模式禁令段。

### 文件历史与回滚

每个会话在 `~/.crab/file-history/{session_id}/` 保存自己的快照（每会话上限 100，LRU 淘汰）。`/rewind [path]` 恢复文件到指定版本。Edit / Write / Notebook 工具的 `track_edit` 钩子是后续计划；`file_history` 原语本身已单元测试覆盖。

### 权限系统

6 种模式对齐 Claude Code 行为：

- **default** —— 潜在危险操作需要确认
- **acceptEdits** —— 自动批准文件编辑，Bash 仍要求确认
- **trust-project** —— 项目内写入自动批准，项目外 / 危险操作仍要求确认
- **dontAsk** —— 所有操作自动批准（无提示）
- **dangerously** —— 除 `denied_tools` 外一切自动批准（谨慎使用）
- **plan** —— 只读规划模式，变更需显式 plan 批准

另外支持工具级过滤 `--allowedTools` / `--disallowedTools`（glob 模式：`Bash(git:*)`、`mcp__*`、`Edit`）。

### 斜杠命令（33）

REPL 遇到 `/<字母>…` 前缀就走 `SlashCommandRegistry` 分发，不发给 LLM；`/tmp/foo` 这种路径仍按普通输入透传。

`/help` `/clear` `/compact` `/cost` `/status` `/memory` `/init` `/model` `/config` `/permissions` `/resume` `/history` `/export` `/doctor` `/diff` `/review` `/plan` `/exit` `/fast` `/effort` `/add-dir` `/files` `/thinking` `/rewind` `/skills` `/plugin` `/mcp` `/branch` `/commit` `/theme` `/keybindings` `/copy` `/rename`

### LLM 提供商

- **Anthropic** —— Messages API + SSE 流式（默认 `claude-sonnet-4-6`）
- **OpenAI 兼容** —— Chat Completions API（GPT、DeepSeek、Qwen、Ollama、vLLM 等）
- **AWS Bedrock** —— SigV4 签名 + 推理配置文件发现
- **GCP Vertex AI** —— ADC 认证

### MCP（模型上下文协议）

- 支持 stdio、SSE、WebSocket 三种传输
- `McpToolAdapter` 将 MCP 工具桥接到原生 `Tool` trait
- 通过 `~/.crab/settings.json` 或 `--mcp-config` 配置

### 会话管理

- 对话自动存到 `~/.crab/sessions/`
- `--continue` / `-c` 继续最近一次会话
- `--resume <id>` 恢复指定会话
- `--fork-session` 恢复时分叉而非原地续
- `--name` 友好会话名
- 每会话文件历史快照支持 `/rewind`

### Hook 系统

- `PreToolUse` / `PostToolUse` / `UserPromptSubmit` 触发器
- Shell 命令执行，返回 `Allow` / `Deny` / `Modify`
- 在 `settings.json` 中配置

### 交互式 TUI

- `ratatui` + `crossterm` 即时模式渲染
- Markdown 渲染 + 语法高亮 + GFM 表格（响应式水平/竖排布局）
- OSC 8 可点击超链接（Markdown 链接和图片，不支持时自动退化为纯文本）
- Toast 通知队列（key 去重 + 自动过期）
- OS 级终端通知（OSC 9/99/777，支持 iTerm2、Kitty、WezTerm、Ghostty）
- Vim 编辑模式（normal、insert、visual，支持 motion、operator、register）
- 斜杠命令和文件路径自动补全
- 会话侧栏（Recent / Saved 标签切换）
- 权限确认对话框（allow / deny / always）
- 状态栏实时费用追踪
- 暗色/亮色主题通过 OSC 11 自动检测

## 架构

24 个 Rust crate，按 4 层组织：

```
Layer 4 (入口)     cli          daemon
                    |             |
Layer 3 (编排)     agent      engine      session
                    |          |           |
Layer 2 (服务)    api  tools  mcp  tui  skill  plugin  telemetry  acp  ide  job  remote  sandbox
                    |    |     |    |     |       |         |
Layer 1 (基础)    common   core   config   auth   fs   memory   process
```

关键设计决策：
- **异步运行时**：tokio（多线程）
- **LLM 派发**：`enum LlmBackend`，零动态派发、穷尽匹配
- **工具系统**：`trait Tool` + `ToolRegistry` + `ToolExecutor`，通过 JSON Schema 发现
- **TUI**：`ratatui` + `crossterm` 即时模式
- **错误处理**：库用 `thiserror`、应用用 `anyhow`

> 完整架构详见 [`docs/architecture.md`](docs/architecture.md)

## 配置路径

```bash
# 全局
~/.crab/settings.json        # API 密钥、提供商设置、MCP 服务器
~/.crab/memory/              # 持久化记忆文件
~/.crab/sessions/            # 保存的对话会话
~/.crab/file-history/        # 按会话的编辑前快照
~/.crab/skills/              # 全局 skill 定义

# 项目
your-project/CRAB.md                # 项目指令（相当于 CLAUDE.md）
your-project/.crab/settings.json    # 项目级设置覆盖
your-project/.crab/skills/          # 项目级 skill
```

## CLI 使用

```bash
crab                              # 交互式 TUI 模式
crab "your prompt"                # 单次模式
crab -p "your prompt"             # Print 模式（非交互）
crab -c                           # 继续最近会话
crab --provider openai            # 使用 OpenAI 兼容提供商
crab --model opus                 # 模型别名（sonnet / opus / haiku）
crab --permission-mode plan       # Plan 模式
crab --effort high                # 设置推理强度
crab --resume <session-id>        # 恢复指定会话
crab doctor                       # 诊断检查
crab auth login                   # 配置认证
```

内部开关（只走 env，不暴露 `--help`）：
- `CRAB_COORDINATOR_MODE=1` —— 启用 Coordinator Mode 叠加层
- `CRAB_AUTO_DREAM=1` —— 启用 `auto-dream` 内存整理门控（需配合 `auto-dream` cargo feature）

## 构建与开发

```bash
cargo build --workspace                    # 构建全部
cargo test --workspace                     # 运行所有测试（4500+）
cargo clippy --workspace -- -D warnings    # Lint（CI 视 warning 为 error）
cargo fmt --all --check                    # 格式检查
cargo run --bin crab                       # 运行 CLI
```

实验性 cargo feature（默认全部关闭）：
- `crab-agent` 的 `auto-dream` —— 对齐 CCB 的记忆整理调度器（forked-agent 运行器仍为桩）
- `crab-agent` 的 `proactive` —— 对齐 CCB `feature('PROACTIVE')` 的占位骨架
- `crab-agent` 的 `mem-ranker` —— 透传 `crab-memory/mem-ranker`

## 对比

| | Crab Code | Claude Code | [OpenCode](https://github.com/anomalyco/opencode) | Codex CLI |
|--|-----------|-------------|----------|-----------|
| 开源 | Apache 2.0 | 闭源 | MIT | Apache 2.0 |
| 语言 | Rust | TypeScript (Bun) | TypeScript | Rust |
| 模型无关 | 任意提供商 | Anthropic + AWS/GCP | 任意提供商 | 仅 OpenAI |
| 自托管 | 支持 | 不支持 | 支持 | 支持 |
| MCP | stdio + SSE + WS | 6 种传输 | LSP | 2 种传输 |
| TUI | ratatui | Ink (React) | 自研 | ratatui |
| 内置工具 | 46 | 52 | ~10 | ~10 |
| 权限模式 | 6 | 6 | 2 | 3 |

## 贡献

欢迎参与！需要帮助的方向：

- 对齐 Claude Code 功能 —— 已知缺口：`auto-dream` forked-agent 运行器、Edit / Write / Notebook 工具级 `track_edit` 钩子、`proactive` mini-agent
- OS 级沙箱（Landlock / Seatbelt / Windows Job Object）
- 端到端集成测试
- 其他 LLM 提供商适配与测试
- 文档与 i18n

## 许可证

[Apache License 2.0](LICENSE)

---

<div align="center">

**用 Rust 构建，作者 [lingcoder](https://github.com/lingcoder)**

*Claude Code 给我们展示了 Agentic Coding 的未来。Crab Code 让它对所有人开放。*

</div>
