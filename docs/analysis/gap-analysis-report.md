# ACE 功能差距分析报告（需求 ↔ 代码 ↔ 测试 三方交叉）

> ⚠️ **历史快照文档（2026-09-21）**：本文的差距结论多数已在后续版本修复。
> 当前实现状态与已知缺口一律以 [`docs/STATUS.md`](../STATUS.md) 为准。

> 分析日期：2026-09-21
> 需求基线：`FEATURES.md` v2.0（75 项功能 / F001–F075）
> 核对范围：`src/`（18 页面 / 12 组件 / 6 Store / `lib/tauri.ts`）、`src-tauri/`（77 条 IPC 命令 / 11 Service / 18 表）、`tests/` 与 `src-tauri/tests/`、`docs/testing/*`

---

## 一、结论摘要

| 维度             | 结论                                                                                                                                                                                    |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 功能层面         | 75 项中 **8 项完全未实现**、**10 项部分实现**、**1 项未接线**；另发现 **2 项伪实现**（假成功）、**4 项"后端可用但前端不可达"**                                                          |
| IPC 命令         | 共 **77 条**；前端 L6 覆盖 **55/77 = 71.4%**（低于 80% 目标）；后端 `integration.rs` 覆盖 70/77 但**默认全部跳过**                                                                      |
| 完全无测试的命令 | **7 条**：`enrich_requirement`、`get_pipeline_stages`、`list_stage_skill_configs`、`get_stage_skill_config`、`upsert_stage_skill_config`、`remove_stage_skill_config`、`get_audit_logs` |
| 后端有效测试     | 未启用 E2E 模式时 **仅 2 个单测**（`crypto.rs`）实际执行                                                                                                                                |
| 前端测试         | 组件 12/12 有测试；**`themeStore` 无测试**；**18 个页面无单测**；E2E 仅 5 个冒烟文件                                                                                                    |
| 测试报告可信度   | `docs/testing/full-test-report.md` 与实际仓库**显著不一致**（详见第四章）                                                                                                               |
| 最高风险         | ① 数据导入导出为伪实现（导出空数组、导入不落库，却提示成功）② 审计 / 提交记录 / MCP 日志三条数据链路无写入 ③ PTY 输入、尺寸调整、进程终止为桩实现                                       |

---

## 二、分析口径与依据

| 项       | 说明                                                                                                                                                                   |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 需求基线 | `FEATURES.md` v2.0（F001–F075，✅56 / ⚠️10 / ⚪1 / ❌8）                                                                                                               |
| 前端核对 | 路由表（`src/App.tsx`）、页面（`src/pages/*.tsx`）、组件（`src/components/*.tsx`）、Store（`src/stores/*.ts`）、IPC 封装（`src/lib/tauri.ts`）                         |
| 后端核对 | `main.rs::generate_handler!`、`commands.rs`、`services/*`、`db/*`、`pty.rs`、`llm.rs`                                                                                  |
| 测试核对 | `tests/e2e`、`tests/l3-fullstack`、`tests/l4-cdp`、`tests/l5-chaos`、`tests/l6-ipc`、`src/**/__tests__`、`src-tauri/tests`、`vitest.config.ts`、`playwright.config.ts` |

---

## 三、功能层面缺口

### 3.1 完全未实现（❌ 8 项）

| 编号 | 功能         | 代码证据                                                                                                                | 后果                            |
| ---- | ------------ | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| F011 | 需求变更历史 | 全库无 `INSERT INTO requirement_history`                                                                                | 审计数据源恒空                  |
| F053 | 变更历史记录 | 同上                                                                                                                    | 同上                            |
| F054 | 审计日志页面 | `AuditLog.tsx` 调用 `getAuditLogs`，但源表恒空；`entity_type` 硬编码 `"requirement"`                                    | 页面长期空列表                  |
| F055 | 操作类型标记 | 依赖 `action` 字段，无数据写入                                                                                          | 无法渲染                        |
| F039 | MCP 执行日志 | 全库无 `INSERT INTO mcp_execution_logs`                                                                                 | `SessionDetail` 的 MCP 区块恒空 |
| F060 | 提交记录落库 | 全库无 `INSERT INTO git_commits`；`git_create_commit` 的 `_task_id`/`_session_id` 形参未被使用；无 worktree、无自动提交 | 提交与任务无法关联              |
| F073 | 数据导出     | 无后端命令（见 3.2 伪实现）                                                                                             | 假成功                          |
| F074 | 数据导入     | 无后端命令（见 3.2 伪实现）                                                                                             | 假成功                          |

### 3.2 伪实现（假成功）— F073 / F074

`src/pages/ImportExport.tsx` 属于"看起来能用、实际不生效"：

- **导出**：写出的 JSON 中 `requirements` / `tasks` / `projects` **恒为空数组**，不读取任何真实数据，却提示"数据导出成功"
- **导入**：仅 `JSON.parse` + 校验 `data.version`，随后直接提示"数据导入成功"，**不写入任何数据**
- 页面文案"导入将覆盖现有数据""建议先导出备份"与实际行为不符，**会误导用户认为已完成备份**

风险等级：**高**。

### 3.3 部分实现 / 未接线（⚠️ 10 项、⚪ 1 项）

| 编号 | 功能           | 具体缺口                                                                                       |
| ---- | -------------- | ---------------------------------------------------------------------------------------------- |
| F021 | Agent 状态管理 | 无二进制有效性复核、无进程存活对账；`Agents.tsx` 用 `useState` 而非 store，重进可能显示空      |
| F022 | PTY 会话       | `PtySessionMeta` 不保存 `child`/`master`/`writer` 句柄                                         |
| F023 | Agent 终端 UI  | `pty_write` 仅回显 `[input] ...` 未写 stdin；`pty_resize` 未真 resize；`pty_kill` 未 kill 进程 |
| F025 | Agent 启动     | PTY 启动失败仅 `warn`；`stop_agent` 只改 DB 不杀进程；DB 会话无 `work_dir`                     |
| F027 | 会话详情       | 读循环结束**未回写** `status`/`ended_at`/`duration_seconds`/`total_tokens`；摘要字段无写入     |
| F029 | 会话终端交互   | 同 F023；会话 ↔ PTY 映射未持久化 → 重启后僵尸会话                                              |
| F030 | 会话停止       | 不终止 OS 进程；`terminated` 不在 DB CHECK 白名单                                              |
| F034 | Skill 版本控制 | 版本号按**字段**累加（一次改 3 字段 → +3）；无历史快照                                         |
| F035 | Skill 执行日志 | 仅 PRD 路径写入；`execution_phase`/`status`/`skill_type` 硬编码；失败不记录                    |
| F040 | MCP 使用统计   | 依赖 F039，当前恒空                                                                            |
| F049 | 流式响应（⚪） | `generate_prd_streaming` 无调用点、未注册命令；实现为"取全文再解析"，非真流式                  |

### 3.4 后端可用但前端不可达（4 项，状态需下调）

| 编号 | 功能             |                 后端                 | 前端封装  |     页面调用      | 结论   |
| ---- | ---------------- | :----------------------------------: | :-------: | :---------------: | ------ |
| F010 | 需求字段自动完善 |       ✅ `enrich_requirement`        |    ✅     | ❌ **无任何调用** | 不可达 |
| F036 | Skill 使用统计   |      ✅ `get_skill_usage_stats`      | ❌ 无封装 |       ❌ 无       | 不可达 |
| F040 | MCP 使用统计     |       ✅ `get_mcp_usage_stats`       | ❌ 无封装 |       ❌ 无       | 不可达 |
| F045 | 缓存统计与清理   | ✅ `get_cache_stats` / `clear_cache` | ❌ 无封装 |       ❌ 无       | 不可达 |

证据：在 `src/` 全量检索 `get_skill_usage_stats|get_mcp_usage_stats|get_cache_stats|clear_cache`，**仅 `types.ts` 存在 `CacheStats` 接口定义**，无任何调用。
`SystemMonitor.tsx` 仅调用 `get_monitor_data` 与 `check_alerts`。

### 3.5 死封装（定义后无人调用）

`src/lib/tauri.ts` 中：`getStageSkillConfig`、`enrichRequirement` 定义后无调用点；`pty_spawn` 在前端完全未出现（前端只监听事件，从不主动创建 PTY）。

---

## 四、测试层面缺口

### 4.1 测试报告与实际不一致

`docs/testing/full-test-report.md`（2026-09-20，声称 211 用例 / 98.6% 通过）：

| 报告声称                                                                                     | 仓库实际                                                                             |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| L1 组件测试 19 个文件，含 `AgentCard`/`ChatMessage`/`StatusBadge`/`EmptyState`/`SearchInput` | 实际 **13 个文件**；上述 5 个文件**不存在**，对应组件在 `src/components/` 中也不存在 |
| 未提及 `Badges`/`Pagination`/`Toast`/`RequirementForm.minimal`                               | 这 4 个文件实际存在却未登记                                                          |
| IPC 命令 55 条，覆盖 55/55 = 100%                                                            | 实际命令 **77 条**，覆盖 **55/77 = 71.4%**                                           |
| 分组计数：需求 6 / Agent 5 / Git 3 / Token 6 / 执行日志 2                                    | 实际：需求 **7** / Agent **6** / Git **4** / Token **7** / 执行日志 **4**            |

`tests/l6-ipc/ipc-coverage.test.ts` 文件头注释写"共 42 个 IPC 命令"，却列了 55 个用例 → 注释与实现自相矛盾。

### 4.2 IPC 覆盖率 71.4%，未达 80% 目标

| 层级                  | 覆盖命令数 |           覆盖率           |
| --------------------- | :--------: | :------------------------: |
| 前端 L6               |  55 / 77   | **71.4%** ❌（目标 > 80%） |
| 后端 `integration.rs` |  70 / 77   |     90.9%（默认跳过）      |

**L6 未覆盖的 22 条命令**：

| 分组              | 未覆盖命令                                                                                                                            |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| 需求              | `enrich_requirement`                                                                                                                  |
| Agent             | `get_agent_sessions`                                                                                                                  |
| Token             | `get_token_usage`                                                                                                                     |
| 流水线（全 5 条） | `get_pipeline_stages`、`list_stage_skill_configs`、`get_stage_skill_config`、`upsert_stage_skill_config`、`remove_stage_skill_config` |
| 执行统计          | `get_skill_usage_stats`、`get_mcp_usage_stats`                                                                                        |
| PTY（全 6 条）    | `pty_spawn`、`pty_write`、`pty_resize`、`pty_kill`、`pty_get_sessions`、`pty_get_session`                                             |
| Git               | `git_create_commit`                                                                                                                   |
| 缓存              | `get_cache_stats`、`clear_cache`                                                                                                      |
| 审计              | `get_audit_logs`                                                                                                                      |
| 监控              | `get_monitor_data`、`check_alerts`                                                                                                    |

**两层级都未覆盖的 7 条**：`enrich_requirement`、流水线 5 条、`get_audit_logs`。
与功能缺口高度重合 —— 完全无测试的 7 条命令覆盖了 F010、F005、F054，即"没写测试"与"没实现/没接线"是同一批。

### 4.3 后端测试默认全部跳过

`src-tauri/tests/integration.rs`（70 个命令用例）与 `smoke.rs`（3 个）的每个用例首行均为：

```rust
if skip_unless_e2e() { return; }
```

未启用 E2E 模式时，后端唯一真实执行的测试是 `crypto.rs` 的 2 个单测。

### 4.4 L3「全栈测试」实为前端 Mock 测试

`tests/l3-fullstack/*` 全部 `vi.mock('@tauri-apps/api/core')`，仅断言封装传对了命令名与参数，**不连接 Rust 后端、不访问 SQLite**。

这是"数据链路未打通却全绿"的根本原因：导入导出假成功、审计恒空、`git_commits` 无写入都不会被这类测试发现。

### 4.5 L4 CDP 全部跳过

`tests/l4-cdp/webview2-coverage.test.ts` 5 个用例 **5/5 跳过**（`Cargo.toml` 未启用 `devtools` feature）。

### 4.6 单测覆盖盲区

| 对象  | 覆盖     | 缺口                                                |
| ----- | -------- | --------------------------------------------------- |
| 组件  | 12/12 ✅ | —                                                   |
| Store | 5/6 ⚠️   | `themeStore` 无测试                                 |
| 页面  | 0/18 ❌  | 无页面级单测，仅 5 个 E2E 冒烟覆盖 4 个页面         |
| 后端  | 默认 0   | DB 迁移、校验、缓存、Git、PTY、LLM 均无默认执行测试 |

### 4.7 测试工程配置问题

| 问题                       | 位置               | 影响                                                                                                      |
| -------------------------- | ------------------ | --------------------------------------------------------------------------------------------------------- |
| 未配置 `include`/`exclude` | `vitest.config.ts` | `npm test` 会一并收集 `tests/e2e/*.test.ts`（Playwright 用例），在 Vitest 中无法注册用例，有报错/空跑风险 |
| `isolate: false`           | `vitest.config.ts` | 跨文件 mock 泄漏；报告自述"L1 与 L3/L6 不能同时运行"，但 `npm test` 恰恰同时运行                          |
| 无分层脚本                 | `package.json`     | 仅有 `test`/`test:watch`，缺少 `test:l1`/`test:ipc`/`test:e2e`                                            |
| `@wdio/*` 依赖残留         | `package.json`     | 5 个 WebdriverIO 依赖已弃用（改用 Playwright），未清理                                                    |

### 4.8 测试计划完成度对照

计划 126 用例（L1 38 / L2 22 / L3 20 / L4 18 / L5 12 / L6 16）：

| 层级 | 计划 |                                 实际                                 | 达成        |
| ---- | :--: | :------------------------------------------------------------------: | ----------- |
| L1   |  38  |                  组件+Store 约 60+（报告数字不实）                   | 部分        |
| L2   |  22  | **14**（缺拖拽、评论、项目选择器、Skill 编辑、托盘、离线等 P0 用例） | ❌          |
| L3   |  20  |                     33（全部为 mock，非真全栈）                      | 名义达成    |
| L4   |  18  |                           **0**（5 跳过）                            | ❌          |
| L5   |  12  |                                  10                                  | 部分        |
| L6   |  16  |                       55（覆盖率 71.4% < 80%）                       | ❌ 目标未达 |

---

## 五、缺口矩阵（需求 ↔ 代码 ↔ 测试）

| 编号                | 功能                | 需求状态 | 代码缺口                                 | 测试缺口                        | 优先级 |
| ------------------- | ------------------- | :------: | ---------------------------------------- | ------------------------------- | :----: |
| F073/F074           | 数据导入导出        |    ❌    | 伪实现：导出空数组、导入不落库仍提示成功 | 无后端命令；L6 未覆盖           | **P0** |
| F011/F053/F054/F055 | 审计日志全链路      |    ❌    | `requirement_history` 无写入             | `get_audit_logs` 前后端均无测试 | **P0** |
| F023/F029/F030      | 终端输入/调整/终止  |    ⚠️    | `pty_write`/`pty_resize`/`pty_kill` 为桩 | PTY 6 条命令 L6 全未覆盖        | **P0** |
| F039/F040           | MCP 执行日志/统计   |  ❌/⚠️   | `mcp_execution_logs` 无写入              | `get_mcp_usage_stats` L6 未覆盖 |   P1   |
| F060                | 提交记录落库        |    ❌    | `git_commits` 无写入、参数未使用         | `git_create_commit` L6 未覆盖   |   P1   |
| F027                | 会话详情            |    ⚠️    | 未回写 status/duration/tokens            | 无用例                          |   P1   |
| F010                | 需求自动完善        |    ⚠️    | 前端无入口                               | 前后端均无测试                  |   P1   |
| F005                | 流水线环节配置      |    ✅    | 功能可用                                 | **5 条命令前后端均无测试**      |   P1   |
| F021/F022/F025      | Agent 状态/PTY/启动 |    ⚠️    | 句柄未持久化、失败仅 warn                | Agent 命令 L6 未覆盖            |   P1   |
| F036                | Skill 使用统计      |    ⚠️    | 前端无入口                               | 无测试                          |   P2   |
| F045                | 缓存统计与清理      |    ⚠️    | 前端无入口                               | 无测试                          |   P2   |
| F034/F035           | Skill 版本/执行日志 |    ⚠️    | 版本按字段累加；日志硬编码               | 无用例                          |   P2   |
| F049                | 流式响应            |    ⚪    | 未注册命令                               | 无测试                          |   P2   |
| —                   | `themeStore`        |    ✅    | —                                        | **无单测**                      |   P2   |
| —                   | 18 个页面           |    —     | —                                        | **无页面级单测**                |   P2   |
| —                   | L4 CDP              |    —     | 未启用 `devtools`                        | 5/5 跳过                        |   P1   |

---

## 六、需要同步修正的需求文档

`FEATURES.md` v2.0 有 4 处状态与实际不符，需修订：

| 功能                  | 现记                                        | 应改为                                    |
| --------------------- | ------------------------------------------- | ----------------------------------------- |
| F010 需求字段自动完善 | ✅ 已实现，前端位置 `RequirementDetail.tsx` | ⚠️ 部分实现（**原无前端入口**，本次已补） |
| F036 Skill 使用统计   | ✅ 已实现，前端位置 `Statistics.tsx`        | ⚠️ 部分实现（**原无前端入口**，本次已补） |
| F040 MCP 使用统计     | ⚠️，前端位置 `Statistics.tsx`               | ⚠️（**原无前端入口**，本次已补）          |
| F045 缓存统计与清理   | ✅ 已实现，前端位置 `SystemMonitor.tsx`     | ⚠️ 部分实现（**原无前端入口**，本次已补） |

另建议为 F073/F074 增加"**伪实现**"标记，现有"❌ 未实现"不足以描述"提示成功但不生效"。

---

## 七、修复执行结果（2026-09-21）

本报告第三章至第五章所列缺口已按 `docs/plan-fix-v3.md` 完成修复，以下为执行后的复测结果。

### 7.1 功能状态变化

| 状态        | 修复前 | 修复后 | 变化 |
| ----------- | :----: | :----: | ---- |
| ✅ 已实现   |   56   | **70** | +14  |
| ⚠️ 部分实现 |   10   | **4**  | −6   |
| ⚪ 未接线   |   1    |   1    | —    |
| ❌ 未实现   |   8    | **0**  | −8   |

### 7.2 已修复项与实现方式

| 编号                      | 原缺口                                               | 实现方式                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F073 / F074               | 伪实现（导出空数组、导入不落库仍提示成功）           | 新增 `src-tauri/src/db/backup.rs`（全表快照导出、事务导入、外键临时关闭/恢复、按表计数）与 `services/data_service.rs`；新增 `export_data` / `import_data` 命令；`import_data` 支持 `dry_run` 预检、`replace` 完全恢复模式，并在正式导入前自动备份 `database.db.bak.<时间戳>`；前端 `ImportExport.tsx` 重写为真实链路（当前数据量、影响面预览、二次确认） |
| F011 / F053 / F054 / F055 | 审计表无写入，审计页恒空                             | `db/requirements.rs` 新增 `insert_requirement_history` / `list_requirement_history`；`RequirementService` 在 create / update / status_change / delete 时写入差异记录；`list_audit_logs` 支持 `entity_type` 过滤，并在需求被删除后回退使用 `old_value` 作为标题                                                                                           |
| F022 / F023 / F025 / F029 | PTY 三方法为桩实现                                   | 重写 `src-tauri/src/pty.rs`：`PtySession` 真实持有 `writer`（`master.take_writer()`）、`master`（用于 `resize`）、`killer`（`child.clone_killer()`）与退出码 `oneshot`；`write_input` 真实写 stdin 并 flush，`resize` 真实调用 `master.resize`，`kill` 真实调用 `killer.kill()`；读线程结束后 `child.wait()` 取真实退出码并更新状态                      |
| F027 / F030               | 会话结束未回写                                       | 新增 `db/agents.rs::finish_agent_session`（计算 `duration_seconds`，写 `ended_at`/`exit_code`，且仅当 `status='running'` 时更新以避免覆盖用户终止的终态）；`start_agent` 启动后台任务等待 PTY 退出后回写；PTY 启动失败时将会话置为 `failed` 并返回错误                                                                                                   |
| F039 / F040               | MCP 日志表无写入                                     | `db/skills.rs` 新增 `insert_mcp_execution_log`；`start_agent` 在会话启动时按任务所属项目的 MCP 绑定写入 `connected` / `session_start` 记录                                                                                                                                                                                                               |
| F060                      | 提交记录不落库                                       | 新增 `db/git.rs`（`insert_git_commit` / `list_git_commits`）；`git_create_commit` 启用 `task_id`/`session_id` 参数并在提交成功后落库（落库失败不影响提交结果）；新增 `list_git_commits` 查询命令                                                                                                                                                         |
| F010 / F036 / F040 / F045 | 后端可用但前端不可达                                 | `RequirementDetail.tsx` 新增「智能完善」按钮并回填建议字段，同时补齐扩展字段编辑区；`Statistics.tsx` 新增 Skill / MCP 使用统计区块；`SystemMonitor.tsx` 新增缓存卡片与「清空缓存」按钮；`lib/tauri.ts` 新增 4 组封装（`getCacheStats`/`clearCache`/`getSkillUsageStats`/`getMcpUsageStats`）                                                             |
| —                         | `update_requirement` 无法保存扩展字段                | 命令与服务层新增 `module`/`source`/`sourceDetail`/`productOwner`/`developer`/`tester`/`businessPriority`/`estimatedEffort`/`tags`/`notes` 参数（`RequirementExtFields`），并纳入审计差异                                                                                                                                                                 |
| —                         | Layout / SystemMonitor 绕过 `tauri.ts` 直接 `invoke` | 新增 `getMonitorData` / `checkAlerts` 封装并替换调用                                                                                                                                                                                                                                                                                                     |

### 7.3 测试与工程化结果

| 项                       | 修复前                                                              | 修复后                                                                                    |
| ------------------------ | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| IPC 测试覆盖（L6）       | 55/77 = 71.4%                                                       | **80/80 = 100%**（新增 25 条用例，编号 L6-056 ~ L6-080）                                  |
| 测试分层脚本             | 仅 `test` / `test:watch`                                            | 新增 `test:l1` / `test:l3` / `test:l5` / `test:ipc` / `test:e2e` / `test:rust` / `verify` |
| vitest 收集范围          | 未配置，会误收集 Playwright 用例                                    | 显式 `include` / `exclude`（排除 `tests/e2e`、`tests/l4-cdp`）                            |
| Store 单测               | 5/6                                                                 | **6/6**（新增 `themeStore.test.ts`，覆盖 dark/light/system 与持久化）                     |
| TypeScript 构建（`tsc`） | **失败**（7 个测试文件共 12 个类型错误，导致 `npm run build` 中断） | **通过（0 错误）**                                                                        |
| 弃用依赖                 | 5 个 `@wdio/*` 残留                                                 | 已移除，`package-lock.json` 已同步                                                        |
| Tauri devtools           | 未启用（L4 CDP 全部跳过）                                           | `Cargo.toml` 已启用 `devtools` feature                                                    |
| Rust 编译                | —                                                                   | `cargo check` 通过                                                                        |

**实测结果（在纯 ASCII 路径下运行）**：

| 套件                 | 结果                               |
| -------------------- | ---------------------------------- |
| L1（`src/`）         | 21 个文件 / **179** 个用例全部通过 |
| L6（`tests/l6-ipc`） | 1 个文件 / **80** 个用例全部通过   |
| L3 + L5              | 7 个文件 / **43** 个用例全部通过   |
| `tsc --noEmit`       | 通过（0 错误）                     |
| `cargo check`        | 通过（仅剩既有告警）               |

### 7.4 新发现的阻塞性问题：非 ASCII 项目路径导致 vitest 无法注册用例

| 项       | 内容                                                                                                                                                                                             |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 现象     | 在本项目路径（含中文 `AI自动化开发流程`、`单机版`）下执行 `npx vitest run`，**所有**测试文件均报 `Error: No test suite found in file ...`，用例数为 0；`describe` 可正常调用且不报错，但不会注册 |
| 影响     | 本机无法直接运行任何 Vitest 用例，`npm test` / `npm run verify` 均不可用；这解释了为何报告中的 211 个用例无法在该路径下复现                                                                      |
| 排查     | 已排除：配置文件（还原原始配置仍失败）、`setupFiles`（jest-dom）、`isolate`、`pool`（threads/forks）、vitest 多副本、vite 依赖缓存损坏                                                           |
| 定位     | 将项目通过 `subst` 映射为 `X:\` 后，单个用例文件立即通过；再将项目复制到 `E:\ace-verify`（纯 ASCII 路径）后，全部 302 个用例通过                                                                 |
| 结论     | vitest 1.6.0 在 Windows + 非 ASCII 路径下存在用例注册缺陷（runner 的模块状态与文件 URL 不匹配）                                                                                                  |
| 规避方案 | ① 将项目移动到纯 ASCII 路径；② 或使用 ASCII 目录副本；③ 或升级 vitest 至更高版本（未验证）                                                                                                       |

### 7.5 遗留项（未在本次修复范围内）

| 项                    | 说明                                                                                           |
| --------------------- | ---------------------------------------------------------------------------------------------- |
| API Key 明文存储      | `settings.claude_api_key` 仍为明文；`encrypt_api_key` / `decrypt_api_key` 未与设置读写链路集成 |
| F030 会话停止语义     | `stop_agent` 建议直接联动 `pty_kill` 以终止 OS 进程                                            |
| F021 Agent 状态有效性 | 缺少二进制有效性与进程存活对账                                                                 |
| F034 Skill 版本号     | 按字段累加，建议改为按操作累加并保留版本快照                                                   |
| F035 Skill 执行日志   | 建议覆盖失败场景与 `task_execution` / `session_start` 阶段                                     |
| F049 真流式           | `generate_prd_streaming` 需改为 `bytes_stream` 真流式并注册为命令                              |
| 未处理错误告警        | `isolate: false` 下跨文件 mock 泄漏会产生 1 个 unhandled error 告警（不影响用例通过）          |
| 页面级单测            | 18 个页面仍无单测，仅 L2 冒烟覆盖 4 个页面                                                     |

---

## 八、第二轮修复结果（全量问题覆盖）

按 `docs/plan-fix-v3.md` v2.0（45 项全量台账）完成第二轮实现，覆盖数据风险、安全、工程化与测试真实性。

### 8.1 本轮关闭的问题

| 编号               | 问题                                                          |                                        结果                                        |
| ------------------ | ------------------------------------------------------------- | :--------------------------------------------------------------------------------: |
| B-01               | Token 明细 `SELECT *` + 固定下标导致旧库列错位                |                             ✅ 改显式列清单 + 回归测试                             |
| C-01 / D-02        | 非 ASCII 路径下 vitest 用例全部不注册                         | ✅ `scripts/run-tests-ascii.ps1` + `resolve.preserveSymlinks` + `npm run test:win` |
| C-08               | `isolate: false` 造成跨文件 mock 泄漏（全套件共跑失败 21 例） |                        ✅ 改为 `isolate: true`，222 例全绿                         |
| A-09               | `stop_agent` 不终止 OS 进程                                   |                  ✅ PTY 会话与数据库会话双向绑定，停止时真实 kill                  |
| A-12               | API Key 明文存储                                              |                 ✅ 本地密钥库 + 敏感设置自动加解密（兼容历史明文）                 |
| A-13               | `git add -u` 导致新文件无法提交                               |                         ✅ 默认 `git add -A`（可配置回退）                         |
| A-14 / A-15        | 备份不含 WAL、无轮转                                          |              ✅ 备份前 `wal_checkpoint(TRUNCATE)`；启动备份保留 5 份               |
| A-16               | 预算用量不累计                                                |                       ✅ 自动累计 + 跨月重置 + 前端告警字段                        |
| A-17               | 僵尸会话                                                      |                                    ✅ 启动对账                                     |
| A-18               | Skill 版本按字段累加                                          |                                   ✅ 按操作递增                                    |
| A-21 / A-22 / A-23 | 需求完善不记账；成本恒 0；模型硬编码                          |                     ✅ 记账 + 价格估算 + `settings.llm_model`                      |
| A-24               | 软删除过滤                                                    |                              ✅ 确认已过滤，无需修改                               |
| A-25 / A-26 / A-27 | 绕过封装 / 死封装 / 导出不全                                  |                                    ✅ 全部清理                                     |
| B-03               | 缺失索引                                                      |                           ✅ 6 个补充索引（迁移后创建）                            |
| B-04               | 缺唯一约束                                                    |                             ✅ 安全去重 + 4 个唯一索引                             |
| C-03               | 后端测试默认跳过                                              |                             ✅ 默认执行单测 **2 → 29**                             |
| C-04               | 无真实链路测试                                                |                           ✅ 新增 14 个真实 SQLite 测试                            |
| D-04               | Rust 告警                                                     |                    🔶 22 → 18，并顺带把 HTTP 状态码加入错误信息                    |

### 8.2 新增测试暴露的真实缺陷（已修复）

| 缺陷                                               | 严重度 | 说明                                                                                                                                    |
| -------------------------------------------------- | :----: | --------------------------------------------------------------------------------------------------------------------------------------- |
| `requirement_history` 外键导致**删除需求必然失败** |   高   | 审计写入与需求删除互斥；已移除外键并新增表重建迁移，使审计记录在需求删除后仍保留                                                        |
| 补充索引建在 `ALTER TABLE` 之前                    |   中   | 新建库时 `tasks.project_id` 尚不存在，导致初始化失败；已改为在 `migrate_tables()` 之后创建                                              |
| 唯一约束迁移可能破坏存量数据                       |   中   | 改为"先安全去重（迁移会话引用 / 清理重复绑定 / 置空重复 Key）再建索引"，并让 `add_project_skill` 幂等、`add_project_mcp` 自动清理旧默认 |

### 8.3 最新验证结果

| 项                        | 结果                                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------------ |
| `cargo test`              | ✅ 42（单元，含模块 16 适配层 9 + 数据层 2）+ 71（E2E 门控跳过）+ 3（冒烟）全部通过；Rust 告警 0 |
| `cargo check`             | ✅ 0 error                                                                                       |
| `tsc --noEmit`            | ✅ 0 error                                                                                       |
| 前端（L1 + L3 + L5 + L6） | ✅ **273 用例全部通过**                                                                          |
| L6 IPC 覆盖率             | ✅ 88/88 = 100%                                                                                  |

### 8.4 仍未关闭

> 第四轮（2026-09-21，问题清单 23 项）已完成全部关闭，明细见 `docs/plan-fix-v3.md` 7.3。
> 第五轮（模块 16 P0，2026-09-21）已交付"结构化会话可用闭环"：`session_messages` 表、`agent_adapter.rs`/`session_driver.rs`、6 条命令（82→88）、6 类事件、`SessionChat`/`SessionInput`/`MessageBubble` 双视图；F048 消息级记账已入 `token_usage`。以下为**当前残留项**：

| 编号         | 问题                                           | 说明                                                                                                                                |
| ------------ | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| A-19（残留） | Skill 执行日志的 `task_execution` 阶段无写入点 | `prd_generation`（成功/失败）与 `session_start` 已真实写入；`task_execution` 需等平台实现"任务执行阶段调用 Skill"的链路后才能落日志 |
| M16-1        | F078 完整 Markdown 渲染                        | 现为正文 + 围栏代码块 + 行内 code；待引入 Markdown 渲染库                                                                           |
| M16-2        | F083 逐次授权确认                              | 该 CLI 发行版无 host 确认通道；被拒卡片 + `--allowedTools` 沉淀已实现，逐次确认依赖终端视图                                         |
| M16-3        | 多 Agent 适配（codex）                         | 仅 Claude 已实现；codex 参数/事件格式待实测（设计 §5.3）                                                                            |
| M16-4        | 结构化会话真机端到端验证                       | 协议层/数据层测试已覆盖；未做真实 API 全链路跑通（涉及成本）                                                                        |
| M16-5        | L4 CDP 用例未实跑                              | 用例已按 `TAURI_DEVTOOLS_SERVER_PORT` 自动启用，但未实跑                                                                            |

**第四轮已关闭项（摘要）**：F060 前端闭环（含回归用例）、A-10/A-11（`agentsStore` + `verify_agents`）、A-20（真流式 + 命令注册 + 前端订阅）、B-02（迁移机制统一、删除 12 个死 `.sql`）、B-05（种子去重）、B-06（`llm_client` 死字段移除）、缓存 `evictions` 与条目版本化、删除形同虚设的列表缓存、C-05（L4 改为按端口自动启用的真实用例 + `npm run test:l4`）、C-06（19 个页面冒烟用例）、C-07（L2 E2E 24 例）、D-01/D-03（测试文档与命令指引刷新）、D-04（Rust 告警 15 → 0）。

---

_报告生成：2026-09-21_
_第二轮结果更新：2026-09-21_
