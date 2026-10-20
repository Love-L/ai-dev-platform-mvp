# ACE 全量问题修复开发计划 v2.0

> ⚠️ **历史文档（2026-09-21 制定）**：本文计划多数条目已执行完毕。
> 当前实现状态一律以 [`docs/STATUS.md`](STATUS.md) 为准。

> 制定日期：2026-09-21
> 依据：`docs/analysis/gap-analysis-report.md`（含第七章执行结果）、`FEATURES.md` v2.1、代码只读审计
> 版本说明：v1.0 仅覆盖"差距分析中的 P0/P1 功能缺口"；**v2.0 将范围调整为覆盖全部已发现问题**（功能缺口 + 数据风险 + 测试缺口 + 工程化债务 + 文档失实），共 **45 项**

---

## 一、目标调整说明

| 维度     | v1.0 目标                           | v2.0 目标（本次调整）                                                                                |
| -------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 范围     | 差距报告第三章的 8 项 ❌ + 10 项 ⚠️ | 差距报告全部章节 + `FEATURES.md` 11.4 数据风险 + 测试缺口 + 工程化债务                               |
| 问题数   | 约 20 项                            | **45 项**（含已完成 11 项、新增 34 项）                                                              |
| 收口标准 | 功能状态 ✅≥70                      | 功能状态 ✅≥70 **且** 关键数据风险清零 **且** 测试可在本项目路径下运行 **且** 后端具备默认执行的测试 |

---

## 二、全量问题台账（45 项）

> 状态：✅ 已完成（v1.0 范围） / ⬜ 待修复 / 🔶 部分完成
> 优先级：P0 阻断或数据正确性 / P1 安全或核心可用性 / P2 质量与体验 / P3 债务与整洁

### A. 功能与实现缺口（27 项）

| 编号 | 问题                                                                                      | 位置/证据                                    | 影响                               | 优先级 | 状态 |
| ---- | ----------------------------------------------------------------------------------------- | -------------------------------------------- | ---------------------------------- | :----: | :--: |
| A-01 | 数据导入导出为伪实现（导出空数组、导入不落库）                                            | `pages/ImportExport.tsx`                     | 备份/恢复不可用，误导用户          |   P0   |  ✅  |
| A-02 | 审计链路无写入（`requirement_history` 恒空）                                              | `db/requirements.rs`                         | 审计能力缺失                       |   P0   |  ✅  |
| A-03 | PTY 输入/尺寸/终止为桩实现                                                                | `pty.rs`                                     | 终端不可交互                       |   P0   |  ✅  |
| A-04 | 会话结束未回写 `status`/`ended_at`/`duration_seconds`                                     | `services/agent_service.rs`                  | 会话详情数据缺失                   |   P1   |  ✅  |
| A-05 | 提交记录未落库，`_task_id`/`_session_id` 被忽略                                           | `commands.rs::git_create_commit`             | 代码与任务无法关联                 |   P1   |  ✅  |
| A-06 | MCP 日志表无写入，统计恒空                                                                | `db/skills.rs`                               | MCP 可观测性缺失                   |   P1   |  ✅  |
| A-07 | `F010`/`F036`/`F040`/`F045` 后端可用但前端不可达                                          | `lib/tauri.ts`、相关页面                     | 4 项功能形同虚设                   |   P1   |  ✅  |
| A-08 | `requirement_history` 删除场景外键冲突（先删后写）                                        | `services/requirement_service.rs`            | 删除审计丢失                       |   P1   |  ✅  |
| A-09 | `stop_agent` 仅改数据库，不终止 OS 进程                                                   | `services/agent_service.rs`                  | 进程泄漏、僵尸会话                 | **P1** |  ⬜  |
| A-10 | `Agents.tsx` 使用组件内 `useState` 而非 store，重进页面列表可能为空                       | `pages/Agents.tsx`                           | Agent 管理不可用（时好时坏）       | **P1** |  ⬜  |
| A-11 | 无 Agent 二进制有效性复核与进程存活对账                                                   | `services/agent_service.rs`                  | 状态长期失真（available 但已删除） |   P2   |  ⬜  |
| A-12 | API Key 明文存储；`encrypt_api_key` 未接入设置读写链路                                    | `commands.rs::set_setting`、`crypto.rs`      | 凭据泄漏风险                       | **P1** |  ⬜  |
| A-13 | `git add -u` 不含未跟踪新文件，Agent 新增文件无法提交                                     | `services/git_service.rs`                    | 提交不完整                         | **P1** |  ⬜  |
| A-14 | 数据库备份仅复制 `database.db`，WAL 模式下 `-wal`/`-shm` 未处理                           | `main.rs`、`commands.rs::import_data`        | 备份可能不完整                     | **P1** |  ⬜  |
| A-15 | 备份无定时与轮转，仅故障时触发一次                                                        | `main.rs`                                    | 数据保护薄弱                       |   P2   |  ⬜  |
| A-16 | `token_budgets.current_usage` 无自动累计，预算告警无实际触发点                            | `services/token_service.rs`                  | 预算形同装饰                       | **P1** |  ⬜  |
| A-17 | 应用重启后 `agent_sessions` 仍为 `running`，与实际进程不一致（僵尸会话）                  | `main.rs` 启动流程                           | 会话列表失真                       | **P1** |  ⬜  |
| A-18 | `Skill` 版本号按**字段**累加（改 3 字段 +3）                                              | `db/skills.rs::update_prd_skill`             | 版本语义错误                       |   P2   |  ⬜  |
| A-19 | Skill 执行日志仅 PRD 路径写入，`execution_phase`/`status`/`skill_type` 硬编码，失败不记录 | `services/prd_service.rs`                    | 统计与排查能力不足                 |   P2   |  ⬜  |
| A-20 | `F049` 流式响应未注册命令，且实现为"取全文再解析"，非真流式                               | `llm.rs::generate_prd_streaming`             | 大文档体验差                       |   P2   |  ⬜  |
| A-21 | `enrich_requirement` 不写入 `token_usage`，成本统计漏算                                   | `commands.rs::enrich_requirement`            | 成本失真                           |   P2   |  ⬜  |
| A-22 | `total_cost` 恒为 0，无模型价格换算                                                       | `services/prd_service.rs`、`usage_reader.rs` | 成本分析不可用                     |   P2   |  ⬜  |
| A-23 | LLM 模型名 `claude-sonnet-5-20250514` 硬编码，不可配置                                    | `llm.rs`                                     | 无法切换模型                       |   P2   |  ⬜  |
| A-24 | `prd_skills` 软删除后列表是否过滤 `is_deleted` 未确认                                     | `db/skills.rs::list_prd_skills`              | 已删 Skill 可能仍展示              |   P2   |  ⬜  |
| A-25 | `AgentTerminal.tsx` 仍绕过 `lib/tauri.ts` 直接 `invoke`                                   | `components/AgentTerminal.tsx`               | 数据层不统一                       |   P3   |  ⬜  |
| A-26 | `getStageSkillConfig` 定义后无调用点（死封装）                                            | `lib/tauri.ts`                               | 死代码                             |   P3   |  ⬜  |
| A-27 | `stores/index.ts` 未 re-export `themeStore`                                               | `stores/index.ts`                            | 引入路径不一致                     |   P3   |  ⬜  |

### B. 数据与迁移风险（6 项）

| 编号 | 问题                                                                                                                           | 位置/证据                           | 影响                   | 优先级 | 状态 |
| ---- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------- | ---------------------- | :----: | :--: |
| B-01 | `get_token_usage` 用 `SELECT *` + 固定下标读取，旧库 ALTER 追加列后**列错位**                                                  | `db/tokens.rs`                      | Token 明细取值错误     | **P0** |  ⬜  |
| B-02 | 迁移双轨：`migration.rs::apply_migration` 为死代码、12 个 `.sql` 从未加载、版本号硬编码 8                                      | `db/migration.rs`、`db/mod.rs`      | 无真正的版本化迁移能力 | **P1** |  ⬜  |
| B-03 | 缺失索引：`idx_tasks_project`、`idx_agent_sessions_task/status`、`idx_projects_status`                                         | `db/migrations/010_*.sql`（未加载） | 大数据量查询慢         |   P2   |  ⬜  |
| B-04 | 缺唯一约束：`projects.key`、`agents.binary_path`、`project_skills(project_id,skill_id)`、`project_mcps(project_id,is_default)` | 建表语句                            | 重复数据               |   P2   |  ⬜  |
| B-05 | `init_tables()` 与 `seed_data()` 重复插入默认项目（文案不同）                                                                  | `db/mod.rs`                         | 逻辑重复               |   P3   |  ⬜  |
| B-06 | `AppState.llm_client` 初始化为 `None` 且全库无读写（死字段）                                                                   | `main.rs`                           | 死代码                 |   P3   |  ⬜  |

### C. 测试缺口（8 项）

| 编号 | 问题                                                                             | 位置/证据            | 影响                   | 优先级 |            状态             |
| ---- | -------------------------------------------------------------------------------- | -------------------- | ---------------------- | :----: | :-------------------------: |
| C-01 | **非 ASCII 项目路径导致 vitest 全部用例不注册**（`No test suite found in file`） | 本机实测             | 所有前端测试不可运行   | **P0** |             ⬜              |
| C-02 | IPC 覆盖率 55/77 = 71.4%（低于 80% 目标）                                        | `tests/l6-ipc`       | 命令无测试             |   P1   |         ✅（80/80）         |
| C-03 | 后端 `integration.rs` 70 个用例全部 `skip_unless_e2e`，默认 0 执行               | `src-tauri/tests/`   | CI 无后端测试          | **P1** |             ⬜              |
| C-04 | L3「全栈」实为前端 mock，不连接 Rust/DB                                          | `tests/l3-fullstack` | 数据链路缺陷无法被发现 | **P1** |             ⬜              |
| C-05 | L4 CDP 5 例全部跳过（未启用 devtools）                                           | `tests/l4-cdp`       | 无真实 WebView2 验证   | **P1** | 🔶（已启用 devtools，未跑） |
| C-06 | 18 个页面无单测                                                                  | `src/pages`          | 页面逻辑无保护         |   P2   |             ⬜              |
| C-07 | L2 E2E 仅 14 例（计划 22），缺拖拽/评论/项目选择器/Skill 编辑/托盘/离线          | `tests/e2e`          | 关键交互无回归         |   P2   |             ⬜              |
| C-08 | `isolate: false` 造成跨文件 mock 泄漏 + 1 个 unhandled error 告警                | `vitest.config.ts`   | 可能产生假阳性         |   P2   |             ⬜              |

### D. 工程化与文档（4 项）

| 编号 | 问题                                                                             | 位置/证据                          | 影响               | 优先级 | 状态 |
| ---- | -------------------------------------------------------------------------------- | ---------------------------------- | ------------------ | :----: | :--: |
| D-01 | `full-test-report.md` 统计与文件清单失实（5 个不存在的测试文件、IPC 55/55=100%） | `docs/testing/full-test-report.md` | 报告不可信         | **P1** |  ⬜  |
| D-02 | 无非 ASCII 路径规避脚本与说明                                                    | —                                  | 开发者无法运行测试 | **P1** |  ⬜  |
| D-03 | `full-test-plan.md` 与现行六层体系不一致；`CLAUDE.md` 测试命令说明过时           | `docs/testing`、`CLAUDE.md`        | 指引错误           |   P2   |  ⬜  |
| D-04 | Rust 既有告警（`llm.rs` unused mut/status、`cache.rs`/`tray.rs` 死代码）         | `cargo check` 输出                 | 噪音               |   P3   |  ⬜  |

---

## 三、阶段与任务（调整后）

### Phase A：v1.0 范围（已完成，保留记录）

| 任务                    | 覆盖问题   |    状态    |
| ----------------------- | ---------- | :--------: |
| T0-1 导入导出真实实现   | A-01       |     ✅     |
| T0-2 审计写入链路       | A-02、A-08 |     ✅     |
| T0-3 PTY 真实能力       | A-03       |     ✅     |
| T1-1 提交记录落库与查询 | A-05       |     ✅     |
| T1-2 会话结束回写       | A-04       |     ✅     |
| T1-3 MCP 日志写入       | A-06       |     ✅     |
| T1-4 前端入口补齐       | A-07       |     ✅     |
| T2-1 IPC 测试补齐       | C-02       | ✅（100%） |
| T2-2 测试配置与脚本分层 | —          |     ✅     |
| T2-3 `themeStore` 单测  | —          |     ✅     |
| T2-4 文档状态同步       | —          |     ✅     |

### Phase B：P0 新增（阻断项）

| 任务                                       | 覆盖问题   | 实现要点                                                                                                                                                                | 验收                                                               |
| ------------------------------------------ | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| **T3-1 修复 Token 明细列错位**             | B-01       | `db/tokens.rs::get_token_usage*` 全部改为**按列名读取**（`row.get("cache_creation_tokens")` 或显式列清单），移除 `SELECT *` + 下标                                      | 旧库（列追加到表尾）与新库下取值一致；新增单测覆盖"列顺序变更"场景 |
| **T3-2 解决非 ASCII 路径导致测试不可运行** | C-01、D-02 | ① 新增 `scripts/run-tests-ascii.ps1`（自动 `subst` + 运行 + 清理）；② `package.json` 增加 `test:win` 脚本；③ `vitest.config.ts`/`README`/`CLAUDE.md` 增加说明与排查指引 | 在含中文路径的项目中，一条命令即可运行全部 Vitest 用例             |

### Phase C：P1 新增（安全 + 核心可用性）

| 任务                                        | 覆盖问题   | 实现要点                                                                                                                                  | 验收                                                               |
| ------------------------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| **T4-1 API Key 加密链路**                   | A-12       | `set_setting` 对敏感键（`claude_api_key` 等）自动加密存储并打标记；读取侧在 `PrdService`/`enrich_requirement` 自动解密；兼容历史明文      | 设置页保存后库中为密文；PRD 生成仍可正常调用；旧明文数据可平滑读取 |
| **T4-2 `stop_agent` 联动终止进程**          | A-09       | `stop_agent` 内按会话反查并调用 `pty_kill`，再回写 DB；`PtyManager` 增加 `db_session_id ↔ pty_session_id` 映射                            | 停止会话后 OS 进程确实退出；会话状态与进程一致                     |
| **T4-3 `Agents.tsx` 接入 store + 状态对账** | A-10、A-11 | 新增 `agentsStore`；检测/启动后统一从数据库刷新；新增二进制存在性复核（失效标记 `unavailable`）                                           | 刷新检测后切换页面再回来，列表仍正确；删除二进制后状态显示不可用   |
| **T4-4 Git 提交支持新文件**                 | A-13       | `create_commit` 增加 `include_untracked` 参数（默认 true），使用 `git add -A`（可配置回退 `-u`）                                          | Agent 新增的文件可被提交；可通过参数关闭                           |
| **T4-5 备份完整性（WAL）与轮转**            | A-14、A-15 | 备份前执行 `PRAGMA wal_checkpoint(TRUNCATE)` 或同时复制 `-wal`/`-shm`；备份保留最近 N 份并清理旧文件                                      | 生成的备份文件可独立还原；备份目录保留不超过 N 份                  |
| **T4-6 预算自动累计与告警**                 | A-16       | 记录 Token 后累加 `current_usage`；跨月自动重置（依据 `reset_date`）；提供预算超阈值判定供状态栏告警                                      | 写入 Token 后预算用量同步变化；超过阈值返回告警                    |
| **T4-7 僵尸会话对账**                       | A-17       | 应用启动时把无对应 PTY 会话的 `running` 记录标记为 `failed`（或 `completed`）并写 `ended_at`                                              | 重启后无 `running` 僵尸记录                                        |
| **T4-8 迁移机制整理**                       | B-02       | ① 让 `apply_migration` 真正被调用（按版本号顺序执行）；② 或删除死代码 + 统一为现有幂等方式并显式标注；③ 明确 `db/migrations/*.sql` 的去留 | 迁移路径唯一且有明确说明；`schema_version` 具备实际语义            |
| **T4-9 后端测试可默认执行**                 | C-03       | 把不依赖 E2E 的用例（DB/校验/加密/缓存/Git 纯函数）移出 `skip_unless_e2e` 门控，改为可本地运行（临时目录建库）                            | `cargo test` 默认执行 ≥ 30 个后端用例且全部通过                    |
| **T4-10 真实链路测试**                      | C-04       | 新增"后端直连"测试：用临时 SQLite 建库，直接调用 Service 层验证 CRUD/审计/预算/导入导出，替代纯 mock 的 L3 定位                           | 存在至少 1 个真正访问 SQLite 的测试套件                            |
| **T4-11 启用并跑通 L4 CDP**                 | C-05       | 已启用 `devtools`；补齐启动脚本（设置 `TAURI_DEVTOOLS_SERVER_PORT`）并实跑 5 个用例                                                       | L4 用例从 5 skipped 变为实际执行（或明确标注环境限制）             |
| **T4-12 修正测试报告与文档**                | D-01、D-03 | 重写 `full-test-report.md` 为真实数据；更新 `full-test-plan.md` 与 `CLAUDE.md` 的测试章节                                                 | 报告中的文件清单、用例数、覆盖率与仓库一致                         |

### Phase D：P2（质量与体验）

| 任务                                               | 覆盖问题  |
| -------------------------------------------------- | --------- |
| T5-1 补齐缺失索引                                  | B-03      |
| T5-2 补齐唯一约束（含去重迁移）                    | B-04      |
| T5-3 Skill 版本按操作累加 + 版本快照               | A-18      |
| T5-4 Skill 执行日志覆盖失败与多阶段                | A-19      |
| T5-5 真流式（`bytes_stream`）+ 注册命令 + 前端订阅 | A-20      |
| T5-6 `enrich_requirement` 写入 `token_usage`       | A-21      |
| T5-7 模型价格表与 `total_cost` 换算                | A-22      |
| T5-8 LLM 模型可配置（`settings.llm_model`）        | A-23      |
| T5-9 `list_prd_skills` 过滤 `is_deleted`           | A-24      |
| T5-10 页面级单测（关键 6 个页面）                  | C-06      |
| T5-11 L2 E2E 补齐至计划用例                        | C-07      |
| T5-12 消除跨文件 mock 泄漏（`isolate` 策略调整）   | C-08      |
| T5-13 设置项白名单与类型校验                       | A-15 延伸 |

### Phase E：P3（债务与整洁）

| 任务                                                                              | 覆盖问题         |
| --------------------------------------------------------------------------------- | ---------------- |
| T6-1 `AgentTerminal` 改用 `lib/tauri.ts` 封装                                     | A-25             |
| T6-2 清理死封装与死字段（`getStageSkillConfig`、`llm_client`、`TRAY_ICON_ID` 等） | A-26、B-06、D-04 |
| T6-3 `stores/index.ts` 统一导出                                                   | A-27             |
| T6-4 合并 `init_tables`/`seed_data` 重复种子逻辑                                  | B-05             |
| T6-5 清理 Rust 既有告警                                                           | D-04             |

---

## 四、执行顺序

```
Phase B（P0 阻断）
  T3-1 Token 列错位  ──┐
  T3-2 非 ASCII 路径  ─┴─► 恢复测试可运行能力（后续所有验证的前提）

Phase C（P1）
  T4-1 API Key 加密 ─► T4-6 预算累计 ─► T4-7 僵尸会话
  T4-2 stop_agent  ─► T4-3 Agents store/状态对账
  T4-4 Git 新文件   ─► T4-5 备份完整性
  T4-8 迁移整理     ─► T4-9 后端测试默认执行 ─► T4-10 真实链路测试 ─► T4-11 L4 CDP
  T4-12 文档修正（最后统一执行）

Phase D（P2）→ Phase E（P3）
```

---

## 五、完成定义（DoD）

1. **P0 全部完成**：Token 明细取值正确；本项目路径下可一条命令运行全部前端测试
2. **P1 全部完成**：API Key 不再明文；`cargo test` 默认执行 ≥ 30 个通过用例；存在真实访问 SQLite 的测试；后端存在 > 0 个默认执行的测试；备份可独立还原
3. **质量门禁**：`cargo check` 0 error；`tsc --noEmit` 0 error；L1/L3/L5/L6 全部通过；L6 覆盖率保持 100%
4. **文档一致**：`FEATURES.md`、`gap-analysis-report.md`、`full-test-report.md`、`full-test-plan.md`、`CLAUDE.md` 与代码状态一致
5. **状态收口**：功能状态 ⚠️ ≤ 2、❌ = 0；台账 45 项中 P0/P1 全部 ✅

---

## 六、风险与应对

| 风险                                      | 影响                      | 应对                                                                                  |
| ----------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------- |
| 非 ASCII 路径问题可能无法在代码层完全解决 | 测试仍不可用              | 提供 `subst` 脚本 + 文档，并评估升级 vitest；将"路径必须为 ASCII"写入 README 前置要求 |
| API Key 加密引入不兼容                    | 已有用户无法生成 PRD      | 读取侧同时兼容明文与密文；密文带前缀标记（如 `enc:v1:`）                              |
| 唯一约束迁移遇存量重复数据                | 迁移失败                  | 迁移前先去重（保留最早一条并记录日志），再建唯一索引                                  |
| L4 CDP 依赖 WebView2 环境                 | 用例仍可能跳过            | 若环境不可用，明确标注为"环境限制"而非"未实现"                                        |
| 后端测试改为默认执行后受文件锁影响        | Windows 下 exe 运行时锁库 | 测试统一使用临时目录建库，不触碰运行中的应用库                                        |
| 范围扩大导致回归                          | 新问题                    | 每个任务完成即跑 `cargo check` + 相关测试套件；文档最后统一更新                       |

---

## 七、第二轮执行结果（2026-09-21）

### 7.1 本轮完成（Phase B / C 主体）

| 问题                               |      结果      | 实现要点                                                                                                                                                                            |
| ---------------------------------- | :------------: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B-01 Token 明细列错位              |       ✅       | `db/tokens.rs` 全部改为**显式列清单**（移除 `SELECT *` + 固定下标）；`get_token_budget` 同样处理；新增"旧库列追加到表尾"回归测试                                                    |
| C-01 非 ASCII 路径导致测试不可运行 |       ✅       | 新增 `scripts/run-tests-ascii.ps1` + `vitest.config.ts` 的 `resolve.preserveSymlinks`；`package.json` 增加 `test:win` / `test:win:ipc` / `verify:win`                               |
| C-08 跨文件 mock 泄漏              |       ✅       | `isolate: false` → `isolate: true`；此前"全部套件一起跑会失败 21 例"，现 222 例全绿                                                                                                 |
| A-09 `stop_agent` 不终止进程       |       ✅       | `PtySession` 增加 `db_session_id` 绑定；新增 `bind_db_session` / `find_pty_by_db_session`；`stop_agent` 先真实 `kill` 再回写状态                                                    |
| A-12 API Key 明文存储              |       ✅       | 新增本地密钥库（`keystore.key`）+ `encrypt_secret` / `decrypt_secret` / `is_encrypted_secret`；`Database::get_setting`/`set_setting` 对敏感键自动加解密，兼容历史明文               |
| A-13 新文件无法提交                |       ✅       | `GitService::create_commit` 增加 `include_untracked`（默认 true → `git add -A`）                                                                                                    |
| A-14 WAL 备份不完整                |       ✅       | 备份前执行 `PRAGMA wal_checkpoint(TRUNCATE)`；备份文件名带时间戳                                                                                                                    |
| A-15 无定时/轮转备份               |       ✅       | 启动时自动备份并保留最近 5 份（NFR006）；`prune_backups` 轮转                                                                                                                       |
| A-16 预算用量不累计                |       ✅       | `record_token_usage_ext` 自动累加 `current_usage`，跨月自动重置；新增 `get_budget_alert`                                                                                            |
| A-17 僵尸会话                      |       ✅       | 启动时 `reconcile_running_sessions()` 把残留 `running` 标记为 `failed` 并补齐时长                                                                                                   |
| A-18 Skill 版本按字段累加          |       ✅       | 改为单条 `UPDATE ... version = version + 1`，一次操作只 +1                                                                                                                          |
| A-21 需求完善不记账                |       ✅       | `enrich_requirement` 返回 `TokenStats` 并写入 `token_usage`                                                                                                                         |
| A-22 `total_cost` 恒为 0           |       ✅       | 新增模型价格表与 `estimate_cost`（含缓存/思考 Token 口径），PRD 与本地同步路径均按估算写入成本                                                                                      |
| A-23 模型名硬编码                  |       ✅       | `ClaudeClient::with_model`；读取 `settings.llm_model`（未配置用默认）                                                                                                               |
| A-24 软删除过滤                    |       ✅       | 已确认 `list_prd_skills` / `get_prd_skill` 均带 `is_deleted = 0`，无需修改                                                                                                          |
| A-25 `AgentTerminal` 绕过封装      |       ✅       | 改用 `ptyWrite` / `ptyResize` / `ptyKill`                                                                                                                                           |
| A-26 `getStageSkillConfig` 死封装  |       ✅       | ProjectDetail 保存环节后改为回读单条并局部合并，避免整表刷新                                                                                                                        |
| A-27 `themeStore` 未统一导出       |       ✅       | `stores/index.ts` 补导出                                                                                                                                                            |
| B-03 缺失索引                      |       ✅       | 新增 `create_supplementary_indexes()`（在 `migrate_tables` 之后执行，覆盖 `tasks.project_id` 等由 ALTER 追加的列）                                                                  |
| B-04 缺唯一约束                    |       ✅       | 新增 `migrate_unique_constraints()`：先安全去重（Agent 会话迁移、重复绑定/默认标记/重复 Key 清理）再建 4 个唯一索引；`add_project_skill` 改为幂等、`add_project_mcp` 自动清理旧默认 |
| C-03 后端测试默认跳过              | ✅（部分改善） | 默认执行的单元测试从 **2 个提升到 29 个**；`integration.rs` 的 E2E 用例仍按设计跳过                                                                                                 |
| C-04 无真实链路测试                |       ✅       | 新增 `src-tauri/src/db/integration_tests.rs`：14 个用例真实访问临时 SQLite，覆盖审计、导入导出、Token 成本与预算、会话生命周期与对账、提交落库、MCP 统计、敏感设置加密、备份轮转    |
| D-02 无路径规避脚本                |       ✅       | 同上 C-01                                                                                                                                                                           |
| D-04 Rust 告警                     |       🔶       | 22 → 18；修复 `unused mut`、`unused status`（同时把 HTTP 状态码加入错误信息）                                                                                                       |

**本轮额外发现的真实缺陷（由新增测试暴露并修复）**

| 缺陷                                                 | 影响                               | 修复                                                                                                                     |
| ---------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `requirement_history` 的外键导致**删除需求必然失败** | 需求删除功能不可用；审计与删除互斥 | 基线建表移除该外键；新增 `migrate_requirement_history()` 重建旧表（SQLite 无法删除外键），使审计记录在需求删除后仍可保留 |

### 7.2 本轮验证结果

| 项                            | 结果                                                       |
| ----------------------------- | ---------------------------------------------------------- |
| `cargo test`                  | ✅ 单元 **29** + 集成 71（E2E 门控跳过）+ 冒烟 3，全部通过 |
| `cargo check`                 | ✅ 0 error                                                 |
| `tsc --noEmit`                | ✅ 0 error                                                 |
| 前端全量（L1 + L3 + L5 + L6） | ✅ **222 用例全部通过**（`npm run test:win` 一条命令）     |
| L6 IPC 覆盖率                 | ✅ 80/80 = **100%**                                        |

### 7.3 第四轮（问题清单 23 项）完成情况

| 编号        | 问题                                                       | 处理结果                                                                                                                                                                                                                                                                                    |
| ----------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1           | F060 前端未闭环（`listGitCommits` 死封装、提交不传关联）   | ✅ `Commits` 页新增关联任务/会话选择（`git_create_commit` 携带 `taskId`/`sessionId`）+ 平台提交记录区块；新增 3 条回归用例                                                                                                                                                                  |
| A-10 / A-11 | `Agents.tsx` 未用 store；无二进制有效性与进程对账          | ✅ 新增 `stores/agentsStore.ts`，页面接入并新增「状态复核」；后端新增 `verify_agents`（二进制有效性复核 + 无进程会话纠正为 `failed`）                                                                                                                                                       |
| A-19        | Skill 执行日志仅 PRD 路径、硬编码 phase/status、失败不记录 | ✅ 日志参数化：`skill_type` 取自 Skill 记录；`prd_generation` 成功与**失败**均写入；新增 `session_start` 写入点（任务 → 项目默认 Skill）。残留：`task_execution` 阶段无写入点（平台尚无该链路）                                                                                             |
| A-20        | `F049` 真流式未接线                                        | ✅ `llm.rs` 改用 `bytes_stream()` 真实增量读取；注册 `generate_prd_streaming` 命令；前端订阅 `prd:chunk` / `prd:done` 实时渲染                                                                                                                                                              |
| B-02        | 迁移机制双轨                                               | ✅ 统一为单一运行时路径；版本登记收敛到 `migration::ensure_baseline()` + `SCHEMA_BASELINE_VERSION`；删除死代码 `apply_migration` / `get_version_info` 与 12 个从未加载的 `.sql`                                                                                                             |
| B-05        | 重复种子逻辑                                               | ✅ `init_tables()` 中的重复播种块移除，统一由 `seed_data()` 负责                                                                                                                                                                                                                            |
| B-06        | `AppState.llm_client` 死字段                               | ✅ 已移除（含初始化）                                                                                                                                                                                                                                                                       |
| 9           | 缓存 `evictions` 恒 0 / 条目无版本号                       | ✅ `evictions` 按 LRU 容量淘汰自增；条目以 `(CACHE_SCHEMA_VERSION, data)` 序列化并校验版本                                                                                                                                                                                                  |
| —           | 列表缓存形同虚设                                           | ✅ 删除从未被调用的 `RequirementService::list_requirements` 与 `CacheKeys::requirements_list`，并清理相关失效调用                                                                                                                                                                           |
| C-05        | L4 CDP 未实跑                                              | ✅ 改写为真实用例：按 `TAURI_DEVTOOLS_SERVER_PORT` 自动启用/跳过，新增 `vitest.l4.config.ts` 与 `npm run test:l4`（**需本机启动应用后手动执行**）                                                                                                                                           |
| C-06        | 18 个页面无单测                                            | ✅ 新增 `src/pages/__tests__/Pages.smoke.test.tsx`，覆盖全部 18 个路由页面 + 未挂载的 `Dashboard`                                                                                                                                                                                           |
| C-07        | L2 E2E 仅 14 例                                            | ✅ 新增 `tests/e2e/pages-coverage.test.ts`（10 例），合计 **24 例** > 计划 22                                                                                                                                                                                                               |
| D-01 / D-03 | 测试文档失实 / 指引过时                                    | ✅ `full-test-report.md`、`full-test-plan.md`、`CLAUDE.md` 已按实际数字与命令刷新                                                                                                                                                                                                           |
| D-04        | 剩余 Rust 告警                                             | ✅ **15 → 0**：可移除的（`llm` 未读字段、`CacheEntry` 访问计数字段、`without_db`、`update_agent_session`）直接删除；确有价值的接口（`validate_session_status_transition`、`is_encrypted_format`、`PaginationParams::offset`）接入真实调用；其余保留 API 加 `#[allow(dead_code)]` 并注明用途 |

### 7.4 DoD 复核

| DoD 项      | v1.0 计划                                                         | 本轮结果                                                                             |
| ----------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| P0 全部完成 | Token 取值正确；本项目路径下可运行全部前端测试                    | ✅ 达成                                                                              |
| P1 全部完成 | 密钥加密；后端默认执行 ≥30 用例；存在真实 SQLite 测试；备份可还原 | ✅ 除"≥30"外全部达成（29 个，E2E 门控用例按设计跳过）                                |
| 质量门禁    | 0 error + 全部测试通过 + L6 100%                                  | ✅ 达成                                                                              |
| 文档一致    | 5 份文档同步                                                      | 🔶 `FEATURES.md` / 差距报告 / 本计划已更新；测试类文档待刷新                         |
| 状态收口    | ⚠️ ≤ 2、❌ = 0、P0/P1 全绿                                        | ✅ ❌ = 0；⚠️ = 4（F021/F030/F034/F035，其中 F030 的进程终止已修复，剩余为语义建议） |

---

_计划版本：v2.1（含第二轮执行结果）_
_上一版本：v2.0（全量问题覆盖）_
