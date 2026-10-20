# CLAUDE.md

## 项目概述

AI Collaboration Engine (ACE) - 基于Tauri 2.0的桌面应用，用于AI辅助研发协作。

## 技术栈

- **前端**: React + TypeScript + Tailwind CSS + Zustand
- **后端**: Rust + Tauri 2.0
- **数据库**: SQLite (WAL模式)
- **状态管理**: Zustand
- **UI组件**: Lucide Icons, Framer Motion

## 开发命令

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建
npm run tauri build

# 测试（前端全量：L1 组件/Store/页面 + L3 契约 + L5 混沌 + L6 IPC）
npm test

# 项目路径含中文时（Windows 必须用这个，内部 subst 映射为 ASCII 盘符）
npm run test:win

# 分层测试
npm run test:l1       # 组件 / Store / 页面（src/）
npm run test:pages    # 仅页面级单测
npm run test:ipc      # L6 IPC 覆盖率（82/82）
npm run test:l3       # L3 IPC 契约（mock）
npm run test:l5       # L5 混沌
npm run test:rust     # 后端（默认 29 个单测；门控用例需 Victauri 运行环境）
npm run test:e2e      # L2 Playwright（24 例）

# L4 CDP（需先启动应用并暴露调试端口）
# 终端 1: $env:TAURI_DEVTOOLS_SERVER_PORT=9222; npm run tauri dev
# 终端 2:
npm run test:l4

# 一键质量门禁（前端全套）
npm run verify

# 监听模式测试
npm run test:watch
```

> **后端测试说明**：`cargo test` 默认执行 29 个单测（含 `db/integration_tests.rs` 的真实 SQLite 链路测试）。
> `tests/integration.rs`（71 例）与 `tests/smoke.rs`（3 例）通过 `skip_unless_e2e()` 门控，
> 需在 Victauri 测试运行环境中才会实际执行，属设计行为而非缺失。

## 项目结构

```
├── src/                    # 前端代码
│   ├── components/        # 共享组件（含 SessionChat/SessionInput/MessageBubble）
│   ├── pages/             # 页面组件
│   ├── stores/            # Zustand状态管理（含 sessionMessagesStore）
│   └── lib/               # 工具函数和类型定义
├── src-tauri/             # 后端代码
│   └── src/
│       ├── commands.rs        # Tauri IPC命令（88 条）
│       ├── agent_adapter.rs   # Agent CLI 适配层（结构化会话参数/消息编码/事件归一化）
│       ├── session_driver.rs  # 结构化会话驱动（子进程生命周期/落库/事件推送）
│       ├── db/                # 数据库操作（含 session_messages）
│       ├── services/          # 业务逻辑服务
│       └── main.rs            # 应用入口
├── docs/                  # 文档
└── design.md              # 设计文档
```

## 代码规范

- **Rust**: 使用rustfmt格式化，clippy检查
- **TypeScript**: 使用eslint + prettier
- **提交信息**: 遵循Conventional Commits规范

## 数据库

- 使用SQLite WAL模式提升并发性能
- 通过migration.rs管理数据库版本
- 所有表使用TEXT类型的UUID作为主键

## 关键设计模式

- **Service层**: 所有业务逻辑通过Service访问数据库
- **缓存**: 使用CacheWrapper进行内存缓存
- **错误处理**: 使用AppError枚举统一错误类型

<!-- VICTAURI:BEGIN (added by `victauri init` — delete this block to opt out) -->

## Victauri (App Inspection & Testing)

This app has **Victauri** integrated — an MCP server embedded inside the Tauri process
that gives full-stack access to the webview DOM, IPC layer, Rust backend, and native
windows. Available when the app is running in debug mode.

**Use Victauri MCP tools for all app inspection and testing tasks.** Victauri runs inside
the app process with sub-ms response times and direct AppHandle access — it sees
everything, not just the webview.

Key Victauri tools (the read-only backend/DB introspection below has no CDP/Playwright
equivalent — and on macOS/Linux CDP can't attach to a Tauri webview at all):

- `invoke_command` — call any registered Tauri command directly (also records timing and
  honours fault injection; an eval-capable tool can also reach `__TAURI_INTERNALS__.invoke`)
- `verify_state` — cross-boundary frontend/backend state verification
- `detect_ghost_commands` — find frontend calls absent from the introspection registry (read the `reliability` field: only a real "no backend handler" bug when the registry mirrors the app's full command set; with no/partial `#[inspectable]` it lists real, uninstrumented commands)
- `check_ipc_integrity` — verify IPC pipeline health
- `introspect` — command timings, IPC contract testing, coverage, startup timing, capabilities
- `fault` — inject IPC faults (delay, error, drop, corrupt) for chaos engineering
- `explain` — natural-language narration of what happened in the app
- `get_memory_stats` — real OS process memory stats
- `audit_accessibility` — WCAG accessibility checks
- `get_performance` — navigation timing, JS heap, resource loading

### Connecting reliably (read this before reaching for CDP)

`.mcp.json` connects through `victauri bridge` — a stdio proxy that **discovers the running
app's port at connect time and re-discovers on restart**, so you are never pinned to a stale
or wrong port. Do **not** replace it with a fixed `"url": "http://127.0.0.1:7373/mcp"`: a
hardcoded port can point at a _different_ Victauri app (e.g. a leftover demo) and every call
then fails with `422`/`404`. The bridge avoids that by design.

- **The server connects even before the app is running.** The bridge answers the MCP
  handshake itself, so `victauri` shows up connected in a fresh terminal whether or not the
  app is up. While the app is down the tool list is a static fallback and a tool _call_
  returns a clear "backend not reachable — start the app" error (not a hang). Start the app
  (`npm run tauri dev` / `pnpm tauri dev`) and the bridge connects automatically — tools go
  live with **no `/mcp` reconnect**. So if a call reports the backend unreachable, the fix is
  to start/await the app, not to switch to CDP.
- **First contact:** call `get_plugin_info` once and check `app.identifier` — confirm you
  reached the intended app, not another Victauri instance.
- **Multiple apps running?** Pin the bridge with `--app <bundle-identifier>` in `.mcp.json`
  (`"args": ["bridge", "--app", "com.your.app"]`), or set `VICTAURI_APP`. `init` bakes this in
  automatically when it can read your identifier.
- **If a tool call fails after an app rebuild/restart:** the bridge re-resolves the backend
  automatically — just retry (the transport is stateless by default, so there is usually no
  session to lose). If the MCP path is genuinely wedged, the **sessionless
  REST API is the fallback, NOT CDP**: `POST http://127.0.0.1:<port>/api/tools/<tool>` with
  the Bearer token from `<temp>/victauri/<pid>/token` (same capabilities, no session).

### Awaiting async backend work (don't guess with sleeps)

Many Tauri commands are **fire-and-forget**: they spawn background work and return
immediately (often `null`) while the real work runs. Don't poll by hand or sprinkle fixed
`sleep`s — use `wait_for` to await true completion:

- **Pollable status (no app changes):** `wait_for` with `condition: "expression"` evaluates a
  JS expression every poll until it is truthy (or equals `expected`). It may `await`, so you
  can await a status command directly:
  `wait_for { condition: "expression", value: "(await window.__TAURI_INTERNALS__.invoke('get_status')).running === false" }`.
  Level-triggered and race-free.
- **Completion event:** `wait_for` with `condition: "event"` blocks until a named Tauri event
  fires, with a `since_ms` look-back so an event emitted in the gap after your `invoke_command`
  is still caught: `wait_for { condition: "event", value: "analysis-complete" }`. (Custom events
  must be registered via `VictauriBuilder::listen_events(&["…"])`.)

The robust pattern is `invoke_command(...)` then `wait_for(expression|event, ...)` — never a bare sleep.

### Reading app-specific backend state

If the app registers state probes (`VictauriBuilder::probe("name", || json!({...}))`), call
`app_state` to read domain state directly from the Rust process — no IPC round-trip, no log
grepping. `app_state` with no args lists probe names; `app_state { probe: "name" }` returns its
snapshot. Use this for pipeline/queue/cache internals (version, depth, stats) instead of
reverse-engineering them from `query_db` + logs.

### Driving specific code paths & mutating test state

- To exercise a specific backend code path, call the relevant command via `invoke_command`
  (with args). If the path you need isn't reachable from any command, that's an app gap — add a
  small debug command and drive it.
- `query_db` is intentionally **read-only**. To mutate state for a test, go through the app's
  own commands with `invoke_command` (which respects app invariants) rather than writing the DB.

Prefer Victauri over Playwright or CDP for any Tauri-app task it handles; fall back to
Playwright only for browser-only work unrelated to this app.
<!-- VICTAURI:END -->
