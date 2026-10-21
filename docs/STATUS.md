# 项目实现状态（唯一事实来源）

> 本文档是 ACE 项目**当前实现状态**的唯一权威来源（最后核验：2026-10-20，基于代码逐项核实）。
> 其它历史文档（需求缺口报告、分析报告、规划路线图）反映的是当时的结论，
> 与实现不一致处**一律以本文为准**。

## 一、已完整实现（经代码核实 ✅）

| 能力                                                                                                         | 证据位置                                                                     |
| ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| 项目/需求/任务三级管理 + 看板拖拽                                                                            | `db/{requirements,tasks}.rs`、`pages/{Requirements,Tasks}.kt*`               |
| Skill CRUD（含自定义/版本字段）                                                                              | `db/skills.rs`（`create_custom_skill_from_builtin`、`list_skills_by_type`）  |
| **Skill 自进化闭环**：L1 规则诊断 / L2 优化草稿 / LLM-as-Judge 评分 / 棘轮回滚判定 / 振荡检测 / 人在回路应用 | `services/skill_evolution.rs`（41KB，13 个单测）、`pages/SkillEvolution.tsx` |
| Skill 执行日志（by_task / by_session 双查询键）                                                              | `db/skills.rs` `get_skill_execution_logs_*`                                  |
| Agent 发现 / 启停 / PTY 会话驱动（双模式自动创建）                                                           | `agent_adapter.rs`、`session_driver.rs`、`db/agent_sessions.rs`              |
| Claude API PRD 生成（含流式）                                                                                | `services/prd_service.rs`、`commands::generate_prd_streaming`                |
| Token 多维统计 + 预算告警                                                                                    | `db/tokens.rs`、`services/token_service.rs`                                  |
| Git 集成（status/diff/log/commit）                                                                           | `services/git_service.rs`、`commands` git_*                                  |
| 审计日志                                                                                                     | `db/mod.rs` `record_audit_log` + `pages/AuditLog.tsx`                        |
| Toast 通知系统                                                                                               | `components/Toast.tsx`、`stores/toast.ts`                                    |
| 数据导出 / 导入（含库内文件二进制）                                                                          | `db/io.rs`（payload `__database__`）                                         |
| 定时备份（保留 5 份）+ 启动备份                                                                              | `services/backup_service.rs`、`db/backup.rs`                                 |
| MCP 配置 + 执行日志                                                                                          | `db/mcp.rs`、`db/mcp_execution_logs.rs`                                      |
| 系统监控（CPU/内存/磁盘/温度/网络/电池）                                                                     | `services/system_monitor.rs`                                                 |
| 富文本（TipTap）+ 字段智能完善 + 需求历史                                                                    | `components/RichTextEditor.tsx`、`db/requirement_history.rs`                 |

## 一.5、任务/Agent/Skill/提交 四模块 P0–P2（2026-10-21，未提交待审）

| 级别 | 交付                                                                                                                                                              |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0   | **假 diff 修复**：`git_get_diff` 新增 `commit_hash`，`git show` 视角，Commits 页传 hash（此前所有提交展开显示同一份工作区 diff）                                  |
| P0   | **看板任务挂载**：创建表单加「所属需求/所属项目」下拉；`create_task` 三层打通 `project_id`，经需求自动归属项目；统计口径经 `COALESCE(t.project_id, r.project_id)` |
| P0   | **update_task 补参**：`estimated_hours/priority_level/blocked_reason/notes` 全链可回改（此前创建后即死数据），TaskDetail 编辑区接通                               |
| P0   | **stage_skill_config 接通**：环节配置（stage=prd_generation）插入 PRD 的 Skill 解析链（显式→环节→项目默认→内置），流水线 Tab 文案同步为实况                       |
| P0   | **死代码清理**：删除永不触发的「任务置 in_progress 自动建会话」分支及 `with_agent_service`；孤儿 `start_agent_from_task` 一并删除                                 |
| P1   | **会话→任务回写**：`sync_task_from_session`——completed 会话推进任务状态（pending/in_progress→completed），会话时长累加进 `actual_hours`                           |
| P1   | **Token 回写**：同一路径按 `task_id` 汇总 `token_usage` 表写回 `tasks.token_usage`（消除僵尸字段，失败会话同样累计）                                              |
| P1   | **任务详情追溯视图**：关联会话（跳转会话页）+ 关联提交（hash/文件数/日期，跳提交页）                                                                              |
| P1   | **任务状态机统一**：`validation::task_transitions()` 唯一真源 + `get_task_transitions` 下发；TaskDetail/看板拖拽均改后端表预检                                    |
| P1   | **Agent 管理补全**：`add_agent`（真实执行 `--version` 验证）/ `delete_agent`（运行中会话拒绝）；verify 升级为真探测（可发现"文件在但损坏"）                       |
| P1   | **Skill 编辑器**：变量插入按钮（`{title}/{description}/{priority}`）+ 预览渲染 + `skill_type` 后端白名单校验                                                      |
| P1   | **提交统计后端化**：`git_count_commits`（`rev-list --count`，total/today/week）取代前端 100 条窗口估算；落库失败经 `db_warning` 透出 toast                        |
| P2   | **Agent 能力分层明示**：徽章标注 L3/L2/L1 并附 tooltip 说明；Codex 一次性调用补 `--model` 透传                                                                    |
| P2   | **Git 能力扩展**：`git_list_branches`/`git_switch_branch`（分支名白名单防注入）/`git_show_files`（提交文件清单 status+path）                                      |
| P2   | **Skill 日志标注**：会话启动日志标 `inferred:true`（推定绑定非实际使用证明）；英文错误信息中文化                                                                  |

**验证**：`cargo test` 155 通过（含新增"经需求归属项目"断言）+ `clippy -D warnings` 0 + `vitest` 35 文件全过 + `tsc` 0 + `eslint` 0 error + `prettier` ✓ — **全部未提交**。

## 二、本轮工程加固（P0–P3，2026-10-20）

| 项目              | 说明                                                                                                                                                             |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CI 修复           | ubuntu 缺失的 webkit 系统依赖补装；新增 rust job（`clippy -D warnings` + `cargo test`）；clippy 全库清零                                                         |
| API Key 安全      | 删除前端硬编码口令层（曾导致加密形同虚设）；历史数据读取时透明迁移；LLM 链路拿到的不再是坏 blob                                                                  |
| keystore 落盘保护 | Windows DPAPI 封装（绑定当前用户），裸文件自动升级；禁止无效文件静默重建                                                                                         |
| 禁止明文降级      | keystore 缺失时敏感项**拒绝读写**（写报错、读 None），绝不静默明文                                                                                               |
| 启动容错          | 数据库损坏/数据目录异常 → 系统错误对话框（含处置指引），替代 panic 闪退                                                                                          |
| 版本化迁移框架    | `db/migration.rs`：v9+ 顺序升级、单条独立事务、失败回滚不登记；v9 补齐 7 个高频索引                                                                              |
| 质量门禁          | ESLint 9（0 error / 39 warn，React 19 新规则渐进整改）+ Prettier 全库格式化，接入 CI                                                                             |
| 结构化 IPC 错误   | `AppError{code,message}`（validation/business/not_found/database/external/crypto/legacy）序列化过 IPC；前端 `tauri.ts` invoke 包装层统一规整为带 `code` 的 Error |
| Release 分发      | `v*` 标签触发三平台构建（tauri-action），产出 msi/exe/dmg/AppImage/deb，自动建 Draft Release                                                                     |
| 自动更新          | tauri-plugin-updater 接入（端点 = GitHub Releases latest.json，minisign 密钥已生成于本机 `~/.tauri/`），设置页「检查更新」                                       |
| Skill 模板分享    | 导出/导入 JSON 定义（`schema_version` 版本化格式、名称冲突自动加序号、UI 按钮 + 导出对话框）                                                                     |
| 并发会话上限      | `max_concurrent_sessions` 设置项（默认 3，clamp 1–16），结构化会话启动时强制检查（此前仅文档声明、无实现）                                                       |
| i18n 框架         | 渐进式零依赖方案：中文即源文案、en 字典按模块补充、缺译回退中文；侧边栏/设置核心 chrome 双语 + 语言切换器（localStorage 持久化）                                 |
| 巨文件拆分        | `commands.rs`（78KB/101 命令）→ `commands/` 14 个业务域模块，对外路径不变；`wdio.conf.ts` 死配置删除                                                             |

## 三、项目与需求管理升级（P0–P2，2026-10-21，未提交待审）

| 项目                  | 说明                                                                                                                                                                                       |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 需求截断修复（P0）    | `list_requirements` 默认分页 20 条且前端不传分页 → 第 21 条起全站消失；wrapper 改循环分页拉尽（每页 200，安全阀 500 页）                                                                   |
| PRD 快照保护（P0）    | 每次 `generate_prd*` 落库前自动写快照（`requirement_history` action=`prd_snapshot`，零迁移）；`list_prd_versions`/`restore_prd_version` 命令；详情页「历史」下拉可恢复；重新生成前确认弹窗 |
| 项目归档逃生口（P0）  | 项目列表 hover「归档」+「显示已归档」开关 + 详情页危险区归档/恢复；删除被关联数据拒绝时 toast 透出人话错误；删除确认改统一 ConfirmDialog                                                   |
| 创建 API 对象化（P0） | `createRequirement` 14 个位置参数 → `CreateRequirementParams` 对象；`handleCreate` 去 any                                                                                                  |
| 孤儿清理（P0）        | 删除从未挂载的 `src/pages/Dashboard.tsx`（工作台成果已并入 Workspace），smoke 测试同步清理                                                                                                 |
| 需求 project 过滤     | 后端 `list_requirements` 三层（command/service/db）支持 `project_id`；ProjectDetail 不再全量拉取本地过滤                                                                                   |
| 项目聚合命令          | `get_projects_overview`：一次聚合全部项目 需求/任务 完成计数（任务经 requirement JOIN 归属项目）；项目列表卡双进度条（需求/任务）                                                          |
| 项目字段补齐          | 起止日期/负责人录入 UI + `update_project` 三层扩展 `start_date/end_date/owner_id`；详情概览 Tab 升级为统计头（4 块）+ 表单 + 危险区                                                        |
| PRD Markdown 渲染     | 引入 `react-markdown` + `remark-gfm`（含 GFM 表格）；预览/源码切换；`.prd-markdown` 排版样式                                                                                               |
| LLM 智能拆解          | `generate_task_breakdown`：需求+PRD → 结构化任务清单（API/Agent CLI 双通道、枚举白名单消毒、Token 记账）；详情页清单勾选 → 批量落库跳看板；原单任务创建保留为兜底                          |
| 需求列表项目筛选      | 筛选栏「所属项目」下拉（含未归属）                                                                                                                                                         |
| 状态机统一            | `requirement_transitions()` 提取为后端唯一真源，校验与下发同源；`get_requirement_transitions` 命令下发；前端保留兜底表仅用于拉取失败场景                                                   |
| 状态↔任务联动         | 需求下任务全部完成且状态为开发中/测试中时，详情页出现推进提示条 + 一键推进                                                                                                                 |
| 需求附件（路径管理）  | `update_requirement` 打通 attachments 列；详情页附件区：添加/移除路径，JSON 数组存储，Agent 执行可读取（文件表仍是后续项）                                                                 |
| 看板/会话项目筛选     | Board 按项目过滤（任务直接字段或经需求归属）；Sessions 按 `project_id` 过滤                                                                                                                |
| 质量                  | cargo test 155 + clippy `-D warnings` 0；vitest 35 文件全过（L3 断言对齐新 API）；tsc 0；eslint 0 error（告警均存量）；prettier ✓ — **全部未提交**                                         |

## 四、Agent 对话体验升级（P0–P2，2026-10-21，未提交待审）

| 项目              | 说明                                                                                                                                                                      |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 工作目录修复      | 终端会话 `start_agent` 此前固定用进程 cwd（Agent 在错误目录执行且无感知）；现解析顺序 = 显式传入 →「项目路径」设置 → cwd 兜底，元数据落库可查                             |
| 会话单查          | `get_agent_session(id)` 命令；详情页不再 `getAgentSessions().find()`（O(n) 全量拉取）                                                                                     |
| 暗色主题修复      | `SessionDetail` 硬编码 gray-* 全部换 `var(--color-*)` token                                                                                                               |
| 容错与失败出口    | `skills`/`mcps` JSON 解析 try/catch（脏数据不崩页）；发送失败错误常驻输入区上方 +「重建会话」按钮                                                                         |
| 会话管理          | 重命名 / 归档（`archived` 列，默认列表隐藏，数据全保留）/ 删除（级联清理消息+Skill/MCP 日志+Token 记录）/ 导出 Markdown（写 `exports/` 返回路径）                         |
| 原子重建          | `force_rebuild_session`：后端 interrupt → 等注册表释放 → 重建，取代前端"轮询正在运行错误 ×12"hack                                                                         |
| 自动命名          | 无任务关联的会话自动生成「agent · MM-DD HH:MM」标题，列表告别 UUID（UUID 降级为悬浮提示）                                                                                 |
| 冷启动引导        | 空会话显示关联任务卡 +「按任务开始」一键发送任务描述 + 建议开场白（`session_opening_prompts` 设置可自定义，Settings 页编辑）                                              |
| 运行态成本仪表    | 详情页「成本」面板：订阅 `session:result` 事件单查刷新，Token/费用/耗时实时可见                                                                                           |
| 附件体验          | 附件以 marker 区块注入消息；气泡解析渲染图片缩略图（asset protocol scope = `$APPDATA/attachments/**`）+ 文件 chip，不再展示裸路径字符串                                   |
| 消息操作          | 消息气泡 hover 复制全文                                                                                                                                                   |
| 状态自动刷新      | `session:status` 事件（spawn/退出/终止/删除 emit）；`sessionsStore` 模块级单例订阅，列表与统计卡自动更新                                                                  |
| 原始事件流视图    | 结构化会话新增「原始流」tab：seq/role/block_type/tool/status + content 原文，排障用                                                                                       |
| 卡片墙            | 会话列表新增卡片视图切换（表格 ↔ 卡片），卡片含状态/Agent/时长/Token/成本与全部管理动作                                                                                   |
| 终端模式参数对齐  | `start_agent` 支持 `work_dir`/`model` 元数据（model 仅记录，PTY 内 `/model` 自行切换）                                                                                    |
| P2-13 统一 handle | 评估结论：`ptyAttach(db_session_id)` 已是前端唯一 db_id→pty_id 解析入口（AgentTerminal 内聚管理 ptyIdRef），无新增双 ID 使用点，维持现状不重复建设                        |
| 质量              | cargo test 155 + clippy `-D warnings` 0；vitest 35 文件全过（新增附件解析/归档过滤/级联删除测试）；tsc 0；eslint 0 error（40 warn 均为存量）；prettier ✓ — **全部未提交** |

## 五、已知缺口（⚠️ 明确未做）

| 缺口                          | 现状                                                                   | 建议                                                                   |
| ----------------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| 附件后端                      | 会话附件仍为 marker 路径注入（气泡已可渲染缩略图），无文件表/传输管理  | 引入 `attachments` 表 + 二进制落盘（对齐导入导出机制）                 |
| i18n 覆盖面                   | 框架已建（P3），仅侧边栏/设置核心双语；业务页面仍中文                  | 按页面补 en 字典                                                       |
| 自动更新发布密钥              | 签名密钥在本机 `~/.tauri/ace-key.pem`                                  | 私钥配到 GitHub Secrets `UPDATER_PRIVATE_KEY` 后才产出签名 latest.json |
| Playwright E2E / l4-cdp       | 不在任何 CI（纯 vite 前端导航层）；真后端链路由 victauri workflow 覆盖 | 按需补导航回归                                                         |
| React 19 新规则               | 39 个 warn（refs/static-components/immutability 等，15 个文件）        | 按文件逐步整改后升回 error                                             |
| 单库 100MB 上限               | design.md NFR 声明                                                     | 扩容或分库                                                             |
| vitest 中文路径               | 需 `subst` ASCII 盘符 workaround                                       | CI 固定 ASCII 路径 / 升级 vitest                                       |
| `system_monitor` 命令错误类型 | 仍为 String（未纳入结构化 AppError）                                   | 后续统一                                                               |

## 六、历史文档导航（均为快照，勿作现状依据）

- `requirements/requirement-gap-report.md`（2026-09-20 快照：其中"Skill CRUD 未实现"等结论已过期）
- `analysis-report.md`（2026-09-20 快照：多数问题已修复）
- `analysis/gap-analysis-report.md`、`plan-fix-v3.md`（历史缺口分析与修复计划）
- `requirements/feature-completeness-spec.md`、`features/feature-spec.md`（立项规格）
- `features/multi-agent-adaptation-plan.md`、`features/session-interaction-design.md`（历史设计）
- `testing/full-test-report.md`、`full-test-plan.md`、`auto-test-plan.md`（历史测试计划/报告）
