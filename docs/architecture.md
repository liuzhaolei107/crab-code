# Crab Code Architecture

> Version: v2.2
> Updated: 2026-04-14

---

## 1. Architecture Overview

### Four-Layer Architecture

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| **Layer 4** Entry Layer | `crates/cli` `crates/daemon` | CLI entry point (clap), background daemon |
| **Layer 3** Engine Layer | `agent` `session` | Multi-Agent orchestration, session management, context compaction |
| **Layer 2** Service Layer | `tools` `mcp` `api` `fs` `process` `plugin` `skill` `memory` `telemetry` `tui` | Tool system, MCP protocol stack, multi-model API client, file/process operations, skill system, persistent memory, TUI components |
| **Layer 1** Foundation Layer | `core` `common` `config` `auth` | Domain model, config hot reload, authentication |

> Dependency direction: upper layers depend on lower layers; reverse dependencies are prohibited. `core` defines the `Tool` trait to avoid circular dependencies between tools/agent.

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Layer 4: Entry Layer                         │
│  ┌──────────────────────────┐   ┌────────────────────────────────┐  │
│  │      crates/cli          │   │       crates/daemon            │  │
│  │  clap subcommands +      │   │  background daemon +           │  │
│  │  tokio runtime           │   │  session pool                  │  │
│  └────────────┬─────────────┘   └──────────────┬─────────────────┘  │
├───────────────┼────────────────────────────────┼────────────────────┤
│               │        Layer 3: Engine Layer   │                    │
│  ┌────────────▼─────────────┐   ┌──────────────▼─────────────────┐  │
│  │         agent            │   │          session               │  │
│  │  Agent orchestration +   │   │  Session state + context       │  │
│  │  task dispatch           │   │  compaction + memory           │  │
│  └──┬───────────┬───────────┘   └───────┬──────────────┬─────────┘  │
├─────┼───────────┼───────────────────────┼──────────────┼────────────┤
│     │           │   Layer 2: Service    │              │            │
│     │           │   Layer              │              │            │
│  ┌──▼────┐  ┌───▼───┐  ┌────┐  ┌───────▼──┐  ┌───────▼────┐      │
│  │ tools │  │  mcp  │  │tui │  │   api    │  │  telemetry │      │
│  │ 21+   │  │JSON-  │  │rata│  │LlmBack- │  │OpenTelemetry│      │
│  │built- │  │RPC    │  │tui │  │end enum  │  │  traces    │      │
│  │in     │  │       │  │    │  │          │  │            │      │
│  └┬────┬─┘  └───────┘  └────┘  └──────────┘  └────────────┘      │
│   │    │                                                           │
│  ┌▼──┐ ┌▼──────┐  ┌──────┐  ┌──────┐  ┌──────┐                    │
│  │fs │ │process │  │plugin│  │skill │  │memory│                    │
│  │glob│ │sub-   │  │hooks │  │regis-│  │store │                    │
│  │grep│ │process│  │WASM  │  │try + │  │rank  │                    │
│  │    │ │signal │  │MCP↔  │  │built-│  │age   │                    │
│  └───┘ └───────┘  └──────┘  └──────┘  └──────┘                    │
├───────────────────────────────────────────────────────────────────┤
│                      Layer 1: Foundation Layer                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │   core   │  │  common  │  │  config  │  │   auth   │          │
│  │Domain    │  │Error/    │  │Multi-    │  │OAuth/Key │          │
│  │model     │  │utility   │  │layer     │  │Keychain  │          │
│  │Tool trait│  │Path/text │  │config    │  │          │          │
│  │          │  │          │  │CRAB.md   │  │          │          │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘          │
└───────────────────────────────────────────────────────────────────┘
```

### Mapping to Claude Code's Five-Layer Architecture

| Claude Code (TS) | Path | Crab Code (Rust) | Notes |
|-------------------|------|-------------------|-------|
| **Entry Layer** entrypoints/ | `cli.tsx` `main.tsx` | `crates/cli` `crates/daemon` | CC uses React/Ink for rendering, Crab uses ratatui |
| **Command Layer** commands/ | `query.ts` `QueryEngine.ts` | `agent` + `session` | CC's query loop maps to agent orchestration |
| **Tool Layer** tools/ | 52 Tool directories | `tools` + `mcp` | CC mixes tools and MCP in services/; Crab separates them |
| **Service Layer** services/ | `api/` `mcp/` `oauth/` `compact/` `memdir/` | `api` `mcp` `auth` `skill` `plugin` `memory` `telemetry` | CC's service layer is flat; Crab splits by responsibility. CC's `memdir/` maps to `memory` |
| **Foundation Layer** utils/ types/ | `Tool.ts` `context.ts` | `core` `common` `config` | CC scatters types across files; Crab centralizes them in core |

### Core Design Philosophy

1. **core has zero I/O** -- Pure data structures and trait definitions, reusable by any frontend (CLI/GUI/WASM)
2. **Message loop driven** -- Everything revolves around the query loop: user input -> API call -> tool execution -> result return
3. **Workspace isolation** -- 17 library crates with orthogonal responsibilities; incremental compilation only triggers on changed parts
4. **Feature flags control dependencies** -- No Bedrock? AWS SDK is not compiled. No WASM? wasmtime is not compiled.

---

## 2. Why Rust

### 2.1 Go vs Rust Comparison

| Dimension | Go | Rust | Conclusion |
|-----------|-----|------|------------|
| **Development speed** | Fast, low learning curve | 2-3x slower, lifetime/ownership friction | Go wins |
| **CLI ecosystem** | cobra is mature | clap is equally mature | Tie |
| **TUI** | Charm (bubbletea) is excellent | ratatui is excellent | Tie |
| **GUI extensibility** | Good (Wails WebView-based, fyne/gio native) | **Strong** (Tauri 2.0 desktop + mobile) | Slight Rust edge |
| **WASM** | Go->WASM ~10MB+, poor performance | **First-class citizen**, small output, native perf | **Rust wins** |
| **FFI/cross-language** | cgo has performance penalty | **Zero-overhead FFI**, native C ABI | **Rust wins** |
| **AI/ML ecosystem** | Few bindings | candle, burn, ort (ONNX) | **Rust wins** |
| **Serialization** | encoding/* is adequate | serde is **dominant** | **Rust wins** |
| **Compile speed** | 10-30s | 5-15min | Go wins |
| **Cross-compilation** | Extremely simple | Moderate (needs target toolchain) | Go wins |
| **Hiring** | Larger developer pool | Smaller developer pool | Go wins |

### 2.2 Five Core Reasons for Choosing Rust

1. **High ceiling for future expansion** -- CLI -> Tauri desktop -> browser WASM -> mobile, 100% core logic sharing
2. **Tauri ecosystem** -- Mainstream Electron alternative, 20-30MB memory vs 150MB+, 5-15MB bundle vs 100MB+
3. **Third-party library quality** -- serde, tokio, ratatui, clap are all top-tier implementations in their domains
4. **Local AI inference** -- Future integration of local models via candle/burn, no cgo bridging needed
5. **Plugin sandbox** -- wasmtime itself is written in Rust; WASM plugin system is a natural fit

### 2.3 Expected Performance Comparison

| Metric | TypeScript/Bun | Rust | Factor |
|--------|---------------|------|--------|
| **Cold start** | ~135ms | ~5-10ms | **15-25x** |
| **Memory usage (idle)** | ~80-150MB | ~5-10MB | **10-20x** |
| **API streaming** | Baseline | ~Equal | 1x (I/O bound) |
| **Terminal UI rendering** | Slower (React overhead) | Fast (ratatui zero-overhead) | **3-5x** |
| **JSON serialization** | Fast (V8 built-in) | Fastest (serde zero-copy) | **2-3x** |
| **Binary size** | ~100MB+ (including runtime) | ~10-20MB | **5-10x** |

---

## 3. Core Library Alternatives

28 TS -> Rust mappings in total, grouped by function. Versions are pinned in `Cargo.toml` and omitted here to avoid staleness.

### 3.1 CLI / UI

| # | Function | TypeScript Original | Rust Alternative | Docs |
|---|----------|---------------------|------------------|------|
| 1 | CLI framework | Commander.js | clap (derive) | [docs.rs/clap](https://docs.rs/clap) |
| 2 | Terminal UI | React/Ink | ratatui + crossterm | [ratatui.rs](https://ratatui.rs) |
| 3 | Terminal styling | chalk | crossterm Style | [docs.rs/crossterm](https://docs.rs/crossterm) |
| 4 | Markdown rendering | marked | pulldown-cmark | [docs.rs/pulldown-cmark](https://docs.rs/pulldown-cmark) |
| 5 | Syntax highlighting | highlight.js | syntect | [docs.rs/syntect](https://docs.rs/syntect) |
| 6 | Fuzzy search | Fuse.js | nucleo *(planned)* | [docs.rs/nucleo](https://docs.rs/nucleo) |

### 3.2 Network / API

| # | Function | TypeScript Original | Rust Alternative | Docs |
|---|----------|---------------------|------------------|------|
| 7 | HTTP client | axios/undici | reqwest | [docs.rs/reqwest](https://docs.rs/reqwest) |
| 8 | WebSocket | ws | tokio-tungstenite | [docs.rs/tokio-tungstenite](https://docs.rs/tokio-tungstenite) |
| 9 | Streaming SSE | Anthropic SDK | eventsource-stream | [docs.rs/eventsource-stream](https://docs.rs/eventsource-stream) |
| 10 | OAuth | google-auth-library | oauth2 | [docs.rs/oauth2](https://docs.rs/oauth2) |

### 3.3 Serialization / Validation

| # | Function | TypeScript Original | Rust Alternative | Docs |
|---|----------|---------------------|------------------|------|
| 11 | JSON | Built-in JSON | serde + serde_json | [serde.rs](https://serde.rs) |
| 12 | YAML | yaml | serde_yml | [docs.rs/serde_yml](https://docs.rs/serde_yml) |
| 13 | TOML | -- | toml | [docs.rs/toml](https://docs.rs/toml) |
| 14 | Schema validation | Zod | schemars | [docs.rs/schemars](https://docs.rs/schemars) |

> Note: `serde_yml` is the community successor to the archived `serde_yaml` (dtolnay). It is the correct modern choice.

### 3.4 File System / Search

| # | Function | TypeScript Original | Rust Alternative | Docs |
|---|----------|---------------------|------------------|------|
| 15 | Glob | glob | globset | [docs.rs/globset](https://docs.rs/globset) |
| 16 | Grep/search | ripgrep bindings | grep-searcher + grep-regex + ignore | [docs.rs/grep-searcher](https://docs.rs/grep-searcher) |
| 17 | Gitignore | -- | ignore | [docs.rs/ignore](https://docs.rs/ignore) |
| 18 | File watching | chokidar | notify | [docs.rs/notify](https://docs.rs/notify) |
| 19 | Diff | diff | similar | [docs.rs/similar](https://docs.rs/similar) |
| 20 | File locking | proper-lockfile | fd-lock | [docs.rs/fd-lock](https://docs.rs/fd-lock) |

> Note on #16: ripgrep is built from a family of crates by BurntSushi: `grep-searcher` (streaming search with binary detection), `grep-regex` (regex adapter), `grep-matcher` (abstract trait), `ignore` (gitignore-aware walker), `regex` (pattern engine). We use the full `grep-searcher` + `grep-regex` + `ignore` stack — the same core as the `rg` command line tool.

### 3.5 System / Process

| # | Function | TypeScript Original | Rust Alternative | Docs |
|---|----------|---------------------|------------------|------|
| 21 | Subprocess | execa | tokio::process | [docs.rs/tokio](https://docs.rs/tokio) |
| 22 | Process tree | tree-kill | sysinfo | [docs.rs/sysinfo](https://docs.rs/sysinfo) |
| 23 | System directories | -- | directories | [docs.rs/directories](https://docs.rs/directories) |
| 24 | Keychain | Custom impl | keyring | [docs.rs/keyring](https://docs.rs/keyring) |

### 3.6 Observability / Cache

| # | Function | TypeScript Original | Rust Alternative | Docs |
|---|----------|---------------------|------------------|------|
| 25 | OpenTelemetry | @opentelemetry/* | opentelemetry | [docs.rs/opentelemetry](https://docs.rs/opentelemetry) |
| 26 | Logging/tracing | console.log | tracing | [docs.rs/tracing](https://docs.rs/tracing) |
| 27 | LRU cache | lru-cache | lru | [docs.rs/lru](https://docs.rs/lru) |
| 28 | Error handling | Error class | thiserror + anyhow | [docs.rs/thiserror](https://docs.rs/thiserror) |

---

## 4. Workspace Project Structure

### 4.1 Complete Directory Tree

```
crab-code/
├── Cargo.toml                         # workspace root
├── Cargo.lock
├── rust-toolchain.toml                # pinned toolchain
├── rustfmt.toml                       # formatting config
├── clippy.toml                        # lint config
├── .gitignore
├── LICENSE
│
├── crates/
│   ├── common/                        # crab-common: shared foundation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                 # exports error, result, utils
│   │       ├── error.rs               # thiserror unified error enum
│   │       ├── result.rs              # type Result<T>
│   │       └── utils/                 # utility functions (no business semantics)
│   │           ├── mod.rs
│   │           ├── id.rs              # ULID generation
│   │           ├── path.rs            # cross-platform path normalization
│   │           ├── text.rs            # Unicode width, ANSI strip
│   │           └── debug.rs           # debug categories, tracing init
│   │
│   ├── core/                          # crab-core: domain model
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── message.rs             # Message, Role, ContentBlock
│   │       ├── conversation.rs        # Conversation, Turn
│   │       ├── tool.rs                # trait Tool + ToolContext + ToolOutput
│   │       ├── model.rs               # ModelId, TokenUsage, CostTracker
│   │       ├── permission/            # Permission system (module directory)
│   │       │   ├── mod.rs             # PermissionMode, PermissionPolicy, re-exports
│   │       │   ├── rule_parser.rs     # [P0] Rule AST parsing: "Bash(cmd:git*)" format
│   │       │   ├── path_validator.rs  # [P0] File path permission engine, symlink resolution
│   │       │   ├── denial_tracker.rs  # [P1] Consecutive denial counting, pattern detection
│   │       │   ├── explainer.rs       # [P1] Human-readable permission decision explanation
│   │       │   └── shadowed_rules.rs  # [P2] Shadowed rule detection
│   │       ├── config.rs              # trait ConfigSource
│   │       ├── event.rs               # Domain event enum (inter-crate decoupled communication)
│   │       └── capability.rs          # Agent capability declaration
│   │
│   ├── config/                        # crab-config: configuration system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── settings.rs            # settings.json read/write, layered merging
│   │       ├── crab_md.rs             # CRAB.md parsing (project/user/global)
│   │       ├── hooks.rs               # Hook definition and triggering
│   │       ├── feature_flag.rs        # [P1] Runtime feature flag management (local evaluation)
│   │       ├── policy.rs              # [P0] Permission policy restrictions, MDM/managed-path
│   │       ├── keybinding.rs          # [P1] Keybinding schema/parsing/validation/resolver
│   │       ├── config_toml.rs         # config.toml multi-provider configuration
│   │       ├── hot_reload.rs          # settings.json hot reload monitoring
│   │       ├── permissions.rs         # Unified permission decision entry point
│   │       ├── validation.rs          # [P1] Settings validation engine
│   │       ├── settings_cache.rs      # [P1] Memoized settings cache
│   │       ├── change_detector.rs     # [P2] Per-source change detection
│   │       └── mdm.rs                 # [P2] Enterprise MDM managed settings
│   │
│   ├── auth/                          # crab-auth: authentication
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── oauth.rs               # OAuth2 PKCE flow
│   │       ├── keychain.rs            # System Keychain (macOS/Win/Linux)
│   │       ├── api_key.rs             # API key management
│   │       ├── bedrock_auth.rs        # AWS SigV4 signing (feature)
│   │       ├── vertex_auth.rs         # GCP Vertex authentication
│   │       ├── aws_iam.rs             # AWS IAM Roles + IRSA
│   │       ├── gcp_identity.rs        # GCP Workload Identity Federation
│   │       └── credential_chain.rs    # Credential chain (priority-ordered resolution)
│   │
│   ├── api/                           # crab-api: LLM API client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                 # LlmBackend enum + create_backend()
│   │       ├── types.rs               # Internal unified request/response/event types
│   │       ├── anthropic/             # Standalone Anthropic Messages API client
│   │       │   ├── mod.rs
│   │       │   ├── client.rs          # HTTP + SSE + retry
│   │       │   ├── types.rs           # Anthropic native API types
│   │       │   └── convert.rs         # Anthropic <-> internal type conversion
│   │       ├── openai/                # Standalone OpenAI Chat Completions client
│   │       │   ├── mod.rs
│   │       │   ├── client.rs          # HTTP + SSE + retry
│   │       │   ├── types.rs           # OpenAI native API types
│   │       │   └── convert.rs         # OpenAI <-> internal type conversion
│   │       ├── bedrock.rs             # AWS Bedrock (feature, wraps anthropic)
│   │       ├── vertex.rs              # Google Vertex (feature, wraps anthropic)
│   │       ├── rate_limit.rs          # Shared rate limiting, exponential backoff
│   │       ├── cache.rs               # Prompt cache (Anthropic path)
│   │       ├── error.rs
│   │       ├── streaming.rs           # Streaming tool call parsing
│   │       ├── fallback.rs            # Multi-model fallback chain
│   │       ├── capabilities.rs        # Model capability negotiation and discovery
│   │       ├── context_optimizer.rs   # Context window optimization + smart truncation
│   │       ├── retry_strategy.rs      # Enhanced retry strategy
│   │       ├── error_classifier.rs    # Error classification (retryable/non-retryable)
│   │       ├── token_estimation.rs    # [P1] Approximate token count estimation
│   │       ├── ttft_tracker.rs        # [P1] Time-to-first-token latency tracking
│   │       ├── fast_mode.rs           # [P1] Fast mode switching
│   │       └── usage_tracker.rs       # [P1] Usage aggregation (per-session/model)
│   │
│   ├── mcp/                           # crab-mcp: MCP facade + protocol adaptation layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── protocol.rs            # JSON-RPC message definitions
│   │       ├── client.rs              # MCP client
│   │       ├── server.rs              # MCP server
│   │       ├── manager.rs             # Lifecycle management, multi-server coordination
│   │       ├── transport/
│   │       │   ├── mod.rs
│   │       │   ├── stdio.rs           # stdin/stdout transport
│   │       │   └── ws.rs              # WebSocket (feature)
│   │       ├── resource.rs            # Resource caching, templates
│   │       ├── discovery.rs           # Server auto-discovery
│   │       ├── sse_server.rs          # SSE server transport (crab as server)
│   │       ├── sampling.rs            # MCP sampling (LLM inference requests)
│   │       ├── roots.rs               # MCP roots (workspace root directory declaration)
│   │       ├── logging.rs             # MCP logging protocol messages
│   │       ├── handshake.rs           # Initialization handshake flow
│   │       ├── negotiation.rs         # Capability negotiation
│   │       ├── capability.rs          # Capability declaration types
│   │       ├── notification.rs        # Server notification push
│   │       ├── progress.rs            # Progress reporting
│   │       ├── cancellation.rs        # Request cancellation mechanism
│   │       ├── health.rs              # Health check + heartbeat
│   │       ├── auth.rs                # [P1] MCP OAuth2/API key authentication
│   │       ├── channel_permissions.rs # [P1] Channel-level tool/resource permissions
│   │       ├── elicitation.rs         # [P1] User input request handling
│   │       ├── env_expansion.rs       # [P1] ${VAR} environment variable expansion in config
│   │       ├── official_registry.rs   # [P2] Official MCP server registry
│   │       └── normalization.rs       # [P2] Tool/resource name normalization
│   │
│   ├── fs/                            # crab-fs: file system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── glob.rs                # globset wrapper
│   │       ├── grep.rs                # ripgrep core integration
│   │       ├── gitignore.rs           # .gitignore rule parsing
│   │       ├── watch.rs               # notify file watching (with debouncing, batching)
│   │       ├── lock.rs                # File locking (fd-lock)
│   │       ├── diff.rs                # similar wrapper, patch generation
│   │       └── symlink.rs             # Symbolic link handling + secure resolution
│   │
│   ├── process/                       # crab-process: subprocess management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── spawn.rs               # Subprocess launching, environment inheritance
│   │       ├── pty.rs                 # Pseudo-terminal (feature = "pty")
│   │       ├── tree.rs                # Process tree kill (sysinfo)
│   │       ├── signal.rs              # Signal handling, graceful shutdown
│   │       └── sandbox.rs             # Sandbox policy (feature = "sandbox")
│   │
│   ├── tools/                         # crab-tools: tool system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs            # ToolRegistry: registration, lookup
│   │       ├── executor.rs            # Unified executor with permission checking
│   │       ├── builtin/
│   │       │   ├── mod.rs
│   │       │   ├── bash.rs            # BashTool
│   │       │   ├── bash_security.rs   # Bash security checks
│   │       │   ├── bash_classifier.rs # [P0] Bash command classification (read-only/write/dangerous)
│   │       │   ├── read.rs            # ReadTool
│   │       │   ├── read_enhanced.rs   # Enhanced file reading (PDF/image/Notebook)
│   │       │   ├── edit.rs            # EditTool (diff-based)
│   │       │   ├── write.rs           # WriteTool
│   │       │   ├── glob.rs            # GlobTool
│   │       │   ├── grep.rs            # GrepTool
│   │       │   ├── lsp.rs             # LSP integration tool
│   │       │   ├── web_search.rs      # WebSearchTool
│   │       │   ├── web_fetch.rs       # WebFetchTool
│   │       │   ├── web_cache.rs       # Web page cache
│   │       │   ├── web_formatter.rs   # Web page formatter
│   │       │   ├── web_browser.rs     # [P2] Playwright/CDP browser automation
│   │       │   ├── agent.rs           # AgentTool (sub-Agent)
│   │       │   ├── send_message.rs    # [P0] SendMessageTool (cross-Agent messaging)
│   │       │   ├── skill.rs           # [P0] SkillTool (invoke skill by name)
│   │       │   ├── notebook.rs        # NotebookTool
│   │       │   ├── task.rs            # TaskCreate/Get/List/Update
│   │       │   ├── todo_write.rs      # [P1] TodoWriteTool (structured TODO)
│   │       │   ├── team.rs            # TeamCreate/Delete
│   │       │   ├── mcp_tool.rs        # MCP tool adapter
│   │       │   ├── mcp_resource.rs    # [P1] ListMcpResources + ReadMcpResource
│   │       │   ├── mcp_auth.rs        # [P1] MCP server authentication tool
│   │       │   ├── worktree.rs        # Git Worktree tool
│   │       │   ├── ask_user.rs        # User interaction tool
│   │       │   ├── image_read.rs      # Image reading tool
│   │       │   ├── plan_mode.rs       # Plan mode tool
│   │       │   ├── plan_file.rs       # Plan file operations
│   │       │   ├── plan_approval.rs   # Plan approval tool
│   │       │   ├── verify_plan.rs     # [P1] Plan execution verification
│   │       │   ├── config_tool.rs     # [P1] ConfigTool (programmatic settings read/write)
│   │       │   ├── brief.rs           # [P1] BriefTool (conversation summary)
│   │       │   ├── snip.rs            # [P1] SnipTool (trim large tool output)
│   │       │   ├── sleep.rs           # [P1] SleepTool (async wait)
│   │       │   ├── tool_search.rs     # [P1] ToolSearchTool (search available tools)
│   │       │   ├── monitor.rs         # [P2] MonitorTool (file/process monitoring)
│   │       │   ├── workflow.rs        # [P2] WorkflowTool (multi-step workflow)
│   │       │   ├── send_user_file.rs  # [P2] SendUserFileTool
│   │       │   ├── powershell.rs      # PowerShellTool
│   │       │   ├── cron.rs            # CronCreate/Delete/List
│   │       │   └── remote_trigger.rs  # RemoteTriggerTool
│   │       ├── permission.rs          # Tool permission checking logic
│   │       ├── sandbox.rs             # Tool sandbox policy
│   │       ├── schema.rs              # Tool schema conversion
│   │       └── tool_use_summary.rs    # [P1] Tool result summary generation
│   │
│   ├── session/                       # crab-session: session management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── conversation.rs        # Conversation state machine, multi-turn management
│   │       ├── context.rs             # Context window management
│   │       ├── compaction.rs          # Message compaction strategies (5 levels)
│   │       ├── micro_compact.rs       # [P0] Micro-compaction: per-message replacement of large tool results
│   │       ├── auto_compact.rs        # [P1] Auto-compaction trigger + cleanup
│   │       ├── snip_compact.rs        # [P1] Snip compaction: "[snipped]" marker
│   │       ├── history.rs             # Session persistence, recovery, search, export
│   │       ├── memory.rs              # Memory system (file persistence)
│   │       ├── memory_types.rs        # [P1] Memory type schema (user/project/feedback)
│   │       ├── memory_relevance.rs    # [P1] Memory relevance matching and scoring
│   │       ├── memory_extract.rs      # [P2] Automatic memory extraction
│   │       ├── memory_age.rs          # [P2] Memory aging and decay
│   │       ├── team_memory.rs         # [P2] Team memory paths and loading
│   │       ├── cost.rs                # Token counting, cost tracking
│   │       ├── template.rs            # Session template + quick recovery
│   │       └── migration.rs           # [P2] Data migration system
│   │
│   ├── agent/                         # crab-agent: multi-Agent system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── coordinator.rs         # Agent orchestration, workers pool + work-stealing scheduler
│   │       ├── query_loop.rs          # Core message loop
│   │       ├── task.rs                # TaskList, dependency graph
│   │       ├── team.rs                # Team creation, member management
│   │       ├── message_bus.rs         # Inter-Agent messaging (tokio::mpsc)
│   │       ├── message_router.rs      # Inter-Agent message routing
│   │       ├── worker.rs              # Sub-Agent worker
│   │       ├── system_prompt/         # System prompt (module directory)
│   │       │   ├── mod.rs             # re-exports
│   │       │   ├── builder.rs         # [Refactored] Main assembly logic (formerly system_prompt.rs)
│   │       │   ├── sections.rs        # [P0] Modular section architecture + dynamic boundaries
│   │       │   └── cache.rs           # [P1] Per-section memoized cache
│   │       ├── token_budget.rs        # [P1] Token budget management
│   │       ├── stop_hooks.rs          # [P1] Stop condition hooks
│   │       ├── summarizer.rs          # Conversation summary generation
│   │       ├── rollback.rs            # Rollback mechanism
│   │       ├── error_recovery.rs      # Error recovery strategy
│   │       ├── retry.rs              # Auto-retry mechanism
│   │       ├── slash_commands.rs      # Slash command registration and execution
│   │       ├── repl_commands.rs       # REPL commands (/undo /branch /fork)
│   │       ├── effort.rs              # Model effort level
│   │       ├── git_context.rs         # Git context collection
│   │       ├── pr_context.rs          # PR context collection
│   │       ├── prompt_suggestion.rs   # [P2] Follow-up prompt suggestions
│   │       └── tips.rs                # [P2] Contextual tips
│   │
│   ├── tui/                           # crab-tui: terminal UI (21 components)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs                 # App state machine, main loop
│   │       ├── event.rs               # crossterm Event -> AppEvent mapping/dispatch
│   │       ├── layout.rs              # Layout calculation
│   │       ├── runner.rs              # TUI runner (startup/shutdown)
│   │       ├── keybindings.rs         # Keybinding configuration
│   │       ├── ansi.rs                # ANSI escape -> ratatui Span conversion
│   │       ├── components/
│   │       │   ├── mod.rs
│   │       │   ├── input.rs           # Multi-line input box + Vim motion
│   │       │   ├── markdown.rs        # Markdown rendering
│   │       │   ├── syntax.rs          # Code highlighting (syntect)
│   │       │   ├── spinner.rs         # Loading indicator
│   │       │   ├── diff.rs            # Diff visualization (unified)
│   │       │   ├── select.rs          # Selection list
│   │       │   ├── dialog.rs          # Confirmation/permission dialog
│   │       │   ├── cost_bar.rs        # Token/cost status bar
│   │       │   ├── task_list.rs       # Task progress panel
│   │       │   ├── autocomplete.rs    # Autocomplete popup
│   │       │   ├── code_block.rs      # Code block + copy button
│   │       │   ├── command_palette.rs # Command palette (Ctrl+P)
│   │       │   ├── input_history.rs   # Input history (up/down arrows)
│   │       │   ├── loading.rs         # Loading animation component
│   │       │   ├── notification.rs    # Toast notification system
│   │       │   ├── progress_indicator.rs # Progress indicator
│   │       │   ├── search.rs          # Global search panel
│   │       │   ├── shortcut_hint.rs   # Shortcut hint bar
│   │       │   ├── status_bar.rs      # Enhanced status bar
│   │       │   ├── tool_output.rs     # Collapsible tool output display
│   │       │   ├── output_styles.rs   # [P1] Output formatting style configuration
│   │       │   ├── permission_dialog.rs # [P1] Dedicated permission prompt dialog
│   │       │   ├── session_sidebar.rs # [P1] Session sidebar
│   │       │   └── context_collapse.rs # [P2] Context collapse/expand
│   │       ├── vim/
│   │       │   ├── mod.rs
│   │       │   ├── motion.rs
│   │       │   ├── operator.rs
│   │       │   └── mode.rs
│   │       └── theme.rs               # Color theme (customizable)
│   │
│   ├── skill/                         # crab-skill: skill system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs               # Skill, SkillTrigger, SkillContext, SkillSource
│   │       ├── frontmatter.rs         # YAML frontmatter parsing
│   │       ├── registry.rs            # SkillRegistry (discover, find, match)
│   │       ├── builder.rs             # SkillBuilder fluent API
│   │       └── bundled/               # Built-in skills (one file per skill)
│   │           ├── mod.rs
│   │           ├── commit.rs
│   │           ├── review_pr.rs
│   │           ├── debug.rs
│   │           ├── loop_skill.rs
│   │           ├── remember.rs
│   │           ├── schedule.rs
│   │           ├── simplify.rs
│   │           ├── stuck.rs
│   │           ├── verify.rs
│   │           └── update_config.rs
│   │
│   ├── plugin/                        # crab-plugin: plugin/hook system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── skill_builder.rs       # MCP → Skill bridge
│   │       ├── wasm_runtime.rs        # WASM sandbox (feature = "wasm")
│   │       ├── manifest.rs            # Plugin manifest parsing
│   │       ├── manager.rs             # Plugin lifecycle management
│   │       ├── hook.rs                # Lifecycle hook execution
│   │       ├── hook_registry.rs       # Async hook registry + event broadcast
│   │       ├── hook_types.rs          # Agent/Http/Prompt hooks + SSRF guard
│   │       ├── hook_watchers.rs       # File change triggered hook re-registration
│   │       └── frontmatter_hooks.rs   # Frontmatter YAML hook registration
│   │
│   ├── telemetry/                     # crab-telemetry: observability
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tracer.rs              # OpenTelemetry tracer
│   │       ├── metrics.rs             # Custom metrics
│   │       ├── cost.rs                # Cost tracking
│   │       ├── export.rs              # [P1] Local OTLP export (no remote)
│   │       └── session_recorder.rs    # [P2] Session recording (local transcript)
│   │
│   # NOTE: IDE integration is planned via `crates/acp/` (Agent Client Protocol,
│   # JetBrains + Zed joint standard). Formerly `crates/bridge/` — that was a
│   # port of CCB's remote-session client (cloud session bridge, not IDE), and
│   # has been removed.
│
│   ├── cli/                           # crab-cli: terminal entry (binary crate)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                # #[tokio::main]
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── chat.rs            # Default interactive mode
│   │       │   ├── run.rs             # Non-interactive single execution
│   │       │   ├── session.rs         # ps, logs, attach, kill
│   │       │   ├── config.rs          # Configuration management
│   │       │   ├── mcp.rs             # MCP server mode
│   │       │   └── serve.rs           # Serve mode
│   │       └── setup.rs               # Initialization, signal registration, panic hook
│   │
│   ├── daemon/                        # crab-daemon: daemon process (binary crate)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── protocol.rs            # IPC message protocol
│   │       ├── server.rs              # Daemon server
│   │       └── session_pool.rs        # Session pool management
│
└── xtask/                             # Build helper scripts
    ├── Cargo.toml
    └── src/
        └── main.rs                    # codegen, release, bench
```

### 4.2 Crate Statistics

| Type | Count | Notes |
|------|-------|-------|
| Library crate | 14 | `crates/*` |
| Binary crate | 2 | `crates/cli` `crates/daemon` |
| Helper crate | 1 | `xtask` |
| **Total** | **17** | -- |
| Total modules | ~257 | Across 16 library crates |
| Total tests | ~2654 | `cargo test --workspace` (2026-04-06) |

> Note: [P0]/[P1]/[P2] markers indicate CCB feature alignment priority. Unmarked files are already implemented.

---

## 5. Crate Dependency Graph

### 5.1 Dependency Diagram

```
                       ┌────────────┐
                       │ crates/cli │
                       └────┬───────┘
                            │ depends on all crates
             ┌──────────────┼──────────────┐
             │              │              │
        ┌────▼────┐   ┌────▼─────┐  ┌─────▼────────┐
        │   tui   │   │  agent   │  │crates/daemon │
        └────┬────┘   └────┬─────┘  └──────┬───────┘
              │             │               │
              │        ┌────▼─────┐         │
              │        │ session  │◄────────┘
              │        └────┬─────┘
              │             │
              │        ┌────▼─────┐
              │        │  tools   │◄────────┐
                       └┬──┬──┬──┘         │
                        │  │  │            │
               ┌────────┘  │  └────────┐   │
               │           │           │   │
          ┌────▼──┐   ┌────▼──┐   ┌────▼───▼──┐
          │  fs   │   │  mcp  │   │  process   │
          └───┬───┘   └───┬───┘   └──────┬─────┘
              │           │              │
              │      ┌────▼──────────────┘
              │      │
         ┌────▼──────▼───┐    ┌───────────┐
         │     api       │    │  plugin   │
         └───────┬───────┘    └─────┬─────┘
                 │                  │
         ┌───────▼───────┐         │
         │     auth      │         │
         └───────┬───────┘         │
                 │                 │
         ┌───────▼─────────────────▼─────┐
         │            config             │
         └───────────────┬───────────────┘
                         │
         ┌───────────────▼───────────────┐
         │             core              │
         └───────────────┬───────────────┘
                         │
         ┌───────────────▼───────────────┐
         │            common             │
         └───────────────────────────────┘

                   ┌────────────┐
                   │ telemetry  │ <-- Independent sidecar, optional dependency for any crate
                   └────────────┘
```

### 5.2 Dependency Manifest (Bottom-Up)

| # | Crate | Internal Dependencies | Notes |
|---|-------|-----------------------|-------|
| 1 | **common** | None | Zero-dependency foundation layer |
| 2 | **core** | common | Pure domain model |
| 3 | **config** | core, common | Configuration read/write/merge |
| 4 | **auth** | config, common | Authentication credential management |
| 5 | **api** | core, auth, common | LlmBackend enum + Anthropic/OpenAI-compatible standalone clients |
| 6 | **fs** | common | File system operations |
| 7 | **process** | common | Subprocess management |
| 8 | **mcp** | core, common | MCP protocol client/server |
| 9 | **telemetry** | common | Independent sidecar, optional |
| 10 | **tools** | core, fs, process, mcp, config, common | 40+ built-in tools |
| 11 | **session** | core, api, config, common | Session + context compaction + memory system |
| 12 | **agent** | core, session, tools, common | Agent orchestration |
| 13 | **skill** | common | Skill discovery, registry, bundled definitions |
| 14 | **plugin** | core, common, skill | Hook system, WASM sandbox, MCP↔skill bridge |
| 15 | **tui** | core, session, config, skill, common | Terminal UI (does not depend on tools directly; receives tool state via core::Event) |
| 16 | **cli** (bin) | All crates | Extremely thin entry point |
| 17 | **daemon** (bin) | core, session, api, tools, config, agent, common | Background service |

### 5.3 Dependency Direction Principles

```
Rule 1: Upper layer -> lower layer; reverse dependencies are prohibited
Rule 2: Same-layer crates do not depend on each other (fs <-> process is prohibited)
Rule 3: core decouples via traits (Tool trait defined in core, implemented in tools)
Rule 4: telemetry is a sidecar; it does not participate in the main dependency chain
Rule 5: cli/daemon only do assembly; they contain no business logic
```

---

## 6. Detailed Crate Designs

### 6.1 `crates/common/` -- Shared Foundation

**Responsibility**: A pure utility layer with zero business logic; the lowest-level dependency for all crates

**Directory Structure**

```
src/
├── lib.rs
├── error.rs              // thiserror unified error types
├── result.rs             // type Result<T> = std::result::Result<T, Error>
├── text.rs               // Unicode width, ANSI strip, Bidi handling
├── path.rs               // Cross-platform path normalization
└── id.rs                 // ULID generation
```

**Core Types**

```rust
// error.rs -- common layer base errors (only variants with zero external dependencies)
// Http/Api/Mcp/Tool/Auth errors stay in their respective crates to avoid common pulling in reqwest etc.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {message}")]
    Config { message: String },

    #[error("{0}")]
    Other(String),
}

// result.rs
pub type Result<T> = std::result::Result<T, Error>;

// text.rs
pub fn display_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(strip_ansi(s).as_str())
}

pub fn strip_ansi(s: &str) -> String {
    let bytes = strip_ansi_escapes::strip(s);
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    // Truncate by display width, handling CJK characters
    let mut width = 0;
    let mut result = String::new();
    for ch in s.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + w > max_width {
            break;
        }
        width += w;
        result.push(ch);
    }
    result
}

// path.rs
use std::path::{Path, PathBuf};

pub fn normalize(path: &Path) -> PathBuf {
    // Unify forward slashes, resolve ~, remove redundant ..
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn home_dir() -> PathBuf {
    directories::BaseDirs::new()
        .expect("failed to resolve home directory")
        .home_dir()
        .to_path_buf()
}
```

**Per-Crate Error Type Examples**

```rust
// crates/api/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: status={status}, message={message}")]
    Api { status: u16, message: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Common(#[from] crab_common::Error),
}

// crates/mcp/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("MCP error: code={code}, message={message}")]
    Mcp { code: i32, message: String },

    #[error("transport error: {0}")]
    Transport(String),

    #[error(transparent)]
    Common(#[from] crab_common::Error),
}

// crates/tools/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool {name}: {message}")]
    Execution { name: String, message: String },

    #[error(transparent)]
    Common(#[from] crab_common::Error),
}

// crates/auth/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("auth error: {message}")]
    Auth { message: String },

    #[error(transparent)]
    Common(#[from] crab_common::Error),
}
```

> Each crate defines its own `Error` + `type Result<T>`, with `#[from] crab_common::Error` enabling upward conversion.
> Upper-layer crates (such as agent) can use `anyhow::Error` or a custom aggregate enum when unified handling is needed.

**External Dependencies**: `thiserror`, `unicode-width`, `strip-ansi-escapes`, `ulid`, `dunce`, `directories`

---

### 6.2 `crates/core/` -- Domain Model

**Responsibility**: Pure data structures + trait definitions with no I/O operations. Defines "what it is", not "how to do it".

**Directory Structure**

```
src/
├── lib.rs
├── message.rs        // Message, Role, ContentBlock, ToolUse, ToolResult
├── conversation.rs   // Conversation, Turn, context window abstraction
├── tool.rs           // trait Tool { fn name(); fn execute(); fn schema(); }
├── model.rs          // ModelId, TokenUsage, CostTracker
├── permission.rs     // PermissionMode, PermissionPolicy
├── config.rs         // trait ConfigSource, config layered merge logic
├── event.rs          // Domain event enum (inter-crate decoupling)
└── capability.rs     // Agent capability declaration
```

**Core Type Definitions**

```rust
// message.rs -- Message model (corresponds to CC src/types/message.ts)
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    Image {
        source: ImageSource,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // "base64"
    pub media_type: String,  // "image/png"
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }
}
```

```rust
// tool.rs -- Tool trait (corresponds to CC src/Tool.ts)
// Returns Pin<Box<dyn Future>> instead of native async fn because dyn Trait requires object safety
// (Arc<dyn Tool> requires the trait to be object-safe; RPITIT's impl Future does not satisfy this)
use serde_json::Value;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

use crate::permission::PermissionMode;
use crab_common::Result;

/// Tool source classification -- determines the column in the permission matrix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    /// Built-in tools (Bash/Read/Write/Edit/Glob/Grep etc.)
    BuiltIn,
    /// Tools provided by external MCP servers (untrusted source, Default/TrustProject require Prompt)
    McpExternal { server_name: String },
    /// Created by sub-Agent (AgentTool, TrustProject auto-approves)
    AgentSpawn,
}

pub trait Tool: Send + Sync {
    /// Unique tool identifier
    fn name(&self) -> &str;

    /// Tool description (used in system prompt)
    fn description(&self) -> &str;

    /// JSON Schema describing input parameters
    fn input_schema(&self) -> Value;

    /// Execute the tool and return the result
    /// Long-running tools should check for cancellation via ctx.cancellation_token
    fn execute(&self, input: Value, ctx: &ToolContext) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>>;

    /// Tool source (defaults to BuiltIn) -- affects the permission checking matrix
    fn source(&self) -> ToolSource {
        ToolSource::BuiltIn
    }

    /// Whether user confirmation is required (defaults to false)
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// Whether the tool is read-only (read-only tools can skip confirmation)
    fn is_read_only(&self) -> bool {
        false
    }
}

// --- Tool implementation example ---
// impl Tool for BashTool {
//     fn name(&self) -> &str { "bash" }
//     fn description(&self) -> &str { "Execute a shell command" }
//     fn input_schema(&self) -> Value { /* ... */ }
//     fn execute(&self, input: Value, ctx: &ToolContext) -> Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + '_>> {
//         Box::pin(async move {
//             let command = input.get("command").and_then(|v| v.as_str()).unwrap_or("");
//             let output = crab_process::run(/* ... */).await?;
//             Ok(ToolOutput::success(output.stdout))
//         })
//     }
// }

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_dir: PathBuf,
    pub permission_mode: PermissionMode,
    pub session_id: String,
    /// Cancellation token -- long-running tools (e.g., Bash) should check periodically and exit early
    pub cancellation_token: CancellationToken,
    /// Permission policy (from merged configuration)
    pub permission_policy: crate::permission::PermissionPolicy,
}

/// Tool output content block -- supports text, image, and structured JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolOutputContent {
    Text { text: String },
    Image { media_type: String, data: String },
    Json { value: Value },
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: Vec<ToolOutputContent>,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolOutputContent::Text { text: text.into() }],
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolOutputContent::Text { text: text.into() }],
            is_error: true,
        }
    }

    pub fn text(&self) -> String {
        self.content.iter()
            .filter_map(|c| match c { ToolOutputContent::Text { text } => Some(text.as_str()), _ => None })
            .collect::<Vec<_>>().join("")
    }
}
```

```rust
// model.rs -- Model and token tracking
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl ModelId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cost_usd: f64,
}

impl CostTracker {
    pub fn record(&mut self, usage: &TokenUsage, cost: f64) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.total_cache_read_tokens += usage.cache_read_tokens;
        self.total_cache_creation_tokens += usage.cache_creation_tokens;
        self.total_cost_usd += cost;
    }
}
```

```rust
// event.rs -- Domain events (inter-crate decoupled communication)
use crate::model::TokenUsage;
use crate::permission::PermissionMode;

#[derive(Debug, Clone)]
pub enum Event {
    // --- Message lifecycle ---
    /// New conversation turn started
    TurnStart { turn_index: usize },
    /// API response message started
    MessageStart,
    /// Text delta
    ContentDelta(String),
    /// Message ended
    MessageEnd { usage: TokenUsage },

    // --- Tool execution ---
    /// Tool call started
    ToolUseStart { id: String, name: String },
    /// Tool input delta (streaming)
    ToolUseInput(String),
    /// Tool execution result
    ToolResult { id: String, content: String, is_error: bool },

    // --- Permission interaction ---
    /// Request user confirmation for tool execution permission
    PermissionRequest { tool_name: String, input_summary: String, request_id: String },
    /// User permission response
    PermissionResponse { request_id: String, approved: bool },

    // --- Context compaction ---
    /// Compaction started
    CompactStart { strategy: String, before_tokens: u64 },
    /// Compaction completed
    CompactEnd { after_tokens: u64, removed_messages: usize },

    // --- Token warnings ---
    /// Token usage exceeded threshold (80%/90%/95%)
    TokenWarning { usage_percent: u8, used: u64, limit: u64 },

    // --- Errors ---
    Error(String),
}
```

```rust
// permission.rs -- Permission model
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// All tools require confirmation
    Default,
    /// Trust file operations within the project
    TrustProject,
    /// Auto-approve everything (dangerous)
    Dangerously,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub mode: PermissionMode,
    pub allowed_tools: Vec<String>,
    /// denied_tools supports glob pattern matching (e.g., "mcp__*", "bash"),
    /// uses the globset crate for matching, supporting * / ? / [abc] syntax
    pub denied_tools: Vec<String>,
}
```

**External Dependencies**: `serde`, `serde_json`, `tokio-util` (sync), `crab-common` (note: `std::pin::Pin` / `std::future::Future` are from std, no extra dependencies)

**Feature Flags**: None (pure type definitions)

---

### 6.3 `crates/config/` -- Configuration System

**Responsibility**: Read/write and merge multi-layered configuration (corresponds to CC `src/services/remoteManagedSettings/` + `src/context/` config sections)

**Directory Structure**

```
src/
├── lib.rs
├── settings.rs           // settings.json read/write, layered merging
├── crab_md.rs            // CRAB.md parsing (project/user/global)
├── hooks.rs              // Hook definition and triggering
├── feature_flag.rs       // Feature flag integration
├── policy.rs             // Permission policy, restrictions
├── keybinding.rs         // Keybinding configuration
├── config_toml.rs        // config.toml multi-provider configuration format
├── hot_reload.rs         // settings.json hot reload (notify watcher)
└── permissions.rs        // Unified permission decision entry point
```

**Configuration Layers (three-level merge, low priority -> high priority)**

```
1. Global defaults   ~/.config/crab-code/settings.json
2. User overrides    ~/.crab-code/settings.json
3. Project overrides .crab-code/settings.json
```

**Core Types**

The `Settings` struct covers: `api_provider`, `api_base_url`, `api_key`, `model`, `small_model`, `permission_mode`, `system_prompt`, `mcp_servers`, `hooks`, `theme`, and more. The three configuration levels are merged via `load_merged_settings()` (global -> user -> project), with higher-priority fields overriding lower-priority ones.

```rust
// crab_md.rs -- CRAB.md parsing
pub struct CrabMd {
    pub content: String,
    pub source: CrabMdSource,
}

pub enum CrabMdSource {
    Global,   // ~/.crab/CRAB.md
    User,     // User directory
    Project,  // Project root
}

/// Collect all CRAB.md content by priority
pub fn collect_crab_md(project_dir: &std::path::Path) -> Vec<CrabMd> {
    // Global -> user -> project, stacking progressively
    // ...
}
```

**External Dependencies**: `serde`, `serde_json`, `jsonc-parser`, `directories`, `crab-core`, `crab-common`

**Feature Flags**: None

---

### 6.4 `crates/auth/` -- Authentication

**Responsibility**: Unified management of all authentication methods (corresponds to CC `src/services/oauth/` + authentication-related code)

**Directory Structure**

```
src/
├── lib.rs
├── oauth.rs              // OAuth2 PKCE flow
├── keychain.rs           // System Keychain (macOS/Windows/Linux)
├── api_key.rs            // API key management (environment variable / file)
├── bedrock_auth.rs       // AWS SigV4 signing (feature = "bedrock")
├── vertex_auth.rs        // GCP Vertex AI authentication
├── aws_iam.rs            // AWS IAM Roles + IRSA (pod-level)
├── gcp_identity.rs       // GCP Workload Identity Federation
└── credential_chain.rs   // Credential chain (priority-ordered probing: env -> keychain -> file -> IAM)
```

**Core Interface**

```rust
// lib.rs -- Unified authentication interface
pub enum AuthMethod {
    ApiKey(String),
    OAuth(OAuthToken),
    Bedrock(BedrockCredentials),
}

/// Authentication provider trait
/// Returns Pin<Box<dyn Future>> instead of native async fn because dyn Trait requires object safety
/// (Box<dyn AuthProvider> requires the trait to be object-safe; RPITIT's impl Future does not satisfy this)
/// Implementations use tokio::sync::RwLock internally to protect the token cache;
/// get_auth() takes a read lock on the hot path, refresh() takes a write lock to refresh
pub trait AuthProvider: Send + Sync {
    /// Get the currently valid authentication info (read lock, typically <1us)
    fn get_auth(&self) -> Pin<Box<dyn Future<Output = crab_common::Result<AuthMethod>> + Send + '_>>;
    /// Refresh authentication (e.g., OAuth token expired) -- may trigger network requests
    fn refresh(&self) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>>;
}

// api_key.rs
pub fn resolve_api_key() -> Option<String> {
    // Priority: environment variable -> keychain -> config file
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .or_else(|| keychain::get("crab-code", "api-key").ok())
}

// keychain.rs -- Uses the auth crate's local AuthError, not crab_common::Error
// (the common layer does not include Auth variants; Auth errors are defined in crates/auth/src/error.rs)
use crate::error::AuthError;

pub fn get(service: &str, key: &str) -> Result<String, AuthError> {
    let entry = keyring::Entry::new(service, key)
        .map_err(|e| AuthError::Auth { message: format!("keychain init failed: {e}") })?;
    entry.get_password().map_err(|e| AuthError::Auth {
        message: format!("keychain read failed: {e}"),
    })
}

pub fn set(service: &str, key: &str, value: &str) -> Result<(), AuthError> {
    let entry = keyring::Entry::new(service, key)
        .map_err(|e| AuthError::Auth { message: format!("keychain init failed: {e}") })?;
    entry.set_password(value).map_err(|e| AuthError::Auth {
        message: format!("keychain write failed: {e}"),
    })
}
```

**External Dependencies**: `keyring`, `oauth2`, `reqwest`, `crab-config`, `crab-common`

**Feature Flags**

```toml
[features]
default = []
bedrock = ["aws-sdk-bedrockruntime", "aws-config"]
```

---

### 6.5 `crates/api/` -- LLM API Client

**Responsibility**: Encapsulate all LLM API communication with two independent clients implementing the two major API standards (corresponds to CC `src/services/api/`)

**Core Design**: No unified trait abstraction is used -- the Anthropic Messages API and OpenAI Chat Completions API
differ too much (message format, streaming event granularity, tool call protocol). Forcing unification would create a
"lowest common denominator" trap, losing provider-specific capabilities (Anthropic's prompt cache / extended thinking,
OpenAI's logprobs / structured output).

Uses **two fully independent clients + enum dispatch**:
- `anthropic/` -- Complete Anthropic Messages API client with its own types, SSE parsing, authentication
- `openai/` -- Complete OpenAI Chat Completions client, covering all compatible endpoints (Ollama/DeepSeek/vLLM/Gemini etc.)
- `LlmBackend` enum -- Determined at compile time, zero dynamic dispatch, exhaustive match ensures nothing is missed

The agent/session layer interacts through the `LlmBackend` enum. The internal unified `MessageRequest` / `StreamEvent`
are Crab Code's own data model, not an API abstraction. Each client independently handles format conversion internally.

**Directory Structure**

```
src/
├── lib.rs                // LlmBackend enum + create_backend()
├── types.rs              // Internal unified request/response/event types (Crab Code's own format)
├── anthropic/            // Fully independent Anthropic Messages API client
│   ├── mod.rs
│   ├── client.rs         // HTTP + SSE + retry
│   ├── types.rs          // Anthropic API native request/response types
│   └── convert.rs        // Anthropic types <-> internal types
├── openai/               // Fully independent OpenAI Chat Completions client
│   ├── mod.rs
│   ├── client.rs         // HTTP + SSE + retry
│   ├── types.rs          // OpenAI API native request/response types
│   └── convert.rs        // OpenAI types <-> internal types
├── bedrock.rs            // AWS Bedrock adapter (feature = "bedrock", wraps anthropic client)
├── vertex.rs             // Google Vertex adapter (feature = "vertex", wraps anthropic client)
├── rate_limit.rs         // Shared rate limiting, exponential backoff
├── cache.rs              // Prompt cache management (Anthropic path only)
├── error.rs
├── streaming.rs          // Streaming tool call parsing (partial tool argument streaming)
├── fallback.rs           // Multi-model fallback chain (primary fails -> backup model)
├── capabilities.rs       // Model capability negotiation and discovery
├── context_optimizer.rs  // Context window optimization + smart truncation strategy
├── retry_strategy.rs     // Enhanced retry strategy (backoff + jitter)
└── error_classifier.rs   // Error classification (retryable/non-retryable/rate-limited)
```

**Core Interface**

```rust
// types.rs -- Crab Code internal unified types (not an API abstraction, but its own data model)
use crab_core::message::Message;
use crab_core::model::{ModelId, TokenUsage};

/// Internal message request -- each client converts it to its own API format internally
#[derive(Debug, Clone)]
pub struct MessageRequest<'a> {
    pub model: ModelId,
    pub messages: std::borrow::Cow<'a, [Message]>,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub tools: Vec<serde_json::Value>,
    pub temperature: Option<f32>,
}

/// Internal unified stream event -- each client maps its own SSE format to this enum
#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart { id: String },
    ContentBlockStart { index: usize, content_type: String },
    ContentDelta { index: usize, delta: String },
    ContentBlockStop { index: usize },
    MessageDelta { usage: TokenUsage },
    MessageStop,
    Error { message: String },
}
```

```rust
// lib.rs -- Enum dispatch (no dyn trait, determined at compile time, zero dynamic dispatch overhead)
use futures::stream::{self, Stream, StreamExt};
use either::Either;

/// LLM backend enum -- provider count is limited (2 standards + 2 cloud variants), enum is sufficient
/// If third-party provider extension is needed in the future, WASM plugin system can support it in Phase 2
pub enum LlmBackend {
    Anthropic(anthropic::AnthropicClient),
    OpenAi(openai::OpenAiClient),
    // Bedrock and Vertex are essentially different entry points for the Anthropic API, wrapping AnthropicClient
    #[cfg(feature = "bedrock")]
    Bedrock(anthropic::AnthropicClient),  // Different auth + base_url
    #[cfg(feature = "vertex")]
    Vertex(anthropic::AnthropicClient),   // Different auth + base_url
}

impl LlmBackend {
    /// Stream a message
    pub fn stream_message<'a>(
        &'a self,
        req: types::MessageRequest<'a>,
    ) -> impl Stream<Item = crab_common::Result<types::StreamEvent>> + Send + 'a {
        match self {
            Self::Anthropic(c) => Either::Left(c.stream(req)),
            Self::OpenAi(c) => Either::Right(c.stream(req)),
            // Bedrock/Vertex use the Anthropic path
        }
    }

    /// Non-streaming send (used for lightweight tasks like compaction)
    pub async fn send_message(
        &self,
        req: types::MessageRequest<'_>,
    ) -> crab_common::Result<(crab_core::message::Message, crab_core::model::TokenUsage)> {
        match self {
            Self::Anthropic(c) => c.send(req).await,
            Self::OpenAi(c) => c.send(req).await,
        }
    }

    /// Provider name
    pub fn name(&self) -> &str {
        match self {
            Self::Anthropic(_) => "anthropic",
            Self::OpenAi(_) => "openai",
        }
    }
}

/// Construct backend from configuration
pub fn create_backend(settings: &crab_config::Settings) -> LlmBackend {
    match settings.api_provider.as_deref() {
        Some("openai") | Some("ollama") | Some("deepseek") => {
            let base_url = settings.api_base_url.as_deref()
                .unwrap_or("https://api.openai.com/v1");
            let api_key = std::env::var("OPENAI_API_KEY").ok()
                .or_else(|| settings.api_key.clone());
            LlmBackend::OpenAi(openai::OpenAiClient::new(base_url, api_key))
        }
        _ => {
            let base_url = settings.api_base_url.as_deref()
                .unwrap_or("https://api.anthropic.com");
            let auth = crab_auth::create_auth_provider(settings);
            LlmBackend::Anthropic(anthropic::AnthropicClient::new(base_url, auth))
        }
    }
}
```

```rust
// anthropic/client.rs -- Anthropic Messages API (fully independent implementation)
pub struct AnthropicClient {
    http: reqwest::Client,
    base_url: String,
    auth: Box<dyn crab_auth::AuthProvider>,
}

impl AnthropicClient {
    pub fn new(base_url: &str, auth: Box<dyn crab_auth::AuthProvider>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build HTTP client");

        Self { http, base_url: base_url.to_string(), auth }
    }

    /// Streaming call -- POST /v1/messages, stream: true
    pub fn stream<'a>(
        &'a self,
        req: crate::types::MessageRequest<'a>,
    ) -> impl Stream<Item = crab_common::Result<crate::types::StreamEvent>> + Send + 'a {
        // 1. MessageRequest -> Anthropic native request (self::types::AnthropicRequest)
        // 2. POST /v1/messages, set stream: true
        // 3. Parse Anthropic SSE: message_start / content_block_delta / message_stop
        // 4. self::convert::to_stream_event() maps to internal StreamEvent
        // ...
    }

    /// Non-streaming call
    pub async fn send(
        &self,
        req: crate::types::MessageRequest<'_>,
    ) -> crab_common::Result<(crab_core::message::Message, crab_core::model::TokenUsage)> {
        // ...
    }
}
```

```rust
// openai/client.rs -- OpenAI Chat Completions API (fully independent implementation)
//
// Covers all backends compatible with /v1/chat/completions:
// OpenAI, Ollama, DeepSeek, vLLM, TGI, LiteLLM, Azure OpenAI, Google Gemini (OpenAI-compatible endpoint)
pub struct OpenAiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl OpenAiClient {
    pub fn new(base_url: &str, api_key: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build HTTP client");

        Self { http, base_url: base_url.to_string(), api_key }
    }

    /// Streaming call -- POST /v1/chat/completions, stream: true
    pub fn stream<'a>(
        &'a self,
        req: crate::types::MessageRequest<'a>,
    ) -> impl Stream<Item = crab_common::Result<crate::types::StreamEvent>> + Send + 'a {
        // 1. MessageRequest -> OpenAI native request (self::types::ChatCompletionRequest)
        //    - system prompt -> messages[0].role="system"
        //    - ContentBlock::ToolUse -> tool_calls array
        //    - ContentBlock::ToolResult -> role="tool" message
        // 2. POST /v1/chat/completions, stream: true
        // 3. Parse OpenAI SSE: data: {"choices":[{"delta":...}]}
        // 4. self::convert::to_stream_event() maps to internal StreamEvent
        // ...
    }

    /// Non-streaming call
    pub async fn send(
        &self,
        req: crate::types::MessageRequest<'_>,
    ) -> crab_common::Result<(crab_core::message::Message, crab_core::model::TokenUsage)> {
        // ...
    }
}
```

**Key Differences Between the Two API Standards** (handled by each client's `convert.rs` internally, not exposed to upper layers)

| Dimension | Anthropic Messages API | OpenAI Chat Completions API |
|-----------|----------------------|---------------------------|
| system prompt | Separate `system` field | `messages[0].role="system"` |
| Message content | `content: Vec<ContentBlock>` | `content: string` |
| Tool calls | `ContentBlock::ToolUse` | `tool_calls` array |
| Tool results | `ContentBlock::ToolResult` | `role="tool"` message |
| Streaming format | `content_block_delta` events | `choices[].delta` |
| Token stats | `input_tokens` / `output_tokens` | `prompt_tokens` / `completion_tokens` |
| Provider-specific | prompt cache, extended thinking | logprobs, structured output |

```rust
// rate_limit.rs -- Shared rate limiting and backoff
use std::time::Duration;

pub struct RateLimiter {
    pub remaining_requests: u32,
    pub remaining_tokens: u32,
    pub reset_at: std::time::Instant,
}

/// Exponential backoff strategy
pub fn backoff_delay(attempt: u32) -> Duration {
    let base = Duration::from_millis(500);
    let max = Duration::from_secs(30);
    let delay = base * 2u32.pow(attempt.min(6));
    delay.min(max)
}
```

**External Dependencies**: `reqwest`, `tokio`, `serde`, `eventsource-stream`, `futures`, `either`, `crab-core`, `crab-auth`, `crab-common`

**Feature Flags**

```toml
[features]
default = []
bedrock = ["aws-sdk-bedrockruntime", "aws-config"]
vertex = ["gcp-auth"]
proxy = ["reqwest/socks"]
```

---

### 6.6 `crates/mcp/` -- MCP Facade

**Responsibility**: Crab's own MCP facade and protocol adaptation layer (corresponds to CC `src/services/mcp/`)

MCP is an open protocol that lets LLMs connect to external tools/resources, based on JSON-RPC 2.0.
`crab-mcp` does not directly expose the underlying SDK to `cli` / `tools` / `session`; instead, it absorbs the official SDK internally and exposes a stable Crab-side interface: `McpClient`, `McpManager`, `ToolRegistryHandler`, `mcp__<server>__<tool>` naming, and config discovery logic all live in this layer.

**Directory Structure**

```
src/
├── lib.rs
├── protocol.rs             // Crab's own MCP facade types
├── client.rs               // MCP client facade (internally may delegate to rmcp)
├── server.rs               // MCP server facade (exposes own tools to external callers)
├── manager.rs              // Lifecycle management, multi-server coordination
├── transport/
│   ├── mod.rs              // Compatible Transport trait / local transport abstraction
│   ├── stdio.rs            // Legacy stdin/stdout transport
│   └── ws.rs               // WebSocket transport (feature = "ws")
├── resource.rs             // Resource caching, templates
├── discovery.rs            // Server auto-discovery
├── sse_server.rs           // SSE server transport (crab as MCP server)
├── sampling.rs             // MCP sampling (server requests LLM inference)
├── roots.rs                // MCP roots (workspace root directory declaration)
├── logging.rs              // MCP logging protocol (structured log messages)
├── handshake.rs            // Initialization handshake flow (initialize/initialized)
├── negotiation.rs          // Capability negotiation (client/server capability sets)
├── capability.rs           // Capability declaration types (resources/tools/prompts/sampling)
├── notification.rs         // Server notification push (tool changes/resource updates)
├── progress.rs             // Progress reporting (long-running tool execution)
├── cancellation.rs         // Request cancellation mechanism ($/cancelRequest)
└── health.rs               // Health check + heartbeat (auto-reconnect)
```

**Boundary Principles**

- `crab-mcp` exposes Crab's own facade types, not raw `rmcp` types
- Client-side stdio / HTTP connections preferably reuse the official `rmcp`
- The config layer only retains `stdio` / `http` / `ws` as transport options
- Upper-layer crates only depend on `crab_mcp::*`; they never directly depend on the underlying MCP SDK
- `protocol.rs` continues to carry Crab-side stable data structures, preventing SDK type leakage

**Core Types**

```rust
// protocol.rs -- JSON-RPC 2.0 messages
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String, // "2.0"
    pub id: u64,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}
```

```rust
// transport/mod.rs -- Transport abstraction
// Returns Pin<Box<dyn Future>> instead of native async fn because dyn Trait requires object safety
// (Box<dyn Transport> requires the trait to be object-safe; RPITIT's impl Future does not satisfy this)
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use std::future::Future;
use std::pin::Pin;

pub trait Transport: Send + Sync {
    /// Send a request and wait for a response
    fn send(&self, req: JsonRpcRequest) -> Pin<Box<dyn Future<Output = crab_common::Result<JsonRpcResponse>> + Send + '_>>;
    /// Send a notification (no response expected)
    fn notify(&self, method: &str, params: serde_json::Value) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>>;
    /// Close the transport
    fn close(&self) -> Pin<Box<dyn Future<Output = crab_common::Result<()>> + Send + '_>>;
}

// --- Transport implementation example ---
// impl Transport for StdioTransport {
//     fn send(&self, req: JsonRpcRequest) -> Pin<Box<dyn Future<Output = crab_common::Result<JsonRpcResponse>> + Send + '_>> {
//         Box::pin(async move {
//             self.write_message(&req).await?;
//             self.read_response().await
//         })
//     }
//     // ... notify, close similarly
// }
```

```rust
// client.rs -- MCP client facade
use crate::protocol::{McpToolDef, ServerCapabilities, ServerInfo};

pub struct McpClient {
    server_name: String,
    server_info: ServerInfo,
    capabilities: ServerCapabilities,
    tools: Vec<McpToolDef>,
}

impl McpClient {
    /// Connect to a stdio MCP server via the official SDK
    pub async fn connect_stdio(...) -> crab_common::Result<Self> { /* ... */ }

    /// Connect to an HTTP MCP endpoint via the official SDK
    pub async fn connect_streamable_http(...) -> crab_common::Result<Self> { /* ... */ }

    /// Call an MCP tool
    pub async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> crab_common::Result<serde_json::Value> {
        // ...
    }

    /// Read an MCP resource
    pub async fn read_resource(&self, uri: &str) -> crab_common::Result<String> {
        // ...
    }

    pub fn tools(&self) -> &[McpToolDef] {
        &self.tools
    }
}
```

**External Dependencies**: `tokio`, `serde`, `serde_json`, `rmcp`, `crab-core`, `crab-common`

**Feature Flags**

```toml
[features]
default = []
ws = ["tokio-tungstenite"]
```

---

### 6.7 `crates/fs/` -- File System Operations

**Responsibility**: Encapsulate all file system related operations (corresponds to the underlying logic of GlobTool/GrepTool/FileReadTool in CC)

**Directory Structure**

```
src/
├── lib.rs
├── glob.rs               // globset wrapper
├── grep.rs               // ripgrep core integration
├── gitignore.rs          // .gitignore rule parsing and filtering
├── watch.rs              // notify file watching (with debouncing + batch aggregation)
├── lock.rs               // File locking (fd-lock)
├── diff.rs               // similar wrapper, edit/patch generation
└── symlink.rs            // Symbolic link handling + secure path resolution (escape prevention)
```

**Core Interface**

```rust
// glob.rs -- File pattern matching
use std::path::{Path, PathBuf};

pub struct GlobResult {
    pub matches: Vec<PathBuf>,
    pub truncated: bool,
}

/// Search files in a directory by glob pattern
pub fn find_files(
    root: &Path,
    pattern: &str,
    limit: usize,
) -> crab_common::Result<GlobResult> {
    // Uses ignore crate (automatically respects .gitignore)
    // Sorted by modification time
    // ...
}

// grep.rs -- Content search
pub struct GrepMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

pub struct GrepOptions {
    pub pattern: String,
    pub path: PathBuf,
    pub case_insensitive: bool,
    pub file_glob: Option<String>,
    pub max_results: usize,
    pub context_lines: usize,
}

/// Search content in a directory by regex
pub fn search(opts: &GrepOptions) -> crab_common::Result<Vec<GrepMatch>> {
    // Uses grep-regex + grep-searcher
    // Automatically respects .gitignore
    // ...
}

// diff.rs -- Diff generation
pub struct EditResult {
    pub old_content: String,
    pub new_content: String,
    pub unified_diff: String,
}

/// Exact replacement based on old_string -> new_string
pub fn apply_edit(
    file_content: &str,
    old_string: &str,
    new_string: &str,
) -> crab_common::Result<EditResult> {
    // Uses similar to generate unified diff
    // ...
}
```

**External Dependencies**: `globset`, `grep-regex`, `grep-searcher`, `ignore`, `notify`, `similar`, `fd-lock`, `crab-common`

**Feature Flags**: None

---

### 6.8 `crates/process/` -- Subprocess Management

**Responsibility**: Subprocess lifecycle management (corresponds to the underlying execution logic of CC's BashTool)

**Directory Structure**

```
src/
├── lib.rs
├── spawn.rs              // Subprocess launching, environment inheritance
├── pty.rs                // Pseudo-terminal allocation (feature = "pty")
├── tree.rs               // Process tree kill (sysinfo)
├── signal.rs             // Signal handling, graceful shutdown
└── sandbox.rs            // Sandbox policy (feature = "sandbox")
```

**Core Interface**

```rust
// spawn.rs -- Subprocess execution
use std::path::Path;
use std::time::Duration;

pub struct SpawnOptions {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<std::path::PathBuf>,
    pub env: Vec<(String, String)>,
    pub timeout: Option<Duration>,
    pub stdin_data: Option<String>,
}

pub struct SpawnOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Execute a command and wait for the result
pub async fn run(opts: SpawnOptions) -> crab_common::Result<SpawnOutput> {
    use tokio::process::Command;
    // 1. Build Command
    // 2. Set working_dir, env
    // 3. Wrap with tokio::time::timeout if timeout is set
    // 4. Collect stdout/stderr
    // ...
}

/// Execute a command and stream output
pub async fn run_streaming(
    opts: SpawnOptions,
    on_stdout: impl Fn(&str) + Send,
    on_stderr: impl Fn(&str) + Send,
) -> crab_common::Result<i32> {
    // ...
}

// tree.rs -- Process tree management
/// Kill a process and all its child processes
pub fn kill_tree(pid: u32) -> crab_common::Result<()> {
    use sysinfo::{Pid, System};
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    // Recursively find child processes and kill
    // ...
}

// signal.rs -- Signal handling
/// Register Ctrl+C / SIGTERM handler
pub fn register_shutdown_handler(
    on_shutdown: impl Fn() + Send + 'static,
) {
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        on_shutdown();
    });
}
```

**External Dependencies**: `tokio` (process, signal), `sysinfo`, `crab-common`

**Feature Flags**

```toml
[features]
default = []
pty = ["portable-pty"]
sandbox = []
```

---

### 6.9 `crates/tools/` -- Tool System

**Responsibility**: Tool registration, lookup, execution, including all built-in tools (corresponds to CC `src/tools/`)

**Directory Structure**

```
src/
├── lib.rs
├── registry.rs       // ToolRegistry: registration, lookup, schema generation
├── executor.rs       // Unified executor with permission checking
├── permission.rs     // Tool permission checking logic
│
├── builtin/          // Built-in tools
│   ├── mod.rs        // register_all_builtins()
│   ├── bash.rs       // BashTool -- shell command execution
│   ├── read.rs       // ReadTool -- file reading
│   ├── edit.rs       // EditTool -- diff-based file editing
│   ├── write.rs      // WriteTool -- file creation/overwrite
│   ├── glob.rs       // GlobTool -- file pattern matching
│   ├── grep.rs       // GrepTool -- content search
│   ├── web_search.rs // WebSearchTool -- web search
│   ├── web_fetch.rs  // WebFetchTool -- web page fetching
│   ├── agent.rs      // AgentTool -- sub-Agent launching
│   ├── notebook.rs   // NotebookTool -- Jupyter support
│   ├── task.rs       // TaskCreate/Get/List/Update/Stop/Output
│   ├── mcp_tool.rs   // MCP tool Tool trait adapter
│   ├── lsp.rs        // LSP integration tool
│   ├── worktree.rs   // Git Worktree tool
│   ├── ask_user.rs   // User interaction tool
│   ├── image_read.rs // Image reading tool
│   ├── read_enhanced.rs // Enhanced file reading
│   ├── bash_security.rs // Bash security checks
│   ├── plan_mode.rs  // Plan mode tool
│   ├── plan_file.rs  // Plan file operations
│   ├── plan_approval.rs // Plan approval tool
│   ├── web_cache.rs  // Web page cache
│   └── web_formatter.rs // Web page formatter
│
└── schema.rs         // Tool schema -> API tools parameter conversion
```

**Core Types**

```rust
// registry.rs -- Tool registry
use crab_core::tool::Tool;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Find by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Get JSON Schema for all tools (for API requests)
    pub fn tool_schemas(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "input_schema": t.input_schema(),
                })
            })
            .collect()
    }

    /// List all tool names
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
}
```

```rust
// executor.rs -- Unified executor
use crab_core::tool::{Tool, ToolContext, ToolOutput};
use crate::registry::ToolRegistry;
use std::sync::Arc;

pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
}

impl ToolExecutor {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// Execute a tool (with permission checking)
    ///
    /// **Permission decision matrix** (mode x tool_type x path_scope):
    ///
    /// | PermissionMode | read_only | write(in project) | write(outside project) | dangerous | mcp_external | agent_spawn | denied_list |
    /// |----------------|-----------|-------------------|----------------------|-----------|-------------|-------------|-------------|
    /// | Default        | Allow     | **Prompt**        | **Prompt**           | **Prompt**| **Prompt**  | **Prompt**  | **Deny**    |
    /// | TrustProject   | Allow     | Allow             | **Prompt**           | **Prompt**| **Prompt**  | Allow       | **Deny**    |
    /// | Dangerously    | Allow     | Allow             | Allow                | Allow     | Allow       | Allow       | **Deny**    |
    ///
    /// - denied_list is denied in all modes (from settings.json `deniedTools`)
    /// - allowed_list match skips normal Prompt (but does not exempt dangerous detection)
    /// - dangerous = BashTool contains `rm -rf`/`sudo`/`curl|sh`/`chmod`/`eval` and other high-risk patterns
    /// - mcp_external: tools provided by external MCP servers; Default/TrustProject both require Prompt (untrusted source)
    /// - agent_spawn: sub-Agent creation; TrustProject auto-approves; sub-Agents inherit parent Agent's permission_mode
    pub async fn execute(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> crab_common::Result<ToolOutput> {
        let tool = self
            .registry
            .get(tool_name)
            .ok_or_else(|| crab_common::Error::Other(
                format!("tool not found: {tool_name}"),
            ))?;

        // 1. Check denied list -- denied in all modes
        //    denied_tools supports glob matching (e.g., "mcp__*", "bash")
        //    Uses globset for pattern matching, supporting * / ? / [abc] glob syntax
        if ctx.permission_policy.denied_tools.iter().any(|pattern| {
            globset::Glob::new(pattern)
                .ok()
                .and_then(|g| g.compile_matcher().is_match(tool_name).then_some(()))
                .is_some()
        }) {
            return Ok(ToolOutput::error(format!("tool '{tool_name}' is denied by policy")));
        }

        // 2. Dangerously mode short-circuit -- skip all permission checks (including allowed_tools and dangerous detection)
        //    Placed after denied_tools: even in Dangerously mode, denied_tools still applies
        if ctx.permission_mode == PermissionMode::Dangerously {
            return tool.execute(input, ctx).await;
        }

        // 3. Check allowed list -- explicitly allowed skips prompt
        let explicitly_allowed = ctx.permission_policy.allowed_tools.contains(&tool_name.to_string());

        // 4. Decide by matrix (combining tool.source() + mode + path_scope)
        // allowed_tools only exempts normal Prompt, not dangerous detection
        let needs_prompt = if explicitly_allowed {
            self.is_dangerous_command(&input) // allowed_tools only exempts normal Prompt, not dangerous detection
        } else {
            match tool.source() {
                // MCP external tools: Default/TrustProject both require Prompt (untrusted source)
                ToolSource::McpExternal { .. } => true,
                // Sub-Agent creation: TrustProject auto-approves, Default requires Prompt
                ToolSource::AgentSpawn => {
                    ctx.permission_mode == PermissionMode::Default
                }
                // Built-in tools: follow the original matrix
                ToolSource::BuiltIn => {
                    match ctx.permission_mode {
                        PermissionMode::Dangerously => unreachable!(), // Already short-circuited above
                        PermissionMode::TrustProject => {
                            if tool.is_read_only() {
                                false
                            } else {
                                let in_project = self.is_path_in_project(tool_name, &input, &ctx.working_dir);
                                !in_project || self.is_dangerous_command(&input)
                            }
                        }
                        PermissionMode::Default => {
                            !tool.is_read_only()
                        }
                    }
                }
            }
        };

        if needs_prompt {
            // Request user confirmation via event channel
            let approved = self.request_permission(tool_name, &input, ctx).await?;
            if !approved {
                return Ok(ToolOutput::error("user denied permission"));
            }
        }

        tool.execute(input, ctx).await
    }

    /// Check whether the tool operation path is within the project directory
    ///
    /// **TOCTOU + symlink protection**:
    /// - Uses `std::fs::canonicalize()` to resolve symbolic links before comparison, preventing symlink bypass
    /// - File operations should use `O_NOFOLLOW` (or Rust equivalent) to prevent TOCTOU race conditions
    /// - Note: canonicalize only works on existing paths; non-existent paths need parent directory canonicalization
    fn is_path_in_project(&self, tool_name: &str, input: &serde_json::Value, project_dir: &std::path::Path) -> bool {
        // BashTool special handling: input contains "command" not "file_path"
        // Need to parse possible path references from the command string
        if tool_name == "bash" {
            return self.bash_paths_in_project(input, project_dir);
        }

        // Other tools: extract file_path/path field from input
        input.get("file_path")
            .or_else(|| input.get("path"))
            .and_then(|v| v.as_str())
            .map(|p| {
                let raw = std::path::Path::new(p);
                // Canonicalize first to resolve symlinks, preventing symlink bypass of project boundary
                // Fallback for non-existent paths: canonicalize nearest existing ancestor directory + remaining relative segments
                let resolved = std::fs::canonicalize(raw).unwrap_or_else(|_| {
                    // Path does not exist yet (e.g., a new file about to be created), walk up to find existing ancestor
                    let mut ancestor = raw.to_path_buf();
                    let mut suffix = std::path::PathBuf::new();
                    while !ancestor.exists() {
                        if let Some(file_name) = ancestor.file_name() {
                            suffix = std::path::Path::new(file_name).join(&suffix);
                        }
                        if !ancestor.pop() { break; }
                    }
                    std::fs::canonicalize(&ancestor)
                        .unwrap_or(ancestor)
                        .join(suffix)
                });
                resolved.starts_with(project_dir)
            })
            .unwrap_or(true) // Tools without path parameters default to being considered in-project
    }

    /// BashTool path detection: extract absolute paths from the command string and check them
    /// In TrustProject mode, commands referencing absolute paths outside the project require Prompt
    ///
    /// **Important: This is best-effort heuristic detection**
    /// Path extraction from shell commands cannot be 100% accurate (variable expansion, subshells, nested quotes, etc.).
    /// Conservative strategy: when path analysis is uncertain, return Uncertain -> maps to Prompt.
    /// Specific scenarios:
    /// - Cannot extract any path tokens -> Uncertain (variables/subshells may reference paths)
    /// - Contains shell metacharacters ($, `, $(...)) -> Uncertain (paths may be dynamically constructed)
    ///
    /// **Core principle: when reliable parsing is impossible, default to requiring Prompt -- better to ask too much than miss.**
    fn bash_paths_in_project(&self, input: &serde_json::Value, project_dir: &std::path::Path) -> bool {
        let cmd = input.get("command").and_then(|v| v.as_str()).unwrap_or("");

        // Conservative strategy: cannot reliably extract paths with shell metacharacters, return false (require Prompt)
        let shell_metacharacters = ['$', '`'];
        if cmd.chars().any(|c| shell_metacharacters.contains(&c)) || cmd.contains("$(") {
            return false; // Uncertain -> maps to Prompt
        }

        // cd to absolute path changes the working directory for subsequent commands, treated as path reference
        // e.g., `cd /etc && cat passwd` actually operates on files outside the project
        if cmd.starts_with("cd ") || cmd.contains("&& cd ") || cmd.contains("; cd ") || cmd.contains("|| cd ") {
            // Extract cd target path and check if it's within the project
            for segment in cmd.split("&&").chain(cmd.split(";")).chain(cmd.split("||")) {
                let trimmed = segment.trim();
                if trimmed.starts_with("cd ") {
                    let target = trimmed.strip_prefix("cd ").unwrap().trim();
                    if target.starts_with('/') || target.starts_with("~/") {
                        let expanded = if target.starts_with("~/") {
                            crab_common::path::home_dir().join(&target[2..])
                        } else {
                            std::path::PathBuf::from(target)
                        };
                        if !expanded.starts_with(project_dir) {
                            return false; // cd to outside project -> Prompt
                        }
                    }
                }
            }
        }

        // Extract all absolute path tokens from the command
        let abs_paths: Vec<&str> = cmd.split_whitespace()
            .filter(|token| token.starts_with('/') || token.starts_with("~/"))
            .collect();

        // Conservative strategy: return false when no paths can be extracted (Uncertain -> Prompt)
        // Note: pure relative path commands (e.g., `cargo build`) don't have / prefix, will reach here
        // But these commands are usually safe in-project operations, so still return true
        if abs_paths.is_empty() {
            return true;
        }

        // Any absolute path outside the project -> return false (require Prompt)
        abs_paths.iter().all(|p| {
            let expanded = if p.starts_with("~/") {
                crab_common::path::home_dir().join(&p[2..])
            } else {
                std::path::PathBuf::from(p)
            };
            expanded.starts_with(project_dir)
        })
    }

    /// Detect dangerous command patterns
    /// Covers: destructive operations, privilege escalation, remote code execution, file overwrite, chained dangerous commands
    ///
    /// **Important: all pattern matching must use a shell tokenizer to exclude quoted content**
    /// All `cmd.contains(pattern)` below should be replaced with tokenize-then-match in actual implementation:
    /// 1. Use `shell-words` crate (or equivalent tokenizer) to split cmd into tokens
    /// 2. Only match dangerous patterns in non-quoted tokens
    /// 3. Example: `echo "rm -rf /" > log.txt` should NOT trigger `rm -rf` detection (inside quotes)
    ///    but `> log.txt` redirect is outside quotes and should be detected normally
    /// 4. When tokenizer fails (e.g., unclosed quotes), handle conservatively -> treat as dangerous
    fn is_dangerous_command(&self, input: &serde_json::Value) -> bool {
        let cmd = input.get("command").and_then(|v| v.as_str()).unwrap_or("");

        // 1. Direct dangerous patterns
        // Two-tier strategy: Level 1 exact match (list below) + Level 2 heuristic detection (interpreter + -c/-e combos)
        let dangerous_patterns = [
            // -- Destructive file operations --
            "rm -rf", "rm -fr",
            // -- Privilege escalation --
            "sudo ",
            // -- Disk/device operations --
            "mkfs", "dd if=", "> /dev/",
            // -- Remote code execution (pipe to shell) --
            "curl|sh", "curl|bash", "wget|sh", "wget|bash",
            "curl | sh", "curl | bash", "wget | sh", "wget | bash",
            // -- Permission modification --
            "chmod ", "chown ",
            // -- Dynamic execution (can bypass static detection) --
            "eval ", "exec ", "source ",
            // -- Interpreter inline execution (Level 1: exact match interpreter + -c/-e) --
            "python -c", "python3 -c", "perl -e", "node -e", "ruby -e",
            // -- Dangerous batch operations --
            "xargs ",      // xargs + dangerous target (e.g., xargs rm)
            "crontab",     // Cron job modification
            "nohup ",      // Background persistent execution
            // -- File overwrite redirect --
            // (Quote exclusion logic handled by function-level tokenizer, see function header comment)
            "> ",   // Overwrite redirect
            ">> ",  // Append redirect (writing to sensitive files like .bashrc)
        ];

        // Level 2 heuristic: `find` + `-exec` combo detection
        if cmd.contains("find ") && (cmd.contains("-exec") || cmd.contains("-execdir")) {
            return true;
        }

        // 2. Check direct patterns
        if dangerous_patterns.iter().any(|p| cmd.contains(p)) {
            return true;
        }

        // 3. Check pipe to dangerous commands (e.g., `cat file | sudo tee`, `echo x | sh`)
        let pipe_dangerous_targets = ["sh", "bash", "sudo", "tee", "eval", "exec"];
        if cmd.contains('|') {
            let segments: Vec<&str> = cmd.split('|').collect();
            for seg in &segments[1..] {
                let target = seg.trim().split_whitespace().next().unwrap_or("");
                if pipe_dangerous_targets.contains(&target) {
                    return true;
                }
            }
        }

        // 4. Check for dangerous commands in && / || chains
        let chain_ops = ["&&", "||", ";"];
        for op in &chain_ops {
            if cmd.contains(op) {
                for sub_cmd in cmd.split(op) {
                    let first_word = sub_cmd.trim().split_whitespace().next().unwrap_or("");
                    if ["rm", "sudo", "mkfs", "dd", "chmod", "chown", "eval", "exec"].contains(&first_word) {
                        return true;
                    }
                }
            }
        }

        false
    }
}
```

**CC Tool Mapping Table (CC has 52 tools; below are the core mappings)**

| CC Tool | Crab Tool | File |
|---------|----------|------|
| BashTool | BashTool | `bash.rs` |
| FileReadTool | ReadTool | `read.rs` |
| FileEditTool | EditTool | `edit.rs` |
| FileWriteTool | WriteTool | `write.rs` |
| GlobTool | GlobTool | `glob.rs` |
| GrepTool | GrepTool | `grep.rs` |
| WebSearchTool | WebSearchTool | `web_search.rs` |
| WebFetchTool | WebFetchTool | `web_fetch.rs` |
| AgentTool | AgentTool | `agent.rs` |
| NotebookEditTool | NotebookTool | `notebook.rs` |
| TaskCreateTool | TaskCreateTool | `task.rs` |
| MCPTool | McpToolAdapter | `mcp_tool.rs` |

**External Dependencies**: `crab-core`, `crab-fs`, `crab-process`, `crab-mcp`, `crab-config`, `crab-common`

**Feature Flags**: None

---

### 6.10 `crates/session/` -- Session Management

**Responsibility**: State management for multi-turn conversations (corresponds to CC `src/services/compact/` + `src/services/SessionMemory/` + `src/services/sessionTranscript/`). Memory system extracted to `crates/memory/`; session re-exports core memory types.

**Directory Structure**

```
src/
├── lib.rs
├── conversation.rs    // Conversation state machine, multi-turn management
├── context.rs         // Context window management, auto-compaction trigger
├── compaction.rs      // Message compaction strategies (5 levels: Snip/Microcompact/Summarize/Hybrid/Truncate)
├── history.rs         // Session persistence, recovery, search, export, statistics
├── memory.rs          // Re-exports from crab-memory (MemoryStore, MemoryFile, etc.)
├── memory_extract.rs  // Conversation → memory extraction (heuristic, depends on crab-core::Message)
├── cost.rs            // Token counting, cost tracking
└── template.rs        // Session template + quick recovery
```

**Core Types**

```rust
// conversation.rs -- Conversation state machine
use crab_core::message::Message;
use crab_core::model::TokenUsage;

pub struct Conversation {
    /// Session ID
    pub id: String,
    /// System prompt
    pub system_prompt: String,
    /// Message history
    pub messages: Vec<Message>,
    /// Cumulative token usage
    pub total_usage: TokenUsage,
    /// Context window limit
    pub context_window: u64,
}

impl Conversation {
    pub fn new(id: String, system_prompt: String, context_window: u64) -> Self {
        Self {
            id,
            system_prompt,
            messages: Vec::new(),
            total_usage: TokenUsage::default(),
            context_window,
        }
    }

    /// Append a message
    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Estimate current token count
    ///
    /// **Current**: Rough estimate of text_len/4 (error margin +/-30%), suitable for MVP phase
    /// **Future**: Integrate tiktoken-rs for precise counting (Claude tokenizer is compatible with cl100k_base)
    ///
    /// ```rust
    /// // TODO(M2+): Replace with precise counting
    /// // use tiktoken_rs::cl100k_base;
    /// // let bpe = cl100k_base().unwrap();
    /// // bpe.encode_with_special_tokens(text).len() as u64
    /// ```
    pub fn estimated_tokens(&self) -> u64 {
        let text_len: usize = self.messages.iter().map(|m| {
            m.content.iter().map(|c| match c {
                crab_core::message::ContentBlock::Text { text } => text.len(),
                _ => 100, // Fixed estimate for tool calls
            }).sum::<usize>()
        }).sum();
        (text_len / 4) as u64 // Temporary: +/-30% error margin
    }

    /// Whether compaction is needed
    pub fn needs_compaction(&self) -> bool {
        self.estimated_tokens() > self.context_window * 80 / 100
    }
}

// compaction.rs -- 5-level compaction strategy (progressively triggered by token usage rate)
pub enum CompactionStrategy {
    /// Level 1 (70-80%): Trim full output of old tool calls, keeping only summary lines
    Snip,
    /// Level 2 (80-85%): Replace large results (>500 tokens) with AI-generated single-line summary
    Microcompact,
    /// Level 3 (85-90%): Summarize old messages using a small model
    Summarize,
    /// Level 4 (90-95%): Keep recent N turns + summarize the rest
    Hybrid { keep_recent: usize },
    /// Level 5 (>95%): Emergency truncation, discard oldest messages
    Truncate,
}

use std::future::Future;
use std::pin::Pin;

/// Compaction client abstraction -- decouples compaction logic from specific API client
/// Facilitates testing (mock) and swapping different LLM providers
pub trait CompactionClient: Send + Sync {
    /// Send a compaction/summary request, return summary text
    fn summarize(
        &self,
        messages: &[crab_core::message::Message],
        instruction: &str,
    ) -> Pin<Box<dyn Future<Output = crab_common::Result<String>> + Send + '_>>;
}

// LlmBackend adapts to CompactionClient via enum dispatch (in crab-api)
// impl CompactionClient for LlmBackend { ... }

pub async fn compact(
    conversation: &mut Conversation,
    strategy: CompactionStrategy,
    client: &impl CompactionClient,
) -> crab_common::Result<()> {
    // Compact messages according to strategy, using client.summarize() to generate summaries
    // ...
}

// memory.rs -- Memory system
pub struct MemoryStore {
    pub path: std::path::PathBuf, // ~/.crab-code/memory/
}

impl MemoryStore {
    /// Save session memory
    pub fn save(&self, session_id: &str, content: &str) -> crab_common::Result<()> {
        // ...
    }

    /// Load session memory
    pub fn load(&self, session_id: &str) -> crab_common::Result<Option<String>> {
        // ...
    }
}
```

**External Dependencies**: `crab-core`, `crab-api`, `crab-config`, `tokio`, `serde_json`, `crab-common`

**Feature Flags**: None

---

### 6.11 `crates/agent/` -- Multi-Agent System

**Responsibility**: Agent orchestration, task dispatch, message loop (corresponds to CC `src/query.ts` + `src/QueryEngine.ts` + `src/coordinator/` + `src/tasks/`)

This is the **core engine** of the entire system, implementing the most critical query loop.

**Directory Structure**

```
src/
├── lib.rs
├── coordinator.rs        // Agent orchestration, workers pool
├── query_loop.rs         // Core message loop (the most important file)
├── task.rs               // TaskList, TaskUpdate, dependency graph
├── team.rs               // Team creation, member management
├── message_bus.rs        // Inter-Agent messaging (tokio::mpsc)
├── message_router.rs     // Inter-Agent message routing (by name/broadcast)
├── worker.rs             // Sub-Agent worker lifecycle
├── system_prompt.rs      // System prompt building + CRAB.md injection
├── summarizer.rs         // Conversation summary generation
├── rollback.rs           // Rollback mechanism (/undo)
├── error_recovery.rs     // Error recovery strategy (auto-retry/degradation)
├── retry.rs              // Auto-retry mechanism (exponential backoff)
└── repl_commands.rs      // REPL commands (/undo /branch /fork /checkpoint)
```

**Message Loop (Core)**

```rust
// query_loop.rs -- Core message loop
// Corresponds to CC src/query.ts query() function
use crab_core::event::Event;
use crab_core::message::{ContentBlock, Message};
use crab_session::Conversation;
use crab_tools::executor::ToolExecutor;
use crab_api::LlmBackend;
use tokio::sync::mpsc;

/// Message loop: user input -> API -> tool execution -> continue -> until no tool calls
pub async fn query_loop(
    conversation: &mut Conversation,
    api: &LlmBackend,
    tools: &ToolExecutor,
    event_tx: mpsc::Sender<Event>,
) -> crab_common::Result<()> {
    loop {
        // 1. Check if context needs compaction
        if conversation.needs_compaction() {
            // -> See [session#compaction]
            todo!("compact conversation");
        }

        // 2. Build API request (borrow messages to avoid clone)
        let req = crab_api::MessageRequest {
            model: crab_core::model::ModelId("claude-sonnet-4-20250514".into()),
            messages: std::borrow::Cow::Borrowed(&conversation.messages),
            system: Some(conversation.system_prompt.clone()),
            max_tokens: 16384,
            tools: tools.registry().tool_schemas(),
            temperature: None,
        };

        // 3. Stream to API
        let mut stream = api.stream_message(req);

        // 4. Collect assistant response
        let mut assistant_content: Vec<ContentBlock> = Vec::new();
        let mut has_tool_use = false;

        // (streaming processing details omitted, collecting ContentBlocks)
        // ...

        // 5. Add assistant message to conversation
        conversation.push(Message {
            role: crab_core::message::Role::Assistant,
            content: assistant_content.clone(),
        });

        // 6. If no tool calls, loop ends
        if !has_tool_use {
            break;
        }

        // 7. Partition tool calls by read/write and execute concurrently
        //    Read tools (is_read_only=true) use FuturesUnordered concurrently (max 10)
        //    Write tools execute serially to ensure ordering consistency
        let tool_calls: Vec<_> = assistant_content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse { id, name, input } => Some((id, name, input)),
                _ => None,
            })
            .collect();

        let (read_tools, write_tools) = partition_tools(&tool_calls, &tools);

        let cancel = tokio_util::sync::CancellationToken::new();
        let ctx = crab_core::tool::ToolContext {
            working_dir: std::env::current_dir()?,
            permission_mode: crab_core::permission::PermissionMode::Default,
            session_id: conversation.id.clone(),
            cancellation_token: cancel.clone(),
            permission_policy: crab_core::permission::PermissionPolicy {
                mode: crab_core::permission::PermissionMode::Default,
                allowed_tools: Vec::new(),
                denied_tools: Vec::new(),
            },
        };

        // 7a. Execute read tools concurrently (max 10 concurrent)
        let mut tool_results: Vec<ContentBlock> = Vec::new();
        {
            use futures::stream::{FuturesUnordered, StreamExt};
            let mut futures = FuturesUnordered::new();
            let semaphore = Arc::new(tokio::sync::Semaphore::new(10));

            for (id, name, input) in &read_tools {
                let permit = semaphore.clone().acquire_owned().await?;
                let id = (*id).clone();
                let name = (*name).clone();
                let input = (*input).clone();
                let tools = tools.clone();
                let ctx = ctx.clone();
                let event_tx = event_tx.clone();
                futures.push(tokio::spawn(async move {
                    event_tx.send(Event::ToolUseStart {
                        id: id.clone(), name: name.clone(),
                    }).await.ok();
                    let output = tools.execute(&name, input, &ctx).await;
                    drop(permit);
                    (id, output)
                }));
            }
            while let Some(result) = futures.next().await {
                let (id, output) = result?;
                let output = output?;
                event_tx.send(Event::ToolResult {
                    id: id.clone(), content: output.text(), is_error: output.is_error,
                }).await.ok();
                tool_results.push(ContentBlock::ToolResult {
                    tool_use_id: id, content: output.text(), is_error: output.is_error,
                });
            }
        }

        // 7b. Execute write tools serially
        for (id, name, input) in &write_tools {
            event_tx.send(Event::ToolUseStart {
                id: (*id).clone(), name: (*name).clone(),
            }).await.ok();
            let output = tools.execute(name, (*input).clone(), &ctx).await?;
            event_tx.send(Event::ToolResult {
                id: (*id).clone(), content: output.text(), is_error: output.is_error,
            }).await.ok();
            tool_results.push(ContentBlock::ToolResult {
                tool_use_id: (*id).clone(), content: output.text(), is_error: output.is_error,
            });
        }

        // 8. Add tool results as a user message to the conversation
        conversation.push(Message {
            role: crab_core::message::Role::User,
            content: tool_results,
        });

        // 9. Return to step 1, continue loop
    }

    Ok(())
}
```

```rust
/// Partition tool calls into read/write groups by is_read_only()
fn partition_tools<'a>(
    calls: &[(&'a String, &'a String, &'a serde_json::Value)],
    executor: &ToolExecutor,
) -> (
    Vec<(&'a String, &'a String, &'a serde_json::Value)>,
    Vec<(&'a String, &'a String, &'a serde_json::Value)>,
) {
    let mut reads = Vec::new();
    let mut writes = Vec::new();
    for &(id, name, input) in calls {
        if executor.registry().get(name).map_or(false, |t| t.is_read_only()) {
            reads.push((id, name, input));
        } else {
            writes.push((id, name, input));
        }
    }
    (reads, writes)
}
```

```rust
// coordinator.rs -- Multi-Agent orchestration
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Running worker handle
pub struct RunningWorker {
    pub worker_id: String,
    pub cancel: CancellationToken,
    pub handle: tokio::task::JoinHandle<WorkerResult>,
}

/// Multi sub-agent orchestrator
pub struct AgentCoordinator {
    backend: Arc<LlmBackend>,
    executor: Arc<ToolExecutor>,
    tool_ctx: ToolContext,
    loop_config: QueryLoopConfig,
    event_tx: mpsc::Sender<Event>,
    running: HashMap<String, RunningWorker>,
    completed: Vec<WorkerResult>,           // Summary (without conversation history)
    cancel: CancellationToken,
}

impl AgentCoordinator {
    /// Spawn a new sub-agent worker
    pub async fn spawn_worker(
        &mut self,
        config: WorkerConfig,
        task_prompt: String,
    ) -> crab_common::Result<String>;

    /// Wait for a specific worker to complete
    pub async fn wait_for(&mut self, worker_id: &str) -> Option<WorkerResult>;

    /// Wait for all workers to complete
    pub async fn wait_all(&mut self) -> Vec<WorkerResult>;

    /// Cancel a specific worker
    pub fn cancel_worker(&mut self, worker_id: &str) -> bool;

    /// Cancel all workers
    pub fn cancel_all(&mut self);
}

// worker.rs -- Sub-agent worker lifecycle
pub struct WorkerConfig {
    pub worker_id: String,
    pub system_prompt: String,
    pub max_turns: Option<usize>,
    pub max_duration: Option<std::time::Duration>,
    pub context_window: u64,
}

pub struct WorkerResult {
    pub worker_id: String,
    pub output: Option<String>,             // Last assistant text message
    pub success: bool,
    pub usage: TokenUsage,
    pub conversation: Conversation,         // Full conversation history
}

pub struct AgentWorker {
    config: WorkerConfig,
    backend: Arc<LlmBackend>,
    executor: Arc<ToolExecutor>,
    tool_ctx: ToolContext,
    loop_config: QueryLoopConfig,
    event_tx: mpsc::Sender<Event>,
    cancel: CancellationToken,
}

impl AgentWorker {
    /// Run an independent query loop in a tokio task
    pub fn spawn(self, task_prompt: String) -> tokio::task::JoinHandle<WorkerResult>;
}
```

**Task System (Implemented)**

```rust
// task.rs -- TaskStore + TaskItem + dependency graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub active_form: Option<String>,
    pub status: TaskStatus,
    pub owner: Option<String>,
    pub metadata: serde_json::Value,
    pub blocks: Vec<String>,         // Tasks blocked by this task
    pub blocked_by: Vec<String>,     // Tasks that block this task
}

/// Thread-safe task store with dependency graph support
pub struct TaskStore {
    items: HashMap<String, TaskItem>,
    next_id: usize,
}

impl TaskStore {
    pub fn create(&mut self, subject: String, description: String, ...) -> TaskItem;
    pub fn get(&self, id: &str) -> Option<&TaskItem>;
    pub fn list(&self) -> Vec<&TaskItem>;
    pub fn update(&mut self, id: &str, ...) -> Option<String>;
    pub fn add_dependency(&mut self, task_id: &str, blocked_by_id: &str);
}
```

**Streaming Tool Execution (StreamingToolExecutor)**

CC starts tool execution immediately once the `tool_use` JSON is fully parsed during API streaming,
without waiting for the `message_stop` event. Crab Code should implement the same optimization:

```
API SSE stream:  [content_block_start: tool_use] -> [input_json_delta...] -> [content_block_stop]
                                                                                  |
                                                JSON complete -> spawn immediately -+
                                                               |
Subsequent blocks continue streaming <---- parallel with tool execution ------->| tool result ready
```

```rust
/// Streaming tool executor -- starts tools early during API streaming
pub struct StreamingToolExecutor {
    pending: Vec<tokio::task::JoinHandle<(String, crab_common::Result<ToolOutput>)>>,
}

impl StreamingToolExecutor {
    /// Called immediately when a tool_use block's JSON is fully parsed
    pub fn spawn_early(&mut self, id: String, name: String, input: Value, ctx: ToolContext, executor: Arc<ToolExecutor>) {
        let handle = tokio::spawn(async move {
            let result = executor.execute(&name, input, &ctx).await;
            (id, result)
        });
        self.pending.push(handle);
    }

    /// After message_stop, collect all completed/in-progress tool results
    pub async fn collect_all(&mut self) -> Vec<(String, crab_common::Result<ToolOutput>)> {
        let mut results = Vec::new();
        for handle in self.pending.drain(..) {
            results.push(handle.await.expect("tool task panicked"));
        }
        results
    }
}
```

**External Dependencies**: `crab-core`, `crab-session`, `crab-tools`, `crab-api`, `tokio`, `tokio-util`, `futures`, `crab-common`

**Feature Flags**: None

---

### 6.12 `crates/tui/` -- Terminal UI

**Responsibility**: All terminal interface rendering (corresponds to CC `src/components/` + `src/screens/` + `src/ink/` + `src/vim/`)

CC uses React/Ink to render the terminal UI; Crab uses ratatui + crossterm to achieve equivalent experience.

**Directory Structure**

```
src/
├── lib.rs
├── app.rs                  // App state machine, main loop
├── event.rs                // crossterm Event -> AppEvent mapping (KeyEvent/MouseEvent/Resize)
├── layout.rs               // Layout calculation (panel allocation, responsive)
├── runner.rs               // TUI runner (initialize/start/stop terminal)
├── keybindings.rs          // Keybinding configuration (user-customizable)
├── ansi.rs                 // ANSI escape -> ratatui Span conversion
│
├── components/             // UI components (21)
│   ├── mod.rs
│   ├── input.rs            // Multi-line input box + Vim motion
│   ├── markdown.rs         // Markdown rendering (pulldown-cmark -> ratatui)
│   ├── syntax.rs           // Code highlighting (syntect -> ratatui Style)
│   ├── spinner.rs          // Loading indicator (thinking/executing)
│   ├── diff.rs             // Diff visualization (unified red/green comparison)
│   ├── select.rs           // Selection list (tool confirmation/slash commands)
│   ├── dialog.rs           // Confirmation/permission dialog
│   ├── cost_bar.rs         // Token/cost status bar
│   ├── task_list.rs        // Task progress panel
│   ├── autocomplete.rs     // Autocomplete popup (triggered on input)
│   ├── code_block.rs       // Code block + one-click copy button
│   ├── command_palette.rs  // Command palette (Ctrl+P, fuzzy search all commands)
│   ├── input_history.rs    // Input history (up/down arrow key browsing)
│   ├── loading.rs          // Loading animation component (multiple animation styles)
│   ├── notification.rs     // Toast notification system (top popup/auto-dismiss)
│   ├── progress_indicator.rs // Progress indicator (percentage + progress bar)
│   ├── search.rs           // Global search panel (content/filename search)
│   ├── shortcut_hint.rs    // Shortcut hint bar (always visible at bottom)
│   ├── status_bar.rs       // Enhanced status bar (mode/provider/token/latency)
│   └── tool_output.rs      // Collapsible tool output display (expandable/collapsible)
│
├── vim/                    // Vim mode
│   ├── mod.rs
│   ├── motion.rs           // hjkl, w/b/e, 0/$, gg/G
│   ├── operator.rs         // d/c/y + motion
│   └── mode.rs             // Normal/Insert/Visual
│
└── theme.rs                // Color theme (dark/light/solarized/customizable)
```

**App Main Loop**

```rust
// app.rs -- ratatui App
use ratatui::prelude::*;
use crossterm::event::{self, Event as TermEvent, KeyCode};
use crab_core::event::Event;
use tokio::sync::mpsc;

/// App-level shared resources (initialized once, avoid rebuilding on each render)
pub struct SharedResources {
    pub syntax_set: syntect::parsing::SyntaxSet,
    pub theme_set: syntect::highlighting::ThemeSet,
}

impl SharedResources {
    pub fn new() -> Self {
        Self {
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme_set: syntect::highlighting::ThemeSet::load_defaults(),
        }
    }
}

pub struct App {
    /// Input buffer
    input: String,
    /// Status bar update channel (watch channel -- only cares about latest value, no backlog)
    status_watch_rx: tokio::sync::watch::Receiver<StatusBarData>,
    /// Message display area
    messages: Vec<DisplayMessage>,
    /// Composite state (replaces single enum, supports overlay layers)
    state: UiState,
    /// Events from agent
    event_rx: mpsc::Receiver<Event>,
    /// Shared resources (SyntaxSet/ThemeSet etc., initialized once)
    resources: SharedResources,
}

/// Composite state pattern -- main state + overlay + notifications + focus + active tool progress
pub struct UiState {
    /// Main interaction state
    pub main: MainState,
    /// Modal overlay (only one modal at a time: permission dialog or command palette)
    /// Uses Option instead of Vec: modal UI only shows one at a time; queuing multiples is meaningless
    pub overlay: Option<Overlay>,
    /// Non-modal notification queue (toast style, auto-dismiss, doesn't block input)
    pub notifications: std::collections::VecDeque<Toast>,
    /// Current focus position (determines which component receives keyboard events)
    pub focus: FocusTarget,
    /// Active tool execution progress (supports concurrent tool tracking)
    pub active_tools: Vec<ToolProgress>,
}

/// Non-modal notification (toast-like, auto-dismisses after display)
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: std::time::Instant,
    /// Display duration (default 3 seconds)
    pub ttl: std::time::Duration,
}

pub enum ToastLevel {
    Info,
    Warning,
    Error,
}

/// Focus target -- determines keyboard event routing
pub enum FocusTarget {
    /// Input box (default focus) -- receives text input and Enter to submit
    InputBox,
    /// Modal overlay -- receives Esc to close, arrow keys to select, Enter to confirm
    Overlay,
    /// Message scroll area -- receives j/k/PgUp/PgDn scrolling
    MessageScroll,
}

// Focus routing logic:
// - When overlay.is_some(), focus is forced to FocusTarget::Overlay
// - When overlay closes, focus returns to FocusTarget::InputBox
// - User can press Ctrl+Up/Down to temporarily switch to MessageScroll to browse history

pub enum MainState {
    /// Waiting for user input
    Idle,
    /// API call in progress (show spinner)
    Thinking,
    /// Streaming response being received -- supports incremental rendering
    Streaming(StreamingMessage),
}

/// Streaming message state -- supports delta appending + incremental parsing
/// Note: "incremental" here means **parsing optimization** (avoid re-parsing already processed Markdown),
/// not skipping rendering -- each frame still fully renders all parsed blocks.
pub struct StreamingMessage {
    /// Complete text received so far
    pub buffer: String,
    /// Parsed offset (only need to parse buffer[parsed_offset..] for new content)
    pub parsed_offset: usize,
    /// List of parsed render blocks (Markdown -> structured blocks, incrementally appended)
    pub parsed_blocks: Vec<RenderedBlock>,
    /// Whether complete
    pub complete: bool,
}

/// Parsed render block (structured representation of Markdown parse results)
pub enum RenderedBlock {
    Paragraph(String),
    CodeBlock { language: String, code: String },
    Heading { level: u8, text: String },
    List(Vec<String>),
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    BlockQuote(String),
    HorizontalRule,
    Link { text: String, url: String },
    Image { alt: String, url: String }, // placeholder -- terminal cannot render images, shows alt text
}

impl StreamingMessage {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            parsed_offset: 0,
            parsed_blocks: Vec::new(),
            complete: false,
        }
    }

    /// Append incremental text
    pub fn append_delta(&mut self, delta: &str) {
        self.buffer.push_str(delta);
    }

    /// Incremental parse: only parse new content in buffer[parsed_offset..], append to parsed_blocks
    pub fn parse_pending(&mut self) {
        let new_content = &self.buffer[self.parsed_offset..];
        // Use pulldown-cmark to parse new content, generate RenderedBlock
        // Note: need to handle block boundaries (e.g., unclosed code block spanning deltas)
        // ...
        self.parsed_offset = self.buffer.len();
    }
}

pub enum Overlay {
    /// Permission confirmation dialog
    PermissionDialog { tool_name: String, request_id: String },
    /// Command palette (Ctrl+K)
    CommandPalette,
}

pub struct ToolProgress {
    pub id: String,
    pub name: String,
    pub started_at: std::time::Instant,
}

pub struct DisplayMessage {
    pub role: String,
    pub content: String,
    pub cost: Option<String>,
}

impl App {
    /// Main render loop
    /// Uses crossterm::event::EventStream instead of spawn_blocking+poll/read
    /// Avoids race conditions: poll and read called from different threads may lose events
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
    ) -> crab_common::Result<()> {
        use crossterm::event::EventStream;
        use futures::StreamExt;

        let mut term_events = EventStream::new();
        let target_fps = 30;
        let frame_duration = std::time::Duration::from_millis(1000 / target_fps);

        // Use tokio::time::interval instead of sleep(saturating_sub)
        // MissedTickBehavior::Skip ensures: if a frame processing overruns, skip missed ticks instead of burst-catching-up
        let mut frame_tick = tokio::time::interval(frame_duration);
        frame_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                // Terminal input (EventStream is an async Stream, no race conditions)
                Some(Ok(term_event)) = term_events.next() => {
                    if let TermEvent::Key(key) = term_event {
                        match key.code {
                            KeyCode::Enter => {
                                self.submit_input();
                            }
                            KeyCode::Char(c) => {
                                self.input.push(c);
                            }
                            KeyCode::Esc => {
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
                // Agent events
                Some(event) = self.event_rx.recv() => {
                    self.handle_agent_event(event);
                }
                // Status bar refresh -- watch channel notification (cost updates, token count changes, etc.)
                // watch::Receiver only keeps the latest value; multiple writes trigger changed() only once
                // More suitable than mpsc for "latest state" scenarios (no backlog, no missed updates)
                Ok(()) = self.status_watch_rx.changed() => {
                    let status = self.status_watch_rx.borrow().clone();
                    self.update_status_bar(status);
                }
                // Frame rate timer -- interval + Skip is more precise than sleep(saturating_sub)
                // Avoids frame rate drift caused by time differences when computing saturating_sub
                _ = frame_tick.tick() => {
                    terminal.draw(|frame| self.render(frame))?;
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        // Use ratatui Layout for partitioning
        // Top: message history (Markdown rendering)
        // Middle: tool output / spinner
        // Bottom: input box + status bar
        // ...
    }
}
```

**External Dependencies**: `ratatui`, `crossterm`, `syntect`, `pulldown-cmark`, `crab-core`, `crab-session`, `crab-config`, `crab-common`

> tui does not directly depend on tools; it receives tool execution state via the `crab_core::Event` enum, with crates/cli responsible for assembling agent+tui.

**Feature Flags**: None (tui itself is an optional dependency of cli)

---

### 6.13 `crates/skill/` -- Skill System

**Responsibility**: Skill discovery, loading, registry, and built-in skill definitions (corresponds to CC `src/skills/`)

**Directory Structure**

```
src/
├── lib.rs            // Public API re-exports
├── types.rs          // Skill, SkillTrigger, SkillContext, SkillSource
├── frontmatter.rs    // YAML frontmatter parsing from .md files
├── registry.rs       // SkillRegistry (discover, register, find, match)
├── builder.rs        // SkillBuilder fluent API
└── bundled/
    ├── mod.rs         // bundled_skills() + BUNDLED_SKILL_NAMES
    ├── commit.rs      // /commit
    ├── review_pr.rs   // /review-pr
    ├── debug.rs       // /debug
    ├── loop_skill.rs  // /loop
    ├── remember.rs    // /remember
    ├── schedule.rs    // /schedule
    ├── simplify.rs    // /simplify
    ├── stuck.rs       // /stuck
    ├── verify.rs      // /verify
    └── update_config.rs // /update-config
```

**External Dependencies**: `crab-common`, `serde`, `serde_json`, `regex`, `tracing`

---

### 6.14 `crates/plugin/` -- Plugin System

**Responsibility**: Plugin lifecycle, hooks, WASM sandbox, MCP↔skill bridge (corresponds to CC `src/services/plugins/`)

**Directory Structure**

```
src/
├── lib.rs
├── skill_builder.rs      // MCP → Skill bridge (load_mcp_skills)
├── hook.rs               // Lifecycle hook execution
├── hook_registry.rs      // Hook registry
├── hook_types.rs         // Hook type definitions
├── hook_watchers.rs      // File watcher hooks
├── frontmatter_hooks.rs  // Parse hooks from skill YAML frontmatter
├── manager.rs            // Plugin discovery and lifecycle
├── manifest.rs           // Plugin manifest parsing
└── wasm_runtime.rs       // WASM plugin sandbox (wasmtime, feature = "wasm")
```

**External Dependencies**: `crab-common`, `crab-core`, `crab-process`, `crab-skill`, `wasmtime` (optional)

**Feature Flags**

```toml
[features]
default = []
wasm = ["wasmtime"]
```

---

### 6.15 `crates/memory/` -- Persistent Memory System

**Responsibility**: File-based cross-session memory storage — user preferences, feedback, project context, external references (corresponds to CC `src/memdir/`)

**Directory Structure**

```
src/
├── lib.rs              // Public API re-exports
├── types.rs            // MemoryType enum, MemoryMetadata, frontmatter parsing
├── store.rs            // MemoryStore — file CRUD + mtime-sorted scan
├── index.rs            // MEMORY.md index read/write + truncation (200 lines / 25KB)
├── relevance.rs        // MemorySelector keyword scoring + MemoryRanker trait
├── age.rs              // Exponential decay scoring (30-day half-life, SystemTime)
├── paths.rs            // Per-project / global / team memory directory resolution
├── security.rs         // Path traversal / symlink / null byte validation
├── prompt.rs           // MemoryPromptBuilder — system prompt injection
├── team.rs             // TeamMemoryStore — shared team memory with slugified filenames
└── ranker.rs           // LlmMemoryRanker — Sonnet sidequery (feature = "mem-ranker")
```

**External Dependencies**: `crab-common`, `serde`, `serde_json`, `serde_yml`, `dunce`. Optional: `crab-api`, `crab-core`, `tokio` (with `mem-ranker` feature)

**Feature Flags**

```toml
[features]
default = []
mem-ranker = ["dep:crab-api", "dep:crab-core", "dep:tokio"]  # LLM-driven memory selection
```

**Key Types**: `MemoryType` (User/Feedback/Project/Reference), `MemoryMetadata`, `MemoryFile`, `MemoryStore`, `MemorySelector`, `MemoryRanker` (trait), `LlmMemoryRanker` (impl, feature-gated), `MemoryPromptBuilder`, `TeamMemoryStore`

---

### 6.16 `crates/telemetry/` -- Observability

**Responsibility**: Distributed tracing and metrics collection (corresponds to CC `src/services/analytics/` + `src/services/diagnosticTracking.ts`)

**Directory Structure**

```
src/
├── lib.rs
├── tracer.rs         // OpenTelemetry tracer initialization
├── metrics.rs        // Custom metrics (API latency, tool execution time, etc.)
├── cost.rs           // Cost tracking
└── export.rs         // OTLP export
```

**Core Interface**

```rust
// tracer.rs
use tracing_subscriber::prelude::*;

/// Initialize the tracing system
pub fn init(service_name: &str, endpoint: Option<&str>) -> crab_common::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .compact();

    let registry = tracing_subscriber::registry().with(fmt_layer);

    #[cfg(feature = "otlp")]
    if let Some(endpoint) = endpoint {
        let _tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;
        // Add OpenTelemetry layer to registry
    }

    #[cfg(not(feature = "otlp"))]
    let _ = (service_name, endpoint); // Suppress unused warnings

    registry.init();
    Ok(())
}
```

**External Dependencies**: `tracing`, `tracing-subscriber`, `crab-common`; OTLP-related are optional dependencies

**Feature Flags**

```toml
[features]
default = ["fmt"]
fmt = ["tracing-subscriber/fmt"]                               # Local log formatting (default)
otlp = [                                                       # OpenTelemetry OTLP export
    "opentelemetry",
    "opentelemetry-otlp",
    "opentelemetry-sdk",
    "tracing-opentelemetry",
]
```

> By default, only `fmt` is enabled (local tracing-subscriber), without pulling in the full opentelemetry stack.
> Production deployments needing OTLP export can enable it with `cargo build -F otlp`.

---

### 6.17 `crates/cli/` -- Terminal Entry Point

**Responsibility**: An extremely thin binary entry point that only does assembly with no business logic (corresponds to CC `src/entrypoints/cli.tsx`)

**Directory Structure**

```
src/
├── main.rs           // #[tokio::main] entry point
├── commands/         // clap subcommand definitions
│   ├── mod.rs
│   ├── chat.rs       // Default interactive mode (crab chat)
│   ├── run.rs        // Non-interactive single execution (crab run -p "...")
│   ├── session.rs    // ps, logs, attach, kill
│   ├── config.rs     // Configuration management (crab config set/get)
│   ├── mcp.rs        // MCP server mode (crab mcp serve)
│   └── serve.rs      // Serve mode
└── setup.rs          // Initialization, signal registration, version check, panic hook
```

**Panic Hook Design**

```rust
// setup.rs -- Terminal state recovery panic hook
// Must be registered after terminal.init() and before entering the main loop
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // 1. Restore terminal state (most important -- otherwise terminal becomes unusable)
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        // 2. Call original hook (print panic info)
        original_hook(panic_info);
        // Recommended alternative: use color-eyre::install() for automatic handling,
        // providing beautified panic reports + backtrace
    }));
}
```

**Entry Point Code**

```rust
// main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crab", version, about = "AI coding assistant")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Pass prompt directly (equivalent to crab run -p)
    #[arg(short, long)]
    prompt: Option<String>,

    /// Permission mode
    #[arg(long, default_value = "default")]
    permission_mode: String,

    /// Specify model
    #[arg(long)]
    model: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive mode (default)
    Chat,
    /// Single execution
    Run {
        #[arg(short, long)]
        prompt: String,
    },
    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// MCP mode
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 1. Initialize telemetry
    crab_telemetry::init("crab-code", None)?;

    // 2. Load configuration
    let config = crab_config::load_merged_settings(None)?;

    // 3. Initialize authentication
    let auth = crab_auth::resolve_api_key()
        .ok_or_else(|| anyhow::anyhow!("no API key found"))?;

    // 4. Dispatch commands
    match cli.command.unwrap_or(Commands::Chat) {
        Commands::Chat => {
            // Start interactive mode
            // ...
        }
        Commands::Run { prompt } => {
            // Single execution
            // ...
        }
        _ => { /* ... */ }
    }

    Ok(())
}
```

**External Dependencies**: All crates, `clap`, `tokio`, `anyhow`

**Feature Flags**

```toml
[features]
default = ["tui"]
tui = ["crab-tui"]
full = ["tui", "crab-plugin/wasm", "crab-api/bedrock", "crab-api/vertex"]
```

---

### 6.18 `crates/daemon/` -- Background Daemon

**Responsibility**: A persistently running background daemon that manages multiple sessions (corresponds to CC `src/daemon/`)

**Directory Structure**

```
src/
└── main.rs
```

**IPC Communication Design**

```
CLI <--- Unix socket (Linux/macOS) / Named pipe (Windows) ---> Daemon
         Protocol: length-prefixed frames + JSON messages
         Format: [4 bytes: payload_len_le32][payload_json]
```

**IPC Message Protocol**

```rust
/// CLI -> Daemon request
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonRequest {
    /// Create new session or attach to existing session
    Attach { session_id: Option<String>, working_dir: PathBuf },
    /// Disconnect but keep session running
    Detach { session_id: String },
    /// List active sessions
    ListSessions,
    /// Terminate session
    KillSession { session_id: String },
    /// Send user input
    UserInput { session_id: String, content: String },
    /// Health check
    Ping,
}

/// Daemon -> CLI response/event push
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    /// Attach successful
    Attached { session_id: String },
    /// Session list
    Sessions { list: Vec<SessionInfo> },
    /// Forward agent Event (streaming push)
    Event(crab_core::event::Event),
    /// Error
    Error { message: String },
    /// Pong
    Pong,
}
```

**Session Pool Management**

```rust
pub struct SessionPool {
    /// Active sessions (max N, default 8)
    sessions: HashMap<String, SessionHandle>,
    /// Shared API connection pool (reused across all sessions)
    api_client: Arc<LlmBackend>,
    /// Idle timeout auto-cleanup (default 30 minutes)
    idle_timeout: Duration,
}

pub struct SessionHandle {
    pub id: String,
    pub working_dir: PathBuf,
    pub created_at: Instant,
    pub last_active: Instant,
    /// Whether a CLI is currently connected
    pub attached: bool,
    /// Session control channel
    pub tx: mpsc::Sender<DaemonRequest>,
}
```

**CLI Attach/Detach Flow**

```
1. CLI starts -> connects to daemon socket
2. Sends Attach { session_id: None } -> daemon creates new session
3. Daemon replies Attached { session_id: "xxx" }
4. CLI sends UserInput -> daemon forwards to query_loop
5. Daemon streams Event -> CLI renders
6. CLI exits -> sends Detach -> session continues running in background
7. CLI re-attaches Attach { session_id: "xxx" } -> resumes conversation
```

**Core Logic**

```rust
// main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 0. Log initialization -- use tracing-appender for log rotation
    //    daemon is a long-running process, must have log rotation to prevent disk from filling up
    let log_dir = directories::ProjectDirs::from("", "", "crab-code")
        .expect("failed to resolve project dirs")
        .data_dir()
        .join("logs");
    let file_appender = tracing_appender::rolling::daily(&log_dir, "daemon.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false) // File logs don't need ANSI colors
        .init();

    // 1. PID file + single instance check (fd-lock)
    // 2. Initialize shared API connection pool
    // 3. Create SessionPool
    // 4. Listen on IPC socket
    // 5. Accept loop: spawn independent handler for each CLI connection
    // 6. Periodically clean up idle sessions
    // ...
}
```

**External Dependencies**: `crab-core`, `crab-session`, `crab-api`, `crab-tools`, `crab-config`, `crab-agent`, `crab-common`, `tokio`, `fd-lock`, `tracing-appender`

---

### 6.19 Global State Split: AppConfig / AppRuntime

Global state shared by CLI and Daemon is split into **immutable configuration** and **mutable runtime** halves,
avoiding a single `Arc<RwLock<AppState>>` where read paths get blocked by write locks.

```rust
/// Immutable configuration -- initialized at startup, unchanged during runtime
/// Arc<AppConfig> shared with zero locks, readable by any thread/task
pub struct AppConfig {
    /// Merged settings.json
    pub settings: crab_config::Settings,
    /// CRAB.md content (global + user + project)
    pub crab_md: Vec<crab_config::CrabMd>,
    /// Permission policy
    pub permission_policy: crab_core::permission::PermissionPolicy,
    /// Model configuration
    pub model_id: crab_core::model::ModelId,
    /// Project root directory
    pub project_dir: std::path::PathBuf,
}

/// Mutable runtime state -- changes frequently during runtime
/// Arc<RwLock<AppRuntime>> read-heavy/write-light, RwLock read locks are non-exclusive
pub struct AppRuntime {
    /// Cost tracker (written after each API call)
    pub cost_tracker: crab_core::model::CostTracker,
    /// Active session list (multiple in daemon mode)
    pub active_sessions: Vec<String>,
    /// MCP connection pool (dynamic connect/disconnect)
    pub mcp_connections: std::collections::HashMap<String, crab_mcp::McpClient>,
}

// Usage:
// let config = Arc::new(AppConfig { ... });     // Built at startup, read-only afterward
// let runtime = Arc::new(RwLock::new(AppRuntime { ... })); // Read/write at runtime
//
// // Hot path: zero-lock config reads
// let model = &config.model_id;
//
// // Write path: update cost (brief write lock)
// runtime.write().await.cost_tracker.record(&usage, cost);
```

---

## 7. Design Principles

| # | Principle | Description | Rationale |
|---|-----------|-------------|-----------|
| 1 | **core has zero I/O** | Pure data structures and traits, no file/network/process operations | Reusable by CLI/GUI/WASM frontends; unit tests need no mocking |
| 2 | **tools as independent crate** | 21+ tools have significant compile cost; keeping them separate means incremental compilation only triggers on changed tools | Changing one tool doesn't recompile everything |
| 3 | **fs and process are separate** | Orthogonal responsibilities: fs handles file content, process handles execution | GlobTool doesn't need sysinfo, BashTool doesn't need globset |
| 4 | **tui is optional** | cli bin uses feature flags to decide whether to compile tui | Future Tauri GUI imports core+session+tools but not tui |
| 5 | **api and session are layered** | api only handles HTTP communication, session manages business state | Replacing an API provider doesn't affect session logic |
| 6 | **Feature flags control optional dependencies** | No Bedrock? Don't compile AWS SDK. No WASM? Don't compile wasmtime. | Reduces compile time and binary size |
| 7 | **workspace.dependencies unifies versions** | All crates share the same version of third-party libraries | Avoids dependency conflicts and duplicate compilation |
| 8 | **Binary crates only do assembly** | cli/daemon only do assembly; all logic lives in library crates | Makes it easy to add new entry points in the future (desktop/wasm/mobile) |

---

## 8. Feature Flag Strategy

### 8.1 Per-Crate Feature Configuration

```toml
# --- crates/api/Cargo.toml ---
[features]
default = []
bedrock = ["aws-sdk-bedrockruntime", "aws-config"]  # AWS Bedrock provider
vertex = ["gcp-auth"]                                 # Google Vertex provider
proxy = ["reqwest/socks"]                             # SOCKS5 proxy support

# --- crates/auth/Cargo.toml ---
[features]
default = []
bedrock = ["aws-sdk-bedrockruntime", "aws-config"]   # AWS SigV4 signing

# --- crates/mcp/Cargo.toml ---
[features]
default = []
ws = ["tokio-tungstenite"]                            # WebSocket transport

# --- crates/plugin/Cargo.toml ---
[features]
default = []
wasm = ["wasmtime"]                                   # WASM plugin sandbox

# --- crates/process/Cargo.toml ---
[features]
default = []
pty = ["portable-pty"]                                # Pseudo-terminal allocation
sandbox = []                                          # Process sandbox

# --- crates/telemetry/Cargo.toml ---
[features]
default = ["fmt"]
fmt = ["tracing-subscriber/fmt"]                             # Local logging (default)
otlp = [                                                     # OTLP export
    "opentelemetry", "opentelemetry-otlp",
    "opentelemetry-sdk", "tracing-opentelemetry",
]

# --- crates/cli/Cargo.toml ---
[features]
default = ["tui"]
tui = ["crab-tui"]                                    # Terminal UI (enabled by default)
full = [                                              # Full-feature build
    "tui",
    "crab-plugin/wasm",
    "crab-api/bedrock",
    "crab-api/vertex",
    "crab-process/pty",
    "crab-telemetry/otlp",
]
minimal = []                                          # Minimal build (no TUI)
```

### 8.2 Build Combinations

| Scenario | Command | What Gets Compiled |
|----------|---------|-------------------|
| Daily development | `cargo build` | cli + tui (default) |
| Minimal build | `cargo build --no-default-features -F minimal` | cli only, no tui |
| Full feature | `cargo build -F full` | All providers + WASM + PTY |
| Library only | `cargo build -p crab-core` | Single crate compilation |
| WASM target | `cargo build -p crab-core --target wasm32-unknown-unknown` | core layer WASM |

### 8.3 Mapping to CC Feature Flags

CC source code manages about 31 runtime flags through `featureFlags.ts`; Crab Code splits them into:

- **Compile-time features**: Provider selection, WASM plugins, PTY, etc. (Cargo features)
- **Runtime flags**: Managed via `config/feature_flag.rs`, with support for remote delivery

---

## 9. Workspace Configuration

### 9.1 Root Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["crates/*", "xtask"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT"
repository = "https://github.com/user/crab-code"
description = "AI coding assistant in Rust"

[workspace.dependencies]
# See root Cargo.toml for complete dependency list
# Main categories: async runtime (tokio), serialization (serde), CLI (clap), HTTP (reqwest),
# TUI (ratatui), error handling (thiserror/anyhow), file system (globset/ignore), etc.

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"

[profile.dev]
opt-level = 0
debug = true

[profile.release]
lto = "thin"
strip = true
codegen-units = 1
opt-level = 3
```

### 9.2 rust-toolchain.toml

```toml
[toolchain]
channel = "1.85.0"    # Minimum version for edition 2024 + async fn in trait
components = ["rustfmt", "clippy", "rust-analyzer"]
```

### 9.3 rustfmt.toml

```toml
edition = "2024"
max_width = 100
tab_spaces = 4
use_field_init_shorthand = true
```

---

## 10. Data Flow Design

### 10.1 Primary Data Flow: Query Loop

```
User input
  |
  v
┌──────────┐    prompt     ┌──────────┐   HTTP POST    ┌──────────────┐
│crates/cli│──────────────>│  agent   │───────────────>│  Anthropic   │
│ (TUI)    │               │query_loop│   /v1/messages │  API Server  │
└──────────┘               └────┬─────┘<───────────────┘──────────────┘
      ^                         │          SSE stream
      |                         |
      | Event::ContentDelta     | Parse assistant response
      |                         |
      |                    ┌────v─────┐
      |                    │ Has tool │──── No ──> Loop ends, display result
      |                    │ calls?   │
      |                    └────┬─────┘
      |                         | Yes
      |                         v
      |                    ┌──────────┐   delegate    ┌────────────┐
      |                    │  tools   │──────────────>│  fs / mcp  │
      |                    │ executor │               │  process   │
      |                    └────┬─────┘<──────────────┘────────────┘
      |                         |         ToolOutput
      | Event::ToolResult       |
      └─────────────────────────┘
            Tool results appended to messages, return to top of query_loop
```

### 10.2 MCP Tool Call

```
┌──────────┐  call_tool   ┌──────────┐  Crab facade   ┌──────────────┐
│  tools   │─────────────>│   mcp    │───────────────>│  MCP Server  │
│ executor │              │  client  │               │  (external    │
│          │              │          │               │   process)    │
└──────────┘              └────┬─────┘               └──────┬───────┘
                               │                             │
                               │     rmcp transport/client   │
                          ┌────v─────────────────────────┐   │
                          │  stdio child process / HTTP  │   │
                          │  handshake / tools/list      │   │
                          │  tools/call / resources      │   │
                          └──────────────────────────────┘   │
                                                             │
                               <─────────────────────────────┘
                                     tool / resource result
```

### 10.3 Context Compaction Decision Flow

```
┌──────────────┐
│ query_loop   │
│ start of     │
│ each turn    │
└──────┬───────┘
       │
       v
┌──────────────┐     estimated_tokens()
│ Estimate     │──────────────────────────┐
│ current      │                          │
│ token count  │                          v
└──────────────┘                   ┌──────────────┐
                                   │ > 70% of     │
                                   │ window?      │
                                   └──────┬───────┘
                                          │
                               ┌─── No ───┼─── Yes ──┐
                               │          │           │
                               v          │           v
                          Continue         │    ┌──────────────┐
                          normally         │    │ Select       │
                                          │    │ compaction   │
                                          │    │ strategy     │
                                          │    └──────┬───────┘
                                          │           │
                                          │    ┌──────v───────┐
                                          │    │  Snip        │ <- 70-80%
                                          │    │  Microcompact│ <- 80-85%
                                          │    │  Summarize   │ <- 85-90%
                                          │    │  Hybrid      │ <- 90-95%
                                          │    │  Truncate    │ <- > 95%
                                          │    └──────┬───────┘
                                          │           │
                                          │           v
                                          │    ┌──────────────┐
                                          │    │ Call small   │
                                          │    │ model to     │
                                          │    │ generate     │
                                          │    │ summary      │
                                          │    └──────┬───────┘
                                          │           │
                                          │           v
                                          │    ┌──────────────┐
                                          │    │ Rebuild      │
                                          │    │ message list │
                                          │    │ [summary] +  │
                                          │    │ recent N     │
                                          │    │ turns        │
                                          │    └──────┬───────┘
                                          │           │
                                          └───────────┘
                                                      │
                                                      v
                                                Continue query_loop
```

---

## 11. Extension System Design

### 11.1 Multi-Model Support Architecture (crab-api)

`crab-api`'s multi-model fallback and error classification layer, stacked on top of the `LlmBackend` enum:

```
User request
    |
    v
┌─────────────────┐
│    fallback.rs   │  -- Multi-model fallback chain (primary -> backup1 -> backup2)
└────────┬────────┘
         │
    ┌────v────────────────┐
    │  retry_strategy.rs  │  -- Enhanced retry (backoff + jitter)
    └────┬────────────────┘
         │
    ┌────v────────────────┐
    │ error_classifier.rs │  -- Error classification (retryable/non-retryable)
    └─────────────────────┘
```

**Module List**:
- `fallback.rs` -- Multi-model fallback chain (auto-switch to backup on primary failure)
- `capabilities.rs` -- Model capability negotiation and discovery
- `context_optimizer.rs` -- Context window optimization + smart truncation
- `streaming.rs` -- Streaming tool call parsing
- `retry_strategy.rs` / `error_classifier.rs` -- Enhanced retry and error classification


### 11.2 MCP Protocol Stack (crab-mcp)

MCP protocol extension modules:

- `crab-mcp` is Crab's MCP facade; it does not directly expose the underlying SDK to upper-layer crates
- Client-side stdio / HTTP connections are handled by the official SDK; Crab handles config discovery, naming, permission integration, and tool bridging
- The config primary path only retains `stdio` / `http` / `ws`
- Server / prompt / resource / tool registry still retain Crab's own abstraction layer

| Module | Function |
|--------|----------|
| `handshake.rs` + `negotiation.rs` | initialize/initialized handshake, capability set negotiation |
| `sampling.rs` | Server requests LLM inference (server -> client sampling) |
| `roots.rs` | Workspace root directory declaration (client tells server accessible paths) |
| `logging.rs` | Structured log message protocol |
| `sse_server.rs` | Crab as MCP server providing SSE transport |
| `capability.rs` | Capability declaration types |
| `notification.rs` | Server notification push |
| `progress.rs` | Progress reporting (long-running tool execution) |
| `cancellation.rs` | Request cancellation (`$/cancelRequest` JSON-RPC notification) |
| `health.rs` | Health check + heartbeat |


### 11.3 Agent Reliability (crab-agent)

**Reliability Subsystem**:
```
error_recovery -> retry -> rollback
summarizer (conversation summary generation)
repl_commands (/undo /branch /fork)
```


### 11.4 TUI Component Library (crab-tui, 21 Components)

**Interactive Components** (user-operated):
- `command_palette` -- Ctrl+P command palette, fuzzy search all commands
- `autocomplete` -- Popup completion suggestions while typing
- `search` -- Global search (filename + content)
- `input_history` -- Up/down arrow to browse input history

**Content Display Components**:
- `code_block` -- Code block + copy button (syntect highlighting)
- `tool_output` -- Collapsible tool output display (expandable/collapsible)

**Status Feedback Components**:
- `notification` -- Toast notification (top popup, 3s auto-dismiss)
- `progress_indicator` -- Percentage progress bar
- `loading` -- Multi-style loading animation (spin/dot/bar)
- `status_bar` -- Enhanced status bar (mode/provider/token count/response latency)
- `shortcut_hint` -- Always-visible shortcut hint bar at bottom


### 11.5 Auth Cloud Platform Authentication (crab-auth)

```
AWS Scenario:
  aws_iam.rs -> Supports IRSA (pod-level IAM roles) + standard IAM credential chain
  credential_chain.rs -> env -> keychain -> file -> IRSA -> instance metadata

GCP Scenario:
  gcp_identity.rs -> Workload Identity Federation
  vertex_auth.rs -> GCP Vertex AI dedicated authentication
```
