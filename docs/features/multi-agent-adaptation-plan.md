# 多 CLI Agent 适配（业务需求方案 + 技术实现方案）

**文档状态**：方案定稿，M1 开发中
**版本**：v1.0
**日期**：2026-09-22
**关联**：`session-interaction-design.md`（模块 16，F076–F086，P0 已实现）、`FEATURES.md`
**范围说明**：仅覆盖**标准 CLI Agent**（Claude/Gemini/Qwen/Codex/opencode/Aider/ZCode）；
QClaw/WorkBuddy/OpenClaw 等桌面工作台与网关类产品**不在本期范围**（2026-09-22 决策）。

---

# 第一部分：业务需求方案

## 1. 背景与问题定义

平台已建成统一会话交互层（模块 16，P0 已实现）：消息模式会话、思考/工具/正文结构化展示、
上下文绑定、成本记账。但适配层（`agent_adapter.rs`）硬编码支持 Claude Code 一家。

业务问题：

1. 团队成员使用不同 CLI Agent，无法纳入平台统一管理（会话、成本、审计均有缺口）；
2. PRD 生成等平台能力强绑定单一厂商，用户无选择权、成本无优化空间；
3. 新 Agent 出现时需要改代码才能支持，响应慢。

本质：把"会话"从 Claude 专属能力升级为**平台与任意 CLI Agent 的标准交互协议**。

## 2. 业务目标与成功度量

| 目标         | 度量                                              |
| ------------ | ------------------------------------------------- |
| Agent 覆盖面 | 主流 CLI 安装即可用，覆盖率 ≥ 7/7                 |
| 体验一致性   | 支持 L3 的 Agent：消息模式功能项 100% 对齐 Claude |
| 零感知切换   | 同一套 UI，能力差异自动降级并明示                 |
| 平台能力普惠 | PRD 生成/智能完善对全部 L2+ Agent 可用            |
| 成本可视     | 会话 Token/成本按 Agent+模型归因，跨 Agent 可比   |

**能力分级**（贯穿全文的口径）：

- **L1 终端模式**：PTY 交互，所有 CLI 天然支持（已实现）
- **L2 一次性调用**：`prompt → text` 单轮调用（PRD 生成/智能完善）
- **L3 结构化事件流**：双向结构化协议（消息模式会话）

## 3. 目标用户与画像

| 画像         | 特征                   | 核心诉求                                            |
| ------------ | ---------------------- | --------------------------------------------------- |
| 多工具开发者 | 同时装 2+ 个 CLI Agent | 统一入口管理，避免多终端切换                        |
| 成本敏感用户 | 关注 API 花费          | 按 Agent 对比成本；CLI 登录态通道（免 API Key）可用 |
| 团队负责人   | 统一团队工具链         | 指定默认 Agent，会话/审计可追溯                     |
| 平台管理员   | 负责接入配置           | 新 Agent 免代码接入（定义表 + 配置驱动）            |

## 4. 核心使用场景（含异常流程）

**S1 消息模式协作（L3）**

- 主流程：Agent 页看到「结构化」徽章 → 消息模式启动 → 发指令 → 结构化展示 → 重进可回放
- 异常：CLI 未登录 → 启动失败并提示登录；协议不兼容 → 自动降级终端模式并告知原因

**S2 PRD 生成自选引擎（L2+）**

- 主流程：设置页选择引擎（默认跟随任务 Agent / 显式指定）→ 需求页生成 → 落库
- 异常：所选 CLI 未安装 → 明确报错并给出可选引擎列表

**S3 任务触发会话（L2+）**

- 主流程：任务详情「启动 Claude 会话」→ 按任务/默认设置选 Agent → 进入消息/终端视图
- 异常：无可用 Agent → 提示刷新检测

**S4 能力透明（全部）**

- 主流程：Agent 卡片展示能力徽章（🟢结构化 / 🔵一次性 / ⚪仅终端）+ 版本
- 异常：探测失败 → 按「仅终端」保守处理，不阻塞列表

## 5. 需求清单

| 编号  | 需求                                            | 优先级 | 验收标准                                                                                                                          |
| ----- | ----------------------------------------------- | :----: | --------------------------------------------------------------------------------------------------------------------------------- |
| MA-01 | Agent 能力探测与登记（flavor + 能力等级持久化） |   P0   | 刷新检测后 `agents.capabilities` 含 flavor 与 structured/one_shot 标记                                                            |
| MA-02 | 能力分级 UI 与降级入口                          |   P0   | 不支持 L3 的 Agent 消息按钮置灰 + tooltip；能力徽章正确                                                                           |
| MA-03 | Gemini CLI 结构化会话                           |   P1   | 消息模式：指令下发/正文/工具/单轮结果/中断/回放全通过（**需本机安装后实测**）                                                     |
| MA-04 | Qwen Code 结构化会话（与 MA-03 共用实现）       |   P1   | 同 MA-03                                                                                                                          |
| MA-05 | 一次性调用通道适配（PRD/完善按 flavor 选参）    |   P1   | 任一 L2 Agent 完成 PRD 生成与智能完善，落库与记账正常                                                                             |
| MA-06 | Codex 结构化会话                                |   P2   | 同 MA-03（`exec --json` 事件映射，**需实测**）                                                                                    |
| MA-07 | opencode 适配                                   |   P2   | L2 必达；L3 视实测协议决定                                                                                                        |
| MA-08 | 纯一次性工具降级路径（Aider 类）                |   P2   | 仅 L1/L2；消息入口置灰，文案明确                                                                                                  |
| MA-09 | 默认 Agent 与引擎后台配置                       |   P1   | 设置页可选默认 Agent（`default_agent_path`）与 PRD 引擎（`prd_generation_mode`，已有）                                            |
| MA-10 | Claude 零回归                                   |   P0   | trait 化重构后现有 42+71+3 后端 / 277 前端用例全通过                                                                              |
| MA-11 | ZCode CLI 适配                                  |   P2   | L2 必达；L3 视 ZCode Protocol 实测（社区有协议先例：Open ACE PR #1074）。注意区分 CLI（适配对象）与 Z Code 桌面编辑器（不在范围） |

**环境状态（2026-09-22 探测）**：本机仅 `claude` 已安装；gemini/qwen/zcode/codex/opencode/aider
均未安装 → 各适配器按公开协议实现 + fixture 单测，**实测验收挂起到对应 CLI 安装后**（上述"需实测"标注）。

## 6. 非功能需求

1. 协议容错：未知事件/字段忽略或降级为 Notice（Claude 实测教训）；
2. 降级策略：L3 失败自动回退 L1 终端模式并告知原因；
3. 报错可操作：失败提示包含下一步动作（安装/登录/刷新检测）；
4. 探测不阻塞：能力探测异步执行；
5. 可扩展：新增 Agent = 一个 adapter 实现 + 定义表一条记录，不改驱动层/前端。

## 7. 业务规则与约束

- 各 Agent 登录态/鉴权自管，平台不存 Agent 凭据；
- 权限预设语义不做跨 Agent 承诺，差异如实展示；
- 成本口径：有 usage 记实际值；无 usage（CLI 文本通道）记 0 并注明；
- 会话/审计保留现有能力，`agent_type` 按实际 flavor 归因。

## 8. 里程碑与发布策略

| 里程碑 | 内容                         | 发布动作                                    |
| ------ | ---------------------------- | ------------------------------------------- |
| M1     | MA-01/02/09/10 + MA-03/04/05 | 随 v2.6；Gemini/Qwen 实测通过后启用对应入口 |
| M2     | MA-06/07/11                  | 对应 CLI 本机安装实测后灰度                 |
| M3     | MA-08 + 文档同步             | 下个版本                                    |

## 9. 业务风险与依赖

| 风险                | 影响             | 对策                            |
| ------------------- | ---------------- | ------------------------------- |
| CLI 协议版本漂移    | 适配失效         | 能力探测 gate + 降级 + 版本记录 |
| 目标 CLI 未本机安装 | 无法实测验收     | 里程碑前置「安装确认」检查点    |
| 第三方包装环境差异  | 行为与官方不一致 | 探测结果如实标注来源与版本      |

---

# 第二部分：技术实现方案

## 1. 现状架构（分层已就绪）

```
agent.rs(发现/探测) → agent_adapter.rs(❗Claude 硬编码) → session_driver.rs(只消费 AgentEvent)
                                                          → commands → 前端 store/组件
```

关键收益点：`session_driver.rs` 与前端只依赖归一化事件 `AgentEvent`
（Init/Delta/ThinkingTokens/Block/ToolResult/PermissionDenied/TurnResult/Notice），
新增 Agent 时**驱动层与前端零改动**。

## 2. 目标架构：trait + 注册表

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flavor { Claude, Gemini, Qwen, Codex, OpenCode, Aider, ZCode, Generic }

#[derive(Serialize, Clone, Copy)]
pub struct Capabilities { pub structured: bool, pub one_shot: bool }

pub trait AgentAdapter: Send + Sync {
    fn flavor(&self) -> Flavor;
    fn capabilities(&self) -> Capabilities;
    fn build_session_command(&self, cfg: &SessionLaunchConfig) -> Option<Command>;
    fn encode_user_message(&self, text: &str) -> String;
    fn decode_line(&self, line: &str) -> Vec<AgentEvent>;
    fn build_oneshot(&self, prompt: &str, model: Option<&str>, work_dir: Option<&str>) -> Option<Command>;
}

pub fn adapter_for(agent_path: &str) -> &'static dyn AgentAdapter;  // 按二进制文件名识别
```

- 现有 4 个自由函数（`build_command`/`encode_user_message`/`decode_line`/`run_oneshot`）
  原样收编为 `ClaudeAdapter` —— **Claude 行为零变化**；
- `run_oneshot` 的进程执行骨架（shell_wrap/.ps1 处理/错误截取）提为公共函数；
- **传输抽象预留**：`enum Transport { Process, HttpGateway }`（本期只实现 Process；
  网关型如 OpenClaw 若未来纳入，走 HttpGateway，驱动层事件源 trait 化后可复用）。

## 3. 协议映射（各 Agent → AgentEvent）

| Agent            | 会话启动                                                                                         | 用户消息              | 事件映射                                                                                                                                         | 状态                      |
| ---------------- | ------------------------------------------------------------------------------------------------ | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------- |
| Claude           | `-p --input-format stream-json --output-format stream-json --verbose --include-partial-messages` | `{"type":"user",...}` | 已实现（实测协议）                                                                                                                               | ✅                        |
| Gemini/Qwen      | `--output-format stream-json`                                                                    | prompt 传参           | `init→Init`、`message→Block(text)`、`tool_use→Block(tool_use)`、`tool_result→ToolResult`、`error→Notice(error)`、`result→TurnResult`；无思考增量 | 🔧 按公开协议实现，待实测 |
| ZCode            | ZCode Protocol（待实测确认参数面）                                                               | 待实测                | 待实测                                                                                                                                           | 🔧 占位实现（L2 保底）    |
| Codex            | `exec --json`                                                                                    | exec 参数             | thread 事件 → 逐项映射，未支持项降级 Notice                                                                                                      | M2                        |
| Generic（Aider） | ❌                                                                                               | ❌                    | 仅 `build_oneshot`                                                                                                                               | M3                        |

## 4. 能力探测（`agent.rs`）

- `AgentCapabilities` 增加字段（serde default，向后兼容）：
  `flavor: String`、`structured_events: bool`、`one_shot: bool`；
- `probe_capabilities()`（现有 `--help` 文本启发式）扩展：匹配 `stream-json`/
  `output-format`/`exec --json` 等特征，并按 `AgentDefinition.name` 直接判定 flavor；
- `AgentDefinition` 增加 `adapter: Flavor`；定义表 2 → 7 条
  （claude/codex/gemini/qwen/zcode/opencode/aider）。

## 5. 数据层

- **零新表**：flavor/能力写入现有 `agents.capabilities`（JSON，upsert 链路已通）；
- 会话归因靠 `agent_sessions.agent_id → agents` JOIN（已接通）。

## 6. IPC 与前端

- **无新命令**：`verify_agents` 返回列表自然携带能力字段；
- `Agents.tsx`：能力徽章；「消息模式启动」按 `capabilities.structured_events` 置灰；
- 设置页：默认 Agent 下拉（写 `default_agent_path`）；`create_session_from_task`
  选路逻辑优先读它，否则 `prd_agent_path`，否则第一个非失效 Agent；
- PRD 通道：`prd_generation_mode` 维持 api/agent；agent 通道内按 flavor 选 `build_oneshot`。

## 7. 测试与兼容

- 每 adapter 的 `decode_line` 以**真实采样载荷**为 fixture（沿用 Claude 适配层 9 用例模式）；
- 探测单测：help 文本 → 能力判定；
- 回归门槛：现有 42+71+3 后端、277 前端用例全通过（MA-10）。

## 8. 风险与对策

| 风险                                 | 对策                                                    |
| ------------------------------------ | ------------------------------------------------------- |
| CLI 协议版本漂移（Codex/ZCode 尤甚） | `--version` gate + 未知事件容错 + Notice 呈现           |
| 本机未装目标 CLI（当前实况）         | fixture 单测先行，实测验收挂起；UI 探测不到即不展示入口 |
| 第三方包装环境差异                   | 探测结果如实标注来源与版本                              |
