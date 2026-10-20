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

## 二、本轮工程加固（P0/P1，2026-10-20）

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

## 三、已知缺口（⚠️ 明确未做）

| 缺口                          | 现状                                                                               | 建议                                                   |
| ----------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------ |
| 附件后端                      | 仅 `requirements.attachments` / `tasks.attachments` JSON 路径列，无文件表/文件传输 | 引入 `attachments` 表 + 二进制落盘（对齐导入导出机制） |
| i18n                          | 纯中文硬编码                                                                       | 引入 i18n 框架，README 已公开需考虑国际用户            |
| 自动更新                      | 无                                                                                 | `tauri-plugin-updater` + Release 工作流                |
| Playwright E2E / l4-cdp       | 不在任何 CI（E2E 跑纯 vite 无 Tauri 后端）                                         | victauri 真后端驱动 E2E                                |
| React 19 新规则               | 39 个 warn（refs/static-components/immutability 等，15 个文件）                    | 按文件逐步整改后升回 error                             |
| 并发会话                      | 硬上限 3、单库 100MB                                                               | 设置化并扩容                                           |
| vitest 中文路径               | 需 `subst` ASCII 盘符 workaround                                                   | CI 固定 ASCII 路径 / 升级 vitest                       |
| `system_monitor` 命令错误类型 | 仍为 String（未纳入结构化 AppError）                                               | 后续统一                                               |

## 四、历史文档导航（均为快照，勿作现状依据）

- `requirements/requirement-gap-report.md`（2026-09-20 快照：其中"Skill CRUD 未实现"等结论已过期）
- `analysis-report.md`（2026-09-20 快照：多数问题已修复）
- `analysis/gap-analysis-report.md`、`plan-fix-v3.md`（历史缺口分析与修复计划）
- `requirements/feature-completeness-spec.md`、`features/feature-spec.md`（立项规格）
- `features/multi-agent-adaptation-plan.md`、`features/session-interaction-design.md`（历史设计）
- `testing/full-test-report.md`、`full-test-plan.md`、`auto-test-plan.md`（历史测试计划/报告）
