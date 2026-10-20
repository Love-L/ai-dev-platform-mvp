# 会话交互与消息（设计文档）

**文档状态**：设计定稿（代码尚未实现）
**版本**：v1.2（Q3 三次修正：**实测证明该 CLI 不支持 host 逐次确认**，改用"预设 + 白名单 + 被拒卡片 + 终端兜底"）
**日期**：2026-09-21
**关联**：`FEATURES.md`（现有 75 项基线 → 本设计新增模块 16，F076–F086）
**前置分析**：本文件替代此前的会话交互需求分析；决策依据见第 0 章

---

## 0. 决策记录（已确认）

| 编号 | 决策项        | 结论                                                                                                                                                                                                                                                                                                     |
| :--: | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|  Q1  | 主交互路径    | **B —— CLI 结构化模式**（`stream-json` 双向流）为主路径                                                                                                                                                                                                                                                  |
|  Q2  | 终端模式      | **保留，并支持同会话双视图**（结构化为主 + 终端为兜底）                                                                                                                                                                                                                                                  |
|  Q3  | 权限确认      | ~~预设~~ → ~~平台交互式应答~~ → **Q3-v3（实测定稿）**：该 CLI **不支持** host 逐次确认（实测 `control_request` 出现 **0 次**，直接下发 `permission_denied`）→ 采用「预设默认 **`acceptEdits`** + `--allowedTools` 白名单 + **被拒卡片**（"允许并重试" / "切终端"）+ **终端视图作为唯一的逐次确认路径**」 |
|  Q4  | 思考内容      | **必须落库**，前端**默认折叠**                                                                                                                                                                                                                                                                           |
|  Q5  | 多 Agent      | **同期支持 claude 与 codex**（经适配层）                                                                                                                                                                                                                                                                 |
|  Q6  | API 直连模式  | 本期**不做**（保留为后续可选路径）                                                                                                                                                                                                                                                                       |
|  Q7  | 任务/需求绑定 | **必须在消息中体现**（上下文块 + 消息级关联字段）                                                                                                                                                                                                                                                        |

---

## 1. 目标与范围

### 1.1 目标

把会话从"PTY 里敲键盘看 ANSI"升级为"**发指令 → 看结构化回复（含思考）**"的产品级交互：

1. 会话内以**消息**方式下发指令；
2. Agent 回复以**结构化消息块**流式呈现（正文 / 思考 / 工具调用 / 结果）；
3. **思考内容可单独展示**（默认折叠）、**可持久化、可回放**；
4. 支持**多轮上下文**与**多 Agent**（claude、codex）；
5. 保留**原始终端视图**作为兜底与排障手段。

### 1.2 范围

| 在范围内                                    | 不在范围内                                   |
| ------------------------------------------- | -------------------------------------------- |
| 会话消息模型、持久化、回放                  | API 直连（Anthropic Messages API）自研 Agent |
| 结构化输入/输出通道与事件归一化             | 逐次交互式权限确认（host 回调）              |
| claude / codex 适配层                       | 会话共享/多人协作                            |
| 双视图（消息 / 终端，只读与可交互语义区分） | 消息全文检索的向量化/语义搜索（仅关键词）    |
| 任务/需求上下文绑定                         | 移动端/Web 端适配                            |

---

## 2. 术语

| 术语                  | 含义                                                                                                                       |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **结构化会话**        | 以 `stream-json` 双向流运行的会话（`mode = structured`），消息视图可输入                                                   |
| **终端会话**          | 以原生 TUI 运行的会话（`mode = terminal`），终端视图可交互                                                                 |
| **消息块（block）**   | 归一化后的最小展示与存储单元：`text` / `thinking` / `tool_use` / `tool_result` / `result` / `context` / `error` / `system` |
| **视图（view）**      | 前端对同一会话的两种呈现：`messages`（消息视图）/ `terminal`（终端视图）                                                   |
| **适配层（adapter）** | 屏蔽各 Agent CLI 参数与事件差异的中间层                                                                                    |

---

## 3. 总体架构

```
┌──────────────────────────── 前端 ────────────────────────────┐
│  SessionDetail / Sessions                                     │
│   ┌───────────────────────┐   ┌───────────────────────────┐   │
│   │ 消息视图（默认）        │   │ 终端视图（兜底/排障）      │   │
│   │ 气泡 + 思考折叠 + 工具块│   │ xterm（structured=只读）   │   │
│   └──────────┬────────────┘   └────────────┬──────────────┘   │
│              │  sessionMessagesStore        │ pty:event        │
└──────────────┼──────────────────────────────┼─────────────────┘
               │ IPC                          │ IPC / 事件
               ▼                              ▼
┌────────────────────────── Rust 后端 ──────────────────────────┐
│  SessionService（新增，编排）                                   │
│    ├─ AgentAdapter（trait）                                    │
│    │    ├─ ClaudeAdapter   （stream-json，已实测）              │
│    │    └─ CodexAdapter    （待实测）                           │
│    ├─ SessionDriver（结构化会话进程驱动，stdin/stdout JSON）     │
│    ├─ MessageNormalizer（事件 → 统一消息块）                     │
│    ├─ PermissionGuard（permission_denied → 卡片动作 → 白名单沉淀 / 重建会话）│
│    └─ SessionMessageRepo（落库 / 回放 / 分页）                   │
│                                                                │
│  PtyManager（既有，终端通道：TUI 与原始字节流）                   │
└────────────────────────────────────────────────────────────────┘
               │
               ▼
        session_messages（新表）
```

**双通道并存**：

- **结构化通道**（新）：`SessionDriver` 直接持有子进程 stdin/stdout，按 JSON 行读写；
- **终端通道**（既有）：`PtyManager` 提供的 PTY 字节流。

---

## 4. 关键设计约束：双视图的语义

> ⚠️ **必须明确的硬约束**：同一 CLI 进程**不能同时**是"原生 TUI"与"`stream-json` 双向流"。
> 因此"双视图"不是"随时自由切换且都可输入"，而是按**会话模式**区分主从。

| 会话模式             | 进程形态                                                                   | 消息视图                                                                 | 终端视图                                                                         |
| -------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `structured`（默认） | `claude -p --input-format stream-json --output-format stream-json`（长驻） | **主视图，可输入**：发指令、收结构化回复与思考                           | **只读**：展示同一进程的原始事件行（时间戳 + JSON，支持复制/导出），**禁止输入** |
| `terminal`（兜底）   | `claude`（原生 TUI）                                                       | **只读，尽力而为**：从 ANSI 流粗提取文本块，显著标注"非结构化，仅供参考" | **主视图，可交互**：现状能力                                                     |

**为什么结构化会话的终端视图必须只读**：向 `stream-json` 会话的 stdin 写入裸键序会破坏 JSON 协议，导致驱动解析失败。若强行开放输入，等价于协议损坏风险。

**模式切换规则**：

| 场景                           | 是否允许 | 处理                                                                                           |
| ------------------------------ | :------: | ---------------------------------------------------------------------------------------------- |
| 会话处于 `running`             |    ❌    | 提示"请先停止会话"                                                                             |
| 会话为终态（completed/failed） |    ✅    | 以新模式**重建**会话；若解析到 `cli_session_id`，默认带 `--resume <cli_session_id>` 续接上下文 |
| 会话创建时未指定               |    —     | 取 `settings.session_default_mode`（默认 `structured`）                                        |

---

## 5. Agent 适配层设计

### 5.1 统一能力契约

```rust
/// Agent 适配器：屏蔽各 CLI 的参数与事件差异
pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &str;                       // "claude" | "codex"
    /// 组装"结构化会话"启动参数；返回 None 表示该 Agent 不支持结构化模式
    fn structured_command(&self, cfg: &SessionLaunchConfig) -> Option<CommandSpec>;
    /// 组装"终端会话"启动参数
    fn terminal_command(&self, cfg: &SessionLaunchConfig) -> CommandSpec;
    /// 将一条用户指令编码为写入子进程 stdin 的一行
    fn encode_user_message(&self, text: &str) -> String;
    /// 解析子进程 stdout 的一行，产出 0..n 个归一化事件
    fn decode_line(&self, line: &str) -> Vec<AgentEvent>;
    /// 能力探测（供 F019 使用）
    fn probe(&self, binary: &Path) -> AgentCapabilities;
}

pub struct AgentCapabilities {
    pub structured_output: bool,   // 支持 stream-json 输出
    pub structured_input: bool,    // 支持 stream-json 输入（双向）
    pub partial_messages: bool,    // 支持增量消息块（thinking_delta 级流式）
    pub resume: bool,              // 支持会话续接
    pub session_id_bind: bool,     // 支持指定会话 ID
    pub worktree_support: bool,
    pub mcp_support: bool,
}
```

### 5.2 Claude 适配（**本机已实测**，版本 2.1.266）

**结构化会话启动参数**：

```
claude -p
  --input-format stream-json
  --output-format stream-json
  --include-partial-messages          # 增量块（thinking_delta / text_delta）
  --verbose
  --session-id <平台会话绑定的 uuid>
  --permission-mode <preset>          # 见 §9：默认 acceptEdits（实测可放行编辑类操作）
  [--allowedTools <工具列表>]         # 会话白名单（由"允许并重试"沉淀，见 §9.4）
  [--disallowedTools <工具列表>]      # 会话黑名单
  # 注意：不要使用 --permission-prompts host —— 实测该发行版下不可达，会直接 denied（§9.1）
  [--model <model>]
  [--add-dir <工作目录>]
  [--append-system-prompt <上下文摘要>]
  [--mcp-config <文件>]
  [--forward-subagent-text]           # 子代理 text/thinking 也转发
```

> **待实测确认**：`--verbose` 在 `-p + stream-json` 下是否必需（help 中多个能力标注 "only works with --print and --output-format=stream-json"）；`--input-format stream-json` 期望的**精确 JSON schema**；`--mcp-config` 的文件格式。实现时须以最小实验锁定，不得凭猜测定稿。

**已实测可用的相关参数**（`claude --help` 输出）：

| 参数                                         | 取值                                                                         | 用途                                                   |
| -------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------ |
| `--output-format`                            | `text` / `json` / `stream-json`                                              | 结构化输出                                             |
| `--input-format`                             | `text` / `stream-json`                                                       | **结构化输入（realtime streaming input）**             |
| `--include-partial-messages`                 | flag                                                                         | 增量消息块                                             |
| `--forward-subagent-text`                    | flag                                                                         | 转发子代理 text 与 thinking（带 `parent_tool_use_id`） |
| `--session-id`                               | UUID                                                                         | 与平台会话一一绑定                                     |
| `--resume` / `--continue` / `--fork-session` | —                                                                            | 续接 / 继续最近 / 分支                                 |
| `--permission-mode`                          | `acceptEdits` / `auto` / `bypassPermissions` / `manual` / `dontAsk` / `plan` | 权限预设                                               |
| `--permission-prompts`                       | `host` / `none`（`--print` 下）                                              | 由谁应答权限请求                                       |
| `--allowedTools` / `--disallowedTools`       | 工具名列表                                                                   | 工具白/黑名单                                          |
| `--add-dir`                                  | 目录列表                                                                     | 扩展工具可访问目录                                     |
| `--append-system-prompt`                     | 文本                                                                         | 上下文注入                                             |
| `--model` / `--fallback-model` / `--effort`  | —                                                                            | 模型与会话级配置                                       |
| `-n/--name`                                  | 名称                                                                         | 会话显示名                                             |
| `--dangerously-skip-permissions`             | flag                                                                         | 全量跳过（危险）                                       |

**事件归一化映射（claude → 统一块）**：

| CLI 事件                                                               | 判定       | 归一化为                                                                                                                                                                   |
| ---------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `system`（含 `subtype: init`、`session_id`、`tools`、`model`）         | 会话初始化 | `system` 块 + 回写 `agent_sessions.cli_session_id`                                                                                                                         |
| `assistant` → `message.content[]` 中 `{type:"text"}`                   | 正文       | `text` 块                                                                                                                                                                  |
| `assistant` → `{type:"thinking"}`                                      | 思考       | **`thinking` 块**（忽略 `signature`）                                                                                                                                      |
| `assistant` → `{type:"tool_use"}`                                      | 工具调用   | `tool_use` 块（`id` / `name` / `input`）                                                                                                                                   |
| `user` → `{type:"tool_result"}`                                        | 工具结果   | `tool_result` 块（关联 `tool_use_id`）                                                                                                                                     |
| `result`（含 `usage` / `total_cost_usd` / `duration_ms` / `is_error`） | 本轮结束   | `result` 块 + 累加 Token/成本                                                                                                                                              |
| `stream_event`（`--include-partial-messages`）                         | 增量       | 按 `content_block_delta.delta.type` 分发：`text_delta` → text 增量；`thinking_delta` → **thinking 增量**；`input_json_delta` → tool_use 入参增量；`signature_delta` → 忽略 |
| 带 `parent_tool_use_id` 的消息                                         | 子代理     | 存储 `parent_tool_use_id`，前端以缩进/子卡展示                                                                                                                             |

### 5.3 Codex 适配（**本机未安装，字段待实测**）

`codex` 未安装于当前环境，**无法实测**。本节仅定义契约与待验证清单：

| 能力       | codex 侧待确认项                                                        |
| ---------- | ----------------------------------------------------------------------- |
| 非交互执行 | 是否有等价的 `exec` / 一次性模式                                        |
| 结构化输出 | 是否有 `--json` / JSONL 事件流；事件字段（消息、推理、工具调用）命名    |
| 结构化输入 | 是否支持双向流式输入；若不支持 → **降级为"每轮独立进程 + resume"** 模型 |
| 会话续接   | `resume` / session id 机制                                              |
| 权限       | 是否有等价的自动批准/沙箱策略开关                                       |
| 工作目录   | `-C/--cd` 类参数                                                        |

**降级策略**：若某 Agent 不支持 `structured_input`，则其结构化会话采用**逐轮进程模型**：
每轮 `spawn → 写入本轮指令 → 读完整轮事件 → 退出`，下一轮用 `resume` 续接。
此模型下"输入框"仍成立，但**无法在 Agent 执行中追加指令**（不支持打断后追问），需在 UI 标注。

---

## 6. 数据模型设计

### 6.1 新表 `session_messages`

| 字段                     | 类型     | 约束/默认                                                 | 说明                                                                                                                   |
| ------------------------ | -------- | --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `id`                     | TEXT     | PRIMARY KEY                                               | UUID                                                                                                                   |
| `session_id`             | TEXT     | NOT NULL, FK → `agent_sessions(id)` **ON DELETE CASCADE** | 所属会话                                                                                                               |
| `seq`                    | INTEGER  | NOT NULL                                                  | 会话内单调递增序号（唯一顺序依据）                                                                                     |
| `role`                   | TEXT     | NOT NULL                                                  | `user` / `assistant` / `system`                                                                                        |
| `block_type`             | TEXT     | NOT NULL                                                  | `text` / `thinking` / `tool_use` / `tool_result` / `result` / `context` / `error` / `system` / **`permission_denied`** |
| `content`                | TEXT     | NULL                                                      | 归一化文本（thinking 为推理文本；tool_use 为入参 JSON）                                                                |
| `tool_name`              | TEXT     | NULL                                                      | `tool_use` 专用                                                                                                        |
| `tool_use_id`            | TEXT     | NULL                                                      | `tool_use` / `tool_result` 配对键                                                                                      |
| `parent_tool_use_id`     | TEXT     | NULL                                                      | 子代理消息归属                                                                                                         |
| `cli_message_id`         | TEXT     | NULL                                                      | CLI 事件 id，用于**幂等去重**                                                                                          |
| `raw_json`               | TEXT     | NULL                                                      | 原始事件（按设置裁剪）                                                                                                 |
| `tokens`                 | INTEGER  | NULL                                                      | 该块 Token（可得时）                                                                                                   |
| `status`                 | TEXT     | DEFAULT `'completed'`                                     | `streaming` / `completed` / `failed` / `denied`                                                                        |
| `related_task_id`        | TEXT     | NULL                                                      | 上下文绑定（Q7）                                                                                                       |
| `related_requirement_id` | TEXT     | NULL                                                      | 上下文绑定（Q7）                                                                                                       |
| `project_id`             | TEXT     | NULL                                                      | 上下文绑定（Q7）                                                                                                       |
| `created_at`             | DATETIME | DEFAULT CURRENT_TIMESTAMP                                 | 时间戳                                                                                                                 |

**索引与约束**

| 名称                        | 定义                         | 用途            |
| --------------------------- | ---------------------------- | --------------- |
| `idx_session_messages_seq`  | UNIQUE(`session_id`, `seq`)  | 顺序与幂等      |
| `idx_session_messages_time` | (`session_id`, `created_at`) | 回放与分页      |
| `idx_session_messages_type` | (`session_id`, `block_type`) | 按思考/工具过滤 |
| `idx_session_messages_req`  | (`related_requirement_id`)   | 需求维度检索    |

### 6.2 `agent_sessions` 增补列

| 字段                   | 类型     | 默认           | 说明                                                       |
| ---------------------- | -------- | -------------- | ---------------------------------------------------------- |
| `cli_session_id`       | TEXT     | NULL           | CLI 侧会话 ID（`--session-id` / `system.init.session_id`） |
| `work_dir`             | TEXT     | NULL           | 工作目录（**补齐长期缺失字段**，见 F027 缺口）             |
| `agent_path`           | TEXT     | NULL           | 启动时使用的二进制路径（便于重启后重建）                   |
| `mode`                 | TEXT     | `'structured'` | `structured` / `terminal`                                  |
| `view`                 | TEXT     | `'messages'`   | 上次使用的视图（体验记忆）                                 |
| `permission_mode`      | TEXT     | NULL           | 该会话的权限预设                                           |
| `permission_allowlist` | TEXT     | NULL           | 会话级"始终允许"清单（JSON 数组）                          |
| `model`                | TEXT     | NULL           | 会话级模型                                                 |
| `last_event_at`        | DATETIME | NULL           | 最后一次事件时间（用于"无响应"检测）                       |

> 迁移方式遵循现有单一路径：`migrate_tables()` 内 `pragma_table_info` 检查后 `ALTER TABLE ADD COLUMN`；新表在 `init_tables()` 建表；索引在 `create_supplementary_indexes()` 创建。

### 6.3 新增设置键（`settings`）

| key                         | 默认值            | 说明                                                                                |
| --------------------------- | ----------------- | ----------------------------------------------------------------------------------- |
| `session_default_mode`      | `structured`      | 新会话默认模式                                                                      |
| `session_permission_mode`   | **`acceptEdits`** | 新建结构化会话的默认权限预设（实测可放行编辑类操作，见 §9.2）                       |
| `session_show_thinking`     | `collapsed`       | `collapsed` / `expanded` / `hidden`                                                 |
| `session_inject_context`    | `true`            | 是否把任务/需求上下文注入首轮 prompt                                                |
| `session_keep_raw_json`     | `false`           | 是否保留 `raw_json`（体积权衡）                                                     |
| `session_message_page_size` | `50`              | 消息分页大小                                                                        |
| `session_allowed_tools`     | （空）            | 会话级工具白名单，由被拒卡片的「允许并重试」写入，重建会话时映射为 `--allowedTools` |

---

## 7. 归一化事件模型

```rust
pub enum AgentEvent {
    Init { cli_session_id: String, model: Option<String>, tools: Vec<String> },
    BlockStart { seq: i64, block: BlockKind },          // 可选：提前创建 streaming 行
    Delta { block_type: BlockType, delta: String },      // text / thinking 增量
    ToolInputDelta { tool_use_id: String, partial_json: String },
    BlockComplete { block: SessionMessage },             // 一个完整块
    TurnResult { usage: Usage, cost_usd: Option<f64>, duration_ms: Option<i64>, is_error: bool },
    Subagent { parent_tool_use_id: String, inner: Box<AgentEvent> },
    Stderr(String),
    Exit { code: i32 },
}
```

**落库与推送策略**

1. `Delta` 事件：**不落库**，仅通过 `session:delta` 事件推送前端做增量渲染（避免高频写库）；
2. `BlockComplete`：写入 `session_messages`（`seq` 递增）并推送 `session:message`；
3. 增量渲染期间前端用本地缓冲拼装，收到 `session:message` 时以落库内容**覆盖**（幂等）；
4. 若进程异常退出，末个 `streaming` 块由 `SessionService` 收尾为 `failed` 并补齐已收到的增量文本。

---

## 8. IPC 命令与事件

### 8.1 新增命令（5 条，命令总数 82 → 87）

| 命令                    | 入参                                                                                 | 出参                             | 错误场景                                                                 |
| ----------------------- | ------------------------------------------------------------------------------------ | -------------------------------- | ------------------------------------------------------------------------ |
| `send_session_message`  | `session_id`、`content`、`mode?`(`structured`\|`terminal`)                           | `SessionMessage`（用户消息回显） | 会话不存在、会话非 running、结构化会话不支持输入（Codex 逐轮模型执行中） |
| `list_session_messages` | `session_id`、`before_seq?`、`limit?`                                                | `PagedResult<SessionMessage>`    | —                                                                        |
| `interrupt_session`     | `session_id`                                                                         | `()`                             | 会话不存在、会话已终态                                                   |
| `set_session_mode`      | `session_id`、`mode`、`permission_mode?`、`model?`、`allowed_tools?`、`resend_last?` | `AgentSession`（重建后的会话）   | 会话 running、Agent 不支持结构化模式                                     |
| `resume_session`        | `session_id`、`fork?`                                                                | `AgentSession`                   | 缺少 `cli_session_id`、CLI 会话记录不存在                                |

> 既有命令 `start_agent` / `create_session_from_task` 增加可选参数 `mode?`、`permission_mode?`、`model?`；**向后兼容**（缺省取 settings）。

### 8.2 新增事件（与 `pty:event` 并列）

| 事件              | 载荷                                                                                                 | 触发时机                                                                                                  |
| ----------------- | ---------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `session:message` | `{ session_id, message: SessionMessage }`                                                            | 一个完整块落库后                                                                                          |
| `session:delta`   | `{ session_id, seq, block_type, delta }`                                                             | text/thinking 增量                                                                                        |
| `session:tool`    | `{ session_id, phase: "start"\|"input"\|"result", tool_use_id, tool_name?, partial_json?, result? }` | 工具调用生命周期                                                                                          |
| `session:result`  | `{ session_id, usage, cost_usd, duration_ms, is_error }`                                             | 单轮结束                                                                                                  |
| `session:error`   | `{ session_id, error, seq? }`                                                                        | 协议解析失败、进程异常                                                                                    |
| `session:notice`  | `{ session_id, level, kind, message, ref_seq? }`                                                     | 平台级提示（如"工具被权限拒绝，可点此重试"）；`permission_denied` 亦以消息块形式随 `session:message` 下发 |

**命令补全**：`session:message` / `session:delta` 等事件须在 Victauri 侧注册（沿用现有 `listen_events` 机制），否则 MCP 检查会报未注册事件。

---

## 9. 权限模型（实测结论驱动）

### 9.1 决定性实测结论

| 事实                                             | 证据（2026-09-21 实测）                                                                                                                                  |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--permission-prompts host` 在**本发行版不可达** | `--permission-mode manual --permission-prompts host` 下 Write 操作**未产生任何 `control_request`**（实测 0 次），而是直接下发 `system/permission_denied` |
| 无独立 `--permission-prompt-tool` 参数           | `claude --help` 全文仅在该选项的**说明文字**中出现，未列为独立 flag                                                                                      |
| 结论                                             | **结构化模式下无法实现"逐次交互确认"** —— 这是 CLI 能力限制，不是设计取舍                                                                                |

> 因此 v1.1 的"平台交互式应答"方案**被实测证伪**，改为下述可行方案。

### 9.2 可行方案：预设 + 工具白名单（主路径）

| 手段             | 参数                             | 作用                                                        |
| ---------------- | -------------------------------- | ----------------------------------------------------------- |
| 权限预设         | `--permission-mode`              | 决定哪些操作**根本不触发**授权判定                          |
| 工具白名单       | `--allowedTools`                 | 精确放行（支持模式匹配，如 `Bash(git *)`、`Edit`、`Write`） |
| 工具黑名单       | `--disallowedTools`              | 明确禁止（安全兜底）                                        |
| 目录扩展         | `--add-dir`                      | 允许工具访问工作目录之外的路径                              |
| 全量放行（危险） | `--dangerously-skip-permissions` | 仅沙箱环境，需二次确认 UI                                   |

**预设实测状态**

| 预设                | 语义               | 实测结果                                        |
| ------------------- | ------------------ | ----------------------------------------------- |
| `plan`              | 只规划不执行       | 未测（最保守）                                  |
| `manual`            | 严格模式           | ✅ 实测：Write **被 denied**                    |
| `dontAsk`           | 不主动询问         | 未测                                            |
| `auto`              | 自动判定           | 未测                                            |
| **`acceptEdits`**   | 自动接受编辑类操作 | ✅ 实测：Write **成功执行**（文件已创建并更新） |
| `bypassPermissions` | 跳过全部检查       | 未测（危险）                                    |

> **推荐默认值：`acceptEdits`** —— 既能让 Agent 正常干活，又不至于像 `bypassPermissions` 那样无边界。
> `settings.session_permission_mode` 的默认值相应由 `manual` 改为 `acceptEdits`。

### 9.3 被拒可视化（关键体验补偿）

即便放宽预设仍会遇到被拒场景。平台捕获 `system/permission_denied` 并归一化为
`permission_denied` 块，在消息流内联渲染为**可操作卡片**。

**实测事件字段（原文）**

```json
{
  "type": "system",
  "subtype": "permission_denied",
  "tool_name": "Write",
  "tool_use_id": "call_0bd0f6873e3146c48feb0e19",
  "message": "Claude requested permissions to write to <path>, but you haven't granted it yet.",
  "uuid": "...",
  "session_id": "..."
}
```

**卡片内容与动作**

| 展示元素     | 来源                                                   |
| ------------ | ------------------------------------------------------ |
| 被拒工具名   | `tool_name`                                            |
| 人类可读原因 | `message`（**可直接展示**，已包含被拒的具体路径/对象） |
| 关联工具调用 | `tool_use_id` → 与同一 `tool_use` 块互相跳转           |

| 动作               | 行为                                                                                                                                    |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| **允许并重试**     | 将 `tool_name` 写入会话 `permission_allowlist` → 以 `--allowedTools` **重建会话**（复用 `cli_session_id` 续接）→ 自动重发上一条用户指令 |
| **改用更宽松预设** | 以 `acceptEdits` / `auto` 重建会话并续接                                                                                                |
| **在终端中执行**   | 切到终端视图，由用户亲自应答/执行（见 §9.4）                                                                                            |
| 忽略               | 仅记录；Agent 会自行调整策略                                                                                                            |

> 实测印证：模型被拒后会输出类似
> _"…please grant write permission for this path and I'll retry."_
> —— 因此「允许并重试」与模型自身行为天然对齐。

### 9.4 终端视图：当前唯一的"逐次确认"路径（Q2 价值兑现）

`mode = terminal` 下 CLI 为原生 TUI，**权限询问由 CLI 自行渲染，用户在终端直接作答**。
这是当前唯一能实现"逐次交互确认"的路径，因此：

- 被拒卡片中的「在终端中执行」不是降级选项，而是**功能等价路径**；
- 会话创建面板需明示："需要逐次授权确认时，请使用终端模式"。

### 9.5 演进路径（保留）

若该 CLI 后续提供独立 `--permission-prompt-tool`（MCP 工具）或官方 SDK host 模式，
可升级为真正的 `control_request` / `control_response` 逐次确认（协议形态见 §19.2）。
届时本节的"预设 + 白名单"仍作为默认免打扰层保留。

---

## 10. 任务/需求上下文绑定（Q7）

### 10.1 上下文块

会话创建时若携带 `task_id` / `requirement_id`，由 `SessionService` 生成一条 `context` 块写入消息表：

| 字段                                                        | 内容                                                                                                                           |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `block_type`                                                | `context`                                                                                                                      |
| `role`                                                      | `system`                                                                                                                       |
| `content`                                                   | 结构化摘要（JSON 字符串）：项目名、需求标题/优先级/状态、任务标题/类型/状态、工作目录、`agent_path`、`mode`、`permission_mode` |
| `related_task_id` / `related_requirement_id` / `project_id` | 冗余存储，便于检索与成本归属                                                                                                   |

前端以"**上下文卡片**"渲染（非气泡）：显示任务/需求/项目的可点击链接（跳转对应详情页）+ 会话配置摘要。

### 10.2 注入策略

| 设置                                    | 行为                                                                                                                                   |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `session_inject_context = true`（默认） | 首轮指令发送时，通过 `--append-system-prompt` 注入上下文摘要（避免污染用户消息体），并在消息流中登记一条 `system` 块说明"已注入上下文" |
| `session_inject_context = false`        | 仅落库展示，不注入 CLI                                                                                                                 |

### 10.3 消息级关联

每条消息块均携带 `related_task_id` / `related_requirement_id` / `project_id`（会话创建时确定并沿用），使：

- `list_session_messages` 支持按任务/需求过滤；
- Token/成本可按需求/任务归集（对接 F041–F044 与 `token_usage`）。

---

## 11. 前端设计

### 11.1 组件结构

| 组件                                          | 职责                                                                                                                                |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `components/SessionChat.tsx`（新增）          | 消息视图：消息列表 + 输入区 + 滚动管理                                                                                              |
| `components/MessageBubble.tsx`（新增）        | 单块渲染：正文（Markdown）/ 思考（折叠）/ 工具 / 结果 / 上下文卡 / 错误                                                             |
| `components/ThinkingBlock.tsx`（新增）        | 思考块：默认折叠，头部显示"已思考 Ns · M tokens"，展开后纯文本 pre-wrap                                                             |
| `components/ToolCallBlock.tsx`（新增）        | 工具名 + 入参（JSON 折叠）+ 结果（截断/展开）+ 状态（成功/被拒/失败）                                                               |
| `components/PermissionDeniedCard.tsx`（新增） | 被拒卡片（内联于消息流）：被拒工具名 + `message` 原文 + 跳转关联工具调用 + 三个动作（允许并重试 / 改用 acceptEdits / 在终端中执行） |
| `components/SessionInput.tsx`（新增）         | 输入区：多行（Enter 发送 / Shift+Enter 换行）、发送、停止（中断）、输入历史 ↑/↓、模式与权限提示                                     |
| `components/AgentTerminal.tsx`（改造）        | 终端视图：`readOnly` 属性（结构化会话强制只读）、原始事件行格式化展示                                                               |
| `pages/SessionDetail.tsx`（改造）             | 承载双视图切换 + 会话信息面板                                                                                                       |
| `stores/sessionMessagesStore.ts`（新增）      | 消息列表、增量缓冲、事件订阅、分页加载                                                                                              |

### 11.2 Markdown 与高亮

- 新增前端依赖：`react-markdown` + `remark-gfm` + `highlight.js`（或 `shiki`）
- 安全：`react-markdown` 默认不渲染原始 HTML，需显式关闭 `rehype-raw`，避免 XSS
- 代码块：右上角"复制"按钮；超长行横向滚动

### 11.3 滚动与增量体验

| 场景         | 行为                                                 |
| ------------ | ---------------------------------------------------- |
| 用户位于底部 | 新内容到达时自动滚动到底部                           |
| 用户已上滚   | **暂停自动滚动**，显示"↓ 新消息"浮标，点击回到底部   |
| 思考增量     | 折叠态只更新"已思考 Ns"与 Token 计数，不引起布局抖动 |

### 11.4 视图切换

- `SessionDetail` 顶部 Segmented 控件：`消息` / `终端`
- `mode = structured` 时选"终端"→ 视图为只读，输入区替换为提示条："原始事件视图（只读）；如需交互请切换会话模式"
- `mode = terminal` 时选"消息"→ 顶部提示条："当前为终端会话，消息视图内容由终端输出粗提取，可能不完整"

---

## 12. 状态机

### 12.1 会话（在既有 3 态基础上扩展字段，不改 DB CHECK）

| 当前                   | 可流转至    | 触发                                          |
| ---------------------- | ----------- | --------------------------------------------- |
| `running`              | `completed` | 进程退出码 0 / 用户停止 / `result` 且驱动关闭 |
| `running`              | `failed`    | 退出码非 0 / 协议解析失败 / 启动失败          |
| `completed` / `failed` | （终态）    | 需继续请用 `resume_session` 或新建会话        |

> `agent_sessions.status` 的 DB CHECK 仍为 `running` / `completed` / `failed`（不新增 `terminated`，保持既有约束）。

### 12.2 消息块

```
tool_use 块：          (创建) ──► streaming ──► completed
                                      ├──────► failed             （工具执行失败 / 解析失败）
                                      └──────► denied             （收到 permission_denied）

permission_denied 块： (创建) ──► pending ──► retried              （用户选择"允许并重试"，已重建会话补白名单）
                                      ├───────► switched_terminal （用户切到终端自行执行）
                                      └───────► ignored           （用户忽略，Agent 自行调整策略）
```

---

## 13. 异常与降级

| 异常                                         | 处理                                                                                                                                                     |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CLI 不支持 `stream-json`                     | `set_session_mode(structured)` 前置校验失败 → 提示并建议终端模式                                                                                         |
| 不支持 `structured_input`（如 Codex 待确认） | 采用"逐轮进程 + resume"模型；UI 标注"执行中不可追加指令"                                                                                                 |
| stdout 出现非 JSON 行                        | 记录为 `system` 块（`stderr`/原始文本），不中断解析                                                                                                      |
| 单行超长（> 1MB）                            | 截断并告警（防内存问题）                                                                                                                                 |
| 解析连续失败 N 次                            | 会话置 `failed`，保留已落库消息，提示切换终端视图排障                                                                                                    |
| 事件通道丢弃                                 | `session:message` 基于 `(session_id, seq)` 幂等；前端每次进入会话**全量拉取**最近一页做校准                                                              |
| 应用重启后 `running` 会话                    | 既有 `reconcile_running_sessions` 标记为 `failed`；消息保留可回放                                                                                        |
| 用户停止                                     | 先写 `system` 块"用户中断"，再 kill 进程（沿用 F030 的 kill 逻辑）                                                                                       |
| **工具调用被权限拒绝**                       | 捕获 `system/permission_denied` → 落 `permission_denied` 块 → 内联卡片给出三个动作；**不中断会话**，Agent 通常会自行调整策略（实测会输出"请授权后重试"） |
| 被拒后选择"允许并重试"                       | 以 `--allowedTools` 追加该工具 + `--resume <cli_session_id>` **重建会话**并自动重发上一条指令；重建失败则保留原会话与全部消息，仅提示                    |
| 同会话高频被拒（≥3 次）                      | 卡片聚合为一条提示"建议改用 acceptEdits 或切换终端模式"，避免刷屏                                                                                        |
| 预设与预期不符（如 acceptEdits 下仍被拒）    | 记录 `tool_name` 与 `message` 到日志；卡片提供"查看当前 CLI 权限能力"链接（指向本设计 §9.1 的实测结论）                                                  |

---

## 14. 非功能需求

| 维度       | 要求                                                                           |
| ---------- | ------------------------------------------------------------------------------ |
| 流式延迟   | 首字（首块）< 800ms；增量间隔 < 100ms                                          |
| 写放大控制 | 增量不落库，仅完整块落库；`raw_json` 默认关闭                                  |
| 数据量     | 单会话消息数上限 10,000（超出提示导出/新开会话）；thinking 全文保留            |
| 隐私       | 思考与消息仅本地存储；`raw_json` 可能含工具入参中的敏感值，默认不保留          |
| 兼容       | Windows ConPTY（终端模式）+ 匿名管道（结构化模式，无需 PTY）                   |
| 一致性     | 消息顺序以 `seq` 为唯一依据；`created_at` 仅用于展示                           |
| 可观测     | 每轮记录 `duration_ms` / `usage` / `is_error`，落 `result` 块并对接 Token 统计 |

---

## 15. 验收标准

**F076 消息持久化**

```
Given 结构化会话 S 处于 running
When  发送指令并等待一轮回复完成（含 thinking 与一次工具调用）
Then  session_messages 中存在：1 条 user、≥1 条 thinking、≥1 条 text、1 条 tool_use、1 条 tool_result、1 条 result，
      且 seq 连续无空洞

Given 上述会话已结束
When  重新打开会话详情页
Then  消息与思考块按 seq 完整回放，思考块默认折叠
```

**F077 指令输入**

```
Given 结构化会话 S 处于 running
When  在输入框输入"列出当前目录"并回车
Then  用户消息立即上屏、写入 session_messages，Agent 开始回复

Given 会话 S 已 completed
When  尝试发送指令
Then  输入被禁用并提示"会话已结束，请重建或续接"
```

**F078 流式回复**

```
Given Agent 正在输出 Markdown（含代码块）
When  观察界面
Then  文本增量出现（无整段等待），代码块正确高亮并可复制
```

**F079 思考内容**

```
Given 模型返回 thinking
When  回复完成
Then  出现默认折叠的思考块，头部显示耗时与 Token；展开后内容与落库 thinking 块一致

Given 关闭设置 session_show_thinking 后重开页面
When  渲染历史消息
Then  思考块按设置隐藏/展开
```

**F081 双视图**

```
Given 会话 mode = structured
When  切换到终端视图
Then  显示只读的原始事件流，输入区被替换为"只读"提示，且无法输入

Given 会话 mode = terminal
When  切换到消息视图
Then  显示"粗提取、可能不完整"提示，内容只读
```

**F083 权限（预设 + 被拒可视化，替代"不可用的逐次确认"）**

```
Given 会话使用 permission_mode = acceptEdits
When  Agent 执行文件写入
Then  写入成功执行，消息流出现 tool_use(completed) 与 tool_result，不出现 permission_denied

Given 会话使用 permission_mode = manual
When  Agent 执行文件写入
Then  收到 system/permission_denied，消息流出现 permission_denied 卡片，
      卡片展示 tool_name 与 message 原文，并给出"允许并重试 / 改用 acceptEdits / 在终端中执行"三个动作

Given 被拒卡片上点击"允许并重试"
When  会话以 --allowedTools 追加该工具重建（复用 cli_session_id 续接）并重发上一条指令
Then  同一操作不再被拒，且原会话消息与思考内容在重建后仍可按会话查看

Given 需要逐次授权确认
When  用户切换到终端视图
Then  可在原生 TUI 中逐次应答权限询问（当前唯一的逐次确认路径）
```

**F082 续接**

```
Given 会话 S 已完成且 cli_session_id 已落库
When  调用 resume_session(S)
Then  新进程使用同一 --session-id 续接，历史上下文在 Agent 侧延续，平台消息表继续追加
```

**F60 回归**：`start_agent` 启动结构化会话后，`agent_sessions.agent_id` 外键成立（沿用既有回归用例）。

---

## 16. 分期与工作量

| 阶段   | 交付               | 关键工作                                                                                                                                                                                       | 估算 |
| ------ | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--: |
| **P0** | 结构化会话可用闭环 | 新表与迁移；`SessionDriver` + Claude 适配（参数与事件映射）；`MessageNormalizer`；5 条命令 + 6 类事件；`SessionChat`/`SessionInput`/`MessageBubble`；Markdown 渲染；**默认预设 `acceptEdits`** |  大  |
| **P1** | 核心体验完整       | thinking 折叠（落库 + UI）；工具块；`--session-id` 绑定与 `resume_session`；上下文卡片与注入（Q7）；**权限：`permission_denied` 卡片 + 三个动作 + `allowedTools` 白名单沉淀**                  |  中  |
| **P2** | 工程完备           | Codex 适配（含逐轮降级模型）；Token/成本按消息归属；消息检索与分页；可选"无人值守模式"（`--permission-prompts none` + 白名单，UI 明确标注后果）                                                |  中  |
| **P3** | 体验优化           | 终端视图原始事件格式化与导出；输入历史；虚拟滚动；无响应检测（`last_event_at`）                                                                                                                |  小  |

**影响面（文件级）**

| 层        | 文件                                                                                                                                                                                                   |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| DB        | `db/mod.rs`（建表/迁移/索引）、新增 `db/session_messages.rs`、`db/integration_tests.rs`（新用例）                                                                                                      |
| 进程/适配 | 新增 `agent_adapter.rs`（trait）、`agent_adapter/claude.rs`、`agent_adapter/codex.rs`、`session_driver.rs`                                                                                             |
| 服务      | 新增 `services/session_service.rs`；`services/agent_service.rs`（接入 mode/permission）                                                                                                                |
| 命令      | `commands.rs`、`main.rs`（注册 5 命令 + 事件）                                                                                                                                                         |
| 前端      | 新增 `SessionChat`/`MessageBubble`/`ThinkingBlock`/`ToolCallBlock`/`SessionInput`、`stores/sessionMessagesStore.ts`；改造 `AgentTerminal`、`Sessions`、`SessionDetail`、`lib/tauri.ts`、`lib/types.ts` |
| 依赖      | 前端 `react-markdown` + `remark-gfm` + `highlight.js`                                                                                                                                                  |

---

## 17. 风险与待办（实现前必须闭环）

|  #  | 风险                                                                                                                                                    | 处理                                                                                                                                                                                       |
| :-: | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1  | **`stream-json` 输入 schema 未实测**（本机只读 help，未做端到端）                                                                                       | P0 第一步用最小实验锁定：`echo '<json>' \| claude -p --input-format stream-json --output-format stream-json` 观察回显                                                                      |
| R2  | `--verbose` 是否为必需                                                                                                                                  | 同上实验确认                                                                                                                                                                               |
| R3  | **Codex 参数与事件格式未验证**（本机未安装）                                                                                                            | P2 前置：安装 codex 后按 §5.3 清单实测；未通过则仅交付"逐轮降级模型"                                                                                                                       |
| R4  | CLI 版本漂移导致事件字段变化                                                                                                                            | 适配层内做**字段容错**（缺失字段不 panic）；在 `system` 块记录 CLI 版本，异常时提示                                                                                                        |
| R5  | 双视图语义被误用（在只读终端里输入）                                                                                                                    | UI 强制禁用输入 + 文案说明；后端 `send_session_message` 对 `mode=structured` 的终端输入不提供通道                                                                                          |
| R6  | thinking 体积导致库膨胀                                                                                                                                 | 提供"按会话清理思考"与"全局保留策略"（P2）；默认保留全文（符合 Q4）                                                                                                                        |
| R7  | ~~`--permission-prompts none` 导致任务静默失败~~                                                                                                        | **已消解**：不再依赖 `none`；改用 §9.2 的「预设 `acceptEdits` + `--allowedTools` 白名单 + 被拒卡片」方案，被拒操作会**显式呈现原因并支持一键重试**                                         |
| R9  | ~~`control_response` 的精确 JSON 形状未实测~~                                                                                                           | **已实测并证伪该路径**：本发行版下 `manual + host` 不产生 `control_request`（0 次），直接下发 `permission_denied`；且无独立 `--permission-prompt-tool` flag → 改用 §9.2 方案（详见 §19.4） |
| R10 | 本机 `claude` 为第三方包装（QClaw 转发 + 非 Anthropic 模型 `mimo-v2.5`，stderr 报 `unrecognized_model`），`--permission-*` 实际行为可能与官方发行版不同 | 适配层不假定模型；从 `system.init` 记录 `model` 与 CLI 版本；权限链路异常时提示"当前 CLI 为自定义发行版，请核对权限参数支持"。**官方发行版需另做一次 §19.4 的两项验证**                    |
| R11 | 用户级 MCP server 会污染 `system.init.tools`（实测出现 `mcp__modao__*` 等），导致工具块列表与"白名单"语义变宽                                           | 工具块渲染按前缀分组折叠（`mcp__` 归入"MCP 工具"）；`allowedTools` 白名单仅写入用户显式点选的具体工具名，不做通配                                                                          |
| R8  | 消息表与既有 `reconcile_running_sessions` 的终态语义                                                                                                    | 会话重启后消息保留；`streaming` 块由对账逻辑一并收尾为 `failed`                                                                                                                            |

---

## 18. 与现有文档的同步项

| 文档                             | 变更                                                                                                                                                                  |      状态       |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------: |
| `FEATURES.md`                    | 新增"模块 16：会话交互与消息"，功能编号 **F076–F086**（11 项，📋 设计中）；状态标记新增 📋；功能总数 75 → **86**；号段与模块统计同步；附录 B 增 v2.4                  |    ✅ 已完成    |
| `FEATURES.md`                    | 实现后同步：数据表 18 → **19**、IPC 命令 82 → **88**、Tauri 事件 3 → **9**；模块 16 状态 📋 → **P0 已实现**（v2.5）                                                   | ✅ 已完成（P0） |
| `FEATURES.md`                    | 修正 F022/F023/F029（终端通道语义）；F048 已实现（结构化会话单轮结束写入 `token_usage`（`source=session`）并自动累计预算）；F027（会话详情加消息流）已随 P0 实现      |   ⚠️ 部分完成   |
| `docs/testing/full-test-plan.md` | L6 88/88 已同步；L1 组件用例：`MessageBubble` 6 例、`SessionInput` 7 例、`SessionChat` 7 例（含契约拉取/增量幂等/事件过滤）；L3 契约用例由 `SessionChat` 集成用例承担 |    ✅ 已完成    |
| `CLAUDE.md`                      | 项目结构新增 `agent_adapter.rs`、`session_driver.rs`、`stores/sessionMessagesStore.ts`、`SessionChat`/`SessionInput`/`MessageBubble`                                  |    ⏳ 待同步    |

### 18.1 P0 实施记录（2026-09-21）

**交付范围**：§20 P0「结构化会话可用闭环」已全部落地。

| 层     | 交付物                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 数据层 | `session_messages` 表（17 字段 + 4 索引，`session_id+seq` 唯一）；`agent_sessions` 9 列迁移；启动时残留 `streaming` 块收尾                                                                                                                                                                                                                                                                                                   |
| 适配层 | `agent_adapter.rs`：`build_command`（`-p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --permission-mode --allowedTools --add-dir --append-system-prompt`，`.ps1/.cmd` 包装自动转解释器）、`encode_user_message`（§19.2.1 已锁 schema）、`decode_line`（init/thinking_tokens/permission_denied/assistant/user/stream_event/result → 8 类归一化事件，hook/status 事件容错忽略） |
| 驱动层 | `session_driver.rs`：spawn/registry/读取循环（`select!` 中断 + kill）、逐事件落库与推送、退出收尾（`fail_streaming` + `finish_agent_session`）、stderr 单独消费防管道阻塞                                                                                                                                                                                                                                                    |
| IPC    | `start_structured_session` / `send_session_message` / `list_session_messages` / `interrupt_session` / `set_session_mode` / `resume_session`（共 6 条，命令总数 82 → 88）                                                                                                                                                                                                                                                     |
| 前端   | `SessionChat`（增量缓冲 + 自动滚动 + 加载更早）、`SessionInput`（Enter 发送 / 中断）、`MessageBubble`（思考折叠、工具块、被拒卡片、上下文卡片、单轮结果）、`sessionMessagesStore`（事件订阅 + 落库幂等）、`SessionDetail` 双视图、`Agents` 页「消息模式启动」                                                                                                                                                                |

**实测结论落地**：

- 默认权限预设 **`acceptEdits`**（`DEFAULT_PERMISSION_MODE`），被拒卡片提供「允许并重试」（并入 `--allowedTools` 白名单 + `resend_last` 自动重发上一条指令）与「改用 acceptEdits」两个出口；
- 上下文注入默认开启（`session_inject_context`），以 `context` 块落库并在 `--append-system-prompt` 中携带任务/需求摘要（Q7）；
- `--resume` 优先于 `--session-id`：重建/续接会话时若已有 `cli_session_id` 则走续接（F082）。

**回归验证**：`cargo test` 42 + 71 + 3 全通过、Rust 告警 0；前端 259 用例全通过（L6 IPC 88/88 = 100%，新增 `MessageBubble` 6 用例）。

**P0 未含（后续批次）**：多 Agent 适配（codex，R3 待实测）、完整 Markdown 渲染、逐次授权确认（依赖终端视图）、`token_usage` 消息级记账（F048）、`SessionChat`/`SessionInput` 组件级单测。

---

## 19. 实测记录（2026-09-21，本机 claude 2.1.266）

### 19.1 已确认的事件结构（真实调用产出）

| 事件                                       | 关键字段                                                                                                                                                                               | 归一化用途                                                          |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `system` / `hook_started`、`hook_response` | `hook_name`、`exit_code`、`outcome`                                                                                                                                                    | 识别用户级 Hook 干扰（可折叠展示或忽略）                            |
| `system` / `init`                          | `cwd`、`session_id`、`tools[]`、`model`                                                                                                                                                | 回写 `cli_session_id`、工作目录、可用工具清单                       |
| `system` / `thinking_tokens`               | `estimated_tokens`、`estimated_tokens_delta`                                                                                                                                           | 折叠态"已思考 N tokens"**实时**更新（无需等 thinking 块）           |
| `assistant`                                | `message.content[]`（`{type:"thinking","thinking":...,"signature":...}` 或 `{type:"text","text":...}`）、`message.usage`、`parent_tool_use_id`                                         | 归一化为 thinking / text 块；子代理消息靠 `parent_tool_use_id` 归属 |
| `result`                                   | `duration_api_ms`、`stop_reason`、`total_cost_usd`、`usage.{input_tokens, cache_creation_input_tokens, cache_read_input_tokens, output_tokens, output_tokens_details.thinking_tokens}` | 单轮结束统计；**成本与缓存 Token 可直接取用**（无需平台自建价格表） |

> 实测示例：thinking 与 text 是**同一 `message.id` 的两条独立 `assistant` 事件**：
>
> ```json
> {"type":"assistant","message":{"id":"msg_...","content":[{"type":"thinking","thinking":"...","signature":""}]}}
> {"type":"assistant","message":{"id":"msg_...","content":[{"type":"text","text":"ok"}]}}
> ```

### 19.2 权限协议（已实测，结论：该发行版不支持 host 逐次确认）

| 侧                     | 实测结果                                                                                                              |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------- |
| CLI 默认值             | ✅ `--permission-prompts` 默认 `host`（`choices: "host", "none", default: "host"`）                                   |
| 独立参数               | ❌ **不存在 `--permission-prompt-tool` flag**（`--help` 全文仅在其说明文字中出现）                                    |
| 请求侧                 | ⚠️ **实测未出现**：`--permission-mode manual --permission-prompts host` 下发起 Write，`control_request` 出现 **0 次** |
| 实际行为               | ✅ 直接下发 `system/permission_denied`（工具未执行）                                                                  |
| `manual` 下 Write      | 被拒（`permission_denied`）                                                                                           |
| `acceptEdits` 下 Write | **成功执行**（文件被创建，随后被更新）                                                                                |

**`system/permission_denied` 实测原文字段**：

```json
{
  "type": "system",
  "subtype": "permission_denied",
  "tool_name": "Write",
  "tool_use_id": "call_0bd0f6873e3146c48feb0e19",
  "message": "Claude requested permissions to write to <path>, but you haven't granted it yet.",
  "uuid": "...",
  "session_id": "..."
}
```

### 19.2.1 输入协议与增量块（已实测）

| 项                            | 实测结果                                                                                                                                                         |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 用户消息 schema（R1）         | ✅ `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"..."}]}}`（配合 `--replay-user-messages` 可见 `user` 回显事件）                    |
| `--verbose`（R2）             | ✅ 与 `-p + stream-json` 组合可正常输出完整事件                                                                                                                  |
| `--include-partial-messages`  | ✅ 产生 `stream_event`：`content_block_delta` + `delta.type = thinking_delta`（含 `thinking` 字段）/ `text_delta`（含 `text` 字段），带 `index` 区分为不同内容块 |
| `--input-format text`（默认） | ⚠️ CLI 会在 3s 内等待 stdin 数据，超时后 `Warning: no stdin data received in 3s, proceeding without it.` → **结构性输入必须显式用 `--input-format stream-json`** |
| 额外事件                      | `system/status`、`system/hook_started`、`system/hook_response` 均实际出现，驱动需容错                                                                            |

### 19.2.2 工具结果结构（已实测）

`tool_result` 位于 **`type: "user"`** 事件中，且额外携带结构化 `tool_use_result`：

```json
{
  "type": "user",
  "message": {
    "role": "user",
    "content": [{ "tool_use_id": "call_...", "type": "tool_result", "content": "...", "is_error": false }]
  },
  "tool_use_result": {
    "type": "create",
    "filePath": "...",
    "content": "hello",
    "stdout": "...",
    "stderr": "",
    "interrupted": false
  }
}
```

`result` 事件额外提供 `modelUsage`（含 `costUSD`、`contextWindow`、`maxOutputTokens`、`canonicalModel`、`provider`）与 `service_tier`、`cache_creation{ephemeral_1h/5m}`、`inference_geo`、`speed`。

### 19.3 环境注意（影响验证与预期）

- 本机 `claude` 为 **第三方包装**（QClaw 的 `.ps1` shim → `node_modules/@anthropic-ai/claude-code/bin/claude.exe`），实际模型为 **`mimo-v2.5`**，stderr 报 `[claude-code:unrecognized_model]`；
- 已配置 **SessionStart Hook**（`hook_started` / `hook_response` 事件可见）→ **驱动必须容忍额外的 `system` 事件**，不能假定事件种类固定；
- 协议层结论仍然有效（stream-json 事件由 Claude Code 本体产生），但**模型能力与权限行为需以该发行版实测为准**（见 R10）。

### 19.4 实测完成情况

| 编号 | 内容                                         | 结果                                         |
| :--: | -------------------------------------------- | -------------------------------------------- |
|  R1  | `--input-format stream-json` 用户消息 schema | ✅ 已锁定（§19.2.1）                         |
|  R2  | `--verbose` 是否必需                         | ✅ 已验证可用                                |
|  R9  | `control_response` 应答形态                  | ✅ 实测**该发行版无此通道** → 改用 §9.2 方案 |
|  —   | `permission_denied` 字段                     | ✅ 已锁定（§19.2）                           |
|  —   | 增量块（thinking_delta/text_delta）          | ✅ 已锁定（§19.2.1）                         |
|  R3  | codex 参数与事件格式                         | ⏳ 本机未安装，待安装后按 §5.3 清单实测      |

**本轮实测总成本**：约 **$0.49**（4 次探测调用：$0.141 + $0.283 + $0.065 + $0.28）。

> 待办：在**官方 Anthropic 发行版**上复测 §19.2 的权限行为，以确认"host 不可达"是本发行版特有还是通用结论（对应 R10）。

---

_文档版本：v1.2（Q3-v3：实测定稿权限方案 —— 预设 + 白名单 + 被拒卡片 + 终端兜底）_
_最后更新：2026-09-21_
_R1 / R2 / R9 已实测闭环；R3（codex）与"官方发行版复测"为实现前待办；实测记录见 §19。_
