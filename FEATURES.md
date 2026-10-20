# AI Collaboration Engine (ACE) — 功能详细设计说明文档

> 文档版本：v2.2
> 更新日期：2026-09-21
> 文档定位：功能需求 + 详细设计（可开发、可验收）
> 技术栈：Tauri 2.0 + React 18 + SQLite (WAL) + Zustand
> 事实来源：以 `FEATURES.md` v1.0 功能盘点为基座，并对 `src/`、`src-tauri/` 做只读核对后修正

---

## 一、文档说明

### 1.1 文档定位

本文档是 ACE 的功能主文档，包含三层结构：

| 层次 | 章节 | 作用 |
|------|------|------|
| 总览层 | 二、系统概览 / 三、功能总览 | 建立全局认知与编号索引 |
| 详细层 | 四、功能详细设计 | 逐功能的结构化规格（开发与验收的直接依据） |
| 索引层 | 五 ~ 十 | 数据模型、IPC 命令、页面交互、状态机、技术特性、验收汇总 |

### 1.2 阅读指南

- **产品/评审**：读「二、系统概览」+「三、功能总览」+「十、验收标准汇总」。
- **开发**：以「四、功能详细设计」中对应 `F0xx` 条目为准，结合「五、数据模型设计」「六、IPC 命令清单」实现。
- **测试**：直接使用每个功能点末尾的 **验收标准**（Given/When/Then）编写用例。
- **编号规则**：功能编号 `F0xx` 与功能模块一一对应，号段划分见「3.2 编号号段」。

### 1.3 实现状态标记约定

| 标记 | 含义 |
|------|------|
| ✅ 已实现 | 前后端链路完整，可正常使用 |
| ⚠️ 部分实现 | 存在可用路径，但功能不完整或存在已知缺陷 |
| ❌ 未实现 | 代码中无对应实现（含"有表无写入""有页面无后端"两种情况） |
| ⚪ 未接线 | 底层能力已实现，但未对外暴露或未接入调用链路 |
| 📋 设计中 | **详细设计已完成、代码尚未实现**（模块 16 专用；设计见 `docs/features/session-interaction-design.md`） |

### 1.4 术语表

| 术语 | 说明 |
|------|------|
| ACE | AI Collaboration Engine，本应用名称 |
| Agent | 可被调用的 AI 命令行工具（如 claude、codex），由 PATH 或常见安装路径发现 |
| Session（会话） | 一次 Agent 执行的完整记录，对应 `agent_sessions` 表 |
| PTY | 伪终端（Pseudo Terminal），用于承载 Agent 交互式进程 |
| Skill | PRD 生成模板（对应 `prd_skills` 表），分内置与自定义 |
| MCP | Model Context Protocol 服务端点，按项目绑定 |
| Stage（环节） | 研发流水线的 6 个环节，见 3.3 节 |
| PRD | 产品需求文档，由 LLM 依据需求标题/描述生成 |
| Token 预算 | 月度 Token 消耗上限与告警阈值配置 |

### 1.5 修订记录

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| v1.0 | 2026-09-21 | 初版，15 个模块的功能清单（表格形式） |
| v2.0 | 2026-09-21 | 重写为逐功能结构化设计文档：新增功能编号体系、75 项功能详细规格、字段级数据模型、命令出入参、页面交互、状态机汇总、验收汇总；修正路由名与命令数量错误；按代码核对结果标注真实实现状态 |

---

## 二、系统概览

### 2.1 产品定位

ACE 是一款 **AI 辅助研发协作桌面应用**，覆盖 **项目管理 → 需求管理 → 任务管理 → Agent 执行 → 统计分析** 的完整研发流程。通过集成 Claude API 实现 PRD 自动生成与需求字段自动完善；通过 PTY 终端承载真正的交互式 Agent 会话；通过 Git 集成管理代码提交与变更追溯。

### 2.2 目标用户与角色

| 角色 | 说明 | 使用诉求 |
|------|------|----------|
| 单人开发者（唯一用户） | 本应用为单机版，无多用户、无权限体系 | 用 AI Agent 加速需求→代码的全流程，并掌握 Token 成本 |

> **权限模型**：无登录、无角色、无租户隔离。所有功能点的"权限控制"字段统一为「无（单用户本地应用）」。
> **Agent 作为"系统参与者"**：Agent 进程由用户启动，代表用户执行开发任务，其产生的会话、Token、提交记录均归属当前用户。

### 2.3 系统边界

**包含**：

- 项目 / 需求 / 任务的三级管理（项目 → 需求 → 任务）
- Agent 发现、启停、PTY 交互式终端
- 会话记录、会话详情、执行日志
- Skill（PRD 模板）与 MCP 端点的配置与使用统计
- Claude API 集成（PRD 生成、需求字段完善）
- Token 消耗统计、预算管理与成本分析
- Git 状态 / Diff / 提交 / 日志
- 审计日志（需求变更历史）
- Token 预算、缓存管理、系统监控、系统托盘

**不包含**：

- 多用户协作与权限管理
- 远程终端 / 多用户共享终端 / 文件传输
- 云端同步与 SaaS 化部署
- 代码审查工作流（仅记录提交，不做 MR/PR 审批流）
- 需求/任务的外部系统双向同步

### 2.4 总体业务流程

```
                    ┌──────────────┐
                    │  1. 创建项目  │  projects
                    └──────┬───────┘
                           │ 绑定 Skill / MCP / 环节配置
                           ▼
                    ┌──────────────┐
                    │  2. 录入需求  │  requirements（draft）
                    └──────┬───────┘
                           │ 可选：LLM 自动完善字段
                           ▼
                    ┌──────────────┐
                    │ 3. 生成 PRD  │  Claude API + Skill 模板
                    └──────┬───────┘
                           │ 评审：reviewing → approved
                           ▼
                    ┌──────────────┐
                    │  4. 拆分任务  │  tasks（pending）
                    └──────┬───────┘
                           │ 状态置 in_progress（可触发自动启动 Agent）
                           ▼
                    ┌──────────────┐
                    │ 5. Agent 执行 │  agent_sessions + PTY 终端
                    └──────┬───────┘
                           │ 输出流 / Token 记录 / Skill·MCP 执行日志
                           ▼
                    ┌──────────────┐
                    │ 6. Git 提交   │  git status / diff / commit / log
                    └──────┬───────┘
                           ▼
                    ┌──────────────┐
                    │ 7. 统计分析   │  Token 趋势 / 模型分布 / 预算告警
                    └──────────────┘
```

### 2.5 功能域划分

将 15 个功能模块归并为 5 大功能域，便于理解与分阶段实施：

| 功能域 | 包含模块 | 核心价值 |
|--------|----------|----------|
| A. 研发资产管理 | 1 项目管理、2 需求管理、3 任务管理 | 建立"项目 → 需求 → 任务"的可追溯结构 |
| B. AI 协作执行 | 4 Agent 管理、5 会话管理、6 Skill 管理、7 MCP 配置管理 | 让 AI Agent 在可控、可观察的环境下执行开发任务 |
| C. 成本与度量 | 8 统计分析、9 LLM 集成 | 量化 Token/成本，驱动模型与流程优化 |
| D. 工程与追溯 | 12 Git 集成、11 审计日志 | 代码与操作的双重可追溯 |
| E. 平台基础能力 | 10 安全、13 系统配置、14 UI 与交互、15 数据管理 | 支撑上层功能的底座能力 |

### 2.6 流水线环节（Stage）

系统预置 6 个研发环节，每个环节可在项目维度绑定默认 Skill（`stage_skill_configs` 表，`project_id + stage` 唯一）：

| 序号 | stage 值 | 显示名 |
|------|----------|--------|
| 1 | `requirement_analysis` | 需求分析 |
| 2 | `prd_generation` | PRD 生成 |
| 3 | `task_planning` | 任务规划 |
| 4 | `code_generation` | 代码生成 |
| 5 | `testing` | 测试 |
| 6 | `review` | Review |

---

## 三、功能总览

### 3.1 功能清单总表

> 说明：**后端命令**列列出该功能对应的 IPC 命令；**实现状态**依据代码只读核对结果标注。

| 编号 | 功能名称 | 所属模块 | 前端位置 | 后端命令 | 数据表 | 实现状态 |
|------|----------|----------|----------|----------|--------|----------|
| F001 | 项目 CRUD | 1 项目管理 | `pages/Projects.tsx` | `create/list/get/update/delete_project` | `projects` | ✅ |
| F002 | 项目 Key 标识 | 1 项目管理 | `pages/Projects.tsx`、`pages/ProjectDetail.tsx` | （随项目 CRUD） | `projects` | ✅ |
| F003 | 项目 Skill 配置 | 1 项目管理 | `pages/ProjectDetail.tsx` | `list/add/remove_project_skill` | `project_skills` | ✅ |
| F004 | 项目 MCP 配置 | 1 项目管理 | `pages/ProjectDetail.tsx` | `list/add/remove_project_mcp` | `project_mcps` | ✅ |
| F005 | 流水线环节配置 | 1 项目管理 | `pages/ProjectDetail.tsx` | `get_pipeline_stages`、`list/get/upsert/remove_stage_skill_config` | `stage_skill_configs` | ✅ |
| F006 | 需求 CRUD | 2 需求管理 | `pages/Requirements.tsx`、`Requirements/RequirementDetail.tsx` | `create/list/get/update/delete_requirement` | `requirements` | ✅ |
| F007 | 需求状态流转 | 2 需求管理 | `pages/RequirementDetail.tsx` | `update_requirement`（含状态校验） | `requirements` | ✅ |
| F008 | 需求扩展字段 | 2 需求管理 | `components/RequirementForm.tsx` | `create/update_requirement` | `requirements` | ✅ |
| F009 | PRD 自动生成 | 2 需求管理 | `pages/RequirementDetail.tsx` | `generate_prd` | `requirements`、`token_usage`、`skill_execution_logs` | ✅ |
| F010 | 需求字段自动完善 | 2 需求管理 | `pages/RequirementDetail.tsx`（「智能完善」按钮） | `enrich_requirement` | `requirements` | ✅ |
| F011 | 需求变更历史 | 2 需求管理 | `pages/AuditLog.tsx` | `get_audit_logs`（写入由 RequirementService 完成） | `requirement_history` | ✅ |
| F012 | 需求表单输入校验 | 2 需求管理 | `components/RequirementForm.tsx` | —（纯前端） | — | ✅ |
| F013 | 任务 CRUD | 3 任务管理 | `pages/Board.tsx`、`pages/TaskDetail.tsx` | `create/list/get/update/delete_task` | `tasks` | ✅ |
| F014 | 任务状态管理 | 3 任务管理 | `pages/Board.tsx`、`pages/TaskDetail.tsx` | `update_task_status` | `tasks` | ✅ |
| F015 | 任务评论 | 3 任务管理 | `pages/TaskDetail.tsx` | `create/list/delete_task_comment` | `task_comments` | ✅ |
| F016 | 任务按项目/需求筛选 | 3 任务管理 | `pages/ProjectDetail.tsx`、`pages/RequirementDetail.tsx` | `list_tasks_by_project`、`list_tasks_by_requirement` | `tasks` | ✅ |
| F017 | 任务扩展字段 | 3 任务管理 | `components/TaskForm.tsx` | `create/update_task` | `tasks` | ✅ |
| F018 | 任务表单输入校验 | 3 任务管理 | `components/TaskForm.tsx` | —（纯前端） | — | ✅ |
| F019 | Agent 自动检测 | 4 Agent 管理 | `pages/Agents.tsx` | `discover_agents`、`get_agents` | `agents` | ✅ |
| F020 | Agent 版本解析 | 4 Agent 管理 | `pages/Agents.tsx` | （随 `discover_agents`） | `agents` | ✅ |
| F021 | Agent 状态管理 | 4 Agent 管理 | `pages/Agents.tsx` | `get_agents` | `agents` | ⚠️ |
| F022 | PTY 伪终端会话 | 4 Agent 管理 | `components/AgentTerminal.tsx` | `pty_spawn`（持有真实 child/master/writer/killer 句柄） | —（内存态） | ✅ |
| F023 | Agent 终端 UI（xterm.js） | 4 Agent 管理 | `components/AgentTerminal.tsx` | `pty_write`（真实写 stdin）、`pty_resize`（真实 resize）、`pty_kill`（真实 kill） | — | ✅ |
| F024 | PTY 事件流 | 4 Agent 管理 | `components/AgentTerminal.tsx` | 事件 `pty:event` | — | ✅ |
| F025 | Agent 启动与会话创建 | 4 Agent 管理 | `pages/Agents.tsx` | `start_agent`、`stop_agent` | `agents`、`agent_sessions` | ⚠️ |
| F026 | 会话列表 | 5 会话管理 | `pages/Sessions.tsx` | `get_agent_sessions` | `agent_sessions` | ✅ |
| F027 | 会话详情 | 5 会话管理 | `pages/SessionDetail.tsx` | `get_agent_sessions`、`get_skill_execution_logs`、`get_mcp_execution_logs` | `agent_sessions`、`skill_execution_logs`、`mcp_execution_logs` | ⚠️ |
| F028 | 从任务创建会话 | 5 会话管理 | `pages/TaskDetail.tsx` | `create_session_from_task` | `agent_sessions` | ✅ |
| F029 | 会话终端交互 | 5 会话管理 | `pages/SessionDetail.tsx`（终端视图）、`pages/Sessions.tsx` | `pty_write`、`pty_resize` | — | ✅ |
| F030 | 会话停止 | 5 会话管理 | `pages/Sessions.tsx` | `stop_agent`、`pty_kill` | `agent_sessions` | ⚠️ |
| F031 | 内置 Skill 模板 | 6 Skill 管理 | `pages/SkillManagement.tsx` | `list_prd_skills` | `prd_skills` | ✅ |
| F032 | 基于内置创建自定义 Skill | 6 Skill 管理 | `pages/SkillManagement.tsx` | `create_custom_skill_from_builtin` | `prd_skills` | ✅ |
| F033 | Skill CRUD | 6 Skill 管理 | `pages/SkillManagement.tsx` | `create/get/update/delete_prd_skill` | `prd_skills` | ✅ |
| F034 | Skill 版本控制 | 6 Skill 管理 | `pages/SkillManagement.tsx` | `update_prd_skill` | `prd_skills` | ⚠️ |
| F035 | Skill 执行日志 | 6 Skill 管理 | `pages/SessionDetail.tsx` | `get_skill_execution_logs` | `skill_execution_logs` | ⚠️ |
| F036 | Skill 使用统计 | 6 Skill 管理 | `pages/Statistics.tsx`（使用统计区块） | `get_skill_usage_stats` | `skill_execution_logs` | ✅ |
| F037 | MCP 服务配置 | 7 MCP 配置 | `pages/ProjectDetail.tsx` | `list/add/remove_project_mcp` | `project_mcps` | ✅ |
| F038 | 默认 MCP | 7 MCP 配置 | `pages/ProjectDetail.tsx` | `add_project_mcp` | `project_mcps` | ✅ |
| F039 | MCP 执行日志 | 7 MCP 配置 | `pages/SessionDetail.tsx` | `get_mcp_execution_logs`（会话启动时写入连接记录） | `mcp_execution_logs` | ✅ |
| F040 | MCP 使用统计 | 7 MCP 配置 | `pages/Statistics.tsx`（使用统计区块） | `get_mcp_usage_stats` | `mcp_execution_logs` | ✅ |
| F041 | Token 使用统计 | 8 统计分析 | `pages/Cost.tsx`、`pages/Statistics.tsx` | `get_token_usage`、`get_token_usage_summary` | `token_usage` | ✅ |
| F042 | 每日使用趋势 | 8 统计分析 | `pages/Cost.tsx` | `get_token_usage_trend` | `token_usage` | ✅ |
| F043 | 模型使用分布 | 8 统计分析 | `pages/Cost.tsx` | `get_token_usage_by_model` | `token_usage` | ✅ |
| F044 | Token 预算管理 | 8 统计分析 | `pages/Cost.tsx` | `get_token_budget`、`set_token_budget` | `token_budgets` | ✅ |
| F045 | 缓存统计与清理 | 8 统计分析 | `pages/SystemMonitor.tsx`（缓存卡片 + 清空按钮） | `get_cache_stats`、`clear_cache` | —（内存） | ✅ |
| F046 | 统计报表聚合 | 8 统计分析 | `pages/Workspace.tsx`、`pages/Statistics.tsx` | `get_statistics` | 多表聚合 | ✅ |
| F047 | Claude API 调用 | 9 LLM 集成 | `pages/RequirementDetail.tsx`（间接） | （`PrdService` 内部） | — | ✅ |
| F048 | 本地用量同步 | 9 LLM 集成 | `pages/Cost.tsx` | `sync_local_usage` | `token_usage` | ✅ |
| F049 | 流式响应（SSE） | 9 LLM 集成 | — | 未暴露为命令 | — | ⚪ |
| F050 | LLM 调用 Token 统计 | 9 LLM 集成 | `pages/Cost.tsx` | `get_token_usage_summary` | `token_usage` | ✅ |
| F051 | API Key 加密存储 | 10 安全 | `pages/Settings.tsx` | `encrypt_api_key`、`decrypt_api_key` | `settings` | ✅ |
| F052 | 全局输入校验 | 10 安全 | `components/RequirementForm.tsx`、`TaskForm.tsx` | —（纯前端） | — | ✅ |
| F053 | 变更历史记录 | 11 审计日志 | `src-tauri/src/services/requirement_service.rs` | —（create/update/status_change/delete 自动写入） | `requirement_history` | ✅ |
| F054 | 审计日志页面 | 11 审计日志 | `pages/AuditLog.tsx` | `get_audit_logs`（支持 entity_type 过滤） | `requirement_history` | ✅ |
| F055 | 操作类型标记 | 11 审计日志 | `pages/AuditLog.tsx` | `get_audit_logs` | `requirement_history` | ✅ |
| F056 | Git 状态查看 | 12 Git 集成 | `pages/Commits.tsx` | `git_get_status` | — | ✅ |
| F057 | Git Diff | 12 Git 集成 | `pages/Commits.tsx` | `git_get_diff` | — | ✅ |
| F058 | Git Commit | 12 Git 集成 | `pages/Commits.tsx` | `git_create_commit` | — | ✅ |
| F059 | Git Log | 12 Git 集成 | `pages/Commits.tsx` | `git_log` | — | ✅ |
| F060 | 提交记录落库 | 12 Git 集成 | `pages/Commits.tsx` | `git_create_commit`（落库）、`list_git_commits` | `git_commits` | ✅ |
| F061 | 设置管理（KV） | 13 系统配置 | `pages/Settings.tsx` | `get_setting`、`set_setting` | `settings` | ✅ |
| F062 | Claude API Key 配置 | 13 系统配置 | `pages/Settings.tsx` | `get/set_setting`、`encrypt/decrypt_api_key` | `settings` | ✅ |
| F063 | 工作台首页 | 14 UI 与交互 | `pages/Workspace.tsx` | `get_statistics`、`list_projects` | 多表 | ✅ |
| F064 | 侧边导航布局 | 14 UI 与交互 | `components/Layout.tsx` | `get_monitor_data` | — | ✅ |
| F065 | 分页组件 | 14 UI 与交互 | `components/Pagination.tsx` | `list_requirements`（分页） | — | ✅ |
| F066 | 表单组件 | 14 UI 与交互 | `components/RequirementForm.tsx`、`TaskForm.tsx` | — | — | ✅ |
| F067 | 状态/优先级徽章 | 14 UI 与交互 | `components/Badges.tsx` | — | — | ✅ |
| F068 | 加载骨架屏 | 14 UI 与交互 | `components/Skeleton.tsx` | — | — | ✅ |
| F069 | Toast 通知 | 14 UI 与交互 | `components/Toast.tsx` | — | — | ✅ |
| F070 | 数字滚动动画 | 14 UI 与交互 | `components/AnimatedNumber.tsx` | — | — | ✅ |
| F071 | 错误边界 | 14 UI 与交互 | `components/ErrorBoundary.tsx` | — | — | ✅ |
| F072 | 暗色/亮色主题 | 14 UI 与交互 | `stores/themeStore.ts` | — | localStorage | ✅ |
| F073 | 数据导出 | 15 数据管理 | `pages/ImportExport.tsx` | `export_data` | 全表快照 | ✅ |
| F074 | 数据导入 | 15 数据管理 | `pages/ImportExport.tsx`（含 dry-run 预检与自动备份） | `import_data` | 全表快照 | ✅ |
| F075 | 数据库异常自动备份 | 15 数据管理 | —（启动时自动） | —（`main.rs` 初始化逻辑） | `database.db.bak` | ✅ |

> 另有 1 个**未挂载路由**的页面 `pages/Dashboard.tsx`（仪表盘），与 F063 工作台首页功能重叠，详见 7.2 节。

**模块 16：会话交互与消息（P0 已实现，11 项）**

| 编号 | 功能名称 | 所属模块 | 前端位置（规划） | 后端命令/事件（规划） | 涉及数据表（规划） | 状态 |
|------|----------|----------|------------------|----------------------|-------------------|:---:|
| F076 | 会话消息持久化 | 16 会话交互与消息 | `pages/SessionDetail.tsx` | `list_session_messages` | `session_messages` | ✅ |
| F077 | 指令输入（发送/中断/历史） | 16 会话交互与消息 | `components/SessionInput.tsx` | `send_session_message`、`interrupt_session` | `session_messages` | ✅ |
| F078 | 回复流式渲染（Markdown） | 16 会话交互与消息 | `components/SessionChat.tsx`、`MessageBubble.tsx` | 事件 `session:message`、`session:delta` | `session_messages` | ⚠️ |
| F079 | 思考内容（thinking，默认折叠） | 16 会话交互与消息 | `components/MessageBubble.tsx`（ThinkingBlock） | 事件 `session:message` | `session_messages` | ✅ |
| F080 | 工具调用与结果块 | 16 会话交互与消息 | `components/MessageBubble.tsx`（ToolUse/ToolResult） | 事件 `session:tool` | `session_messages` | ✅ |
| F081 | 双视图与模式切换 | 16 会话交互与消息 | `pages/SessionDetail.tsx`、`components/AgentTerminal.tsx` | `set_session_mode` | `agent_sessions.mode` / `view` | ✅ |
| F082 | 多轮上下文与会话续接 | 16 会话交互与消息 | `pages/SessionDetail.tsx` | `resume_session` | `agent_sessions.cli_session_id` | ✅ |
| F083 | 权限预设与被拒可视化 | 16 会话交互与消息 | `components/MessageBubble.tsx`（PermissionDeniedCard） | `set_session_mode`（白名单沉淀） | `agent_sessions.permission_allowlist` | ⚠️ |
| F084 | 消息级 Token 与成本归属 | 16 会话交互与消息 | `pages/SessionDetail.tsx`、`pages/Cost.tsx` | 事件 `session:result` | `session_messages.tokens`、`token_usage` | ✅ |
| F085 | 会话历史检索与回放 | 16 会话交互与消息 | `pages/SessionDetail.tsx` | `list_session_messages` | `session_messages` | ✅ |
| F086 | 错误与中断语义 | 16 会话交互与消息 | `pages/SessionDetail.tsx` | `interrupt_session` | `session_messages` | ✅ |

> **模块 16 的完整逐功能规格**（目标用户 / 使用场景 / 主流程 / 异常流程 / 边界条件 / 输入输出 / 依赖 / 约束 / 状态变化 / 权限 / 验收标准）见设计文档 `docs/features/session-interaction-design.md` §15，本节仅登记编号与索引。
> **P0 已同步**：数据表 18 → **19**（`session_messages`）；IPC 命令 82 → **88**（新增 `start_structured_session`、`send_session_message`、`list_session_messages`、`interrupt_session`、`set_session_mode`、`resume_session`）；Tauri 事件 3 → **9**（新增 `session:message`/`session:delta`/`session:tool`/`session:result`/`session:error`/`session:notice`）。
> **P0 范围说明**：F078 为 ⚠️（正文/围栏代码块/行内 code 已渲染，完整 Markdown 待引入渲染库）；F083 为 ⚠️（被拒卡片「允许并重试/改用 acceptEdits」+ 白名单沉淀已实现，逐次授权确认依赖终端视图，P2 再评估）。
> **F048（结构化模式记账）已实现**：单轮结束写入 `token_usage`（`source="session"`，按会话/任务归因，模型取 `modelUsage.canonicalModel`），并自动累计 Token 预算；会话级 `total_tokens`/`total_cost` 同步累加。
> P1 范围：多 Agent 适配（codex）、消息内思考折叠持久化偏好、成本页按会话维度展示。

### 3.2 编号号段

| 号段 | 模块 | 数量 |
|------|------|------|
| F001 – F005 | 1 项目管理 | 5 |
| F006 – F012 | 2 需求管理 | 7 |
| F013 – F018 | 3 任务管理 | 6 |
| F019 – F025 | 4 Agent 管理 | 7 |
| F026 – F030 | 5 会话管理 | 5 |
| F031 – F036 | 6 Skill 管理 | 6 |
| F037 – F040 | 7 MCP 配置管理 | 4 |
| F041 – F046 | 8 统计分析 | 6 |
| F047 – F050 | 9 LLM 集成 | 4 |
| F051 – F052 | 10 安全 | 2 |
| F053 – F055 | 11 审计日志 | 3 |
| F056 – F060 | 12 Git 集成 | 5 |
| F061 – F062 | 13 系统配置 | 2 |
| F063 – F072 | 14 UI 与交互 | 10 |
| F073 – F075 | 15 数据管理 | 3 |
| F076 – F086 | 16 会话交互与消息（设计中） | 11 |
| **合计** | **16 个模块** | **86 项** |

### 3.3 模块统计

| 模块 | 功能数 | ✅ | ⚠️ | ⚪ | ❌ | 📋 |
|------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 项目管理 | 5 | 5 | 0 | 0 | 0 | 0 |
| 2 需求管理 | 7 | 7 | 0 | 0 | 0 | 0 |
| 3 任务管理 | 6 | 6 | 0 | 0 | 0 | 0 |
| 4 Agent 管理 | 7 | 7 | 0 | 0 | 0 | 0 |
| 5 会话管理 | 5 | 5 | 0 | 0 | 0 | 0 |
| 6 Skill 管理 | 6 | 5 | 1 | 0 | 0 | 0 |
| 7 MCP 配置管理 | 4 | 4 | 0 | 0 | 0 | 0 |
| 8 统计分析 | 6 | 6 | 0 | 0 | 0 | 0 |
| 9 LLM 集成 | 4 | 4 | 0 | 0 | 0 | 0 |
| 10 安全 | 2 | 2 | 0 | 0 | 0 | 0 |
| 11 审计日志 | 3 | 3 | 0 | 0 | 0 | 0 |
| 12 Git 集成 | 5 | 5 | 0 | 0 | 0 | 0 |
| 13 系统配置 | 2 | 2 | 0 | 0 | 0 | 0 |
| 14 UI 与交互 | 10 | 10 | 0 | 0 | 0 | 0 |
| 15 数据管理 | 3 | 3 | 0 | 0 | 0 | 0 |
| 16 会话交互与消息（P0 已实现） | 11 | 9 | 2 | 0 | 0 | 0 |
| **合计** | **86** | **83** | **3** | **0** | **0** | **0** |

> 📋 = 详细设计已完成、代码尚未实现。模块 16 设计见 `docs/features/session-interaction-design.md`（v1.2），P0 已实现（详见 §3.1 模块 16 表格的状态列与范围说明）。

---

## 四、功能详细设计

> 章节组织：按 15 个模块顺序展开，每个功能点自成一节，字段标签统一、可直接用于开发与验收。
> **通用约定**：目标用户与角色均为「单人开发者」；权限控制均为「无（单用户本地应用）」；状态变化中"无"表示该功能不改变任何实体状态。

---

### 模块 1：项目管理

#### F001: 项目 CRUD

**功能编号与名称：** F001 项目 CRUD
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 建立研发工作的顶层容器，按项目组织需求与任务
**前置条件：** 应用已启动，数据库已完成初始化（`init_tables()` 已执行）
**主流程：**
1. 用户点击左侧导航「项目」进入项目列表页（`/projects`）
2. 列表按创建时间展示项目卡片/行，支持按名称、描述搜索
3. 点击「新建项目」弹出创建表单，填写项目名称（必填）、描述、项目 Key
4. 提交后调用 `create_project` 写入 `projects` 表，成功后刷新列表
5. 点击项目进入项目详情页（`/projects/:id`），展示基本信息与关联数据
6. 在详情页可编辑项目信息（`update_project`）或删除项目（`delete_project`）
**异常流程：**
- 项目名称为空 → 前端置灰保存按钮
- 项目名称超过 100 字符 → 后端 `validate_project` 返回校验错误
- 删除不存在的项目 → 后端返回错误信息，前端 Toast 提示
**边界条件：**
- 项目名称长度：1 – 100 字符
- 项目 Key 长度：≤ 10 字符（前端 `maxLength={10}`）
- 项目 Key 在数据库层**无唯一约束**，存在重复 Key 的可能（应用层未做去重）
**输入与输出：**
- 输入：`name`（必填）、`description`、`key`、`status`、`owner_id`、`start_date`、`end_date`
- 输出：`Project` 对象（含 `id`、`created_at`、`updated_at`）
**依赖关系：** 依赖 `useProjectsStore`（`fetchProjects`/`addProject`/`removeProject`）、`ProjectService`（含缓存失效）、`projects` 表
**约束条件：** 项目是需求与任务的上游实体，`requirements.project_id` 为 `NOT NULL` 外键指向 `projects(id)`，因此**删除项目前需先处理其关联需求**（不做级联删除）
**状态变化：** 项目状态取值 `active` / `paused` / `archived`（数据库层仅 DEFAULT `'active'`，**无 CHECK 约束**；`paused`/`archived` 为前端约定值）
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目列表为空
When 点击「新建项目」并填写名称"ACE 平台"
Then 创建成功，列表出现新项目，且数据库 projects 表新增 1 行

Given 项目名称输入 101 个字符
When 提交表单
Then 保存被拒绝，提示"项目名称不能超过 100 字符"

Given 项目 X 下有 3 个需求
When 删除项目 X
Then 后端拒绝或提示需先处理关联需求，数据不被静默破坏
```

#### F002: 项目 Key 标识

**功能编号与名称：** F002 项目 Key 标识
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 为项目设定简短英文标识（如 `ORCA`），便于在任务编号、提交信息中引用
**前置条件：** 项目已创建
**主流程：**
1. 在项目创建表单或项目详情编辑态中填写「项目 Key」
2. 输入框限制最大 10 字符
3. 保存后 `key` 字段写入 `projects` 表并在详情页展示
**异常流程：**
- 输入超过 10 字符 → 输入框直接截断（`maxLength`），无法提交超长值
- Key 留空 → 允许保存（字段可空）
**边界条件：**
- 长度上限 10 字符
- 仅前端长度限制，**无格式校验（大小写/字符集未约束）**，**无唯一性校验**
**输入与输出：**
- 输入：`key`（字符串，可空）
- 输出：随 `Project` 对象返回
**依赖关系：** 依赖 F001；字段位于 `projects.key`
**约束条件：** 无
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目创建表单已打开
When 在 Key 输入框输入 11 个字符
Then 输入框最多接受 10 个字符

Given 某项目 Key 为空
When 查看项目详情
Then 页面不报错，Key 区域显示为空占位
```

#### F003: 项目 Skill 配置

**功能编号与名称：** F003 项目 Skill 配置
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 为项目绑定可用的 PRD Skill 模板，并指定默认模板
**前置条件：** 项目已创建；`prd_skills` 表中已有 Skill（系统预置 4 个内置模板）
**主流程：**
1. 进入项目详情页 → 「Skill 配置」Tab
2. 调用 `list_project_skills` 展示已绑定 Skill（含 `skill_name`、`skill_type`、`is_default`）
3. 点击「添加」选择 `prd_skills` 中的 Skill，调用 `add_project_skill` 写入 `project_skills`
4. 可设置某个绑定为默认（`is_default`），默认 Skill 将作为该项目 PRD 生成的兜底模板
5. 点击「移除」调用 `remove_project_skill` 解除绑定
**异常流程：**
- 重复绑定同一 Skill → **不会报错**（`project_skills` 无 `UNIQUE(project_id, skill_id)`），会产生重复行
- 绑定的 Skill 已被软删除（`is_deleted=1`）→ 列表仍会显示该记录
- 移除不存在的绑定 → 返回空影响，前端无感知
**边界条件：**
- 一个项目可绑定 0 – N 个 Skill
- 默认 Skill 数量未在数据库层强制唯一，可能并存多个 `is_default=1`
**输入与输出：**
- 输入：`project_id`、`skill_id`、`is_default`、`custom_params`
- 输出：`ProjectSkill[]`（含联表查询的 `skill_name`/`skill_type`/`skill_description`）
**依赖关系：** 依赖 `ProjectService`（项目 Skill 增删查）、`tauri.ts` 的 `listProjectSkills` / `addProjectSkill` / `removeProjectSkill`、`project_skills` 表与 `prd_skills` 表
**约束条件：** `project_id` 与 `skill_id` 均为外键；删除 Skill 时不会自动清理绑定关系
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目 P 未绑定任何 Skill
When 在「Skill 配置」Tab 添加内置 Skill「功能需求 PRD」并设为默认
Then project_skills 新增 1 行（is_default=1），列表展示该 Skill

Given 项目 P 已绑定 Skill S
When 点击 S 的「移除」
Then project_skills 中对应行被删除，列表不再显示 S
```

#### F004: 项目 MCP 配置

**功能编号与名称：** F004 项目 MCP 配置
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 为项目登记 MCP（Model Context Protocol）服务端点及其认证信息
**前置条件：** 项目已创建
**主流程：**
1. 进入项目详情页 → 「MCP 配置」Tab
2. 调用 `list_project_mcps` 展示已配置的 MCP 列表
3. 点击「添加 MCP」，填写名称、端点 URL、描述、认证配置（`auth_config`）、是否默认
4. 调用 `add_project_mcp` 写入 `project_mcps`
5. 点击「移除」调用 `remove_project_mcp` 解除配置
**异常流程：**
- 名称为空或端点为空 → 后端拒绝（两字段为 `NOT NULL`）
- 端点格式非法 → **无格式校验**，按原样存储
**边界条件：**
- 一个项目可配置 0 – N 个 MCP
- `is_default` 默认 1，未在数据库层强制唯一
- `auth_config` 为文本字段，实际存储结构化信息（JSON 字符串），后端不校验其结构
**输入与输出：**
- 输入：`project_id`、`mcp_name`、`endpoint`、`description`、`is_default`、`auth_config`、`custom_params`
- 输出：`ProjectMcp[]`
**依赖关系：** 依赖 `ProjectService`、`listProjectMcps` / `addProjectMcp` / `removeProjectMcp`、`project_mcps` 表
**约束条件：** `project_id` 外键指向 `projects(id)`
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目 P 未配置 MCP
When 添加 mcp_name="context7"、endpoint="https://mcp.example.com/sse"
Then project_mcps 新增 1 行，列表显示该 MCP

Given 已配置 MCP M
When 点击「移除」
Then project_mcps 中对应行被删除
```

#### F005: 流水线环节配置

**功能编号与名称：** F005 流水线环节配置
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 为研发流程的 6 个环节分别指定默认 Skill，实现"按环节自动选用模板"
**前置条件：** 项目已创建；`prd_skills` 中已有可用 Skill
**主流程：**
1. 调用 `get_pipeline_stages` 获取 6 个环节选项（`value` + `label`）
2. 进入项目详情页「流水线配置」区域，调用 `list_stage_skill_configs` 展示各环节当前绑定
3. 为某个环节选择 Skill 并保存 → 调用 `upsert_stage_skill_config`
4. 后端依赖唯一索引 `idx_stage_skill_unique(project_id, stage)` 执行 `ON CONFLICT DO UPDATE`，实现"存在则更新、否则插入"
5. 点击清除 → 调用 `remove_stage_skill_config` 删除该环节配置
**异常流程：**
- 传入非法 stage 值 → `PipelineStage::from_str` 解析失败，返回错误
- 同一环节重复提交 → 由唯一索引兜底为更新，不产生重复行
- 唯一索引缺失（旧库未迁移）→ upsert 会失败，需重新初始化
**边界条件：**
- 每个项目每个环节**最多 1 条**配置（唯一索引保证）
- 6 个环节可分别配置，也可只配置部分环节
- `stage_skill_configs.stage` 列**无 CHECK 约束**，合法性由后端枚举解析保证
**输入与输出：**
- 输入：`project_id`、`stage`、`skill_id`、`is_default`、`custom_params`
- 输出：`StageSkillConfig[]` / `PipelineStageOption[]`
**依赖关系：** 依赖 `ProjectService`、`getPipelineStages` / `listStageSkillConfigs` / `getStageSkillConfig` / `upsertStageSkillConfig` / `removeStageSkillConfig`、`stage_skill_configs` 表
**约束条件：** `stage` 取值必须属于 `PipelineStage` 枚举（`requirement_analysis`、`prd_generation`、`task_planning`、`code_generation`、`testing`、`review`）
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目 P 的 prd_generation 环节未配置 Skill
When 选择「功能需求 PRD」并保存
Then stage_skill_configs 新增 1 行（project_id=P, stage=prd_generation）

Given 项目 P 的 prd_generation 环节已配置 Skill A
When 改选 Skill B 并保存
Then 该行 skill_id 更新为 B，表中不出现重复行（唯一索引生效）

Given stage 传入 "invalid_stage"
When 调用 upsert_stage_skill_config
Then 后端返回错误，不写入数据
```

---

### 模块 2：需求管理

#### F006: 需求 CRUD

**功能编号与名称：** F006 需求 CRUD
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 录入与维护产品需求，作为任务拆分与 PRD 生成的上游
**前置条件：** 至少存在一个项目（需求必须归属项目）
**主流程：**
1. 进入 `/requirements` 需求列表页，调用 `list_requirements`（分页）获取数据
2. 列表支持按状态筛选、分页浏览（`PagedResult`：`items`/`total`/`page`/`page_size`/`total_pages`）
3. 点击「新建需求」打开 `RequirementForm` 弹窗，填写标题、描述、优先级、所属项目等
4. 提交调用 `create_requirement`，写入 `requirements` 表（扩展字段通过二次 UPDATE 补齐）
5. 点击需求进入 `/requirements/:id` 详情页，可编辑并调用 `update_requirement`
6. 点击删除调用 `delete_requirement`，并级联清理关联记录
**异常流程：**
- 标题为空 → 前端校验拦截
- 标题超过 200 字符 → 后端 `validate_requirement` 拒绝
- 优先级不在 `P0~P3` → 后端校验拒绝（`AppError::Validation`）
- 项目不存在 → 外键约束失败（`foreign_keys=ON`）
- 列表读取失败 → 页面显示错误态
**边界条件：**
- 标题长度：1 – 200 字符
- 优先级：`P0` / `P1` / `P2` / `P3`（默认 `P1`）
- 初始状态固定为 `draft`
- 分页参数上限：`page_size` 最大 100（`PaginationParams.limit()`）
**输入与输出：**
- 输入：`title`（必填）、`description`、`priority`、`project_id`、`module`、`source`、`source_detail`、`product_owner`、`developer`、`tester`、`business_priority`、`estimated_effort`、`tags`、`notes`
- 输出：`Requirement` 对象；列表输出 `PagedResult<Requirement>`
**依赖关系：** 依赖 `useRequirementsStore`（`fetchRequirements`/`addRequirement`/`removeRequirement`）、`RequirementService`（30s 结果缓存 + 写后失效）、`requirements` 表
**约束条件：** `project_id` 为 `NOT NULL` 外键；标题非空；`tags` 字段注释声称 JSON 数组，**实际按纯字符串存储**，未做序列化校验
**状态变化：** 新建即为 `draft`；状态流转见 F007
**权限控制：** 无（单用户）
**验收标准：**
```
Given 已有项目"默认项目"
When 提交标题="用户登录优化"、优先级=P0 的新需求
Then requirements 新增 1 行，status=draft，列表刷新可见

Given 标题为空
When 提交表单
Then 保存被拦截并提示"标题不能为空"

Given 已有 120 条需求，page_size=50
When 请求第 1 页
Then 返回 items 长度 50、total=120、total_pages=3
```

#### F007: 需求状态流转

**功能编号与名称：** F007 需求状态流转
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 按评审与开发节奏推进需求状态，并用校验保证流转合法
**前置条件：** 需求已创建
**主流程：**
1. 在需求详情页选择目标状态并保存
2. 前端调用 `update_requirement`（携带 `status`）
3. 后端先执行 `validate_requirement_status_transition(current, next)`
4. 校验通过则更新 `requirements.status`，并失效相关缓存（单条 + 列表 + 统计）
**异常流程：**
- 非法流转（如 `draft → completed`）→ 后端返回校验错误，状态保持原值
- 状态值与枚举不符 → 校验失败
**边界条件：**
- 合法流转路径（唯一权威定义见 `validation.rs`）：
  - `draft → reviewing`
  - `reviewing → approved | rejected`
  - `approved → frozen | in_development`
  - `rejected → draft`
  - `frozen → draft | in_development`
  - `in_development → testing`
  - `testing → completed | in_development`
  - `completed →` （终态，不可再流转）
**输入与输出：**
- 输入：`id`、`status`（以及可选的 `title`/`description`/`priority`/`prd_content`）
- 输出：`Result<(), String>`（无返回体）
**依赖关系：** 依赖 `update_requirement` 命令、`validation.rs::validate_requirement_status_transition`、`RequirementService`
**约束条件：** 状态取值受 `requirements.status` 的 SQL `CHECK` 约束限制为 8 个枚举值
**状态变化：** 8 状态机，见「八、状态机汇总」
**权限控制：** 无（单用户）
**验收标准：**
```
Given 需求状态为 draft
When 将状态改为 completed
Then 后端返回校验错误，数据库 status 仍为 draft

Given 需求状态为 in_development
When 将状态改为 testing
Then 更新成功，数据库 status=testing

Given 需求状态为 completed
When 尝试改为任意其它状态
Then 被拒绝（终态）
```

#### F008: 需求扩展字段

**功能编号与名称：** F008 需求扩展字段
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 记录需求的业务属性（模块、干系人、来源、工作量、附件说明等）
**前置条件：** 需求已创建或正在创建
**主流程：**
1. 在 `RequirementForm` 中填写扩展字段（分组展示：基本信息 / 归属信息 / 业务信息 / 其它）
2. 提交时随 `create_requirement` 一并传递
3. 后端先插入主记录，再以二次 `UPDATE` 补齐迁移新增的扩展列
4. 详情页按分组展示已填写的扩展字段
**异常流程：**
- `estimated_effort` 为负数 → 前端校验拦截
- `estimated_effort` 超过 1000 人天 → 前端校验拦截
- 扩展字段全部留空 → 允许保存
**边界条件：**
- 可空字段，未填写则为 `NULL`
- `estimated_effort` / `actual_effort` 为 `REAL` 类型
- `attachments` 为文本字段（用于存储附件路径/说明，**无文件上传能力**）
- 扩展字段在旧库中由 `migrate_tables()` 以 `ALTER TABLE ADD COLUMN` 动态补齐
**输入与输出：**
- 输入：`module`、`source`、`source_detail`、`product_owner`、`developer`、`tester`、`business_priority`、`estimated_effort`、`actual_effort`、`tags`、`notes`、`attachments`、`requirement_type`、`target_version`、`due_date`
- 输出：随 `Requirement` 对象返回
**依赖关系：** 依赖 `components/RequirementForm.tsx`、`create_requirement` / `update_requirement`、`requirements` 表扩展列
**约束条件：**
- `requirement_type` 约定取值：`feature` / `bug` / `optimization` / `tech_debt`（数据库默认 `feature`，**无 CHECK**）
- `source` 约定取值：`product` / `tech` / `customer` / `internal`（无 CHECK）
- `business_priority` 约定取值：`critical` / `high` / `medium` / `low`（无 CHECK）
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 新建需求
When 填写 module="认证模块"、developer="张三"、estimated_effort=3.5
Then 保存成功后详情页展示这三个字段的填写值
```

#### F009: PRD 自动生成

**功能编号与名称：** F009 PRD 自动生成
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 依据需求标题与描述，用 Claude API 生成结构化 PRD 文档
**前置条件：**
- 已在设置中配置 Claude API Key（`settings.claude_api_key`）
- 需求已存在（`requirement_id` 有效）
**主流程：**
1. 在需求详情页点击「生成 PRD」
2. 可选选择一个 Skill 模板（`skill_id`）；未选时后端按"项目默认 Skill → 内置默认模板"顺序解析
3. `PrdService` 取需求 → 解析 Skill 模板 → 读取 `claude_api_key` → 构造 `ClaudeClient`
4. 调用 `ClaudeClient::generate_prd`（`POST https://api.anthropic.com/v1/messages`，`max_tokens=4096`，超时 120s/连接 10s）
5. 模板变量 `{title}` / `{description}` / `{priority}` 完成字符串替换；无模板时使用内置 6 章节提示词
6. 将返回正文写入 `requirements.prd_content`
7. 记录 `token_usage`（`total_cost=0`）与 `skill_execution_logs`（`execution_phase='prd_generation'`、`status='success'`）
8. 前端展示 PRD 正文，支持重新生成与复制
**异常流程：**
- 未配置 API Key → 返回错误"未配置 Claude API Key"
- 需求不存在 → 返回错误
- API 返回非 2xx 或超时 → 返回错误信息，`prd_content` 保持原值
- Skill 模板缺失指定变量 → 变量保留原样，不报错
- Skill 已被软删除 → 仍可被解析使用（仅按 `id` 查询）
**边界条件：**
- `max_tokens` 固定 4096，超长需求描述会被截断/受模型输出限制
- 单次生成会写入一条 Token 记录，`total_cost` 恒为 0（未做价格换算）
- Skill 模板类型 `skill_type` 当前**硬编码写入 `"feature"`**
**输入与输出：**
- 输入：`requirement_id`（必填）、`skill_id`（可选）
- 输出：PRD 正文字符串（同时持久化到 `requirements.prd_content`）
**依赖关系：** 依赖 `PrdService`、`ClaudeClient::generate_prd`、`PrdSkillService`、`settings` 表、`requirements`/`token_usage`/`skill_execution_logs` 表
**约束条件：** 需要有效的 Claude API Key；模型固定为 `claude-sonnet-5-20250514`（源码硬编码），**不可配置**；无重试机制
**状态变化：** 写入 `requirements.prd_content`；新增 `token_usage` 与 `skill_execution_logs` 记录；不改变需求 `status`
**权限控制：** 无（单用户）
**验收标准：**
```
Given 已配置有效 API Key，需求 #1 存在
When 点击「生成 PRD」
Then prd_content 被写入内容，token_usage 新增 1 行，skill_execution_logs 新增 1 行

Given 未配置 API Key
When 点击「生成 PRD」
Then 提示"未配置 Claude API Key"，不产生任何记录

Given 传入 skill_id="skill-bugfix"
When 生成 PRD
Then 使用该 Skill 的模板进行生成
```

#### F010: 需求字段自动完善

**功能编号与名称：** F010 需求字段自动完善
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 由 LLM 分析需求标题与描述，自动推断模块、标签、验收标准、业务优先级与工作量
**前置条件：** 已配置 Claude API Key；需求已存在且至少有标题
**主流程：**
1. 在需求详情页点击「智能完善」
2. 后端 `enrich_requirement` 读取需求标题与描述，构造提示词（要求输出 JSON）
3. 调用 `ClaudeClient::enrich_requirement`（`max_tokens=1024`）
4. 解析返回 JSON，字段包括 `module`、`tags`、`acceptance_criteria`、`business_priority`、`estimated_effort`、`requirement_type`、`source`
5. 返回 `serde_json::Value` 给前端，前端将这些建议值回填到表单中供用户确认
**异常流程：**
- 模型未输出合法 JSON → 后端截取首个 `{` 到末个 `}` 之间的内容重试解析
- 二次解析仍失败 → 返回**默认 JSON**（不报错）
- 未配置 API Key → 返回错误
**边界条件：**
- `max_tokens` 1024，字段过多时可能被截断
- 仅返回建议值，**是否落库由用户在前端确认后另行提交**
**输入与输出：**
- 输入：`requirement_id`
- 输出：`serde_json::Value`（结构化建议字段）
**依赖关系：** 依赖 `commands.rs::enrich_requirement`、`ClaudeClient::enrich_requirement`、`settings.claude_api_key`
**约束条件：**
- 约定枚举：`requirement_type` ∈ {`feature`,`bug`,`optimization`,`tech_debt`}，`business_priority` ∈ {`critical`,`high`,`medium`,`low`}，`source` ∈ {`product`,`tech`,`customer`,`internal`}
- **该调用不写入 `token_usage` 表**（与 F009 不同）
**状态变化：** 本身不改变数据库；用户确认提交后由 `update_requirement` 落库
**权限控制：** 无（单用户）
**验收标准：**
```
Given 需求描述包含"登录页密码校验缺失"
When 点击「智能完善」
Then 返回 JSON 建议，其中 module/tags/business_priority 等字段有值

Given 未配置 API Key
When 点击「智能完善」
Then 提示未配置 Key，前端不产生变更
```

#### F011: 需求变更历史

**功能编号与名称：** F011 需求变更历史
**实现状态：** ❌ 未实现（表已建，无写入路径）
**目标用户与角色：** 单人开发者
**使用场景：** 审计需求的创建、更新、状态变更与删除操作，支持事后追溯
**前置条件：** 无（功能尚未生效）
**主流程（设计意图）：**
1. 需求发生 `create` / `update` / `status_change` / `delete` 时，由 `RequirementService` 写入 `requirement_history`
2. 记录 `requirement_id`、`action`、`old_value`、`new_value`、`created_at`
3. 审计页面通过 `get_audit_logs` 读取并展示
**异常流程：** —
**边界条件：**
- **当前代码库中不存在任何 `INSERT INTO requirement_history` 语句**，该表恒为空
- 因此 F053/F054/F055（审计日志相关）实际均不可用
**输入与输出：**
- 输入：（设计）`requirement_id`、`action`、`old_value`、`new_value`
- 输出：（设计）变更记录列表
**依赖关系：** `requirement_history` 表（已建表 + `idx_requirement_history_req` 索引）、`RequirementService`（**缺写入逻辑**）
**约束条件：** 表结构与索引已就绪，仅缺写入链路
**状态变化：** 当前无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 需求 #1 从 draft 改为 reviewing
When 查询审计日志
Then 期望出现 1 条 status_change 记录（当前实际为空）

Given 需求 #1 被编辑标题
When 查询 requirement_history 表
Then 期望新增 1 条 update 记录（当前实际为 0 行）
```

#### F012: 需求表单输入校验

**功能编号与名称：** F012 需求表单输入校验
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 在提交前拦截非法输入，降低后端校验失败带来的交互成本
**前置条件：** 需求表单弹窗已打开
**主流程：**
1. 用户输入时/失焦时执行本地 `validate()`
2. 校验失败则记录 `errors` 并禁用保存按钮
3. 校验通过后允许提交
**异常流程：** —
**边界条件（`components/RequirementForm.tsx` 实测规则）：**

| 字段 | 规则 | 提示文案 |
|------|------|----------|
| `title` | 去空格后不能为空 | 标题不能为空 |
| `title` | 长度 ≤ 200（输入框 `maxLength={200}`） | 标题不能超过 200 字（当前 N） |
| `estimated_effort` | 若填写则为非负数 | 预估工作量必须为非负数 |
| `estimated_effort` | ≤ 1000 人天（input `min=0` `step=0.5`） | 预估工作量不能超过 1000 人天 |

**输入与输出：**
- 输入：表单字段值
- 输出：`errors` 对象；保存按钮可用性
**依赖关系：** 依赖 `components/RequirementForm.tsx`（纯前端，无后端调用）
**约束条件：** 后端另有独立校验（标题 ≤ 200、优先级枚举），前端校验**不替代**后端校验
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 需求表单已打开且标题为空
When 观察保存按钮
Then 按钮为禁用状态

Given 预估工作量输入 -1
When 触发表单校验
Then 显示"预估工作量必须为非负数"，禁止提交

Given 预估工作量输入 1200
When 触发表单校验
Then 显示"预估工作量不能超过 1000 人天"
```

---

### 模块 3：任务管理

#### F013: 任务 CRUD

**功能编号与名称：** F013 任务 CRUD
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 将需求拆解为可执行的开发/测试/修复/评审任务
**前置条件：** 应用已启动（任务可独立于需求存在，`requirement_id` 可空）
**主流程：**
1. 进入 `/board` 任务看板，或从需求详情页/项目详情页进入任务区域
2. 点击「新建任务」打开 `TaskForm`，填写标题、类型、优先级等
3. 提交调用 `create_task` 写入 `tasks` 表
4. 在看板中可拖拽/操作更新状态；在 `/tasks/:id` 详情页可编辑（`update_task`）与删除（`delete_task`）
5. `list_tasks` 获取列表（当前**未分页**）
**异常流程：**
- 标题为空或超过 200 字符 → 后端 `validate_task` 拒绝
- `task_type` / `priority` 不在枚举内 → 校验拒绝
- 关联的 `requirement_id` 不存在 → 外键约束失败
**边界条件：**
- 标题长度：1 – 200 字符
- 任务类型：`requirement` / `development` / `testing` / `bug_fix` / `review`（默认 `development`）
- 优先级：`P0` – `P3`（默认 `P1`）
- 初始状态 `pending`
- **任务列表未使用分页**（仅需求列表分页）
**输入与输出：**
- 输入：`title`（必填）、`task_type`、`priority`、`requirement_id`、`assignee`、`due_date`、`estimated_hours`、`priority_level`、`blocked_reason`、`tags`、`notes`；编辑时另有 `description`、`project_id`
- 输出：`Task` 对象
**依赖关系：** 依赖 `useTasksStore`（`fetchTasks`/`addTask`/`updateStatus`/`removeTask`）、`TaskService`（30s 列表缓存 + 写后失效）、`tasks` 表
**约束条件：** `requirement_id` 外键指向 `requirements(id)`；`tasks.project_id` 为迁移新增字段，**无外键约束**
**状态变化：** 新建即为 `pending`；状态机见 F014
**权限控制：** 无（单用户）
**验收标准：**
```
Given 已有需求 #1
When 创建任务标题="实现登录接口"、类型=development、关联需求 #1
Then tasks 新增 1 行，status=pending

Given 任务类型传入 "invalid_type"
When 调用 create_task
Then 后端返回校验错误，不写入数据

Given 任务标题输入 201 字符
When 提交表单
Then 被拒绝并提示标题超长
```

#### F014: 任务状态管理

**功能编号与名称：** F014 任务状态管理
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 推进任务执行状态；进入执行态时可联动自动启动 Agent
**前置条件：** 任务已创建
**主流程：**
1. 在看板或任务详情页修改任务状态
2. 调用 `update_task_status(id, status)`
3. 后端校验状态流转合法性后更新 `tasks.status`
4. **联动逻辑**：当新状态为 `in_progress` 且系统设置 `settings.session_mode == 'auto'` 时，`TaskService` 自动调用 `start_agent_from_task` 启动 Agent 并创建会话
5. 缓存失效：`tasks:list:` 前缀 + 统计数据
**异常流程：**
- 非法流转（如 `pending → completed`）→ 校验失败，状态不变
- 自动启动 Agent 失败 → 任务状态仍更新成功，Agent 启动失败由日志体现（**不阻塞状态更新**）
**边界条件：**
- 合法流转路径：
  - `pending → in_progress | blocked`
  - `in_progress → completed | blocked | pending`
  - `blocked → pending | in_progress`
  - `completed → pending`
- `session_mode` 默认值 `auto`（`task_service.rs` 兜底），即在未显式配置时**默认会尝试自动启动 Agent**
**输入与输出：**
- 输入：`id`、`status`
- 输出：`Result<(), String>`
**依赖关系：** 依赖 `update_task_status`、`validation.rs::validate_task_status_transition`、`TaskService`、`AgentService`（自动启动链路）、`settings.session_mode`
**约束条件：** `tasks.status` 受 SQL `CHECK` 约束限制为 4 个枚举值
**状态变化：** 4 状态机，见「八、状态机汇总」；可能级联触发"新建 `agent_sessions` 记录"
**权限控制：** 无（单用户）
**验收标准：**
```
Given 任务状态为 pending
When 改为 completed
Then 校验失败，状态仍为 pending

Given 任务状态为 pending，且 session_mode='auto'
When 改为 in_progress
Then 状态更新成功，且系统尝试创建一个 agent_session

Given 任务状态为 in_progress，且 session_mode='manual'
When 改为 in_progress（无变化）
Then 不触发自动启动
```

#### F015: 任务评论

**功能编号与名称：** F015 任务评论
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 在任务下记录执行备注、阻塞原因、决策过程
**前置条件：** 任务已存在
**主流程：**
1. 进入 `/tasks/:id` 详情页，滚动到评论区
2. 调用 `list_task_comments` 加载该任务的全部评论（按创建时间排序）
3. 输入内容并提交 → 调用 `create_task_comment`（`task_id`、`author`、`content`）
4. 删除评论 → 调用 `delete_task_comment`
**异常流程：**
- 内容为空 → 后端 `content NOT NULL`，前端应做非空拦截
- 任务不存在 → 外键约束失败
- 评论不存在 → 删除操作影响 0 行，前端无感知
**边界条件：**
- `author` 默认 `user`（单用户场景前端可传固定值）
- 内容无长度上限约束（数据库为 `TEXT`）
- 评论随任务删除而级联删除（`ON DELETE CASCADE`）
**输入与输出：**
- 输入：`task_id`、`author`、`content`
- 输出：`TaskComment` / `TaskComment[]`
**依赖关系：** 依赖 `createTaskComment` / `listTaskComments` / `deleteTaskComment`、`task_comments` 表（含 `idx_task_comments_task`、`idx_task_comments_created` 索引）
**约束条件：** `task_id` 外键 `ON DELETE CASCADE`
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 任务 #1 存在
When 添加评论"已定位到密码校验缺失"
Then task_comments 新增 1 行，评论区立即显示该条

Given 任务 #1 被删除
When 查询 task_comments
Then 该任务的所有评论已被级联删除

Given 选中一条评论
When 点击删除
Then 该行从数据库与列表中移除
```

#### F016: 任务按项目/需求筛选

**功能编号与名称：** F016 任务按项目/需求筛选
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 在项目详情页或需求详情页查看归属该上下文的任务清单
**前置条件：** 项目或需求已存在且已关联任务
**主流程：**
1. 在项目详情页的「任务」Tab 调用 `list_tasks_by_project(project_id)`
2. 在需求详情页的任务区域调用 `list_tasks_by_requirement(requirement_id)`
3. 后端按外键字段过滤返回任务列表
**异常流程：**
- 传入不存在的 id → 返回空数组（不报错）
- 任务未关联项目（`project_id` 为空）→ 不会被项目维度筛选命中
**边界条件：**
- 项目维度筛选依据 `tasks.project_id`（迁移新增列，**无外键**），历史数据可能为空
- 需求维度筛选依据 `tasks.requirement_id`（有外键）
- 两条件为"或"关系而非"与"，互不叠加
**输入与输出：**
- 输入：`project_id` 或 `requirement_id`
- 输出：`Task[]`
**依赖关系：** 依赖 `listTasksByProject` / `listTasksByRequirement`、`tasks` 表（`idx_tasks_requirement` 索引）
**约束条件：** `tasks.project_id` 无索引（`idx_tasks_project` 仅存在于未被程序加载的 SQL 文件中，运行时不存在）
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目 P 下有 3 个任务、其它项目下有 5 个任务
When 进入项目 P 的任务 Tab
Then 只显示 3 个任务

Given 需求 R 关联 2 个任务
When 进入需求 R 详情页
Then 任务区域显示这 2 个任务
```

#### F017: 任务扩展字段

**功能编号与名称：** F017 任务扩展字段
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 记录任务的执行人、工时、截止日期、阻塞原因、Skill/MCP 覆盖配置等
**前置条件：** 任务已创建或正在创建
**主流程：**
1. 在 `TaskForm` 中填写扩展字段
2. 提交随 `create_task` / `update_task` 传递
3. 后端写入 `tasks` 表对应列（旧库由 `migrate_tables()` 的 15 个 `ADD COLUMN` 补齐）
**异常流程：**
- `estimated_hours` 为负或超过 10000 → 前端校验拦截
- `due_date` 早于今天 → 前端校验拦截
**边界条件：**
- `estimated_hours` / `actual_hours` 为 `REAL`
- `skill_config` / `mcp_config` 为文本字段，用于任务级覆盖项目级配置（**结构未做校验**）
- `blocked_reason` 通常在状态置为 `blocked` 时填写
- `priority_level` 约定取值：`critical` / `high` / `medium` / `low`（无 CHECK）
**输入与输出：**
- 输入：`assignee`、`due_date`、`tags`、`estimated_hours`、`actual_hours`、`blocked_reason`、`priority_level`、`notes`、`attachments`、`project_id`、`skill_config`、`mcp_config`
- 输出：随 `Task` 对象返回
**依赖关系：** 依赖 `components/TaskForm.tsx`、`create_task` / `update_task`、`tasks` 表扩展列
**约束条件：** 扩展字段均可为空
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 新建任务
When 填写 assignee="张三"、estimated_hours=6、due_date=明天
Then 保存成功，详情页正确展示这些字段

Given 任务状态置为 blocked
When 填写 blocked_reason="等待接口联调"
Then 该原因持久化并在详情页可见
```

#### F018: 任务表单输入校验

**功能编号与名称：** F018 任务表单输入校验
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 任务提交前的本地校验
**前置条件：** 任务表单弹窗已打开
**主流程：** 同 F012，`validate()` 通过后才允许提交
**异常流程：** —
**边界条件（`components/TaskForm.tsx` 实测规则）：**

| 字段 | 规则 | 提示文案 |
|------|------|----------|
| `title` | 去空格后不能为空 | 标题不能为空 |
| `title` | 长度 ≤ 200（`maxLength={200}`） | 标题不能超过 200 字（当前 N） |
| `estimated_hours` | 若填写则为非负数 | 预估工时必须为非负数 |
| `estimated_hours` | ≤ 10000 小时（`min=0` `step=0.5`） | 预估工时不能超过 10000 小时 |
| `due_date` | 若填写则不能早于今天（按 00:00 归一化比较） | 截止日期不能早于今天 |

**输入与输出：**
- 输入：表单字段值
- 输出：`errors` 对象；保存按钮可用性
**依赖关系：** 依赖 `components/TaskForm.tsx`（纯前端）
**约束条件：** 后端独立校验标题 ≤ 200、`task_type`/`priority` 枚举
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 截止日期选择昨天
When 触发表单校验
Then 显示"截止日期不能早于今天"并禁止提交

Given 预估工时输入 20000
When 触发表单校验
Then 显示"预估工时不能超过 10000 小时"
```

---

### 模块 4：Agent 管理

#### F019: Agent 自动检测

**功能编号与名称：** F019 Agent 自动检测
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 自动发现本机已安装的 AI Agent（claude、codex 等），无需手工填写路径
**前置条件：** 应用已启动
**主流程：**
1. 进入 `/agents` 页面，调用 `get_agents` 读取数据库中已注册 Agent
2. 点击「刷新检测」调用 `discover_agents`
3. 后端扫描 PATH 及 Windows 常见安装路径（npm global、nvm-windows、scoop、AppData 等）
4. 对每个命中项执行 `--help` 探测能力（`probe_capabilities`），解析版本号
5. 通过 `upsert_agent` 以 `binary_path` 为逻辑唯一键写入 `agents` 表（**注意：数据库层无 UNIQUE 约束，去重由应用层 SQL 实现**）
6. 返回检测结果并刷新列表
**异常流程：**
- 未检测到任何 Agent → 返回空数组，页面显示空状态
- 探测 `--help` 超时/失败 → 能力字段为空，不阻断注册
- 检测到的二进制路径已失效 → 仍会注册，状态判定见 F021
**边界条件：**
- 检测范围：PATH、npm global、nvm-windows、scoop、AppData（Windows 常见路径）
- 同一 `binary_path` 只会注册一条记录
- 检测为**幂等操作**，可重复触发
**输入与输出：**
- 输入：无
- 输出：`Agent[]`（`id`、`name`、`display_name`、`binary_path`、`version`、`status`、`capabilities`）
**依赖关系：** 依赖 `agent.rs::discover_agents`（含 PATH 与路径扫描、能力探测）、`AgentService`、`agents` 表、`useState` 组件状态（**Agents 页面未使用 store**）
**约束条件：** 依赖本机已安装对应 CLI 工具；扫描性能受目录数量影响
**状态变化：** 可能新增或更新 `agents` 记录
**权限控制：** 无（单用户）
**验收标准：**
```
Given 本机已安装 claude CLI
When 点击「刷新检测」
Then agents 表出现该 Agent 记录（name=claude、binary_path 正确）

Given 重复点击「刷新检测」3 次
When 查询 agents 表
Then 同一 binary_path 只有 1 行记录（不重复插入）

Given 本机未安装任何 Agent
When 点击「刷新检测」
Then 返回空列表，页面显示空状态提示
```

#### F020: Agent 版本解析

**功能编号与名称：** F020 Agent 版本解析
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 从 Agent 命令输出中提取标准版本号，用于展示与兼容性判断
**前置条件：** Agent 已被检测到
**主流程：**
1. 检测阶段执行 Agent 的版本命令（如 `--version`）
2. 用正则匹配多种版本格式：`v2.1.266`、`2.1.266`、`version 2.1.266`
3. 提取版本号写入 `agents.version`
4. 前端在 Agent 卡片中展示版本
**异常流程：**
- 输出中无版本号 → `version` 为 `NULL`，前端显示"—"
- 版本命令不存在 → 视为无法解析
**边界条件：**
- 支持带 `v` 前缀、纯数字、带 `version` 前缀三种写法
- 版本号按字符串存储，**不做语义化比较**
**输入与输出：**
- 输入：Agent 版本命令的 stdout
- 输出：版本字符串
**依赖关系：** 依赖 `agent.rs` 的版本正则解析逻辑、`agents.version` 字段
**约束条件：** 无
**状态变化：** 更新 `agents.version`
**权限控制：** 无（单用户）
**验收标准：**
```
Given Agent 输出 "v2.1.266"
When 执行版本解析
Then version 字段为 "2.1.266"

Given Agent 输出 "claude version 2.1.266"
When 执行版本解析
Then version 字段为 "2.1.266"

Given Agent 输出无任何版本号
When 执行版本解析
Then version 为空，页面显示"—"
```

#### F021: Agent 状态管理

**功能编号与名称：** F021 Agent 状态管理
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 区分 Agent 可用、运行中、不可用三种状态
**前置条件：** Agent 已注册
**主流程：**
1. `agents.status` 取值 `available` / `running` / `unavailable`（SQL `CHECK` 约束，默认 `available`）
2. 检测时若二进制可用则置 `available`
3. 启动 Agent 时可将状态置为 `running`
**异常流程：**
- 二进制被删除/移动 → **无自动失效检测**，状态可能长期停留在 `available`
- 状态无自动回滚机制（进程崩溃不会自动置回 `available`）
**边界条件：**
- 三个枚举值受数据库 `CHECK` 约束
- **页面卸载后检测结果丢失**：`Agents.tsx` 使用组件内 `useState` 而非 store，`loadData()` 只读数据库；若检测结果未落库则重新进入页面会显示空
**输入与输出：**
- 输入：无（由检测/启动流程驱动）
- 输出：`Agent.status`
**依赖关系：** 依赖 `agents` 表、`get_agents`、`AgentService`
**约束条件：** 缺少"二进制有效性复核"与"进程存活对账"机制
**状态变化：** `available` ⇄ `running`；`unavailable` 依赖人工/检测流程设置
**权限控制：** 无（单用户）
**验收标准：**
```
Given Agent A 状态为 available
When 启动该 Agent
Then 状态更新为 running

Given Agent A 的二进制文件已被删除
When 重新进入 Agent 页面
Then 期望状态变为 unavailable（当前不会自动变更，需人工核实）
```

#### F022: PTY 伪终端会话

**功能编号与名称：** F022 PTY 伪终端会话
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 为 Agent 进程分配真实伪终端，使其具备交互能力与 ANSI 输出
**前置条件：** Agent 已检测可用
**主流程：**
1. 调用 `pty_spawn(agent_path, work_dir, env_vars?)` 创建 PTY 会话
2. 后端 `PtyManager` 通过 `portable_pty::native_pty_system().openpty` 打开 PTY（初始 `rows=24, cols=80`）
3. 使用 `CommandBuilder` 设置可执行文件、工作目录、环境变量并 spawn 子进程
4. 通过 `master.try_clone_reader()` 获取读取端，启动 `tokio::task::spawn_blocking` 读取循环（4096 字节/次，`String::from_utf8_lossy`）
5. 返回 `session_id`
6. 内存中记录 `PtySessionInfo`（`id`、`agent_path`、`work_dir`、`status`、`started_at`、`exit_code`）
**异常流程：**
- Agent 路径不存在 → `spawn` 失败，返回错误
- `work_dir` 不存在 → spawn 失败
- PTY 打开失败 → 返回错误
**边界条件：**
- 初始终端尺寸固定 `24x80`，需前端 `pty_resize` 自适应
- 读取缓冲区 4096 字节，长行可能被分片
- **`PtySessionMeta` 仅保存元数据，不保存 `child` / `master` / `writer` 句柄**
**输入与输出：**
- 输入：`agent_path`、`work_dir`、`env_vars`（可选 `Vec<(String, String)>`）
- 输出：`session_id`
**依赖关系：** 依赖 `pty.rs::PtyManager::spawn_agent`、`PtyService`、`pty_spawn` 命令、`portable-pty`
**约束条件：** Windows 下依赖 ConPTY；`PtyManager` 状态由 `Arc<RwLock<HashMap>>` 持有，随进程退出而丢失
**状态变化：** 内存态会话记录新增，状态 `running`
**权限控制：** 无（单用户）
**验收标准：**
```
Given Agent 路径有效、工作目录存在
When 调用 pty_spawn
Then 返回非空 session_id，且 pty_get_sessions 中可见该会话

Given agent_path 指向不存在的文件
When 调用 pty_spawn
Then 返回错误信息，不产生会话记录
```

#### F023: Agent 终端 UI（xterm.js）

**功能编号与名称：** F023 Agent 终端 UI（xterm.js）
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 在应用内渲染终端输出，支持输入、搜索、最大化、导出日志
**前置条件：** 已存在 PTY 会话（或由组件初始化时创建）
**主流程：**
1. `components/AgentTerminal.tsx` 初始化 `@xterm/xterm` 实例并挂载到容器
2. 监听 `pty:event` 事件，将 `{type:"data"}` 载荷 `write()` 到终端
3. 用户键盘输入 → `onData` → 调用 `pty_write`
4. 容器尺寸变化 → 计算 `rows`/`cols` → 调用 `pty_resize`
5. 提供搜索（xterm search addon）、最大化、导出日志、停止（`pty_kill`）等操作
**异常流程：**
- 终端初始化失败 → 显示占位错误
- 会话已结束 → 继续输入无效
**边界条件（当前实现的已知限制）：**
- `pty_write` 后端**仅把输入回显为 `[input] ...` 文本事件，并未真正写入 PTY master / 子进程 stdin**（`master` writer 未持久化保存）
- `pty_resize` 后端**仅做会话存在性检查并打日志，未真正调整 PTY 尺寸**
- `pty_kill` 后端**仅将内存状态置为 `terminated` 并移除输入通道，未 kill 操作系统子进程**
**输入与输出：**
- 输入：键盘输入、尺寸变化、搜索关键词
- 输出：终端渲染内容
**依赖关系：** 依赖 `@xterm/xterm`、`components/AgentTerminal.tsx`、`pty_write` / `pty_resize` / `pty_kill`、事件 `pty:event`
**约束条件：** 依赖 WebView 渲染性能；大输出量受 scrollback 限制
**状态变化：** 无（不影响数据库）
**权限控制：** 无（单用户）
**验收标准：**
```
Given Agent 正在输出
When 观察终端面板
Then 输出实时渲染（延迟 < 100ms）

Given 在终端输入框输入文本并发送
When 观察 Agent 侧
Then 期望文本被 Agent 接收（当前仅回显，需补齐写入链路）

Given 在终端中按 Ctrl+F 搜索关键词
When 输入关键词
Then 匹配结果被高亮
```

#### F024: PTY 事件流

**功能编号与名称：** F024 PTY 事件流
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 把 PTY 的输出/退出/错误实时推送到前端，避免轮询
**前置条件：** PTY 会话已创建
**主流程：**
1. 后端读取循环将数据封装为 `PtyEvent` 并 `blocking_send` 到 `mpsc::channel(100)`
2. `main.rs` 中的转发任务消费通道并 `app_handle.emit("pty:event", &event)`
3. 前端注册 `listen("pty:event")` 回调，按 `type` 分发处理
4. 事件类型：
   - `{type:"data", session_id, data}` — 输出数据
   - `{type:"exit", session_id, exit_code}` — 进程退出
   - `{type:"error", session_id, error}` — 错误
**异常流程：**
- 通道满（100 条）→ `blocking_send` 阻塞读取循环，形成背压
- 前端未监听 → 事件被丢弃（无缓冲重放）
**边界条件：**
- 事件名固定为 `pty:event`（**不存在 `pty:data` 独立事件**，数据事件嵌套在 `pty:event` 的 `type` 字段中）
- 读取线程结束后**未回写 `agent_sessions` 的 `status`/`duration`/`tokens`**
- 读取线程结束后的状态更新代码块当前为空操作
**输入与输出：**
- 输入：PTY 读取循环产生的数据
- 输出：Tauri 事件 `pty:event`
**依赖关系：** 依赖 `pty.rs` 的 `PtyEvent` 序列化（`#[serde(tag = "type")]`）、`main.rs` 事件转发任务、前端 `listen`
**约束条件：** 单进程内存事件总线，应用重启后无历史事件
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 已创建 PTY 会话
When Agent 向 stdout 输出 "hello"
Then 前端收到 {type:"data", session_id, data:"hello..."} 事件

Given PTY 子进程退出（exit code 0）
When 事件转发执行
Then 前端收到 {type:"exit", session_id, exit_code:0}
```

#### F025: Agent 启动与会话创建

**功能编号与名称：** F025 Agent 启动与会话创建
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 从 Agent 页面或任务上下文启动 Agent，并自动创建会话记录
**前置条件：** Agent 已注册且状态为 `available`
**主流程：**
1. 在 `/agents` 页面选择 Agent 并（可选）关联任务，点击「启动」
2. 调用 `start_agent(agent_path, task_id?)`
3. 后端校验 Agent 可用性 → `upsert_agent` → 在 `agent_sessions` 创建会话记录
4. 调用 `pty_manager.spawn_agent` 启动 PTY 进程
5. 返回 `AgentSession` 对象，前端跳转/展示终端
6. 停止时调用 `stop_agent(session_id)`
**异常流程：**
- Agent 不可用 → 返回错误
- PTY 启动失败 → **仅打印 warn 日志，不向上返回错误**（会话记录仍会创建，形成"有会话无进程"状态）
- 停止会话 → 仅将 DB 会话置为 `completed` 且 `exit_code=0`，**不真正终止操作系统进程**
**边界条件：**
- `create_session_from_task(task_id)` 会取数据库中的**第一个 Agent** 作为执行者
- `agent_sessions.status` 的 SQL `CHECK` 仅允许 `running` / `completed` / `failed`（**不含 `created`/`launching`/`terminated`**）
- DB 会话**无 `work_dir` 字段**（`work_dir` 仅存在于 PTY 内存态）
**输入与输出：**
- 输入：`start_agent`：`agent_path`、`task_id?`；`stop_agent`：`session_id`；`create_session_from_task`：`task_id`
- 输出：`AgentSession`
**依赖关系：** 依赖 `AgentService`、`PtyManager`、`start_agent` / `stop_agent` / `create_session_from_task`、`agents` 与 `agent_sessions` 表
**约束条件：** 依赖 F022 的 PTY 能力；`work_dir` 未落库导致会话与工作目录无法在重启后关联
**状态变化：** 新增 `agent_sessions` 记录（`status='running'`）；停止后置 `completed`
**权限控制：** 无（单用户）
**验收标准：**
```
Given Agent A 状态为 available
When 调用 start_agent(agent_path=A, task_id=#12)
Then agent_sessions 新增 1 行（agent_id 对应 A，status=running），返回会话对象

Given 会话 S 正在运行
When 调用 stop_agent(S)
Then 会话 status=completed、exit_code=0

Given 任务 #12 存在，数据库中已有 Agent 记录
When 调用 create_session_from_task(task_id=#12)
Then 创建 1 条关联该任务的会话记录
```

---

### 模块 5：会话管理

#### F026: 会话列表

**功能编号与名称：** F026 会话列表
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 集中查看所有 Agent 会话，快速定位运行中/失败/已完成会话
**前置条件：** 至少存在一个会话（由 F025 启动 Agent 或 F028 从任务创建）
**主流程：**
1. 进入 `/sessions` 页面，调用 `get_agent_sessions` 获取会话列表
2. 列表展示会话 ID、Agent 名称、关联任务、状态、开始时间、时长、Token 消耗
3. 提供状态筛选（运行中 / 已完成 / 失败）
4. 运行中的会话可直接进入终端面板或点击停止
5. 点击会话进入 `/sessions/:id` 详情页
**异常流程：**
- 无会话记录 → 显示空状态
- 会话的 `agent_id` 对应 Agent 已被删除 → 列表仍展示，Agent 名称为空
**边界条件：**
- 会话状态受 SQL `CHECK` 限制为 `running` / `completed` / `failed`（**不含 `created`/`launching`/`terminated`**）
- 列表**未使用分页**（一次返回全部）
- `started_at` 默认 `CURRENT_TIMESTAMP`，`ended_at` 在停止时写入
**输入与输出：**
- 输入：（可选）`agent_id` 过滤；状态过滤在前端进行
- 输出：`AgentSession[]`（含联表查询的 `agent_name`、`task_title`）
**依赖关系：** 依赖 `useSessionsStore`（`fetchSessions`/`stopSession`）、`get_agent_sessions`、`agent_sessions` 表（`idx_agent_sessions_agent` 索引）
**约束条件：** `agent_id` 外键指向 `agents(id)`，`task_id` 外键指向 `tasks(id)`
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 数据库中有 5 条会话（2 运行中、2 完成、1 失败）
When 进入会话列表并按"运行中"筛选
Then 只显示 2 条运行中会话

Given 无任何会话记录
When 进入会话列表
Then 显示空状态提示，不报错
```

#### F027: 会话详情

**功能编号与名称：** F027 会话详情
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 查看一次 Agent 执行的完整上下文：元信息、Token 消耗、Skill/MCP 执行记录
**前置条件：** 会话记录已存在
**主流程：**
1. 从会话列表点击进入 `/sessions/:id`
2. 展示基本信息：Agent、关联任务/需求、状态、`started_at`、`ended_at`、`duration_seconds`、`exit_code`
3. 展示 Token 信息：`total_tokens`、`total_cost`
4. 若存在关联 `task_id`，调用 `get_skill_execution_logs` 与 `get_mcp_execution_logs` 展示执行日志摘要
5. 展示 `skills` / `mcps` / `skill_execution_summary` / `mcp_execution_summary` 字段内容
**异常流程：**
- 会话不存在 → 显示错误提示并允许返回
- 无关联任务 → 跳过 Skill/MCP 日志区块
**边界条件（已知缺口）：**
- **会话输出未持久化**：读循环结束时**未回写** `agent_sessions` 的 `status`/`duration_seconds`/`total_tokens`
- `agent_sessions` **无 `work_dir` 字段**（原 v1.0 文档所列 `work_dir` 实际不存在于该表，仅存在于 PTY 内存态）
- `total_tokens` / `total_cost` 仅在人工或特定路径写入，通常为 0
- `skill_execution_summary` / `mcp_execution_summary` 无写入路径
**输入与输出：**
- 输入：`session_id`（来自路由 `:id`）
- 输出：会话详情数据 + Skill/MCP 执行日志列表
**依赖关系：** 依赖 `get_agent_sessions`、`get_skill_execution_logs`、`get_mcp_execution_logs`、`agent_sessions` / `skill_execution_logs` / `mcp_execution_logs` 表
**约束条件：** `mcp_execution_logs` 表**无写入路径**，故 MCP 日志区块恒为空（见 F039）
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 会话 S 关联任务 #12，任务 #12 有 3 条 Skill 执行日志
When 进入会话 S 详情页
Then 显示 3 条 Skill 执行记录（含状态与耗时）

Given 会话 S 关联任务 #12，但无 MCP 日志
When 进入详情页
Then MCP 区块显示空状态而非报错

Given 会话 S 已完成
When 查看详情
Then 显示 ended_at 与 duration_seconds（当前可能为空，因回写缺失）
```

#### F028: 从任务创建会话

**功能编号与名称：** F028 从任务创建会话
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 针对某个任务直接发起 Agent 执行，实现"任务 → 会话"的绑定
**前置条件：** 任务已存在；`agents` 表中至少有一个 Agent 记录
**主流程：**
1. 在任务详情页点击「启动 Agent / 创建会话」
2. 调用 `create_session_from_task(task_id)`
3. 后端取数据库中的**第一个 Agent** 作为执行者，由 `AgentService` 创建 `agent_sessions` 记录
4. 返回 `AgentSession`，前端跳转到会话/终端视图
**异常流程：**
- 无任何 Agent → 返回错误，提示需先检测 Agent
- 任务不存在 → 返回错误
- 选中的 Agent 二进制不可用 → 会话仍创建，PTY 启动失败仅记录 warn
**边界条件：**
- Agent 选择策略为"取第一条"，**不支持指定 Agent**
- 创建出的会话 `task_id` 指向目标任务，`related_task_id` 亦会写入
- `source` 默认 `native`
**输入与输出：**
- 输入：`task_id`
- 输出：`AgentSession`
**依赖关系：** 依赖 `create_session_from_task`、`AgentService`、`PtyManager`、`agents` 与 `agent_sessions` 表
**约束条件：** 依赖 F019 已完成 Agent 注册
**状态变化：** 新增 `agent_sessions` 记录（`status='running'`）
**权限控制：** 无（单用户）
**验收标准：**
```
Given 任务 #12 存在，agents 表有 1 条记录
When 点击任务详情页的「启动 Agent」
Then agent_sessions 新增 1 行，task_id=#12，返回会话对象

Given agents 表为空
When 调用 create_session_from_task(task_id=#12)
Then 返回错误提示"未找到可用 Agent"
```

#### F029: 会话终端交互

**功能编号与名称：** F029 会话终端交互
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 进入会话后直接在其终端中观察输出并发送指令
**前置条件：** 会话对应的 PTY 进程仍在运行
**主流程：**
1. 在 `/sessions` 或 `/sessions/:id` 中打开终端面板
2. `AgentTerminal` 监听 `pty:event`，将 `data` 事件写入 xterm
3. 用户在终端输入 → 触发 `pty_write(session_id, input)`
4. 终端尺寸变化 → `pty_resize(session_id, rows, cols)`
**异常流程：**
- 会话已结束 → 输入无效，仅回显
- 事件监听未注册（组件未挂载）→ 输出丢失
**边界条件（已知缺口）：**
- 与 F023 相同：`pty_write` 当前**仅回显 `[input] ...`，未真正写入子进程 stdin**；`pty_resize` **未真正调整 PTY 尺寸**
- 应用重启后 PTY 会话丢失（内存态），但 `agent_sessions` 记录仍标记为 `running`，出现"僵尸会话"
**输入与输出：**
- 输入：键盘输入、终端尺寸
- 输出：终端渲染内容
**依赖关系：** 依赖 `components/AgentTerminal.tsx`、`pty_write` / `pty_resize`、事件 `pty:event`
**约束条件：** 会话与 PTY 的映射关系**未持久化**，仅在同一次应用运行期内有效
**状态变化：** 无（不影响数据库）
**权限控制：** 无（单用户）
**验收标准：**
```
Given 会话 S 正在运行
When 在终端面板输入 "继续"
Then 终端显示输入回显（期望 Agent 实际收到输入，当前链路未打通）

Given 应用重启后
When 查看会话 S
Then 期望状态与实际进程一致（当前可能出现 DB=running 但进程已不存在）
```

#### F030: 会话停止

**功能编号与名称：** F030 会话停止
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 终止走偏或不再需要的 Agent 执行
**前置条件：** 会话存在
**主流程：**
1. 在会话列表/终端面板点击「停止」
2. 前端调用 `stop_agent(session_id)`（可能伴随 `pty_kill`）
3. `stop_agent` 将 `agent_sessions.status` 置为 `completed`、`exit_code = 0`
4. `pty_kill` 将内存 PTY 会话状态置为 `terminated`、`exit_code = -1`，并移除输入通道
5. 列表刷新，会话状态更新
**异常流程：**
- 会话不存在 → 返回错误
- 会话已结束 → 重复停止无副作用
**边界条件（已知缺口）：**
- **`stop_agent` 不终止操作系统子进程**，仅改数据库状态
- **`pty_kill` 不 kill 子进程**，因 `child` 句柄未持久化保存
- PTY 内存态的 `terminated` 值**不在 `agent_sessions.status` 的 `CHECK` 白名单内**，故不会被写入数据库
- 停止后 `ended_at` / `duration_seconds` 的回写依赖具体实现（当前不完整）
**输入与输出：**
- 输入：`session_id`
- 输出：`Result<(), String>`
**依赖关系：** 依赖 `stop_agent`、`pty_kill`、`AgentService`、`PtyManager`、`agent_sessions` 表
**约束条件：** 需要补充进程句柄持久化能力才能实现真正的进程终止
**状态变化：** `agent_sessions.status`：`running → completed`；重跑参见 F013/F014
**权限控制：** 无（单用户）
**验收标准：**
```
Given 会话 S 状态为 running
When 点击「停止」
Then agent_sessions 中 S 的 status=completed、exit_code=0

Given 会话 S 已停止
When 在操作系统中查看该 Agent 进程
Then 期望进程已退出（当前不会退出，属已知缺口）
```

---

### 模块 6：Skill 管理

#### F031: 内置 Skill 模板

**功能编号与名称：** F031 内置 Skill 模板
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 开箱即用的 PRD 生成模板，按需求类型选择
**前置条件：** 应用首次初始化数据库（`seed_data()` 写入种子数据）
**主流程：**
1. 应用初始化时向 `prd_skills` 插入 4 条内置模板（`is_builtin=1`，`version=1`）
2. 进入 `/skills` 页面，调用 `list_prd_skills` 展示，内置模板标记「内置」
3. 可展开查看模板内容；内置模板**不可修改、不可删除**
**异常流程：**
- 种子数据已存在 → `count > 0` 守卫，不重复插入
- 尝试编辑/删除内置模板 → 后端返回"内置Skill不可修改/删除"
**边界条件（内置模板清单）：**

| Skill ID | 名称 | skill_type |
|----------|------|-----------|
| `skill-feature` | 功能需求 PRD | `feature` |
| `skill-bugfix` | Bug修复 PRD | `bugfix` |
| `skill-optimization` | 技术优化 PRD | `optimization` |
| `skill-refactor` | 架构重构 PRD | `refactor` |

**输入与输出：**
- 输入：无
- 输出：`PrdSkill[]`（`is_builtin=true`）
**依赖关系：** 依赖 `db/mod.rs` 的种子数据插入、`list_prd_skills`、`prd_skills` 表
**约束条件：** 内置模板带 `is_builtin=1`，所有写操作前置校验该标志
**状态变化：** 数据库首次初始化时新增 4 行
**权限控制：** 无（单用户）
**验收标准：**
```
Given 数据库首次初始化
When 查询 prd_skills
Then 存在 4 条 is_builtin=1 的记录

Given 选中内置 Skill「功能需求 PRD」
When 尝试编辑模板内容
Then 后端返回"内置Skill不可修改"

Given 选中内置 Skill
When 尝试删除
Then 后端返回"内置Skill不可删除"，is_deleted 保持 0
```

#### F032: 基于内置创建自定义 Skill

**功能编号与名称：** F032 基于内置创建自定义 Skill
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 以内置模板为蓝本派生自定义版本，保留来源关系
**前置条件：** 目标父 Skill 存在且 `is_builtin=1`
**主流程：**
1. 在 Skill 列表点击内置 Skill 的「创建自定义版本」
2. 填写新名称（可选修改模板内容），调用 `create_custom_skill_from_builtin(parent_id, name, template?)`
3. 后端校验父 Skill 必须为内置（否则报错）
4. 新记录：`is_builtin=0`、`version=1`、`parent_id=父ID`、`skill_type` 与 `description` 继承自父
5. 列表新增该自定义 Skill
**异常流程：**
- `parent_id` 不存在 → 返回"父Skill不存在"
- `parent_id` 指向自定义 Skill → 返回"只能基于内置Skill创建自定义版本"
- 新名称重复 → **不校验唯一性**，允许重名
**边界条件：**
- 未传 `template` 时默认继承父模板
- 新记录与父记录无强约束绑定（父被软删除不影响子记录）
**输入与输出：**
- 输入：`parent_id`、`name`、`template?`
- 输出：`PrdSkill`（新记录）
**依赖关系：** 依赖 `create_custom_skill_from_builtin`、`PrdSkillService::create_custom_skill`、`prd_skills` 表
**约束条件：** 父 Skill 必须 `is_builtin=1`
**状态变化：** `prd_skills` 新增 1 行
**权限控制：** 无（单用户）
**验收标准：**
```
Given 内置 Skill「Bug修复 PRD」存在
When 基于它创建名为"团队 Bug 模板"的自定义 Skill
Then prd_skills 新增 1 行（is_builtin=0、parent_id=skill-bugfix、version=1）

Given 传入一个自定义 Skill 的 id 作为 parent_id
When 调用 create_custom_skill_from_builtin
Then 返回"只能基于内置Skill创建自定义版本"
```

#### F033: Skill CRUD

**功能编号与名称：** F033 Skill CRUD
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 维护自定义 PRD 模板
**前置条件：** 应用已初始化
**主流程：**
1. 列表：`list_prd_skills` 返回全部未过滤 Skill（含软删除后的记录处理由 SQL 决定）
2. 详情：`get_prd_skill`
3. 创建：`create_prd_skill(name, skill_type, description?, template)`，插入 `is_builtin=0`、`version=1`
4. 更新：`update_prd_skill(id, name?, description?, template?)`，内置 Skill 直接拒绝
5. 删除：`delete_prd_skill(id)`，内置拒绝，自定义执行**软删除**（`is_deleted=1`）
**异常流程：**
- 内置 Skill 更新/删除 → 返回明确错误
- 指定的 Skill 不存在 → 更新时为 `UPDATE` 影响 0 行（静默）；删除时同样静默
- `template` 为空 → 数据库 `NOT NULL`，插入失败
**边界条件：**
- 删除为**软删除**，记录仍在表中，但 `is_deleted=1`
- **`list_prd_skills` 未按 `is_deleted` 过滤的明确证据不足，需以 SQL 实测为准**（若不过滤，软删除后的模板仍会出现在列表中）
- 名称、`skill_type` 无唯一性与格式校验
**输入与输出：**
- 输入：`name`、`skill_type`、`description?`、`template`
- 输出：`PrdSkill` / `PrdSkill[]`
**依赖关系：** 依赖 `PrdSkillService`（`list`/`get`/`create`/`update`/`delete`）、`prd_skills` 表
**约束条件：** `name`、`skill_type`、`template` 均为 `NOT NULL`
**状态变化：** 新增/更新/软删除 `prd_skills` 记录
**权限控制：** 无（单用户）
**验收标准：**
```
Given 应用已初始化
When 创建 Skill（name="团队模板"、skill_type="feature"、template="..."）
Then prd_skills 新增 1 行，is_builtin=0、version=1

Given 一个自定义 Skill
When 删除它
Then is_deleted=1（记录仍存在，非物理删除）

Given 一个内置 Skill
When 调用 update_prd_skill
Then 返回"内置Skill不可修改"
```

#### F034: Skill 版本控制

**功能编号与名称：** F034 Skill 版本控制
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 追踪 Skill 模板的修改次数，识别模板是否被更新
**前置条件：** Skill 为自定义（`is_builtin=0`）
**主流程：**
1. 每次调用 `update_prd_skill` 并传入某个字段时，SQL 对该字段执行 `SET ... , version = version + 1`
2. `version` 字段在列表/详情中展示
**异常流程：** —
**边界条件（已知缺陷）：**
- **版本号按"字段"而非按"操作"自增**：一次提交同时修改 `name`、`description`、`template` 时，会执行 3 条独立 `UPDATE`，**版本号 +3**
- 未修改任何字段（三个参数均传 `None`）时，不执行任何 SQL，版本号不变
- 无版本历史快照，仅保留当前 `version` 数字，**不可回溯历史模板内容**
- 内置 Skill 无法被更新，故其 `version` 恒为 1
**输入与输出：**
- 输入：`id`、`name?`、`description?`、`template?`
- 输出：`Result<(), String>`
**依赖关系：** 依赖 `skills.rs::update_prd_skill`、`prd_skills.version` 字段
**约束条件：** 仅记录数字，不保留历史版本内容
**状态变化：** `prd_skills.version` 递增（增量 = 本次实际修改的字段数）
**权限控制：** 无（单用户）
**验收标准：**
```
Given 自定义 Skill S 的 version=1
When 仅修改 template 并保存
Then version 变为 2

Given 自定义 Skill S 的 version=1
When 同时修改 name 与 template 并保存
Then version 变为 3（当前实现按字段累加）

Given 内置 Skill
When 查看 version
Then 恒为 1（不可修改）
```

#### F035: Skill 执行日志

**功能编号与名称：** F035 Skill 执行日志
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 记录每次 Skill 调用的输入参数、输出摘要、Token 消耗与执行状态，用于问题排查
**前置条件：** 发生一次 Skill 调用（当前仅 PRD 生成路径会写入）
**主流程：**
1. `PrdService::generate_prd` 成功后调用 `record_skill_execution_log`
2. 写入字段：`task_id`、`session_id`、`requirement_id`、`skill_id`、`skill_name`、`skill_type`、`execution_phase`、`input_params`、`output_summary`、`token_usage`、`status`、`executed_at`
3. 会话详情页按 `session_id` 或岗位 `task_id` 查询展示
**异常流程：**
- 写入失败 → 记录 error 日志，不影响 PRD 生成结果
- 查询无数据 → 显示空状态
**边界条件（已知缺陷）：**
- **唯一写入点仅有 PRD 生成路径**，且 `execution_phase` **硬编码为 `"prd_generation"`**、`status` **硬编码为 `"success"`**、`skill_type` **硬编码为 `"feature"`**
- 即：**失败的调用不会被记录，任务执行/Session 启动阶段的 Skill 调用不会产生日志**
- `execution_phase` 的设计取值（`prd_generation` / `task_execution` / `session_start`）中，后两者目前无写入路径
- `status` 设计取值（`success` / `failed` / `skipped`）中，仅 `success` 会被写入
**输入与输出：**
- 输入：`task_id?`、`session_id?`、`requirement_id?`、`skill_id`、`skill_name`、`skill_type`、`execution_phase`、`input_params?`、`output_summary?`、`token_usage`、`status`
- 输出：`SkillExecutionLog[]`
**依赖关系：** 依赖 `db/skills.rs::record_skill_execution_log`、`get_skill_execution_logs`、`PrdService`、`skill_execution_logs` 表（含 `idx_skill_exec_task`、`idx_skill_exec_session`）
**约束条件：** `skill_id` 为 `NOT NULL` 外键；`skill_name` 为 `NOT NULL`
**状态变化：** 新增 `skill_execution_logs` 记录
**权限控制：** 无（单用户）
**验收标准：**
```
Given 需求 #1 使用 Skill「功能需求 PRD」生成 PRD 成功
When 查询 skill_execution_logs
Then 新增 1 行（execution_phase='prd_generation'、status='success'）

Given 某个 Skill 调用失败
When 查询 skill_execution_logs
Then 期望出现 status='failed' 的记录（当前不会写入，属已知缺口）
```

#### F036: Skill 使用统计

**功能编号与名称：** F036 Skill 使用统计
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 了解各 Skill 的使用频次、总 Token 消耗与成功率
**前置条件：** `skill_execution_logs` 中已有数据
**主流程：**
1. 在统计页调用 `get_skill_usage_stats`
2. 后端 SQL 按 `skill_name` 分组聚合：`COUNT(*)` 使用次数、`SUM(token_usage)` 总 Token、`SUM(CASE WHEN status='success' ...)` 成功次数
3. 按使用次数倒序返回，前端以表格/卡片展示
**异常流程：**
- 无数据 → 返回空数组，页面显示空状态
**边界条件：**
- 分组键为 `skill_name`（**非 `skill_id`**），同名不同 ID 的 Skill 会被合并统计
- 成功率需前端以 `success_count / use_count` 计算（后端未直接返回比率）
- 因 F035 的写入限制，统计结果**仅覆盖 PRD 生成路径**
**输入与输出：**
- 输入：无
- 输出：`[{ skill_name, skill_type, use_count, total_tokens, success_count }]`
**依赖关系：** 依赖 `get_skill_usage_stats`、`skill_execution_logs` 表
**约束条件：** 统计准确性受 F035 写入完整度制约
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given skill_execution_logs 中有 3 条 skill_name="功能需求 PRD" 的记录
When 调用 get_skill_usage_stats
Then 返回该 Skill 的 use_count=3

Given skill_execution_logs 为空
When 调用 get_skill_usage_stats
Then 返回空数组
```

---

### 模块 7：MCP 配置管理

#### F037: MCP 服务配置

**功能编号与名称：** F037 MCP 服务配置
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 为项目登记 MCP 服务端点及其认证信息，供 Agent 执行时使用
**前置条件：** 项目已创建
**主流程：**
1. 进入项目详情页「MCP 配置」Tab
2. `list_project_mcps` 加载已配置列表
3. 添加：`add_project_mcp(project_id, mcp_name, endpoint, description?, is_default, auth_config?, custom_params?)`
4. 移除：`remove_project_mcp(id)`
**异常流程：**
- `mcp_name` 或 `endpoint` 为空 → `NOT NULL` 约束失败
- `endpoint` 非合法 URL → 无校验，原样存储
- `auth_config` 结构非法 → 无校验，原样存储
**边界条件：**
- 与 F004 为同一功能的两个视角（F004 侧重项目配置管理，F037 侧重 MCP 服务本身）
- `auth_config` / `custom_params` 为文本字段，约定存放结构化数据但无 schema 校验
**输入与输出：**
- 输入：`project_id`、`mcp_name`、`endpoint`、`description`、`is_default`、`auth_config`、`custom_params`
- 输出：`ProjectMcp[]`
**依赖关系：** 依赖 `ProjectService`、`list/add/remove_project_mcp`、`project_mcps` 表（`idx_project_mcps_project` 索引）
**约束条件：** `project_id` 外键指向 `projects(id)`
**状态变化：** 新增/删除 `project_mcps` 记录
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目 P 已创建
When 添加 mcp_name="context7"、endpoint="https://mcp.example.com/sse"、auth_config={"token":"***"}
Then project_mcps 新增 1 行

Given mcp_name 为空
When 调用 add_project_mcp
Then 写入失败并返回错误
```

#### F038: 默认 MCP

**功能编号与名称：** F038 默认 MCP
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 为项目标记默认 MCP，便于自动选用
**前置条件：** 项目已配置至少一个 MCP
**主流程：**
1. 在 MCP 列表中勾选/设置某条为默认（`is_default=1`）
2. 数据写入 `project_mcps.is_default`
3. 默认 MCP 在会话/Agent 执行时作为首选端点
**异常流程：**
- 未设置默认 → 无默认 MCP，不影响其它功能
- 设置多个默认 → **不会报错**（无唯一约束），可能并存多条 `is_default=1`
**边界条件：**
- `is_default` 为 `INTEGER`，默认值 1（即新增即为默认）
- **数据库层无"每个项目仅一个默认"约束**，需前端保证互斥
**输入与输出：**
- 输入：`project_id`、`mcp_name`、`endpoint`、`is_default`
- 输出：`ProjectMcp`
**依赖关系：** 依赖 `add_project_mcp`、`project_mcps.is_default`
**约束条件：** 无强制唯一性
**状态变化：** 更新 `is_default` 标志
**权限控制：** 无（单用户）
**验收标准：**
```
Given 项目 P 有 2 个 MCP
When 将 MCP-A 设为默认
Then project_mcps 中 MCP-A 的 is_default=1

Given 项目 P 已有 1 个默认 MCP
When 再将另一个 MCP 设为默认
Then 允许设置（无唯一约束，可能出现 2 个默认）
```

#### F039: MCP 执行日志

**功能编号与名称：** F039 MCP 执行日志
**实现状态：** ❌ 未实现（表已建，无写入路径）
**目标用户与角色：** 单人开发者
**使用场景：** 记录每次 MCP 调用的状态、耗时与错误信息
**前置条件：** 无（功能尚未生效）
**主流程（设计意图）：**
1. MCP 连接/调用发生时，向 `mcp_execution_logs` 写入 `session_id`、`mcp_name`、`endpoint`、`action`、`status`、`error_message`、`duration_ms`
2. 前端在会话详情页按 `session_id` 查询展示
**异常流程：** —
**边界条件：**
- `mcp_execution_logs` 表结构与索引（`idx_mcp_exec_session`）均已就绪
- **代码库中不存在任何 `INSERT INTO mcp_execution_logs`**，该表恒为空
- 因此会话详情页的"MCP 调用记录"区块**始终显示为空**
**输入与输出：**
- 输入：（设计）`session_id`、`mcp_name`、`endpoint`、`action`、`status`、`error_message`、`duration_ms`
- 输出：（设计）`McpExecutionLog[]`
**依赖关系：** `get_mcp_execution_logs`（查询已实现）、`mcp_execution_logs` 表（**缺写入逻辑**）
**约束条件：** 需要先实现 MCP 客户端调用能力，才能产生日志
**状态变化：** 当前无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 会话 S 执行过程中调用了一个 MCP
When 查询 mcp_execution_logs（session_id=S）
Then 期望返回调用记录（当前恒为空数组）
```

#### F040: MCP 使用统计

**功能编号与名称：** F040 MCP 使用统计
**实现状态：** ⚠️ 部分实现
**目标用户与角色：** 单人开发者
**使用场景：** 聚合各 MCP 的调用次数、成功率与平均耗时
**前置条件：** `mcp_execution_logs` 中有数据（当前恒空）
**主流程：**
1. 调用 `get_mcp_usage_stats`
2. 后端按 `mcp_name` 分组：`COUNT(*)` 调用次数、`SUM(CASE WHEN status IN ('connected','call_success'))` 成功次数、`AVG(duration_ms)` 平均耗时
3. 按调用次数倒序返回
**异常流程：**
- 无数据 → 返回空数组
**边界条件：**
- 统计口径把 `connected` 与 `call_success` 都计为成功
- 因 F039 无写入，**该统计在当前实现下恒为空结果**
- 分组键为 `mcp_name`
**输入与输出：**
- 输入：无
- 输出：`[{ mcp_name, call_count, success_count, avg_duration }]`
**依赖关系：** 依赖 `get_mcp_usage_stats`、`mcp_execution_logs` 表
**约束条件：** 依赖 F039 完成写入链路后才有实际意义
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given mcp_execution_logs 中有 5 条记录
When 调用 get_mcp_usage_stats
Then 返回按 mcp_name 聚合的 call_count 与 avg_duration

Given mcp_execution_logs 为空
When 调用 get_mcp_usage_stats
Then 返回空数组（当前的实际结果）
```

---

### 模块 8：统计分析

#### F041: Token 使用统计

**功能编号与名称：** F041 Token 使用统计
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 掌握 Token 总消耗与成本，以及输入/输出/缓存/思考 Token 的构成
**前置条件：** `token_usage` 表中已有记录（由 PRD 生成或本地用量同步产生）
**主流程：**
1. 调用 `get_token_usage_summary` 获取汇总（缓存 60s）
2. 汇总字段：总量、成本、`input_tokens` / `output_tokens` / `cache_creation_tokens` / `cache_read_tokens` / `thinking_tokens` 分布
3. 调用 `get_token_usage(task_id?)` 获取明细记录
4. 前端在 `/cost` 与 `/statistics` 页面以卡片 + 表格展示
**异常流程：**
- 无数据 → 各项为 0，页面显示空状态
- 明细查询传入不存在的 `task_id` → 返回空数组
**边界条件（已知缺陷）：**
- `token_usage` 表由旧库 `ALTER` 追加 `cache_*` 列后，列顺序变为"表尾追加"
- `db/tokens.rs::get_token_usage` 使用 `SELECT *` 并按**固定下标**读取字段，在旧库场景下**可能发生列错位**，导致 cache/total_cost/source/recorded_at 取值异常
- `total_cost` 在 PRD 生成路径中恒为 0（未做价格换算）
**输入与输出：**
- 输入：`task_id?`（明细）；无参（汇总）
- 输出：`TokenUsageSummary` / `TokenUsage[]`
**依赖关系：** 依赖 `TokenService`（汇总 60s 缓存）、`get_token_usage` / `get_token_usage_summary`、`token_usage` 表（含 `idx_token_usage_session`、`idx_token_usage_recorded`）
**约束条件：** `session_id` 与 `task_id` 均为外键
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given token_usage 中有 3 条记录
When 调用 get_token_usage_summary
Then 返回汇总值等于 3 条记录之和

Given 60 秒内重复调用 get_token_usage_summary
When 期间无新数据写入
Then 第二次命中缓存，不重复查询数据库

Given 传入不存在的 task_id
When 调用 get_token_usage
Then 返回空数组
```

#### F042: 每日使用趋势

**功能编号与名称：** F042 每日使用趋势
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 观察 Token 消耗的日度走势，识别异常消耗
**前置条件：** `token_usage` 中已有带 `recorded_at` 的记录
**主流程：**
1. 调用 `get_token_usage_trend(days)`（前端传天数，如 7/30）
2. 后端按日期聚合 Token 消耗（缓存 60s，缓存 key 含 `days`）
3. 前端以折线/柱状图展示
**异常流程：**
- `days` 为负数或 0 → 无数据或异常，需前端约束
- 无数据 → 返回空数组
**边界条件：**
- 缓存 key 为 `token:trend:{days}`，不同天数各自缓存
- 聚合依据 `recorded_at` 字段
- 时区处理按 `recorded_at` 存储值（RFC3339）进行，未做本地时区归一化
**输入与输出：**
- 输入：`days: i32`
- 输出：`DailyUsage[]`
**依赖关系：** 依赖 `TokenService`、`get_token_usage_trend`、`token_usage.recorded_at`
**约束条件：** `days` 由前端控制，无后端上限校验
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given token_usage 中有最近 3 天的记录
When 调用 get_token_usage_trend(days=7)
Then 返回 3 条有值日期数据，其余天数可为 0 或省略

Given 先调用 days=7 再调用 days=7
When 第二次在 60s 内
Then 命中缓存
```

#### F043: 模型使用分布

**功能编号与名称：** F043 模型使用分布
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 了解各模型的调用量与成本占比，辅助模型选型
**前置条件：** `token_usage` 中已有 `model` 字段数据
**主流程：**
1. 调用 `get_token_usage_by_model`（缓存 60s）
2. 后端按 `model` 分组聚合使用量与成本
3. 前端以饼图/表格展示
**异常流程：**
- 无数据 → 返回空数组
**边界条件：**
- 当前仅有 PRD 生成与本地同步两条写入路径
- LLM 模型名硬编码为 `claude-sonnet-5-20250514`，故分布图中通常只有 1 个模型
- 本地同步写入的记录 `model` 取决于 JSONL 内容
**输入与输出：**
- 输入：无
- 输出：`ModelUsage[]`
**依赖关系：** 依赖 `TokenService`、`get_token_usage_by_model`、`token_usage.model`
**约束条件：** 无
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given token_usage 中存在 model="claude-sonnet-5-20250514" 的记录
When 调用 get_token_usage_by_model
Then 返回该模型的用量与成本聚合
```

#### F044: Token 预算管理

**功能编号与名称：** F044 Token 预算管理
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 设定月度 Token 上限与告警阈值，控制成本
**前置条件：** 应用已初始化（种子数据已插入一条 `scope='global'` 预算）
**主流程：**
1. 调用 `get_token_budget` 读取全局预算（缓存 300s，key 为字面量 `token_budget:global`）
2. 展示 `monthly_limit`（默认 2,000,000）、`alert_threshold`（默认 0.8）、`current_usage`
3. 用户修改并保存 → 调用 `set_token_budget(monthly_limit, alert_threshold)`，随后失效缓存
4. 前端以进度条/百分比展示消耗比例，超过阈值时告警
**异常流程：**
- 无预算记录 → `get_token_budget` 返回 `None`，前端使用默认展示
- `monthly_limit` 为 0 或负数 → **无校验**，可能导致除零或异常比例
**边界条件：**
- 当前仅支持 `scope='global'`（表结构支持多 scope，但仅按 global 查询）
- `current_usage` 无自动累计逻辑（无触发器/定时任务），需外部写入或保持为 0
- `reset_date` 字段存在但无自动重置逻辑
**输入与输出：**
- 输入：`monthly_limit: i64`、`alert_threshold: f64`
- 输出：`Option<TokenBudget>`
**依赖关系：** 依赖 `TokenService`、`get_token_budget` / `set_token_budget`、`token_budgets` 表
**约束条件：** `scope` 默认 `'global'`，`monthly_limit` 与 `alert_threshold` 为 `NOT NULL`
**状态变化：** 更新 `token_budgets` 记录
**权限控制：** 无（单用户）
**验收标准：**
```
Given 数据库首次初始化
When 调用 get_token_budget
Then 返回 monthly_limit=2000000、alert_threshold=0.8

Given 当前预算 monthly_limit=2000000
When 调用 set_token_budget(5000000, 0.9)
Then 再次 get 返回新值

Given 当前用量为 1,800,000（占 90%）
When 前端渲染
Then 进度条显示超阈值告警（阈值 0.8）
```

#### F045: 缓存统计与清理

**功能编号与名称：** F045 缓存统计与清理
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 观测内存缓存命中率与容量，并在需要时手动清空
**前置条件：** 应用已启动（`CacheWrapper` 作为 Tauri 状态注入）
**主流程：**
1. 调用 `get_cache_stats` 获取 `hits`、`misses`、`size`、`l1_size`、`hit_rate()`
2. 在 `/system-monitor` 页面展示命中率与容量
3. 点击「清空缓存」调用 `clear_cache`，清空 LRU 缓存并重置统计
**异常流程：**
- 缓存被禁用（`config.enabled=false`）→ 读写直接穿透
**边界条件（已知缺陷）：**
- 缓存实为**单层 LRU**（命名 `l1_cache` 属遗留命名，无 L2/Redis 层）
- `CacheConfig` 默认：`l1_max_size=1000`、`default_ttl=300s`、`enabled=true`、`key_prefix="ai_platform:"`
- 各业务 TTL：统计 300s、Token 汇总/趋势/模型 60s、Token 预算 300s、设置 300s、需求/任务列表 30s
- `CacheStats.evictions` 字段**定义了但从不自增**，淘汰数恒为 0
- 缓存条目以 `bincode` 序列化为 `Vec<u8>`，**类型变更后反序列化可能失败**（无版本号）
**输入与输出：**
- 输入：无
- 输出：`CacheStats`
**依赖关系：** 依赖 `cache.rs::CacheManager`、`get_cache_stats` / `clear_cache`、`CacheKeys`
**约束条件：** 进程内缓存，应用重启即清空
**状态变化：** 清空后缓存条目归零、统计归零
**权限控制：** 无（单用户）
**验收标准：**
```
Given 应用刚启动
When 调用 get_cache_stats
Then hits 与 misses 初始为 0

Given 多次读取统计页（命中缓存）
When 调用 get_cache_stats
Then hits 大于 0 且 hit_rate 合理

Given 点击「清空缓存」
When 再次调用 get_cache_stats
Then size 归零、hits/misses 重置
```

#### F046: 统计报表聚合

**功能编号与名称：** F046 统计报表聚合
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 在首页与统计页一览需求、任务、Token 的整体情况
**前置条件：** 应用已启动
**主流程：**
1. 调用 `get_statistics`（缓存 300s，key `statistics:main`）
2. 后端跨表聚合，返回 `tasks` / `requirements` / `tokens` 三组指标
3. 前端以统计卡片 + 图表展示（`Workspace` 首页与 `Statistics` 页面共用）
**异常流程：**
- 数据为空 → 各项为 0
- 需求/任务写操作后 → 缓存被主动失效，下次读取重新聚合
**边界条件：**
- 缓存 300s，写操作通过 `delete_pattern` 失效 `statistics` key
- 聚合口径以 `db` 层 SQL 为准（按 status 计数）
- **`pages/Dashboard.tsx` 也实现了类似聚合展示，但未挂载路由**，与 `Workspace` 功能重叠
**输入与输出：**
- 输入：无
- 输出：`serde_json::Value`（含 `tasks`/`requirements`/`tokens` 分组）
**依赖关系：** 依赖 `StatsService`、`get_statistics`、`requirements` / `tasks` / `token_usage` 表
**约束条件：** 聚合为只读查询，不修改数据
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 库中有 10 个需求、20 个任务
When 调用 get_statistics
Then 返回需求总数 10、任务总数 20 及各自状态分布

Given 新增 1 个需求后
When 调用 get_statistics
Then 需求总数变为 11（缓存已失效）
```

---

### 模块 9：LLM 集成

#### F047: Claude API 调用

**功能编号与名称：** F047 Claude API 调用
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 通过 Claude Messages API 完成 PRD 生成等 LLM 任务
**前置条件：** 已在 `settings` 中配置 `claude_api_key`
**主流程：**
1. `ClaudeClient::new(api_key)` 构建 `reqwest::Client`：总超时 **120s**、连接超时 **10s**
2. 请求 `POST https://api.anthropic.com/v1/messages`
3. 请求头：`x-api-key`、`anthropic-version: 2023-06-01`、`content-type: application/json`
4. 请求体：`model = "claude-sonnet-5-20250514"`（源码硬编码）、`max_tokens` 按方法设定（PRD 4096 / enrich 1024）、`messages=[{role:"user", content: prompt}]`
5. 解析响应 `content[].text` 拼接为结果，并从 `usage` 提取 `input_tokens` / `output_tokens`
6. 调用方（`PrdService` / `enrich_requirement`）使用结果并记录副作用
**异常流程：**
- 未配置 API Key → 调用方提前返回错误
- HTTP 非 2xx → 返回错误信息（含状态码与响应体）
- 超时 → 返回超时错误
- 响应体解析失败 → 返回解析错误
**边界条件：**
- **无重试机制**；**base_url 不可配置**；**模型不可配置**（硬编码）
- 每次调用都新建 `ClaudeClient`（不复用连接池）
- 响应中的 `usage` 为 `Option`，缺失时 Token 记 0
**输入与输出：**
- 输入：`api_key`、提示词（由各方法构造）
- 输出：`(生成文本, TokenStats { input_tokens, output_tokens, model })`
**依赖关系：** 依赖 `llm.rs::ClaudeClient`、`PrdService`、`commands.rs::enrich_requirement`、`settings.claude_api_key`
**约束条件：** 需要外部网络与有效 API Key；`reqwest` 作为 HTTP 客户端
**状态变化：** 无（副作用由调用方决定）
**权限控制：** 无（单用户）
**验收标准：**
```
Given 已配置有效 API Key
When 调用 generate_prd
Then 返回非空 PRD 文本与 TokenStats

Given API Key 无效
When 调用 Claude API
Then 返回包含鉴权失败信息的错误

Given 网络不可达
When 调用 Claude API
Then 在超时阈值内返回错误，不永久挂起
```

#### F048: 本地用量同步

**功能编号与名称：** F048 本地用量同步
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 把本机 Claude Code 执行产生的用量从本地 JSONL 导入平台，补齐 Token 统计
**前置条件：** 本机存在 Claude 数据目录（如 `~/.claude/projects/**/*.jsonl`）
**主流程：**
1. 调用 `sync_local_usage`（无参）
2. `LocalUsageReader` 扫描 `~/.claude/projects/**/*.jsonl`
3. 逐文件解析新增行（维护 `parsed_offsets` 增量偏移，避免重复解析）
4. 将解析出的用量写入 `token_usage`，`agent_type='claude-code'`、`source` 为本地来源、`total_cost=0`
5. 返回本次新增的记录条数（`i64`）
**异常流程：**
- 目录不存在 → 返回 0，不报错
- 单个 JSONL 文件格式异常 → 跳过异常行
- 文件被截断/回退 → 偏移量与实际不符，可能漏读
**边界条件：**
- **幂等性依赖内存中的 `parsed_offsets`**，应用重启后偏移丢失，可能重复解析（需确认是否有持久化偏移）
- `total_cost` 恒为 0，不参与成本统计
- 扫描范围仅限 Claude 数据目录，不含其它 Agent
**输入与输出：**
- 输入：无
- 输出：`i64`（新增记录数）
**依赖关系：** 依赖 `usage_reader.rs::LocalUsageReader`、`TokenService::sync_local_usage`、`token_usage` 表
**约束条件：** 依赖本机 Claude Code 的历史数据；文件系统读取性能受文件数量影响
**状态变化：** 新增 `token_usage` 记录
**权限控制：** 无（单用户）
**验收标准：**
```
Given 本机存在含 10 条用量的 JSONL 文件
When 调用 sync_local_usage（首次）
Then 返回 10，token_usage 新增 10 行

Given 文件内容未变化
When 再次调用 sync_local_usage
Then 返回 0（增量偏移生效，不重复导入）

Given 本机无 Claude 数据目录
When 调用 sync_local_usage
Then 返回 0 且不报错
```

#### F049: 流式响应（SSE）

**功能编号与名称：** F049 流式响应（SSE）
**实现状态：** ⚪ 未接线（能力已实现，未对外暴露）
**目标用户与角色：** 单人开发者
**使用场景：** PRD 生成时实时呈现增量文本，缩短等待感知
**前置条件：** 已配置 API Key；底层方法可被调用
**主流程（现状）：**
1. `llm.rs` 已实现 `generate_prd_streaming(..., event_sender: mpsc::Sender<String>)`
2. 请求体附加 `"stream": true`
3. 解析 `data: ` 前缀的 SSE 片段，处理 `content_block_delta`（累积文本并逐片 send）、`message_start`（model + input_tokens）、`message_delta`（output_tokens），遇 `[DONE]` 停止
**异常流程：** 同 F047
**边界条件（关键限制）：**
- **该函数在全代码库中无任何调用点**，也**未注册为 IPC 命令**，前端无法使用
- 实现方式为**先 `response.text().await` 取全文再按行解析**，因此**并非真正的逐字流式**，仅实现"解析 SSE 格式"，不产生实时性收益
- 若要在前端呈现流式效果，需：① 注册为命令并使用 Tauri 事件/通道推送；② 改为 `response.bytes_stream()` 真流式读取
**输入与输出：**
- 输入：提示词、`mpsc::Sender<String>` 事件发送端
- 输出：`(完整文本, TokenStats)`
**依赖关系：** 依赖 `llm.rs::generate_prd_streaming`（**未被引用**）
**约束条件：** 需补充命令注册与前端订阅逻辑
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 已配置 API Key
When 调用 generate_prd_streaming 并订阅事件
Then 每收到一个 content_block_delta 片段即产出一次文本片段

Given 前端需要流式 PRD 生成
When 调用现有 IPC 命令清单
Then 期望存在流式命令（当前不存在，属未接线状态）
```

#### F050: LLM 调用 Token 统计

**功能编号与名称：** F050 LLM 调用 Token 统计
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 每次 LLM 调用后记录 Token 消耗，供 F041–F044 统计使用
**前置条件：** 发生一次成功的 PRD 生成调用
**主流程：**
1. `PrdService` 在 PRD 生成成功后，将 `TokenStats` 写入 `token_usage`
2. 写入字段：`session_id`、`task_id`、`agent_type`、`model`、`input_tokens`、`output_tokens`、`cache_creation_tokens`、`cache_read_tokens`、`thinking_tokens`、`total_cost`、`source`、`recorded_at`
3. 写入后，Token 汇总/趋势/模型的 60s 缓存应失效
**异常流程：**
- 写入失败 → 记录错误日志，不影响 PRD 结果返回
**边界条件（已知缺口）：**
- **仅 PRD 生成路径写入**；`enrich_requirement`（F010）**不写入 Token 记录**
- **`total_cost` 恒为 0**（未做模型价格换算）
- `cache_*` 与 `thinking_tokens` 通常为 0（Anthropic `usage` 中对应字段可能缺失或未映射）
- `session_id` / `task_id` 在 PRD 场景下通常为 `NULL`
**输入与输出：**
- 输入：`TokenStats`（`input_tokens`、`output_tokens`、`model`）
- 输出：无（副作用为插入记录）
**依赖关系：** 依赖 `PrdService`、`token_usage` 表、`TokenService` 缓存失效
**约束条件：** `recorded_at` 默认 `CURRENT_TIMESTAMP`
**状态变化：** 新增 `token_usage` 记录
**权限控制：** 无（单用户）
**验收标准：**
```
Given 一次 PRD 生成成功并返回 input_tokens=8500、output_tokens=3200
When 查询 token_usage
Then 新增 1 行，input_tokens=8500、output_tokens=3200、model=claude-sonnet-5-20250514

Given 调用 enrich_requirement 成功
When 查询 token_usage
Then 期望新增记录（当前不会写入，属已知缺口）
```

---

### 模块 10：安全

#### F051: API Key 加密存储

**功能编号与名称：** F051 API Key 加密存储
**实现状态：** ✅ 已实现（能力完整；与 F062 的落库路径尚未打通，详见约束条件）
**目标用户与角色：** 单人开发者
**使用场景：** 对 API Key 等敏感字符串做本地加密，避免明文落盘
**前置条件：** 应用已启动
**主流程：**
1. 调用 `encrypt_api_key(api_key, password)` 加密
2. 算法链路：`PBKDF2-HMAC-SHA256(password, salt, 100_000, 32B)` 派生密钥 → `AES-256-GCM` 加密
3. 随机生成 `salt`（16 字节）与 `nonce`（12 字节），密文编码为 `base64(salt ‖ nonce ‖ ciphertext)`
4. 调用 `decrypt_api_key(encrypted, password)` 解密，先校验长度 ≥ `SALT_LEN + NONCE_LEN`
5. 辅助方法 `is_encrypted_format(data)` 判断字符串是否为加密格式
**异常流程：**
- 口令错误 → GCM 校验失败，返回中文错误信息
- 密文长度不足 → 返回格式错误
- 密文非法 base64 → 返回解析错误
**边界条件（关键缺口）：**
- **`encrypt_api_key` / `decrypt_api_key` 是独立的加解密能力，未与 `settings` 的写入路径集成**：`settings.claude_api_key` 由 `set_setting` 直接写入，**实际以明文存储**
- 加解密需前端同时提供 `api_key` 与 `password`（口令由用户记忆，无密钥派生存储/找回机制）
- 无密钥轮换、无加密标记字段，无法区分某条设置值是否已加密
**输入与输出：**
- 输入：`api_key`、`password`（加密）；`encrypted`、`password`（解密）
- 输出：加密串（base64）/ 明文
**依赖关系：** 依赖 `crypto.rs`（`aes-gcm 0.10`、`pbkdf2 0.12`、`sha2 0.10`、`base64 0.22`、`rand 0.8`）、`encrypt_api_key` / `decrypt_api_key` 命令
**约束条件：** 参数常量：`PBKDF2_ITERATIONS=100_000`、`SALT_LEN=16`、`NONCE_LEN=12`、`KEY_LEN=32`；含 2 个单元测试（加解密往返、错误口令失败）
**状态变化：** 无（不写数据库）
**权限控制：** 无（单用户）；口令即访问控制
**验收标准：**
```
Given 明文 key="sk-abc"、口令="p@ss"
When 调用 encrypt_api_key 再调用 decrypt_api_key
Then 解密结果等于 "sk-abc"

Given 密文 C 由口令 P1 生成
When 使用错误口令 P2 调用 decrypt_api_key
Then 返回解密失败错误

Given 输入长度不足的字符串
When 调用 decrypt_api_key
Then 返回格式错误，不 panic
```

#### F052: 全局输入校验

**功能编号与名称：** F052 全局输入校验
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 在前后端双层拦截非法输入，保证数据一致性
**前置条件：** 发生表单提交或命令调用
**主流程：**
1. **前端层**：`RequirementForm` / `TaskForm` 的 `validate()`（详见 F012 / F018）
2. **后端层**：`validation.rs` 统一校验
   - `validate_project`：名称长度 ≤ 100
   - `validate_requirement`：标题长度 ≤ 200、优先级必须在 `P0~P3`
   - `validate_task`：标题长度 ≤ 200、`task_type` 必须在枚举内、优先级合法
   - `validate_requirement_status_transition` / `validate_task_status_transition` / `validate_session_status_transition`：状态流转校验
3. 校验失败返回 `AppError::Validation`，`Display` 带中文前缀（如 `[验证]`）
**异常流程：**
- 后端校验失败 → 错误信息透传到前端 Toast
- 前端校验失败 → 就地提示，不发请求
**边界条件：**
- 后端校验为**最终防线**，前端校验可被绕过
- `settings` 的 `set_setting` **无 key/value 白名单校验**（任意键值可写）
- `update_project` 等路径的字段级校验覆盖度以 `validation.rs` 实际实现为准
**输入与输出：**
- 输入：待校验字段
- 输出：`Result<(), AppError>`
**依赖关系：** 依赖 `validation.rs`、`errors.rs`（`AppError` 枚举）、`components/RequirementForm.tsx` / `TaskForm.tsx`
**约束条件：** 校验规则变更需同时更新前端与后端，二者规则目前未共享同一份配置
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 直接调用 create_task 且 task_type="unknown"
When 后端执行 validate_task
Then 返回 [验证] 错误，不写入数据

Given 调用 update_requirement 且 status 流转非法
When 后端执行状态校验
Then 返回错误，原状态不变

Given 调用 set_setting 且 key="任意自定义键"
When 执行写入
Then 允许写入（无白名单，属已知行为）
```

---

### 模块 11：审计日志

#### F053: 变更历史记录

**功能编号与名称：** F053 变更历史记录
**实现状态：** ❌ 未实现（表已建，无写入路径）
**目标用户与角色：** 单人开发者
**使用场景：** 记录需求的创建、更新、状态变更、删除，支持审计追溯
**前置条件：** 无（功能尚未生效）
**主流程（设计意图）：**
1. 需求发生写操作时，向 `requirement_history` 插入记录
2. 字段：`requirement_id`、`action`（`create`/`update`/`delete`/`status_change`）、`old_value`、`new_value`、`created_at`
**异常流程：** —
**边界条件（现状）：**
- `requirement_history` 表与索引 `idx_requirement_history_req` 已建
- **全代码库无任何 `INSERT INTO requirement_history` / `UPDATE` / `DELETE` 语句**，该表恒为 0 行
- `RequirementService` 的创建/更新/删除流程中**未接入审计写入**
**输入与输出：**
- 输入：（设计）`requirement_id`、`action`、`old_value`、`new_value`
- 输出：（设计）变更记录
**依赖关系：** `requirement_history` 表（**缺写入逻辑**）、`RequirementService`
**约束条件：** 表结构就绪，仅缺写入链路
**状态变化：** 当前无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 需求 #1 被创建
When 查询 requirement_history
Then 期望存在 1 条 action='create' 记录（当前实际为 0 行）

Given 需求 #1 从 draft 改为 reviewing
When 查询 requirement_history
Then 期望存在 1 条 action='status_change' 记录（当前实际为 0 行）
```

#### F054: 审计日志页面

**功能编号与名称：** F054 审计日志页面
**实现状态：** ❌ 未实现（页面与查询命令存在，但数据源恒空）
**目标用户与角色：** 单人开发者
**使用场景：** 按实体类型筛选、按关键词搜索历史变更
**前置条件：** `requirement_history` 中存在数据（当前恒空）
**主流程（设计意图）：**
1. 进入 `/audit-log` 页面
2. 调用 `get_audit_logs(limit?)`（默认 200 条）
3. 后端 `LEFT JOIN requirements` 读取 `requirement_history`，返回记录
4. 前端支持按实体类型筛选与关键词搜索
**异常流程：**
- 无数据 → 显示空状态（**当前始终如此**）
**边界条件（现状）：**
- `get_audit_logs` 中 `entity_type` **硬编码为 `"requirement"`**，不支持其他实体类型
- 返回的 `changes` 字段由 `old_value`/`new_value` 拼接生成
- 因数据源为空，页面实际**始终展示空列表**
**输入与输出：**
- 输入：`limit`（默认 200）
- 输出：`AuditLogEntry[]`
**依赖关系：** 依赖 `pages/AuditLog.tsx`、`get_audit_logs`、`requirement_history` 表（**数据源为空**）
**约束条件：** 依赖 F053 完成写入后才能生效
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 应用已运行并产生若干需求变更
When 进入 /audit-log 页面
Then 期望展示变更记录列表（当前为空）

Given 调用 get_audit_logs(limit=50)
When 返回结果
Then 记录数 ≤ 50（当前为 0）
```

#### F055: 操作类型标记

**功能编号与名称：** F055 操作类型标记
**实现状态：** ❌ 未实现（依赖 F053 写入的 `action` 字段）
**目标用户与角色：** 单人开发者
**使用场景：** 用颜色/图标区分 `create` / `update` / `delete` / `status_change` 四类操作
**前置条件：** `requirement_history.action` 中有值（当前恒空）
**主流程（设计意图）：**
1. 审计列表根据 `action` 字段映射不同颜色徽章
2. 四种操作类型：`create`、`update`、`delete`、`status_change`
**异常流程：** —
**边界条件（现状）：**
- 数据库层 `action` 为 `TEXT NOT NULL`，但**无 `CHECK` 约束**限制取值范围
- 前端映射表需覆盖未预期取值（兜底展示）
- 因无写入，标记功能无实际数据可渲染
**输入与输出：**
- 输入：`action` 字符串
- 输出：带颜色的操作类型徽章
**依赖关系：** 依赖 `pages/AuditLog.tsx` 的渲染逻辑、`requirement_history.action`
**约束条件：** 依赖 F053 / F054
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 一条 action='status_change' 的审计记录
When 渲染审计列表
Then 该行显示 status_change 对应颜色的徽章

Given 一条 action='未知类型' 的审计记录
When 渲染
Then 使用兜底样式展示，不报错
```

---

### 模块 12：Git 集成

#### F056: Git 状态查看

**功能编号与名称：** F056 Git 状态查看
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 查看指定工作目录的当前分支与文件变更概览
**前置条件：** 已指定 `work_dir`，且该目录是一个 Git 仓库（含 `.git`）
**主流程：**
1. 调用 `git_get_status(work_dir)`
2. `GitService::validate_work_dir` 校验：目录存在 → 是目录 → 含 `.git`
3. 执行 `git status --porcelain`，解析每行前 2 字符为状态码、第 3 字符起为路径
4. 执行 `git branch --show-current` 获取分支（失败时回退 `"unknown"`）
5. 返回 `{ branch, files: [{status, path}], total_changes }`
**异常流程：**
- 目录不存在 → 返回错误
- 目录不含 `.git` → 返回"不是 Git 仓库"
- `git` 命令不可用 → 执行失败返回错误
**边界条件：**
- **无白名单/沙箱限制**：注释声称"校验是否在允许的工作目录范围内"，实际**仅校验存在性与 `.git`**
- 状态码解析按 `porcelain` 格式（如 `M `、`??`、`A `）
- 分支名为空（分离 HEAD）→ 回退为 `unknown`
**输入与输出：**
- 输入：`work_dir: String`
- 输出：`serde_json::Value`
**依赖关系：** 依赖 `services/git_service.rs`（实现方式为 **`std::process::Command::new("git")` 命令行**，非 `git2`/`libgit2`）、`git_get_status` 命令
**约束条件：** 依赖系统已安装 `git` 并加入 PATH
**状态变化：** 无（只读操作）
**权限控制：** 无（单用户）
**验收标准：**
```
Given work_dir 指向一个含未提交改动的 Git 仓库
When 调用 git_get_status
Then 返回 branch 与 files 列表，total_changes 等于改动文件数

Given work_dir 指向非 Git 目录
When 调用 git_get_status
Then 返回"不是 Git 仓库"错误

Given work_dir 不存在
When 调用 git_get_status
Then 返回目录不存在错误
```

#### F057: Git Diff

**功能编号与名称：** F057 Git Diff
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 查看工作区或指定文件的变更差异，用于提交前审查
**前置条件：** `work_dir` 为合法 Git 仓库
**主流程：**
1. 调用 `git_get_diff(work_dir, file_path?)`
2. 执行 `git diff`（指定 `file_path` 时追加 `-- <file_path>`）
3. 返回 stdout 原始 diff 文本
4. 前端以代码高亮方式展示
**异常流程：**
- 非 Git 目录 → 前置校验失败
- 无变更 → 返回空字符串
- `file_path` 不存在 → `git diff` 返回空或报错
**边界条件：**
- 仅展示**未暂存**的工作区差异（`git diff` 默认行为），不包含 `--staged`
- 未指定文件时返回全量 diff，大仓库可能返回大量文本
- 输出无分页/截断保护
**输入与输出：**
- 输入：`work_dir`、`file_path?`
- 输出：diff 文本（`String`）
**依赖关系：** 依赖 `GitService::get_diff`、`git_get_diff` 命令
**约束条件：** 依赖系统 `git`
**状态变化：** 无（只读）
**权限控制：** 无（单用户）
**验收标准：**
```
Given work_dir 中有 1 个文件被修改
When 调用 git_get_diff（不指定文件）
Then 返回包含 @@ 变更块的 diff 文本

Given 指定 file_path 为某已修改文件
When 调用 git_get_diff
Then 仅返回该文件的 diff

Given 工作区无变更
When 调用 git_get_diff
Then 返回空字符串
```

#### F058: Git Commit

**功能编号与名称：** F058 Git Commit
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 将工作区改动提交到当前分支
**前置条件：** `work_dir` 为合法 Git 仓库；存在已跟踪文件的改动
**主流程：**
1. 调用 `git_create_commit(work_dir, message, task_id?, session_id?)`
2. 后端执行 `git add -u`（**仅暂存已跟踪文件**，避免误提交敏感文件）
3. 执行 `git commit -m <message>`
4. 执行 `git rev-parse HEAD` 获取提交哈希
5. 执行 `git diff --stat HEAD~1` 统计变更
6. 返回 `{ commit_hash, short_hash(前7位), message, files_changed, diff_stat }`
**异常流程：**
- 无已跟踪改动 → `git commit` 失败，返回错误
- 存在未跟踪的新文件 → **不会被提交**（`add -u` 不含新文件）
- 非 Git 目录 → 前置校验失败
**边界条件（已知缺口）：**
- **`task_id` / `session_id` 参数以 `_task_id` / `_session_id` 声明，实现中被显式忽略**，提交与任务/会话的关联未建立
- **提交结果不落库**（见 F060）
- 无 `git push`，提交仅停留在本地
- 无 worktree 隔离能力（见 F060 边界条件）
**输入与输出：**
- 输入：`work_dir`、`message`、`task_id?`、`session_id?`（后两者未使用）
- 输出：`serde_json::Value`
**依赖关系：** 依赖 `GitService::create_commit`、`git_create_commit` 命令
**约束条件：** 需要 `git` 与已初始化的仓库；提交作者信息取自本机 git 配置
**状态变化：** 仓库新增一个 commit（数据库无变化）
**权限控制：** 无（单用户）
**验收标准：**
```
Given work_dir 中有 1 个已跟踪文件被修改
When 调用 git_create_commit(message="feat: 优化登录")
Then 返回 commit_hash，仓库新增 1 个提交

Given 只存在未跟踪的新文件
When 调用 git_create_commit
Then 提交失败或无内容提交（新文件不被 `add -u`）

Given 调用时传入 task_id="#12"
When 查询 git_commits 表
Then 期望存在关联记录（当前无写入，属已知缺口）
```

#### F059: Git Log

**功能编号与名称：** F059 Git Log
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 查看提交历史，了解代码演进
**前置条件：** `work_dir` 为合法 Git 仓库
**主流程：**
1. 调用 `git_log(work_dir, limit?)`
2. 执行 `git log -{limit|50} --format=%H|%h|%s|%an|%ai`
3. 以 `|` 分隔（`splitn(5, '|')`）解析为 `{ hash, short_hash, message, author, date }`
4. 前端列表展示，支持搜索（按消息/哈希/作者）与展开
**异常流程：**
- 空仓库（无提交）→ `git log` 报错，返回错误
- `limit` 非法 → 命令拼接异常
**边界条件：**
- 默认 `limit = 50`
- 提交消息中含 `|` 时解析可能错位（`splitn(5)` 仅切前 4 个 `|`）
- 返回为 `Vec<serde_json::Value>`，无分页（依赖 `limit` 控制）
**输入与输出：**
- 输入：`work_dir`、`limit?`（默认 50）
- 输出：`Vec<{hash, short_hash, message, author, date}>`
**依赖关系：** 依赖 `GitService::log`、`git_log` 命令
**约束条件：** 依赖系统 `git`
**状态变化：** 无（只读）
**权限控制：** 无（单用户）
**验收标准：**
```
Given work_dir 中有 10 个提交
When 调用 git_log(limit=5)
Then 返回 5 条记录，按时间倒序

Given 提交消息为 "fix: 修复 | 登录问题"
When 调用 git_log
Then message 解析不产生字段错位（当前实现可能截断，属已知边界）

Given 仓库无任何提交
When 调用 git_log
Then 返回错误或空列表，页面不崩溃
```

#### F060: 提交记录落库

**功能编号与名称：** F060 提交记录落库
**实现状态：** ❌ 未实现（表已建，无写入路径）
**目标用户与角色：** 单人开发者
**使用场景：** 把提交与任务/会话关联存入平台，实现"任务 → 代码"的可追溯
**前置条件：** 无（功能尚未生效）
**主流程（设计意图）：**
1. `git_create_commit` 成功后，向 `git_commits` 插入 `task_id`、`session_id`、`commit_hash`、`branch_name`、`message`、`files_changed`
2. 平台页面按任务/会话查询提交记录
**异常流程：** —
**边界条件（现状）：**
- `git_commits` 表与索引 `idx_git_commits_task` 已建
- **代码库中不存在任何 `INSERT INTO git_commits`**，也**不存在查询该表的命令**，该表恒为 0 行
- `git_create_commit` 的 `_task_id` / `_session_id` 参数被显式忽略
- **Worktree 隔离未实现**：仅在 `agent.rs::probe_capabilities` 中通过 `--help` 文本探测 `worktree_support` 能力标记，**不执行任何 worktree 操作**
- **自动提交未实现**：无 hook、无定时任务、无"任务完成即提交"联动
**输入与输出：**
- 输入：（设计）`task_id`、`session_id`、`commit_hash`、`branch_name`、`message`、`files_changed`
- 输出：（设计）提交记录列表
**依赖关系：** `git_commits` 表（**缺写入与查询逻辑**）、`git_create_commit`
**约束条件：** 需要先补齐 `git_create_commit` 的参数使用与落库逻辑
**状态变化：** 当前无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 任务 #12 的 Agent 完成了代码提交
When 调用 git_create_commit(task_id="#12")
Then 期望 git_commits 新增 1 行并关联 task_id（当前无写入）

Given 平台需要展示"某任务的提交记录"
When 查询提交记录
Then 期望可按 task_id 过滤（当前无对应查询命令）

Given 需要为 Agent 隔离工作目录
When 启动 Agent
Then 期望创建 worktree（当前未实现）
```

---

### 模块 13：系统配置

#### F061: 设置管理（KV）

**功能编号与名称：** F061 设置管理（KV）
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 以键值方式集中管理应用配置
**前置条件：** 应用已启动（`settings` 表已建）
**主流程：**
1. 读取：`get_setting(key)` → `Option<String>`
2. 写入：`set_setting(key, value)`，后端使用 `INSERT OR REPLACE` 实现 upsert
3. 服务层 `SettingService` 带 300s 缓存（key 前缀 `setting:`），写入后失效对应缓存
4. 前端 `useSettingsStore` 提供 `fetchSetting` / `fetchSettings(keys[])` / `saveSetting`
**异常流程：**
- 键不存在 → 返回 `None`，前端使用默认值
- `value` 为空字符串 → 允许写入（`NOT NULL` 但空串合法）
**边界条件：**
- **无 key 白名单与 value 格式校验**，任意键值可写
- 代码中已使用的 key：`claude_api_key`（Claude API Key）、`session_mode`（会话模式，默认 `auto`）
- 表结构：`key TEXT PRIMARY KEY`、`value TEXT NOT NULL`、`updated_at`
**输入与输出：**
- 输入：`key`、`value`
- 输出：`Option<String>` / `Result<(), String>`
**依赖关系：** 依赖 `settings.rs`、`SettingService`、`get_setting` / `set_setting`、`settings` 表、`useSettingsStore`
**约束条件：** 单表 KV，无分组与类型信息（一切皆为字符串）
**状态变化：** 新增/更新 `settings` 行，`updated_at` 随写入更新
**权限控制：** 无（单用户）
**验收标准：**
```
Given settings 中不存在 key="foo"
When 调用 get_setting("foo")
Then 返回 None

Given 调用 set_setting("foo", "bar") 后再 get_setting("foo")
Then 返回 "bar"（upsert 生效）

Given 300s 内重复读取同一 key
When 无写入发生
Then 命中缓存
```

#### F062: Claude API Key 配置

**功能编号与名称：** F062 Claude API Key 配置
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 在设置页填入 Claude API Key，使 PRD 生成与需求完善可用
**前置条件：** 应用已启动
**主流程：**
1. 进入 `/settings` 页面，在 LLM/Claude 配置区输入 API Key
2. 前端调用 `set_setting("claude_api_key", <key>)`（或先调用 `encrypt_api_key` 再存储）
3. `PrdService` 在生成 PRD 时读取 `settings.claude_api_key`
4. `commands.rs::enrich_requirement` 同样读取该 key
**异常流程：**
- 未配置 Key → PRD 生成/需求完善返回"未配置 Claude API Key"
- Key 无效 → 调用 Anthropic API 时返回鉴权错误
**边界条件（与 F051 相关）：**
- **默认路径下 `claude_api_key` 以明文存储在 `settings` 表**（`set_setting` 未做加密）
- `encrypt_api_key` / `decrypt_api_key` 命令虽已实现，但**未与设置读写链路集成**，需前端自行编排"先加密再存、读后解密"
- 前端若存加密串而未在读取处解密，会导致 API 调用鉴权失败
**输入与输出：**
- 输入：`key = "claude_api_key"`、`value = <API Key 或加密串>`
- 输出：无（副作用为写入 `settings`）
**依赖关系：** 依赖 `get_setting` / `set_setting`、`settings.claude_api_key`、`PrdService`、`commands.rs::enrich_requirement`、`encrypt_api_key`
**约束条件：** 依赖 Anthropic 官方 API 可用
**状态变化：** 新增/更新 `settings` 中 `claude_api_key` 行
**权限控制：** 无（单用户）；建议结合 F051 加密以降低明文风险
**验收标准：**
```
Given 未配置 claude_api_key
When 点击「生成 PRD」
Then 提示"未配置 Claude API Key"

Given 已配置有效 claude_api_key
When 点击「生成 PRD」
Then 调用成功并返回 PRD 内容

Given 前端调用 encrypt_api_key 后调用 set_setting 存入密文
When PrdService 直接读取并用于鉴权
Then 期望先解密再使用（当前后端不解密，会鉴权失败，属已知风险）
```

---

### 模块 14：UI 与交互

#### F063: 工作台首页

**功能编号与名称：** F063 工作台首页
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 应用启动后的默认落地页，一览项目、需求、任务与 Token 概况
**前置条件：** 应用已启动
**主流程：**
1. 路由 `/` 渲染 `pages/Workspace.tsx`
2. 调用 `get_statistics` 获取需求/任务/Token 聚合；调用 `list_projects` 获取项目列表
3. 展示统计卡片（需求总数、任务总数、已完成、已阻塞等）与项目入口、最近活动
4. 点击卡片可跳转至对应列表页并带筛选
**异常流程：**
- 数据加载失败 → 显示错误态/空态而非白屏
- 无数据 → 展示 0 值与引导文案
**边界条件：**
- 数据来源受 `get_statistics` 300s 缓存影响（写操作会主动失效）
- **`pages/Dashboard.tsx` 为功能重叠的未挂载页面**，实际不参与渲染
**输入与输出：**
- 输入：无
- 输出：统计卡片 + 列表视图
**依赖关系：** 依赖 `pages/Workspace.tsx`、`get_statistics`、`list_projects`、`AnimatedNumber`、`Skeleton`
**约束条件：** 首页为只读聚合视图
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 应用已启动且有数据
When 打开首页
Then 显示统计卡片与项目列表，点击卡片可跳转

Given 数据加载中
When 页面首次渲染
Then 显示骨架屏而非空白
```

#### F064: 侧边导航布局

**功能编号与名称：** F064 侧边导航布局
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 在全应用范围内快速切换功能模块
**前置条件：** 应用已启动
**主流程：**
1. `components/Layout.tsx` 作为父路由组件，渲染左侧分组导航 + 主内容区 `Outlet`
2. 导航分组：工作区（首页/项目/需求/任务看板/Agent/会话/提交）、数据（统计/成本）、系统（审计/导入导出/监控/设置）等
3. 底部状态栏每 5 秒轮询 `get_monitor_data` 展示系统指标
4. 当前路由对应的菜单项高亮
**异常流程：**
- `get_monitor_data` 轮询失败 → 状态栏显示占位，不影响主内容
- 路由不匹配 → **无 `*` 兜底路由**，内容区可能为空
**边界条件：**
- 轮询间隔固定 5 秒，无退避策略
- 通过 `@tauri-apps/api/core` 的 `invoke` **直接调用** `get_monitor_data`，未经 `src/lib/tauri.ts` 封装
**输入与输出：**
- 输入：当前路由
- 输出：导航 UI + 内容区 + 状态栏
**依赖关系：** 依赖 `components/Layout.tsx`、`HashRouter`、`get_monitor_data`
**约束条件：** 布局为全局单例，所有页面通过 `Outlet` 渲染
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 应用已启动
When 点击左侧「需求」
Then 路由切换到 /requirements，菜单项高亮，内容区渲染需求列表

Given 主内容路由不存在
When 访问任意未注册 hash 路由
Then 期望显示 404 或首页（当前无兜底路由，内容区为空）
```

#### F065: 分页组件

**功能编号与名称：** F065 分页组件
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 大数据量列表的分页浏览
**前置条件：** 后端返回分页结构
**主流程：**
1. `components/Pagination.tsx` 接收 `currentPage` / `totalPages` / `onPageChange`
2. 页码数 > 7 时以省略号折叠中间页码
3. 点击页码触发 `onPageChange`，由父页面重新请求数据
**异常流程：**
- `totalPages <= 1` → 隐藏或不展示分页
**边界条件（覆盖范围有限）：**
- **后端目前仅 `list_requirements` 使用分页**（`PagedResult{items,total,page,page_size,total_pages}`，`page_size` 上限 100）
- 任务列表、会话列表、Skill 列表、Token 明细等**均未分页**
- 前端 `listRequirements` 封装会**从 `PagedResult.items` 中提取数组**，分页信息在部分调用点被丢弃
**输入与输出：**
- 输入：`currentPage`、`totalPages`
- 输出：分页控件 + `onPageChange` 回调
**依赖关系：** 依赖 `components/Pagination.tsx`、`pagination.rs::PagedResult`、`list_requirements`
**约束条件：** `PaginationParams.limit()` 硬上限 100
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 需求列表共 250 条、page_size=50
When 查看分页控件
Then 显示 5 页，>7 页时使用省略号折叠

Given 点击第 2 页
When 触发 onPageChange
Then 请求 page=2 并渲染该页数据

Given 进入任务看板
When 观察是否存在分页控件
Then 当前无分页（属已知覆盖缺口）
```

#### F066: 表单组件

**功能编号与名称：** F066 表单组件
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 统一需求与任务的新建/编辑交互
**前置条件：** 已打开对应弹窗
**主流程：**
1. `RequirementForm`：字段分组（基本信息/归属/业务/其它），含 `RichTextEditor` 富文本描述、标签、附件列表
2. `TaskForm`：任务字段（类型/优先级/工时/截止日期/执行人等）
3. 两表单均实现本地 `validate()`（见 F012 / F018），校验失败禁用保存按钮
4. 提交时由父页面调用对应 store/命令
**异常流程：**
- 校验失败 → 就地展示错误文案，禁止提交
- 提交失败 → Toast 提示后端错误信息
**边界条件：**
- `RichTextEditor` 基于 **tiptap**，支持加粗/斜体/标题/列表/代码块/撤销重做，输出 HTML 字符串存入 `description`
- `AttachmentList` 按扩展名显示图标、格式化文件大小，**仅管理路径/说明列表，无真实文件上传**
- 项目 Key 输入框 `maxLength=10`（无独立 `validate()`）
**输入与输出：**
- 输入：表单字段值
- 输出：`Requirement` / `Task` 提交载荷
**依赖关系：** 依赖 `components/RequirementForm.tsx`、`components/TaskForm.tsx`、`RichTextEditor.tsx`、`AttachmentList.tsx`、`showToast`
**约束条件：** 表单为受控组件；富文本输出为 HTML，后端按文本存储
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 打开需求新建弹窗
When 在描述区使用富文本编辑器添加加粗与列表
Then 描述以 HTML 形式保存并可在详情页渲染

Given 打开任务新建弹窗且标题为空
When 观察保存按钮
Then 按钮禁用
```

#### F067: 状态/优先级徽章

**功能编号与名称：** F067 状态/优先级徽章
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 以统一视觉语言表达实体状态、优先级与类型
**前置条件：** 渲染含状态字段的列表/详情
**主流程：**
1. `components/Badges.tsx` 命名导出 `Badge`、`StatusBadge`、`PriorityBadge`、`TaskTypeBadge`、`ReqTypeBadge`
2. 各徽章按取值映射颜色与文案
3. 列表与详情页统一引用
**异常流程：**
- 未知取值 → 使用兜底灰色样式
**边界条件：**
- 覆盖范围：需求状态（8 值）、任务状态（4 值）、优先级（P0–P3）、任务类型（5 值）、需求类型（4 值）
- 颜色映射为前端硬编码，不随主题动态调整（依赖 Tailwind 类名）
**输入与输出：**
- 输入：状态/优先级/类型字符串
- 输出：带样式徽章
**依赖关系：** 依赖 `components/Badges.tsx`
**约束条件：** 文案与配色变更需同步前端映射表
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 任务状态为 blocked
When 渲染任务列表
Then 该行展示 blocked 对应颜色与文案的徽章

Given 优先级为 P0
When 渲染
Then 展示高优先级配色徽章

Given 传入未知状态值
When 渲染
Then 使用兜底样式，不报错
```

#### F068: 加载骨架屏

**功能编号与名称：** F068 加载骨架屏
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 数据加载期间提供结构占位，避免布局抖动
**前置条件：** 组件进入加载态
**主流程：**
1. `components/Skeleton.tsx` 命名导出 `CardSkeleton`、`StatCardSkeleton`
2. 页面在 `loading === true` 时渲染骨架屏
3. 数据返回后替换为真实内容
**异常流程：**
- 加载失败 → 由页面切换到错误态，而非无限骨架
**边界条件：**
- 目前仅提供卡片与统计卡两种骨架形态
- 骨架数量/布局需与真实卡片一致，否则仍会产生抖动
**输入与输出：**
- 输入：无
- 输出：占位 UI
**依赖关系：** 依赖 `components/Skeleton.tsx`、各 store 的 `loading` 字段
**约束条件：** 纯展示组件
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 首页数据正在加载
When 页面渲染
Then 显示统计卡骨架而非空白

Given 数据加载完成
When 重新渲染
Then 骨架被真实内容替换
```

#### F069: Toast 通知

**功能编号与名称：** F069 Toast 通知
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 全局反馈操作结果（成功/失败/警告/信息）
**前置条件：** 应用已挂载 `ToastContainer`
**主流程：**
1. 调用命令式 API `showToast(type, message)`
2. `type` ∈ `success` / `error` / `warning` / `info`
3. `ToastContainer` 渲染消息，**3 秒后自动消失**
4. 多个页面在异步操作成功/失败处调用
**异常流程：**
- 同一时刻多条消息 → 垂直堆叠展示
**边界条件：**
- 自动消失时长固定 3 秒，不可配置
- 无手动关闭按钮的明确保证（以组件实现为准）
- 不持久化、无历史记录
**输入与输出：**
- 输入：`type`、`message`
- 输出：屏幕上方的临时提示
**依赖关系：** 依赖 `components/Toast.tsx`（`ToastContainer` + `showToast` + `ToastType`）
**约束条件：** 需在应用根部渲染 `ToastContainer`
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 创建需求成功
When 后端返回成功
Then 展示 success 类型 Toast，约 3 秒后消失

Given 创建失败
When 后端返回错误
Then 展示 error 类型 Toast 并包含错误信息
```

#### F070: 数字滚动动画

**功能编号与名称：** F070 数字滚动动画
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 统计数字变化时提供平滑过渡，增强可读性
**前置条件：** 页面渲染数值
**主流程：**
1. `components/AnimatedNumber.tsx` 基于 `react-countup` 实现
2. 接收 `value` / `duration` / `prefix` / `suffix`
3. 数值变化时执行滚动动画
**异常流程：**
- 传入非数值 → 组件渲染异常（需调用方保证类型）
**边界条件：**
- 动画时长可配置
- 仅在数值**变化**时触发动画，首次渲染按配置表现
**输入与输出：**
- 输入：`value`、`duration?`、`prefix?`、`suffix?`
- 输出：动画数字
**依赖关系：** 依赖 `components/AnimatedNumber.tsx`、`react-countup`
**约束条件：** 依赖 `requestAnimationFrame`（WebView 支持良好）
**状态变化：** 无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 统计卡片显示 Token 总量 1000
When 数据更新为 1500
Then 数字以滚动动画过渡到 1500

Given 传入 prefix="$"
When 渲染
Then 显示形如 "$1500"
```

#### F071: 错误边界

**功能编号与名称：** F071 错误边界
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 捕获子组件渲染错误，避免整页白屏
**前置条件：** 应用根部或页面级已包裹 `ErrorBoundary`
**主流程：**
1. `components/ErrorBoundary.tsx`（class 组件）实现 `componentDidCatch`
2. 捕获错误后设置内部 state，渲染错误 UI（含重试按钮）
3. 用户点击重试 → 重置 state 尝试重新渲染
**异常流程：**
- 重试后仍抛错 → 继续展示错误 UI
- 错误发生在边界组件自身 → 无法捕获
**边界条件：**
- **仅捕获渲染期错误**，不捕获事件处理器、异步 Promise 与后端错误
- 无错误上报（不写日志、不上报服务）
**输入与输出：**
- 输入：子组件树
- 输出：正常渲染 或 错误 UI
**依赖关系：** 依赖 `components/ErrorBoundary.tsx`
**约束条件：** 需在关键层级包裹
**状态变化：** 内部 `hasError` state
**权限控制：** 无（单用户）
**验收标准：**
```
Given 某子组件渲染时抛出异常
When 页面渲染
Then 显示错误 UI 与重试按钮，页面不白屏

Given 错误由异步请求 Promise 拒绝导致
When 发生
Then 期望由调用方 Toast 处理（错误边界不捕获）
```

#### F072: 暗色/亮色主题

**功能编号与名称：** F072 暗色/亮色主题
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 按偏好或环境切换界面主题
**前置条件：** 应用已启动
**主流程：**
1. `stores/themeStore.ts` 维护 `theme`，取值 `light` / `dark` / `system`
2. 使用 Zustand `persist` 中间件，持久化到 `localStorage`（key = `ace-theme`）
3. 通过切换 `<html>` 上的 `.dark` 类驱动 Tailwind 暗色样式
4. 设置页提供主题切换入口
**异常流程：**
- `system` 模式 → 跟随系统偏好（`prefers-color-scheme`）
- `localStorage` 不可用 → 使用默认主题
**边界条件：**
- **`themeStore` 未被 `stores/index.ts` 统一 re-export**（`index.ts` 仅导出 5 个 store），页面需从具体路径引入
- 主题仅影响 `.dark` 类相关的 Tailwind 样式，硬编码颜色（如徽章配色）可能不随主题变化
**输入与输出：**
- 输入：`theme`
- 输出：`<html>.dark` 类切换与持久化值
**依赖关系：** 依赖 `stores/themeStore.ts`、Tailwind 暗色配置、`localStorage`
**约束条件：** `persist` 中间件版本兼容性以 Zustand 实际版本为准
**状态变化：** `theme` state + `localStorage['ace-theme']`
**权限控制：** 无（单用户）
**验收标准：**
```
Given 当前主题为 light
When 切换到 dark
Then html 元素添加 .dark 类，界面变暗

Given 打开应用（此前选择 dark）
When 应用启动
Then 从 localStorage 恢复 dark 主题

Given 选择 system 且系统为暗色
When 应用启动
Then 界面为暗色
```

---

### 模块 15：数据管理

#### F073: 数据导出

**功能编号与名称：** F073 数据导出
**实现状态：** ❌ 未实现（前端页面存在，后端无命令）
**目标用户与角色：** 单人开发者
**使用场景：** 将平台数据导出为文件，用于备份或迁移
**前置条件：** 无（功能尚未生效）
**主流程（设计意图）：**
1. 进入 `/import-export` 页面，选择导出范围
2. 调用导出命令，后端序列化数据并写入文件
3. 前端提示导出路径
**异常流程：** —
**边界条件（现状）：**
- `pages/ImportExport.tsx` 页面已存在并挂载路由
- **后端未注册任何 import/export 相关命令**（v1.0 当时 `generate_handler!` 共 77 条命令，无导出命令；现已补齐 `export_data` / `import_data`）
- 页面上的导出按钮**无可用后端支撑**
**输入与输出：**
- 输入：（设计）导出范围
- 输出：（设计）导出文件
**依赖关系：** `pages/ImportExport.tsx`（**缺后端命令**）
**约束条件：** 需要新增导出命令与文件写入逻辑
**状态变化：** 当前无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 进入 /import-export 页面
When 点击「导出」
Then 期望生成导出文件（当前无后端命令，无法完成）
```

#### F074: 数据导入

**功能编号与名称：** F074 数据导入
**实现状态：** ❌ 未实现（前端页面存在，后端无命令）
**目标用户与角色：** 单人开发者
**使用场景：** 从导出文件恢复或迁移数据
**前置条件：** 无（功能尚未生效）
**主流程（设计意图）：**
1. 选择导入文件
2. 调用导入命令，后端解析并写入数据库
3. 返回导入统计（成功/失败条数）
**异常流程：** —
**边界条件（现状）：**
- 同 F073：**后端无任何导入命令**
- 导入涉及事务与冲突处理策略（覆盖/跳过/合并），当前**未定义**
- 导入前应自动备份数据库，当前无此联动
**输入与输出：**
- 输入：（设计）导入文件
- 输出：（设计）导入结果统计
**依赖关系：** `pages/ImportExport.tsx`（**缺后端命令**）
**约束条件：** 需先设计文件格式与冲突策略
**状态变化：** 当前无
**权限控制：** 无（单用户）
**验收标准：**
```
Given 进入 /import-export 页面
When 选择文件并点击「导入」
Then 期望数据被写入并返回统计（当前无后端命令，无法完成）

Given 导入文件格式非法
When 执行导入
Then 期望事务回滚且数据不被破坏（当前无实现）
```

#### F075: 数据库异常自动备份

**功能编号与名称：** F075 数据库异常自动备份
**实现状态：** ✅ 已实现
**目标用户与角色：** 单人开发者
**使用场景：** 数据库初始化/打开失败时保留现场，避免数据彻底丢失
**前置条件：** 应用启动
**主流程：**
1. 应用启动时 `Database::new()` 尝试打开数据库（`app_data_dir()/database.db`）
2. 打开/初始化失败 → 将原文件重命名备份为 `database.db.bak`
3. 重建空数据库并继续启动
4. 通过日志记录备份事件
**异常流程：**
- 备份文件已存在 → 按实现覆盖或另存（以后端逻辑为准）
- 备份操作本身失败 → 启动失败并报错
**边界条件：**
- 数据库连接参数：`journal_mode=WAL`、`foreign_keys=ON`、`busy_timeout=5000`
- WAL 模式下存在 `-wal` / `-shm` 附属文件，备份单文件可能不完整
- **无定时备份**、无备份数量上限与轮转策略、无手动备份入口
- 备份为一次性的"故障兜底"，不是数据保护方案
**输入与输出：**
- 输入：数据库文件
- 输出：`database.db.bak`
**依赖关系：** 依赖 `main.rs` 初始化逻辑、`db/mod.rs::Database::new`、文件系统
**约束条件：** 仅在数据库打开/初始化失败时触发
**状态变化：** 生成 `.bak` 文件；重建空库（原数据仅以文件形式留存）
**权限控制：** 无（单用户）
**验收标准：**
```
Given database.db 已损坏
When 应用启动
Then 生成 database.db.bak 并重建可用数据库，应用可正常启动

Given 数据库正常
When 应用启动
Then 不产生 .bak 文件

Given 需要定期备份
When 查找定时备份功能
Then 当前不存在（属已知缺口）
```

---

## 五、数据模型设计

### 5.1 数据模型概览

- **数据库类型**：SQLite（WAL 模式）
- **连接参数**：`journal_mode=WAL`、`foreign_keys=ON`、`busy_timeout=5000`
- **主键策略**：所有表使用 `TEXT` 类型的 UUID 作为主键
- **数据库文件**：`app_data_dir()/database.db`（异常时备份为 `database.db.bak`）
- **Schema 定义来源**：`src-tauri/src/db/mod.rs` 的 `init_tables()`（基线表 + 索引）与 `migrate_tables()`（增量列）
- **表总数**：19 张（v2.5 新增 `session_messages`）

| 分组 | 表 |
|------|----|
| 研发资产 | `projects`、`requirements`、`tasks`、`task_comments`、`requirement_history` |
| AI 执行 | `agents`、`agent_sessions`、`skill_execution_logs`、`mcp_execution_logs` |
| Skill / MCP 配置 | `prd_skills`、`project_skills`、`project_mcps`、`stage_skill_configs` |
| 成本与度量 | `token_usage`、`token_budgets` |
| 工程与追溯 | `git_commits` |
| 平台基础 | `settings`、`schema_version` |

### 5.2 表结构明细

#### 5.2.1 `projects` — 项目

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `name` | TEXT | NOT NULL | 项目名称，长度 ≤ 100 |
| `description` | TEXT | NULL | 项目描述 |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |
| `updated_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 更新时间 |
| `key` | TEXT | NULL（迁移新增） | 项目标识，**无 UNIQUE**，长度 ≤ 10 |
| `status` | TEXT | DEFAULT `'active'`（迁移新增） | 状态，**无 CHECK**；约定 `active`/`paused`/`archived` |
| `owner_id` | TEXT | NULL（迁移新增） | 负责人 |
| `start_date` | TEXT | NULL（迁移新增） | 开始日期 |
| `end_date` | TEXT | NULL（迁移新增） | 结束日期 |

**索引**：无（`idx_projects_status` 仅存在于未被程序加载的 SQL 文件中）

#### 5.2.2 `requirements` — 需求

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `project_id` | TEXT | NOT NULL, FK → `projects(id)` | 所属项目 |
| `title` | TEXT | NOT NULL | 标题，长度 ≤ 200 |
| `description` | TEXT | NULL | 描述（富文本 HTML） |
| `priority` | TEXT | CHECK IN ('P0','P1','P2','P3'), DEFAULT 'P1' | 优先级 |
| `status` | TEXT | CHECK IN ('draft','reviewing','approved','rejected','frozen','in_development','testing','completed'), DEFAULT 'draft' | 状态 |
| `prd_content` | TEXT | NULL | PRD 正文 |
| `requirement_type` | TEXT | DEFAULT 'feature' | 约定 feature/bug/optimization/tech_debt，**无 CHECK** |
| `target_version` | TEXT | NULL | 目标版本 |
| `due_date` | TEXT | NULL | 截止日期 |
| `tags` | TEXT | NULL | 标签（注释称 JSON 数组，**实际按纯字符串存储**） |
| `module` | TEXT | NULL（迁移新增） | 所属模块 |
| `source` | TEXT | NULL（迁移新增） | 需求来源，约定 product/tech/customer/internal |
| `source_detail` | TEXT | NULL（迁移新增） | 来源明细 |
| `product_owner` | TEXT | NULL（迁移新增） | 产品负责人 |
| `developer` | TEXT | NULL（迁移新增） | 开发负责人 |
| `tester` | TEXT | NULL（迁移新增） | 测试负责人 |
| `business_priority` | TEXT | NULL（迁移新增） | 业务优先级，约定 critical/high/medium/low |
| `estimated_effort` | REAL | NULL（迁移新增） | 预估工作量（人天） |
| `actual_effort` | REAL | NULL（迁移新增） | 实际工作量（人天） |
| `attachments` | TEXT | NULL（迁移新增） | 附件路径/说明（无文件上传） |
| `notes` | TEXT | NULL（迁移新增） | 备注 |
| `created_at` / `updated_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 时间戳 |

**索引**：`idx_requirements_project(project_id)`、`idx_requirements_status(status)`

#### 5.2.3 `tasks` — 任务

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `requirement_id` | TEXT | FK → `requirements(id)` | 关联需求（可空） |
| `title` | TEXT | NOT NULL | 标题，长度 ≤ 200 |
| `description` | TEXT | NULL | 描述 |
| `task_type` | TEXT | CHECK IN ('requirement','development','testing','bug_fix','review'), DEFAULT 'development' | 任务类型 |
| `status` | TEXT | CHECK IN ('pending','in_progress','completed','blocked'), DEFAULT 'pending' | 状态 |
| `priority` | TEXT | CHECK IN ('P0','P1','P2','P3'), DEFAULT 'P1' | 优先级 |
| `agent_type` | TEXT | NULL | 执行该任务的 Agent 类型 |
| `token_usage` | INTEGER | DEFAULT 0 | 该任务累计 Token |
| `estimated_tokens` | INTEGER | NULL | 预估 Token |
| `assignee` | TEXT | NULL（迁移新增） | 执行人 |
| `due_date` | TEXT | NULL（迁移新增） | 截止日期 |
| `tags` | TEXT | NULL（迁移新增） | 标签 |
| `blocked_reason` | TEXT | NULL（迁移新增） | 阻塞原因 |
| `related_requirement_id` | TEXT | NULL（迁移新增） | 关联需求（冗余） |
| `related_project_id` | TEXT | NULL（迁移新增） | 关联项目（冗余） |
| `priority_level` | TEXT | NULL（迁移新增） | 优先级细分，约定 critical/high/medium/low |
| `notes` | TEXT | NULL（迁移新增） | 备注 |
| `attachments` | TEXT | NULL（迁移新增） | 附件 |
| `estimated_hours` / `actual_hours` | REAL | NULL（迁移新增） | 预估/实际工时 |
| `project_id` | TEXT | NULL（迁移新增，**无 FK**） | 所属项目 |
| `skill_config` / `mcp_config` | TEXT | NULL（迁移新增） | 任务级 Skill/MCP 覆盖配置 |
| `created_at` / `updated_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 时间戳 |

**索引**：`idx_tasks_status`、`idx_tasks_requirement`
> `idx_tasks_project` 仅存在于未被程序加载的 SQL 文件中，运行时**不存在**。

#### 5.2.4 `task_comments` — 任务评论

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `task_id` | TEXT | NOT NULL, FK → `tasks(id)` **ON DELETE CASCADE** | 所属任务 |
| `author` | TEXT | NOT NULL, DEFAULT 'user' | 作者 |
| `content` | TEXT | NOT NULL | 评论内容 |
| `created_at` / `updated_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 时间戳 |

**索引**：`idx_task_comments_task`、`idx_task_comments_created`

#### 5.2.5 `requirement_history` — 需求变更历史（审计）

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `requirement_id` | TEXT | NOT NULL, FK → `requirements(id)` | 关联需求 |
| `action` | TEXT | NOT NULL（无 CHECK） | create / update / delete / status_change |
| `old_value` | TEXT | NULL | 变更前值 |
| `new_value` | TEXT | NULL | 变更后值 |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 发生时间 |

**索引**：`idx_requirement_history_req`
> **无任何写入代码**，该表恒为空，导致审计功能（F053–F055）不可用。

#### 5.2.6 `agents` — Agent 配置

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `name` | TEXT | NOT NULL | 内部名（如 `claude`） |
| `display_name` | TEXT | NOT NULL | 展示名 |
| `binary_path` | TEXT | NOT NULL | 可执行文件路径（**应用层逻辑唯一键**） |
| `version` | TEXT | NULL | 版本号 |
| `status` | TEXT | CHECK IN ('available','running','unavailable'), DEFAULT 'available' | 状态 |
| `capabilities` | TEXT | NULL | 能力标记（含 `worktree_support` 等） |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |

**索引/唯一约束**：无
> 去重由 `db/agents.rs::upsert_agent` 的 `SELECT id FROM agents WHERE binary_path = ?` 保证，**数据库层无 UNIQUE 约束**。

#### 5.2.7 `agent_sessions` — Agent 会话

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `agent_id` | TEXT | NOT NULL, FK → `agents(id)` | 执行 Agent |
| `task_id` | TEXT | FK → `tasks(id)` | 关联任务 |
| `status` | TEXT | CHECK IN ('running','completed','failed'), DEFAULT 'running' | 状态（**不含 created/launching/terminated**） |
| `started_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 开始时间 |
| `ended_at` | DATETIME | NULL | 结束时间 |
| `exit_code` | INTEGER | NULL | 退出码 |
| `duration_seconds` | INTEGER | NULL（迁移新增） | 执行时长（秒） |
| `total_tokens` | INTEGER | DEFAULT 0（迁移新增） | 会话 Token 总量 |
| `total_cost` | REAL | DEFAULT 0（迁移新增） | 会话成本 |
| `project_id` | TEXT | NULL（迁移新增） | 所属项目 |
| `source` | TEXT | DEFAULT 'native'（迁移新增） | 来源 native/imported/manual_import |
| `source_path` | TEXT | NULL（迁移新增） | 导入来源路径 |
| `title` / `description` | TEXT | NULL（迁移新增） | 会话标题/描述 |
| `related_task_id` / `related_requirement_id` | TEXT | NULL（迁移新增） | 关联任务/需求 |
| `skills` / `mcps` | TEXT | NULL（迁移新增） | 使用的 Skill/MCP 列表 |
| `skill_execution_summary` / `mcp_execution_summary` | TEXT | NULL（迁移新增） | 执行摘要（**无写入路径**） |

**索引**：`idx_agent_sessions_agent(agent_id)`
> **注意**：本表**无 `work_dir` 字段**（原 v1.0 文档所列 `work_dir` 实际不存在于此表，仅存在于 PTY 内存态 `PtySessionInfo`）。
> `idx_agent_sessions_task`/`idx_agent_sessions_status` 仅存在于未被程序加载的 SQL 文件中，运行时不存在。

#### 5.2.8 `skill_execution_logs` — Skill 执行日志

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `task_id` | TEXT | FK → `tasks(id)` | 关联任务 |
| `session_id` | TEXT | FK → `agent_sessions(id)` | 关联会话 |
| `requirement_id` | TEXT | FK → `requirements(id)` | 关联需求 |
| `skill_id` | TEXT | NOT NULL, FK → `prd_skills(id)` | Skill ID |
| `skill_name` | TEXT | NOT NULL | Skill 名称（统计分组键） |
| `skill_type` | TEXT | NULL | Skill 类型（当前硬编码 `feature`） |
| `execution_phase` | TEXT | NULL | 约定 prd_generation/task_execution/session_start（仅前者会写入） |
| `input_params` | TEXT | NULL | 输入参数 |
| `output_summary` | TEXT | NULL | 输出摘要 |
| `token_usage` | INTEGER | DEFAULT 0 | Token 消耗 |
| `status` | TEXT | DEFAULT 'success'（无 CHECK） | success/failed/skipped（仅 success 会写入） |
| `executed_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 执行时间 |

**索引**：`idx_skill_exec_task`、`idx_skill_exec_session`

#### 5.2.9 `mcp_execution_logs` — MCP 执行日志

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `session_id` | TEXT | NOT NULL, FK → `agent_sessions(id)` | 关联会话 |
| `mcp_name` | TEXT | NOT NULL | MCP 名称（统计分组键） |
| `endpoint` | TEXT | NULL | 端点 |
| `action` | TEXT | NULL | 动作 |
| `status` | TEXT | DEFAULT 'connected'（无 CHECK） | connected/disconnected/error/call_success/call_failed |
| `error_message` | TEXT | NULL | 错误信息 |
| `duration_ms` | INTEGER | NULL | 耗时（毫秒） |
| `executed_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 执行时间 |

**索引**：`idx_mcp_exec_session`
> **无任何写入代码**，该表恒为空，MCP 日志与统计不可用（F039/F040）。

#### 5.2.10 `prd_skills` — Skill 模板

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID 或种子 ID（如 `skill-feature`） |
| `name` | TEXT | NOT NULL | 名称（无唯一约束） |
| `description` | TEXT | NULL | 描述 |
| `skill_type` | TEXT | NOT NULL | 类型（feature/bugfix/optimization/refactor 等） |
| `template` | TEXT | NOT NULL | 模板正文，支持 `{title}`/`{description}`/`{priority}` 变量 |
| `is_builtin` | INTEGER | DEFAULT 0 | 是否内置（1 = 不可改不可删） |
| `version` | INTEGER | DEFAULT 1（迁移新增） | 版本号（按修改字段数累加） |
| `parent_id` | TEXT | NULL（迁移新增） | 派生来源 Skill ID |
| `is_deleted` | INTEGER | DEFAULT 0（迁移新增） | 软删除标记 |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |

**索引**：无
**种子数据**：`skill-feature`（功能需求 PRD）、`skill-bugfix`（Bug修复 PRD）、`skill-optimization`（技术优化 PRD）、`skill-refactor`（架构重构 PRD）

#### 5.2.11 `project_skills` — 项目-Skill 关联

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `project_id` | TEXT | NOT NULL, FK → `projects(id)` | 项目 |
| `skill_id` | TEXT | NOT NULL, FK → `prd_skills(id)` | Skill |
| `is_default` | INTEGER | DEFAULT 1 | 是否默认 |
| `custom_params` | TEXT | NULL | 自定义参数 |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |

**索引**：`idx_project_skills_project`
> **无 `UNIQUE(project_id, skill_id)`**，重复绑定不会报错。

#### 5.2.12 `project_mcps` — 项目-MCP 关联

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `project_id` | TEXT | NOT NULL, FK → `projects(id)` | 项目 |
| `mcp_name` | TEXT | NOT NULL | MCP 名称 |
| `endpoint` | TEXT | NOT NULL | 服务端点 |
| `description` | TEXT | NULL | 描述 |
| `is_default` | INTEGER | DEFAULT 1 | 是否默认（无唯一约束） |
| `auth_config` | TEXT | NULL | 认证配置（结构化文本，无 schema 校验） |
| `custom_params` | TEXT | NULL | 自定义参数 |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |

**索引**：`idx_project_mcps_project`

#### 5.2.13 `stage_skill_configs` — 环节 Skill 配置

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `project_id` | TEXT | NOT NULL, FK → `projects(id)` | 项目 |
| `stage` | TEXT | NOT NULL（无 CHECK） | 环节值，合法性由后端枚举解析保证 |
| `skill_id` | TEXT | NOT NULL, FK → `prd_skills(id)` | Skill |
| `is_default` | INTEGER | DEFAULT 1 | 是否默认 |
| `custom_params` | TEXT | NULL | 自定义参数 |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |

**索引**：`idx_stage_skill_project`、`idx_stage_skill_stage`、**`idx_stage_skill_unique`（UNIQUE：`project_id, stage`）**
> 唯一索引是 `upsert_stage_skill_config` 执行 `ON CONFLICT(project_id, stage) DO UPDATE` 的前提。

#### 5.2.14 `token_usage` — Token 消耗记录

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `session_id` | TEXT | FK → `agent_sessions(id)` | 关联会话 |
| `task_id` | TEXT | FK → `tasks(id)` | 关联任务 |
| `agent_type` | TEXT | NULL | Agent 类型（本地同步写 `claude-code`） |
| `model` | TEXT | NULL | 模型名 |
| `input_tokens` | INTEGER | DEFAULT 0 | 输入 Token |
| `output_tokens` | INTEGER | DEFAULT 0 | 输出 Token |
| `cache_creation_tokens` | INTEGER | DEFAULT 0（迁移新增） | 缓存写入 Token |
| `cache_read_tokens` | INTEGER | DEFAULT 0（迁移新增） | 缓存读取 Token |
| `thinking_tokens` | INTEGER | DEFAULT 0（迁移新增） | 思考 Token |
| `total_cost` | REAL | DEFAULT 0 | 成本（当前恒 0，未做价格换算） |
| `source` | TEXT | DEFAULT 'api' | 来源 api / 本地同步 |
| `recorded_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 记录时间（趋势聚合依据） |

**索引**：`idx_token_usage_session(session_id)`、`idx_token_usage_recorded(recorded_at)`
> **已知风险**：旧库经 `ALTER` 追加 `cache_*` 列后列位于表尾，而 `db/tokens.rs::get_token_usage` 使用 `SELECT *` 并按固定下标读取，**可能发生列错位**（cache/total_cost/source/recorded_at 取值异常）。

#### 5.2.15 `token_budgets` — Token 预算

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `scope` | TEXT | NOT NULL, DEFAULT 'global' | 作用域（当前仅支持 global） |
| `monthly_limit` | INTEGER | NOT NULL, DEFAULT 2000000 | 月度上限 |
| `alert_threshold` | REAL | NOT NULL, DEFAULT 0.8 | 告警阈值（比例） |
| `current_usage` | INTEGER | DEFAULT 0 | 当前用量（无自动累计） |
| `reset_date` | TEXT | NULL | 重置日期（无自动重置逻辑） |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |

**索引**：无
**种子数据**：1 条 `scope='global'`、`monthly_limit=2000000`、`alert_threshold=0.8`

#### 5.2.16 `git_commits` — Git 提交记录

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `id` | TEXT | PRIMARY KEY | UUID |
| `task_id` | TEXT | FK → `tasks(id)` | 关联任务 |
| `session_id` | TEXT | NULL | 关联会话 |
| `commit_hash` | TEXT | NOT NULL | commit SHA |
| `branch_name` | TEXT | NULL | 分支名 |
| `message` | TEXT | NULL | 提交信息 |
| `files_changed` | INTEGER | DEFAULT 0 | 变更文件数 |
| `created_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 创建时间 |

**索引**：`idx_git_commits_task`
> **无任何写入与查询代码**，该表恒为空（F060 未实现）。

#### 5.2.17 `settings` — 系统配置

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `key` | TEXT | PRIMARY KEY | 配置键 |
| `value` | TEXT | NOT NULL | 配置值（一切皆为字符串） |
| `updated_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 更新时间 |

**索引**：无
**已知键**：`claude_api_key`、`session_mode`（默认 `auto`）

#### 5.2.18 `schema_version` — 数据库版本

| 字段 | 类型 | 约束/默认 | 说明 |
|------|------|-----------|------|
| `version` | INTEGER | PRIMARY KEY | 版本号 |
| `applied_at` | DATETIME | DEFAULT CURRENT_TIMESTAMP | 应用时间 |

**索引**：无
> 由 `db/migration.rs` 惰性创建；`Database::new()` 中**硬编码写入 `version = 8`**，仅在该版本不存在时 INSERT，故最多只有一行 `version=8`。

### 5.3 索引清单（运行时实际创建）

| 索引 | 表 | 列 | 类型 |
|------|----|----|------|
| `idx_requirements_project` | `requirements` | `project_id` | 普通 |
| `idx_requirements_status` | `requirements` | `status` | 普通 |
| `idx_tasks_status` | `tasks` | `status` | 普通 |
| `idx_tasks_requirement` | `tasks` | `requirement_id` | 普通 |
| `idx_agent_sessions_agent` | `agent_sessions` | `agent_id` | 普通 |
| `idx_token_usage_session` | `token_usage` | `session_id` | 普通 |
| `idx_token_usage_recorded` | `token_usage` | `recorded_at` | 普通 |
| `idx_requirement_history_req` | `requirement_history` | `requirement_id` | 普通 |
| `idx_git_commits_task` | `git_commits` | `task_id` | 普通 |
| `idx_task_comments_task` | `task_comments` | `task_id` | 普通 |
| `idx_task_comments_created` | `task_comments` | `created_at` | 普通 |
| `idx_project_skills_project` | `project_skills` | `project_id` | 普通 |
| `idx_project_mcps_project` | `project_mcps` | `project_id` | 普通 |
| `idx_skill_exec_task` | `skill_execution_logs` | `task_id` | 普通 |
| `idx_skill_exec_session` | `skill_execution_logs` | `session_id` | 普通 |
| `idx_mcp_exec_session` | `mcp_execution_logs` | `session_id` | 普通 |
| `idx_stage_skill_project` | `stage_skill_configs` | `project_id` | 普通 |
| `idx_stage_skill_stage` | `stage_skill_configs` | `stage` | 普通 |
| `idx_stage_skill_unique` | `stage_skill_configs` | `project_id, stage` | **UNIQUE** |

> **仅存在于未加载 SQL 文件中的索引（运行时不创建）**：`idx_tasks_project`、`idx_agent_sessions_task`、`idx_agent_sessions_status`、`idx_projects_status`。

### 5.4 外键与删除行为

| 子表 | 外键字段 | 父表 | 删除行为 |
|------|----------|------|----------|
| `requirements` | `project_id` | `projects` | 无级联（默认 RESTRICT/NO ACTION） |
| `tasks` | `requirement_id` | `requirements` | 无级联 |
| `task_comments` | `task_id` | `tasks` | **ON DELETE CASCADE** |
| `agent_sessions` | `agent_id` | `agents` | 无级联 |
| `agent_sessions` | `task_id` | `tasks` | 无级联 |
| `skill_execution_logs` | `task_id` / `session_id` / `requirement_id` / `skill_id` | `tasks` / `agent_sessions` / `requirements` / `prd_skills` | 无级联 |
| `mcp_execution_logs` | `session_id` | `agent_sessions` | 无级联 |
| `project_skills` | `project_id` / `skill_id` | `projects` / `prd_skills` | 无级联 |
| `project_mcps` | `project_id` | `projects` | 无级联 |
| `stage_skill_configs` | `project_id` / `skill_id` | `projects` / `prd_skills` | 无级联 |
| `token_usage` | `session_id` / `task_id` | `agent_sessions` / `tasks` | 无级联 |
| `git_commits` | `task_id` | `tasks` | 无级联 |
| `tasks.project_id` | （无约束） | — | 无外键 |

> `foreign_keys=ON` 已启用，因此插入不存在的外键值会**直接失败**；删除被引用的父记录会**被拒绝**（除 `task_comments` 的级联删除外）。

### 5.5 状态枚举汇总

| 维度 | 取值 | 约束方式 |
|------|------|----------|
| 需求 `status` | `draft`、`reviewing`、`approved`、`rejected`、`frozen`、`in_development`、`testing`、`completed` | SQL CHECK + 流转校验 |
| 需求 `priority` | `P0`、`P1`、`P2`、`P3`（默认 P1） | SQL CHECK + 后端校验 |
| 需求 `requirement_type` | `feature`、`bug`、`optimization`、`tech_debt`（默认 feature） | 仅约定/LLM 提示词，无 CHECK |
| 需求 `business_priority` | `critical`、`high`、`medium`、`low` | 仅约定，无 CHECK |
| 需求 `source` | `product`、`tech`、`customer`、`internal` | 仅约定，无 CHECK |
| 任务 `status` | `pending`、`in_progress`、`completed`、`blocked`（默认 pending） | SQL CHECK + 流转校验 |
| 任务 `task_type` | `requirement`、`development`、`testing`、`bug_fix`、`review`（默认 development） | SQL CHECK + 后端校验 |
| 任务 `priority` | `P0`、`P1`、`P2`、`P3`（默认 P1） | SQL CHECK + 后端校验 |
| 任务 `priority_level` | `critical`、`high`、`medium`、`low` | 仅约定，无 CHECK |
| 会话 `status` | `running`、`completed`、`failed`（默认 running） | SQL CHECK + 流转校验 |
| 会话 `source` | `native`、`imported`、`manual_import`（默认 native） | 仅约定，无 CHECK |
| Agent `status` | `available`、`running`、`unavailable`（默认 available） | SQL CHECK |
| 项目 `status` | `active`（DB 默认）；`paused`/`archived` 为前端约定 | **无 CHECK** |
| 环节 `stage` | `requirement_analysis`、`prd_generation`、`task_planning`、`code_generation`、`testing`、`review` | 后端 `PipelineStage` 枚举 |
| `skill_execution_logs.status` | `success`（默认）、`failed`、`skipped` | 无 CHECK，仅 success 会被写入 |
| `mcp_execution_logs.status` | `connected`（默认）、`disconnected`、`error`、`call_success`、`call_failed` | 无 CHECK，无写入 |
| PTY 内存态 `status` | `running`、`completed`、`terminated` | 内存态，不入库 |

### 5.6 迁移机制

**双轨并存**，需注意二者独立：

| 机制 | 位置 | 是否生效 |
|------|------|----------|
| 正式迁移 API | `db/migration.rs::apply_migration`、`get_version_info` | ❌ 均标注 `#[allow(dead_code)]`，**无调用点** |
| 实际迁移路径 | `db/mod.rs::init_tables()` → `migrate_tables()` → `seed_data()` | ✅ 运行时执行 |
| SQL 迁移文件 | `db/migrations/001_init.sql ~ 012_task_fields.sql`（12 个） | ❌ **无 `include_str!` 引用，属死文件** |

**实际执行流程**：

```
Database::new()
  ├─ 打开连接：journal_mode=WAL / foreign_keys=ON / busy_timeout=5000
  ├─ init_tables()
  │    ├─ CREATE TABLE IF NOT EXISTS（基线表 + 索引）
  │    ├─ migrate_prd_skills()（补 version / parent_id / is_deleted）
  │    └─ migrate_requirements()（补 module 等 11 列）
  ├─ migration::set_version(conn, 8)   ← 硬编码版本号
  ├─ migrate_tables()
  │    └─ pragma_table_info 检查列存在性 → ALTER TABLE ADD COLUMN
  │       （requirements 4 列、tasks 15 列、token_usage 3 列、
  │         agent_sessions 3 + 11 列、projects 5 列）
  │    └─ sqlite_master 检查表 → 整块建表
  │       （project_skills / project_mcps / skill_execution_logs /
  │         mcp_execution_logs / stage_skill_configs）
  └─ seed_data()（默认项目 / 默认预算 / 4 个内置 Skill）
```

**结论**：当前迁移是"幂等 `ALTER` + 硬编码版本号 8"的过渡方案；`schema_version` 只作占位，**不具备按版本号顺序升级的能力**。

### 5.7 种子数据

| 表 | 数据 | 说明 |
|----|------|------|
| `projects` | `default-project` / "默认项目" | `init_tables()` 与 `seed_data()` 两处均有插入逻辑（其中 description 文案不同），均以 `count > 0` 守卫，不会重复 |
| `token_budgets` | `scope='global'`、`monthly_limit=2000000`、`alert_threshold=0.8` | 默认预算 |
| `prd_skills` | 4 条内置 Skill | `skill-feature` / `skill-bugfix` / `skill-optimization` / `skill-refactor` |

---

## 六、IPC 命令清单

### 6.1 命令总览

- **注册位置**：`src-tauri/src/main.rs` 的 `generate_handler!`
- **命令总数**：**88 条**（`commands.rs` 86 条 + `system_monitor.rs` 2 条），与定义处一一对应，无遗漏、无多余
  - v2.5 新增 6 条（模块 16 / P0）：`start_structured_session`、`send_session_message`、`list_session_messages`、`interrupt_session`、`set_session_mode`、`resume_session`
- **调用约定**：前端通过 `@tauri-apps/api/core` 的 `invoke(command_name, args)` 调用；`State<'_, AppState>` 由 Tauri 注入，前端不传

| 分组 | 数量 | 命令 |
|------|:---:|------|
| 需求管理 | 8 | `create_requirement`、`list_requirements`、`get_requirement`、`update_requirement`、`delete_requirement`、`generate_prd`、`generate_prd_streaming`、`enrich_requirement` |
| 任务管理 | 8 | `create_task`、`list_tasks`、`update_task_status`、`delete_task`、`get_task`、`update_task`、`list_tasks_by_project`、`list_tasks_by_requirement` |
| 任务评论 | 3 | `create_task_comment`、`list_task_comments`、`delete_task_comment` |
| Agent 管理 | 7 | `discover_agents`、`get_agents`、`verify_agents`、`start_agent`、`create_session_from_task`、`stop_agent`、`get_agent_sessions` |
| Token 统计 | 7 | `get_token_usage`、`get_token_usage_summary`、`get_token_usage_trend`、`get_token_usage_by_model`、`get_token_budget`、`set_token_budget`、`sync_local_usage` |
| PRD Skill | 6 | `list_prd_skills`、`get_prd_skill`、`create_prd_skill`、`update_prd_skill`、`delete_prd_skill`、`create_custom_skill_from_builtin` |
| 项目管理 | 5 | `create_project`、`list_projects`、`get_project`、`update_project`、`delete_project` |
| 项目 Skill 配置 | 3 | `list_project_skills`、`add_project_skill`、`remove_project_skill` |
| 项目 MCP 配置 | 3 | `list_project_mcps`、`add_project_mcp`、`remove_project_mcp` |
| 流水线/环节配置 | 5 | `list_stage_skill_configs`、`get_stage_skill_config`、`upsert_stage_skill_config`、`remove_stage_skill_config`、`get_pipeline_stages` |
| 执行日志 | 4 | `get_skill_execution_logs`、`get_mcp_execution_logs`、`get_skill_usage_stats`、`get_mcp_usage_stats` |
| PTY 终端 | 6 | `pty_spawn`、`pty_write`、`pty_resize`、`pty_kill`、`pty_get_sessions`、`pty_get_session` |
| Git 集成 | 5 | `git_get_status`、`git_get_diff`、`git_create_commit`、`git_log`、`list_git_commits` |
| 数据导入导出 | 2 | `export_data`、`import_data` |
| 统计 | 1 | `get_statistics` |
| 设置 | 2 | `get_setting`、`set_setting` |
| 安全 | 2 | `encrypt_api_key`、`decrypt_api_key` |
| 缓存 | 2 | `get_cache_stats`、`clear_cache` |
| 审计 | 1 | `get_audit_logs` |
| 会话交互与消息 | 6 | `start_structured_session`、`send_session_message`、`list_session_messages`、`interrupt_session`、`set_session_mode`、`resume_session` |
| 系统监控 | 2 | `get_monitor_data`、`check_alerts` |
> **说明**：v1.0 曾列出"系统（6 个）"，实际系统类命令为 `get_statistics`、`get_cache_stats`、`clear_cache`、`get_setting`、`set_setting`、`encrypt_api_key`、`decrypt_api_key`、`get_audit_logs` 共 8 条，另加 `get_monitor_data`、`check_alerts` 2 条。

### 6.2 命令明细（入参 / 出参 / 错误场景）

> 返回类型统一为 `Result<T, String>`，错误以中文字符串返回。

#### 6.2.1 需求管理（7）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `create_requirement` | `title`、`description?`、`priority`、`project_id?`、`module?`、`source?`、`source_detail?`、`product_owner?`、`developer?`、`tester?`、`business_priority?`、`estimated_effort?`(f64)、`tags?`、`notes?` | `Requirement` | 标题为空/超 200、优先级非法、项目外键不存在 |
| `list_requirements` | `page?`(u32)、`page_size?`(u32)、`status?` | `PagedResult<Requirement>` | 分页参数越界（上限 100） |
| `get_requirement` | `id` | `Option<Requirement>` | 查询失败 |
| `update_requirement` | `id`、`title?`、`description?`、`priority?`、`status?`、`prd_content?` | `()` | 状态流转非法、标题超长 |
| `delete_requirement` | `id` | `()` | 存在外键引用（关联任务）时可能失败 |
| `generate_prd` | `requirement_id`、`skill_id?` | `String`（PRD 正文） | 未配置 API Key、需求不存在、API 调用失败 |
| `generate_prd_streaming` | `requirement_id`、`skill_id?`（`app: AppHandle` 由 Tauri 注入） | `String`（PRD 正文）；过程事件 `prd:chunk` / `prd:done` | 同 `generate_prd`；读取流失败 |
| `enrich_requirement` | `requirement_id` | `serde_json::Value` | 未配置 API Key、需求不存在 |

#### 6.2.2 任务管理（8）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `create_task` | `title`、`requirement_id?`、`task_type`、`priority`、`assignee?`、`due_date?`、`estimated_hours?`(f64)、`priority_level?`、`blocked_reason?`、`tags?`、`notes?` | `Task` | 标题为空/超 200、类型或优先级非法、需求外键不存在 |
| `list_tasks` | `status?`、`task_type?` | `Vec<Task>` | —（未分页） |
| `get_task` | `id` | `Option<Task>` | — |
| `update_task` | `id`、`title?`、`description?`、`task_type?`、`priority?`、`assignee?`、`due_date?`、`tags?`、`project_id?` | `()` | 字段非法 |
| `update_task_status` | `id`、`status` | `()` | 状态流转非法；`session_mode='auto'` 时可能触发自动启动 Agent |
| `delete_task` | `id` | `()` | — |
| `list_tasks_by_project` | `project_id` | `Vec<Task>` | 无匹配时返回空数组 |
| `list_tasks_by_requirement` | `requirement_id` | `Vec<Task>` | 无匹配时返回空数组 |

#### 6.2.3 任务评论（3）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `create_task_comment` | `task_id`、`author`、`content` | `TaskComment` | 任务外键不存在、内容为空 |
| `list_task_comments` | `task_id` | `Vec<TaskComment>` | — |
| `delete_task_comment` | `id` | `()` | 记录不存在时静默成功 |

#### 6.2.4 Agent 管理（7）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `discover_agents` | — | `Vec<Agent>` | 扫描失败（返回空数组） |
| `get_agents` | — | `Vec<Agent>` | — |
| `verify_agents` | — | `Vec<Agent>`（复核后） | —；会将无进程的 `running` 会话纠正为 `failed`，并对失效二进制标记 `unavailable` |
| `start_agent` | `agent_path`、`task_id?` | `AgentSession` | Agent 不可用、DB 会话创建失败；**PTY 启动失败仅 warn 不报错** |
| `stop_agent` | `session_id` | `()` | 会话不存在；**不终止操作系统进程** |
| `create_session_from_task` | `task_id` | `AgentSession` | 无可用 Agent、任务不存在 |
| `get_agent_sessions` | `agent_id?` | `Vec<AgentSession>` | — |

#### 6.2.5 PTY 终端（6）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `pty_spawn` | `agent_path`、`work_dir`、`env_vars?`(`Vec<(String,String)>`) | `String`（session_id） | 路径不存在、work_dir 不存在、openpty 失败 |
| `pty_write` | `session_id`、`input` | `()` | 会话不存在；**当前仅回显，未写入子进程** |
| `pty_resize` | `session_id`、`rows`(u16)、`cols`(u16) | `()` | 会话不存在；**当前未真正调整尺寸** |
| `pty_kill` | `session_id` | `()` | 会话不存在；**当前未 kill 子进程** |
| `pty_get_sessions` | — | `Vec<PtySessionInfo>` | — |
| `pty_get_session` | `session_id` | `Option<PtySessionInfo>` | — |

#### 6.2.6 Token 统计（7）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `get_token_usage` | `task_id?` | `Vec<TokenUsage>` | 旧库列错位风险（见 5.2.14） |
| `get_token_usage_summary` | — | `TokenUsageSummary` | —（缓存 60s） |
| `get_token_usage_trend` | `days`(i32) | `Vec<DailyUsage>` | 负数/0 时无数据 |
| `get_token_usage_by_model` | — | `Vec<ModelUsage>` | — |
| `get_token_budget` | — | `Option<TokenBudget>` | 无记录返回 `None` |
| `set_token_budget` | `monthly_limit`(i64)、`alert_threshold`(f64) | `()` | 无校验，0/负数会被接受 |
| `sync_local_usage` | — | `i64`（新增条数） | 目录不存在返回 0 |

#### 6.2.7 PRD Skill（6）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `list_prd_skills` | — | `Vec<PrdSkill>` | — |
| `get_prd_skill` | `id` | `Option<PrdSkill>` | — |
| `create_prd_skill` | `name`、`skill_type`、`description?`、`template` | `PrdSkill` | `template` 为空（NOT NULL） |
| `update_prd_skill` | `id`、`name?`、`description?`、`template?` | `()` | 内置 Skill 不可修改 |
| `delete_prd_skill` | `id` | `()` | 内置 Skill 不可删除（自定义为软删除） |
| `create_custom_skill_from_builtin` | `parent_id`、`name`、`template?` | `PrdSkill` | 父 Skill 不存在、父 Skill 非内置 |

#### 6.2.8 项目管理与项目配置（11）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `create_project` | `name`、`description?`、`key?`、`status?`、`owner_id?`、`start_date?`、`end_date?` | `Project` | 名称空/超 100 |
| `list_projects` | — | `Vec<Project>` | — |
| `get_project` | `id` | `Option<Project>` | — |
| `update_project` | `id`、`name?`、`description?`、`key?`、`status?`、`owner_id?`、`start_date?`、`end_date?` | `()` | 名称超 100 |
| `delete_project` | `id` | `()` | 存在关联需求时失败（无级联） |
| `list_project_skills` | `project_id` | `Vec<ProjectSkill>` | — |
| `add_project_skill` | `project_id`、`skill_id`、`is_default`、`custom_params?` | `ProjectSkill` | 外键不存在；重复绑定不报错 |
| `remove_project_skill` | `id` | `()` | 记录不存在时静默成功 |
| `list_project_mcps` | `project_id` | `Vec<ProjectMcp>` | — |
| `add_project_mcp` | `project_id`、`mcp_name`、`endpoint`、`description?`、`is_default`、`auth_config?`、`custom_params?` | `ProjectMcp` | 名称为空/端点为空 |
| `remove_project_mcp` | `id` | `()` | 记录不存在时静默成功 |

#### 6.2.9 流水线/环节配置（5）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `get_pipeline_stages` | — | `Vec<PipelineStageOption>` | — |
| `list_stage_skill_configs` | `project_id` | `Vec<StageSkillConfig>` | — |
| `get_stage_skill_config` | `project_id`、`stage` | `Option<StageSkillConfig>` | — |
| `upsert_stage_skill_config` | `project_id`、`stage`、`skill_id`、`is_default`、`custom_params?` | `StageSkillConfig` | stage 非法；唯一索引缺失时 upsert 失败 |
| `remove_stage_skill_config` | `project_id`、`stage` | `()` | 记录不存在时静默成功 |

#### 6.2.10 执行日志与统计（5）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `get_skill_execution_logs` | `task_id?`、`session_id?` | `Vec<SkillExecutionLog>` | 未传参时行为以实现为准 |
| `get_mcp_execution_logs` | `session_id` | `Vec<McpExecutionLog>` | 表恒空 |
| `get_skill_usage_stats` | — | `Vec<Value>`（`skill_name`/`skill_type`/`use_count`/`total_tokens`/`success_count`） | — |
| `get_mcp_usage_stats` | — | `Vec<Value>`（`mcp_name`/`call_count`/`success_count`/`avg_duration`） | 表恒空 |
| `get_statistics` | — | `serde_json::Value` | —（缓存 300s） |

#### 6.2.11 Git 集成（4）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `git_get_status` | `work_dir` | `{branch, files[], total_changes}` | 目录不存在、非 Git 仓库、`git` 不可用 |
| `git_get_diff` | `work_dir`、`file_path?` | `String` | 非 Git 仓库 |
| `git_create_commit` | `work_dir`、`message`、`_task_id?`、`_session_id?`（**未使用**） | `{commit_hash, short_hash, message, files_changed, diff_stat}` | 无非暂存改动、非 Git 仓库 |
| `git_log` | `work_dir`、`limit?`（默认 50） | `Vec<{hash, short_hash, message, author, date}>` | 空仓库、非 Git 仓库 |

#### 6.2.12 系统 / 设置 / 安全 / 缓存 / 审计 / 监控（10）

| 命令 | 入参 | 出参 | 错误场景 |
|------|------|------|----------|
| `get_setting` | `key` | `Option<String>` | —（缓存 300s） |
| `set_setting` | `key`、`value` | `()` | 无白名单校验 |
| `encrypt_api_key` | `api_key`、`password` | `String`（base64 密文） | 口令为空 |
| `decrypt_api_key` | `encrypted`、`password` | `String`（明文） | 口令错误、密文长度不足 |
| `get_cache_stats` | — | `CacheStats`（`hits`/`misses`/`evictions`/`size`/`l1_size`/`hit_rate`） | — |
| `clear_cache` | — | `()` | — |
| `get_audit_logs` | `limit?`（默认 200） | `Vec<{...}>` | 数据源恒空 |
| `get_monitor_data` | — | `SystemMonitorData` | 系统信息读取失败 |
| `check_alerts` | — | `Vec<Alert>` | — |

### 6.3 Tauri 事件通道

| 事件名 | 载荷 | 触发时机 | 前端监听位置 |
|--------|------|----------|--------------|
| `pty:event` | `{type:"data", session_id, data}` / `{type:"exit", session_id, exit_code}` / `{type:"error", session_id, error}` | PTY 输出、进程退出、读取错误 | `components/AgentTerminal.tsx` |
| `prd:chunk` | `{requirement_id, delta}` | 流式生成 PRD 时每解析出一个增量文本片段 | `pages/RequirementDetail.tsx` |
| `prd:done` | `{requirement_id, content}` | 流式生成 PRD 完成（已落库） | `pages/RequirementDetail.tsx` |
| `session:message` | `{session_id, message: SessionMessage}` | 一个完整消息块落库后（正文/思考/工具/结果/被拒/错误/系统） | `stores/sessionMessagesStore.ts` |
| `session:delta` | `{session_id, block_type: "text"\|"thinking"\|"thinking_tokens", delta, total?}` | 结构化会话的增量片段（不落库，仅实时渲染） | `stores/sessionMessagesStore.ts` |
| `session:tool` | `{session_id, phase: "start"\|"result", tool_use_id?, tool_name?, content?, is_error?}` | 工具调用开始 / 工具结果返回 | `stores/sessionMessagesStore.ts` |
| `session:result` | `{session_id, input_tokens, output_tokens, cache_read_input_tokens, cost_usd?, duration_ms?, is_error}` | 单轮结束（`result` 事件） | `stores/sessionMessagesStore.ts` |
| `session:error` | `{session_id, error}` | 会话进程输出读取失败等致命错误 | `stores/sessionMessagesStore.ts` |
| `session:notice` | `{session_id, level, kind, message, tool_name?}` | 平台级提示（进程退出、权限被拒、解析告警） | `stores/sessionMessagesStore.ts` |

> **不存在 `pty:data` 独立事件**；输出数据以 `pty:event` 中 `type="data"` 的载荷下发。后端通道容量 100（`mpsc::channel(100)`）。
> 流式 PRD 事件的推送端为 `services/prd_service.rs::generate_prd_streaming`，前端须在调用命令**之前**完成 `listen("prd:chunk")` 注册。
> 会话事件由 `session_driver.rs` 推送；前端在 `useSessionMessagesStore.subscribe(sessionId)` 中注册监听并**按 `session_id` 过滤**（多会话并行时不串扰）。

### 6.4 前端调用封装映射（`src/lib/tauri.ts`）

| 封装函数 | 后端命令 | 备注 |
|----------|----------|------|
| `createRequirement` / `listRequirements` / `getRequirement` / `updateRequirement` / `deleteRequirement` / `generatePrd` / `enrichRequirement` | 同名 snake_case | `listRequirements` 从 `PagedResult.items` 提取数组 |
| `createTask` / `listTasks` / `getTask` / `updateTask` / `updateTaskStatus` / `deleteTask` / `listTasksByProject` / `listTasksByRequirement` | 同名 snake_case | — |
| `createTaskComment` / `listTaskComments` / `deleteTaskComment` | 同名 snake_case | — |
| `discoverAgents` / `getAgents` / `startAgent` / `stopAgent` / `createSessionFromTask` / `getAgentSessions` | 同名 snake_case | — |
| `getTokenUsageSummary` / `getTokenUsageTrend` / `getTokenUsageByModel` / `getTokenBudget` / `setTokenBudget` / `syncLocalUsage` | 同名 snake_case | — |
| `listPrdSkills` / `getPrdSkill` / `createPrdSkill` / `updatePrdSkill` / `deletePrdSkill` / `createCustomSkillFromBuiltin` | 同名 snake_case | — |
| `createProject` / `listProjects` / `getProject` / `updateProject` / `deleteProject` | 同名 snake_case | — |
| `listProjectSkills` / `addProjectSkill` / `removeProjectSkill` | 同名 snake_case | — |
| `listProjectMcps` / `addProjectMcp` / `removeProjectMcp` | 同名 snake_case | — |
| `getPipelineStages` / `listStageSkillConfigs` / `getStageSkillConfig` / `upsertStageSkillConfig` / `removeStageSkillConfig` | 同名 snake_case | — |
| `getSkillExecutionLogs` / `getMcpExecutionLogs` | 同名 snake_case | — |
| `gitGetStatus` / `gitGetDiff` / `gitCreateCommit` / `gitLog` | 同名 snake_case | — |
| `getStatistics` / `getSetting` / `setSetting` / `encryptApiKey` / `decryptApiKey` / `getAuditLogs` | 同名 snake_case | — |

**绕过 `tauri.ts` 的直接 `invoke` 调用**：

| 调用位置 | 命令 | 用途 |
|----------|------|------|
| `components/AgentTerminal.tsx` | `pty_write` / `pty_resize` / `pty_kill` | 终端输入、尺寸调整、终止会话 |
| `components/Layout.tsx` | `get_monitor_data` | 状态栏每 5 秒轮询系统监控 |
| `pages/SystemMonitor.tsx` | `get_monitor_data` / `check_alerts` | 系统监控页数据与告警检查 |

> `src/lib/tauri.ts` 内还重复定义了 `StageSkillConfig`、`PipelineStageOption`、`AuditLogEntry` 接口（与 `src/lib/types.ts` 存在重复定义）。

---

## 七、前端页面与交互

### 7.1 路由清单（已注册 18 条）

路由定义集中在 `src/App.tsx`，使用 `HashRouter` + 嵌套路由，父路由为 `Layout`。

| # | 路由 | 页面组件 | 文件 | 参数 | 对应功能 |
|---|------|----------|------|:---:|----------|
| 1 | `/`（index） | `Workspace` | `pages/Workspace.tsx` | — | F063、F046 |
| 2 | `/projects` | `Projects` | `pages/Projects.tsx` | — | F001、F002 |
| 3 | `/projects/:id` | `ProjectDetail` | `pages/ProjectDetail.tsx` | `:id` | F003、F004、F005、F016 |
| 4 | `/requirements` | `Requirements` | `pages/Requirements.tsx` | — | F006、F012、F065 |
| 5 | `/requirements/:id` | `RequirementDetail` | `pages/RequirementDetail.tsx` | `:id` | F007、F008、F009、F010、F016 |
| 6 | `/board` | `Board` | `pages/Board.tsx` | — | F013、F014、F018 |
| 7 | `/tasks/:id` | `TaskDetail` | `pages/TaskDetail.tsx` | `:id` | F014、F015、F017、F028 |
| 8 | `/skills` | `SkillManagement` | `pages/SkillManagement.tsx` | — | F031–F034 |
| 9 | `/agents` | `Agents` | `pages/Agents.tsx` | — | F019–F021、F025 |
| 10 | `/sessions` | `Sessions` | `pages/Sessions.tsx` | — | F026、F029、F030 |
| 11 | `/sessions/:id` | `SessionDetail` | `pages/SessionDetail.tsx` | `:id` | F027、F035、F039 |
| 12 | `/commits` | `Commits` | `pages/Commits.tsx` | — | F056–F059 |
| 13 | `/cost` | `Cost` | `pages/Cost.tsx` | — | F041–F044、F048 |
| 14 | `/statistics` | `Statistics` | `pages/Statistics.tsx` | — | F036、F040、F046 |
| 15 | `/audit-log` | `AuditLog` | `pages/AuditLog.tsx` | — | F054、F055 |
| 16 | `/import-export` | `ImportExport` | `pages/ImportExport.tsx` | — | F073、F074 |
| 17 | `/system-monitor` | `SystemMonitor` | `pages/SystemMonitor.tsx` | — | F045、系统监控 |
| 18 | `/settings` | `Settings` | `pages/Settings.tsx` | — | F061、F062、F072 |

> **路由变更说明（相对 v1.0 文档）**：
> - v1.0 所列 `/tasks`（任务列表）实际为 **`/board`**（任务看板承担列表职责），仅保留 `/tasks/:id` 详情路由
> - v1.0 所列 `/audit` 实际为 **`/audit-log`**
> - v1.0 所列 `/monitor` 实际为 **`/system-monitor`**
> - v1.0 所列 `/workspace` 实际为 **`/`**（index 路由）
> - 无 `*` 兜底/404 路由

### 7.2 未挂载页面

| 文件 | 组件 | 说明 |
|------|------|------|
| `pages/Dashboard.tsx` | `Dashboard` | 仪表盘：最近需求、进行中任务、Agent 状态。**存在但未在 `App.tsx` 注册路由**，是唯一孤儿页面；与 `Workspace`（F063）功能重叠 |

### 7.3 页面交互设计

#### 7.3.1 工作台 `/`（Workspace）

| 项目 | 内容 |
|------|------|
| 页面元素 | 统计卡片区（需求/任务/Token）、项目入口列表、最近活动列表 |
| 关键交互 | 点击统计卡跳转对应列表；点击项目进入 `/projects/:id` |
| 加载态 | `StatCardSkeleton` / `CardSkeleton` |
| 空态 | 各区块显示空态引导文案 |
| 错误态 | 显示错误提示，不白屏 |
| 数据来源 | `get_statistics`（300s 缓存）、`list_projects` |

#### 7.3.2 项目 `/projects` 与 `/projects/:id`

| 项目 | 内容 |
|------|------|
| 页面元素 | 列表页：项目卡片、搜索框、「新建项目」按钮。详情页：基本信息、需求 Tab、任务 Tab、Skill 配置 Tab、MCP 配置 Tab、流水线环节配置区 |
| 关键交互 | 搜索（名称/描述）、新建/编辑/删除、Tab 切换、添加/移除 Skill 与 MCP、设置默认项、按环节选择 Skill |
| 加载态 | 卡片骨架屏 |
| 空态 | 无项目时展示引导；Tab 内无数据时展示空态 |
| 错误态 | Toast 提示后端错误（如名称超长） |
| 约束 | 项目 Key 输入框 `maxLength=10` |

#### 7.3.3 需求 `/requirements` 与 `/requirements/:id`

| 项目 | 内容 |
|------|------|
| 页面元素 | 列表页：状态筛选、`Pagination`、需求行（标题/优先级/状态徽章）、「新建需求」。详情页：基本信息、扩展字段、PRD 区（生成按钮 + Skill 选择器）、关联任务区 |
| 关键交互 | 状态筛选、分页翻页、新建/编辑（`RequirementForm` 弹窗）、生成 PRD、智能完善、状态流转 |
| 加载态 | 列表骨架屏；PRD 生成期间按钮进入 loading |
| 空态 | 无需求时展示空态 |
| 错误态 | 表单校验错误就地提示；后端错误 Toast |
| 数据来源 | `list_requirements`（分页）、`get_requirement`、`generate_prd`、`enrich_requirement` |

#### 7.3.4 任务 `/board` 与 `/tasks/:id`

| 项目 | 内容 |
|------|------|
| 页面元素 | 看板列（按状态分列）、任务卡片、「新建任务」。详情页：任务信息、状态操作、评论区、附件列表、「启动 Agent」 |
| 关键交互 | 拖拽/按钮更新状态、新建/编辑（`TaskForm` 弹窗）、添加/删除评论、从任务创建会话 |
| 加载态 | 看板骨架屏 |
| 空态 | 各状态列无任务时展示空列 |
| 错误态 | 状态流转非法时 Toast 提示 |
| 数据来源 | `list_tasks`、`update_task_status`、`list_task_comments`、`create_session_from_task` |

#### 7.3.5 Skill `/skills`

| 项目 | 内容 |
|------|------|
| 页面元素 | Skill 列表（内置/自定义标记）、模板内容折叠区、操作按钮（新建/编辑/删除/派生） |
| 关键交互 | 展开查看模板、创建自定义版本、编辑（内置禁用）、软删除 |
| 加载态 | 列表骨架屏 |
| 空态 | 无自定义 Skill 时仅展示内置 4 条 |
| 错误态 | 编辑/删除内置 Skill 时 Toast 提示"内置Skill不可修改/删除" |
| 数据来源 | `list_prd_skills`、`create_prd_skill`、`update_prd_skill`、`delete_prd_skill`、`create_custom_skill_from_builtin` |

#### 7.3.6 Agent `/agents`

| 项目 | 内容 |
|------|------|
| 页面元素 | Agent 列表（名称/版本/状态/路径）、「刷新检测」按钮、启动/停止操作 |
| 关键交互 | 刷新检测（`discover_agents`）、启动 Agent、查看状态 |
| 加载态 | 检测期间按钮 loading |
| 空态 | 未检测到 Agent 时展示空态与提示 |
| 错误态 | 检测失败 Toast |
| 注意 | **该页面未使用 store，采用组件内 `useState`**；`loadData()` 仅调用 `get_agents`，页面重进后依赖数据库记录 |

#### 7.3.7 会话 `/sessions` 与 `/sessions/:id`

| 项目 | 内容 |
|------|------|
| 页面元素 | 列表页：状态筛选、会话行（Agent/任务/状态/时长/Token）、内嵌 `AgentTerminal`；详情页：基本信息、Token 区、Skill 执行记录、MCP 调用记录 |
| 关键交互 | 筛选、进入终端、停止会话、查看执行日志 |
| 加载态 | 列表骨架屏 |
| 空态 | 无会话时展示空态 |
| 错误态 | 会话不存在时提示并允许返回 |
| 数据来源 | `get_agent_sessions`、`stop_agent`、`pty_write`、`get_skill_execution_logs`、`get_mcp_execution_logs` |

#### 7.3.8 代码提交 `/commits`

| 项目 | 内容 |
|------|------|
| 页面元素 | 统计卡片（总提交/今日/本周/当前分支）、提交列表、Diff 展开区、搜索框 |
| 关键交互 | 选择工作目录、搜索（消息/哈希/作者）、展开 Diff、复制哈希、创建提交 |
| 加载态 | 列表骨架屏 |
| 空态 | 未配置工作目录或无提交时展示空态与提示 |
| 错误态 | 非 Git 仓库时展示错误提示 |
| 数据来源 | `git_log`、`git_get_status`、`git_get_diff`、`git_create_commit` |

#### 7.3.9 成本 `/cost` 与统计 `/statistics`

| 项目 | 内容 |
|------|------|
| 页面元素 | Token 汇总卡片（输入/输出/缓存/思考）、趋势图、模型分布图、预算进度条与设置表单、Skill/MCP 使用统计表 |
| 关键交互 | 切换统计天数、设置预算、同步本地用量 |
| 加载态 | 图表与卡片骨架 |
| 空态 | 无数据时展示 0 值与空态 |
| 错误态 | 预算设置非法值时的提示（后端无校验，建议前端拦截） |
| 数据来源 | `get_token_usage_summary`、`get_token_usage_trend`、`get_token_usage_by_model`、`get_token_budget`、`set_token_budget`、`sync_local_usage`、`get_skill_usage_stats`、`get_mcp_usage_stats`、`get_statistics` |

#### 7.3.10 审计 `/audit-log`

| 项目 | 内容 |
|------|------|
| 页面元素 | 筛选器（实体类型）、搜索框、审计记录列表（含操作类型徽章与变更明细） |
| 关键交互 | 筛选、搜索、分页（如有） |
| 加载态 | 列表骨架 |
| 空态 | **当前恒为空**（数据源无写入） |
| 数据来源 | `get_audit_logs`（`entity_type` 硬编码为 `requirement`） |

#### 7.3.11 导入导出 `/import-export`

| 项目 | 内容 |
|------|------|
| 页面元素 | 导出区、导入区（文件选择） |
| 关键交互 | 点击导出/导入 |
| 加载态 | — |
| 空态 | — |
| 错误态 | — |
| 数据来源 | **无后端命令支撑**（F073/F074 未实现） |

#### 7.3.12 系统监控 `/system-monitor`

| 项目 | 内容 |
|------|------|
| 页面元素 | CPU/内存指标、会话数、告警列表、系统信息、缓存统计与「清空缓存」 |
| 关键交互 | 查看指标、检查告警、清空缓存 |
| 加载态 | 指标卡片骨架 |
| 空态 | 无告警时展示空态 |
| 错误态 | 监控数据读取失败时展示占位 |
| 数据来源 | `get_monitor_data`、`check_alerts`、`get_cache_stats`、`clear_cache` |

#### 7.3.13 设置 `/settings`

| 项目 | 内容 |
|------|------|
| 页面元素 | LLM/Claude 配置区（API Key）、会话模式（`session_mode`）、项目/工作目录、Git 相关、PRD Skill 模板、主题切换 |
| 关键交互 | 保存设置、切换主题、测试 API Key |
| 加载态 | 表单加载态 |
| 空态 | 未配置项显示占位 |
| 错误态 | 未配置 Key 时生成 PRD 报错提示 |
| 数据来源 | `get_setting`、`set_setting`、`encrypt_api_key`、`decrypt_api_key`、`useThemeStore` |

### 7.4 全局 UI 组件

| 组件 | 文件 | 导出 | 职责 |
|------|------|------|------|
| 终端 | `components/AgentTerminal.tsx` | `default AgentTerminal` | xterm.js 终端，监听 `pty:event`，支持写入/调整/停止、搜索、最大化、导出日志 |
| 布局 | `components/Layout.tsx` | `default Layout` | 左侧分组导航 + `Outlet` + 底部状态栏（5s 轮询 `get_monitor_data`） |
| 分页 | `components/Pagination.tsx` | `default Pagination` | 分页控件，>7 页省略号折叠 |
| 徽章 | `components/Badges.tsx` | `Badge`、`StatusBadge`、`PriorityBadge`、`TaskTypeBadge`、`ReqTypeBadge` | 状态/优先级/类型标签 |
| 骨架屏 | `components/Skeleton.tsx` | `CardSkeleton`、`StatCardSkeleton` | 加载占位 |
| 通知 | `components/Toast.tsx` | `default ToastContainer`、`showToast(type, message)`、`ToastType` | 全局 Toast（success/error/warning/info，3s 自动消失） |
| 数字动画 | `components/AnimatedNumber.tsx` | `default AnimatedNumber` | 基于 `react-countup` 的数字滚动 |
| 错误边界 | `components/ErrorBoundary.tsx` | `default ErrorBoundary`（class） | 捕获渲染错误并展示重试 UI |
| 需求表单 | `components/RequirementForm.tsx` | `default RequirementForm` | 需求新建/编辑弹窗（含校验） |
| 任务表单 | `components/TaskForm.tsx` | `default TaskForm` | 任务新建/编辑弹窗（含校验） |
| 富文本 | `components/RichTextEditor.tsx` | `default RichTextEditor` | 基于 tiptap 的富文本编辑器 |
| 附件 | `components/AttachmentList.tsx` | `default AttachmentList` | 附件路径列表，按扩展名显示图标与大小 |

### 7.5 状态管理（Zustand）

统一出口 `src/stores/index.ts`（re-export 5 个 store，**不含 `useThemeStore`**）。

| Store | 文件 | state | actions |
|-------|------|-------|---------|
| `useRequirementsStore` | `stores/requirementsStore.ts` | `requirements`、`loading`、`error` | `fetchRequirements(status?)`、`addRequirement(...)`、`removeRequirement(id)` |
| `useTasksStore` | `stores/tasksStore.ts` | `tasks`、`loading`、`error` | `fetchTasks(status?, taskType?)`、`addTask(...)`、`updateStatus(id, status)`、`removeTask(id)` |
| `useProjectsStore` | `stores/projectsStore.ts` | `projects`、`loading`、`error` | `fetchProjects()`、`addProject(...)`、`removeProject(id)` |
| `useSessionsStore` | `stores/sessionsStore.ts` | `sessions`、`loading`、`error` | `fetchSessions(agentId?)`、`stopSession(sessionId)` |
| `useAgentsStore` | `stores/agentsStore.ts` | `agents`、`sessions`、`loading`、`refreshing`、`error` | `fetchAgents()`、`discover()`、`verify()`（状态复核）、`start(agentPath, taskId?)`、`stop(sessionId)` |
| `useSettingsStore` | `stores/settingsStore.ts` | `settings`（Record）、`loading`、`error` | `fetchSetting(key)`、`fetchSettings(keys[])`、`saveSetting(key, value)` |
| `useThemeStore` | `stores/themeStore.ts` | `theme`（light/dark/system） | `setTheme(theme)`（`persist` → `localStorage['ace-theme']`，驱动 `<html>.dark`） |

### 7.6 前端输入校验汇总

校验集中在两个表单组件的本地 `validate()` 中。

| 组件 | 字段 | 规则 | 提示文案 |
|------|------|------|----------|
| `RequirementForm` | `title` | 非空；长度 ≤ 200（`maxLength=200`） | 标题不能为空 / 标题不能超过 200 字（当前 N） |
| `RequirementForm` | `estimated_effort` | 非负；≤ 1000 人天（`min=0` `step=0.5`） | 预估工作量必须为非负数 / 预估工作量不能超过 1000 人天 |
| `TaskForm` | `title` | 非空；长度 ≤ 200（`maxLength=200`） | 标题不能为空 / 标题不能超过 200 字（当前 N） |
| `TaskForm` | `estimated_hours` | 非负；≤ 10000 小时（`min=0` `step=0.5`） | 预估工时必须为非负数 / 预估工时不能超过 10000 小时 |
| `TaskForm` | `due_date` | 不早于今天（按 00:00 归一化） | 截止日期不能早于今天 |
| `Projects` / `ProjectDetail` | `key` | `maxLength=10`（无 `validate()`） | — |

**枚举默认值**：

| 字段 | 默认值 | 选项 |
|------|--------|------|
| 需求 `priority` | `P1` | P0 / P1 / P2 / P3 |
| 需求 `requirement_type` | `feature` | feature / bug / optimization / tech_debt |
| 需求 `source` | — | product / tech / customer / internal |
| 需求 `business_priority` | — | critical / high / medium / low |
| 任务 `task_type` | `development` | requirement / development / testing / bug_fix / review |
| 任务 `priority` | `P1` | P0 / P1 / P2 / P3 |
| 任务 `priority_level` | — | critical / high / medium / low |

---

## 八、状态机汇总

### 8.1 需求状态机（8 状态）

```
                    ┌─────────┐
                    │  draft  │◄──────────┐
                    └────┬────┘           │
             ┌───────────┘                │
             ▼                            │
      ┌────────────┐   reject    ┌──────────┐
      │ reviewing  ├────────────►│ rejected │
      └──────┬─────┘             └──────────┘
             │ approve
             ▼
      ┌────────────┐  freeze   ┌──────────┐
      │  approved  ├──────────►│  frozen  │
      └──────┬─────┘           └────┬─────┘
             │ start_dev            │ unfreeze → draft
             ▼                      ▼
      ┌────────────────┐  start_dev
      │ in_development │◄───────────┘
      └───────┬────────┘
              │ submit_test
              ▼
        ┌───────────┐  reject → in_development
        │  testing  ├──────────────────────────►
        └─────┬─────┘
              │ pass
              ▼
        ┌───────────┐
        │ completed │  （终态）
        └───────────┘
```

**转换规则**（权威定义：`validation.rs`）：

| 当前状态 | 可流转至 |
|----------|----------|
| `draft` | `reviewing` |
| `reviewing` | `approved`、`rejected` |
| `approved` | `frozen`、`in_development` |
| `rejected` | `draft` |
| `frozen` | `draft`、`in_development` |
| `in_development` | `testing` |
| `testing` | `completed`、`in_development` |
| `completed` | （终态，不可流转） |

### 8.2 任务状态机（4 状态）

```
        ┌─────────┐  block   ┌─────────┐
        │ pending ├─────────►│ blocked │
        └────┬────┘          └────┬────┘
             │ start              │ unblock
             ▼                    ▼
        ┌─────────────┐◄──────────┘
        │ in_progress │
        └──────┬──────┘
               │ complete        （回到 pending 亦可）
               ▼
          ┌───────────┐
          │ completed │
          └───────────┘
```

**转换规则**：

| 当前状态 | 可流转至 |
|----------|----------|
| `pending` | `in_progress`、`blocked` |
| `in_progress` | `completed`、`blocked`、`pending` |
| `blocked` | `pending`、`in_progress` |
| `completed` | `pending` |

> **联动**：`→ in_progress` 且 `settings.session_mode == 'auto'`（默认值）时，自动触发 `start_agent_from_task`，可能新建 `agent_sessions` 记录。

### 8.3 会话状态机

**数据库层（`agent_sessions.status`，SQL CHECK 仅 3 值）**：

```
   ┌─────────┐  exit_code = 0      ┌───────────┐
   │ running ├────────────────────►│ completed │
   └────┬────┘                     └───────────┘
        │  exit_code != 0
        ▼
   ┌────────┐
   │ failed │
   └────────┘
```

| 当前状态 | 可流转至 | 触发条件 |
|----------|----------|----------|
| `running` | `completed` | 正常结束（`stop_agent` 或进程退出码 0） |
| `running` | `failed` | 异常退出（退出码非 0） |
| `completed` / `failed` | （终态） | — |

**PTY 内存态（不入库）**：

```
   running ──(输入通道关闭)──► completed
      └──────(pty_kill)──────► terminated      ← 不在 DB CHECK 白名单内
```

### 8.4 Agent 状态机（3 状态）

```
   ┌───────────────┐  启动   ┌─────────┐
   │  available    ├────────►│ running │
   └───────┬───────┘         └────┬────┘
           │ 不可用                │ 结束
           ▼                       ▼
   ┌───────────────┐        ┌───────────────┐
   │  unavailable  │        │   available   │
   └───────────────┘        └───────────────┘
```

| 当前状态 | 可流转至 | 触发条件 |
|----------|----------|----------|
| `available` | `running` | 启动 Agent |
| `running` | `available` | 会话结束（**当前无自动回写机制**） |
| `available` / `running` | `unavailable` | 二进制失效（**当前无自动检测**） |

### 8.5 项目状态机（约定）

```
   active ──► paused ──► archived
     ▲           │            │
     └───────────┴────────────┘
```

| 当前状态 | 可流转至 | 说明 |
|----------|----------|------|
| `active` | `paused`、`archived` | 默认状态 |
| `paused` | `active`、`archived` | — |
| `archived` | `active` | — |

> **注意**：数据库层**无 CHECK 约束**，`paused` / `archived` 为前端约定值，后端不校验。

---

## 九、技术特性与数据流

### 9.1 技术栈

| 层次 | 技术 | 说明 |
|------|------|------|
| 桌面框架 | Tauri 2.0 | Rust 后端 + WebView 前端；插件：`tauri_plugin_shell`、`victauri_plugin` |
| 前端框架 | React 18 + TypeScript | 入口 `src/main.tsx`（`React.StrictMode` → `App`） |
| 状态管理 | Zustand | 6 个 store（5 个统一导出 + `themeStore`）；`themeStore` 使用 `persist` |
| UI 样式 | Tailwind CSS | 暗色模式通过 `<html>.dark` 类 |
| 图标 | Lucide Icons | — |
| 动画 | Framer Motion | 配合 `react-countup`（`AnimatedNumber`） |
| 富文本 | tiptap | `RichTextEditor` |
| 路由 | HashRouter | 18 条路由，父路由 `Layout` |
| 终端渲染 | `@xterm/xterm` | `AgentTerminal`，含 search/fit 能力 |
| PTY | `portable-pty`（Rust） | Windows 下使用 ConPTY |
| 后端异步 | tokio | 读循环使用 `spawn_blocking` |
| HTTP 客户端 | reqwest | Claude Messages API |
| 数据库 | SQLite（WAL） | 通过 `rusqlite` |
| 加密 | `aes-gcm 0.10` + `pbkdf2 0.12` + `sha2 0.10` + `base64 0.22` + `rand 0.8` | AES-256-GCM |
| 缓存 | `lru 0.12` + `bincode` | 单层 LRU |
| 系统监控 | `sysinfo` | `system_monitor.rs`（全局 `MONITOR` lazy_static） |
| 并行加速 | 无 `rayon`；Git 通过命令行 `git` | 未使用 `git2` |
| 日志 | `tracing` | 错误与 warn 记录 |

### 9.2 架构分层

```
┌──────────────────────────────────────────────────────────┐
│  前端（WebView）                                          │
│  pages/ ──► components/ ──► stores/ ──► lib/tauri.ts      │
│                                            │
│                        invoke()            │  listen("pty:event")
└────────────────────────────────────────────┼──────────────┘
                                             ▼
┌──────────────────────────────────────────────────────────┐
│  后端（Rust / Tauri）                                     │
│  commands.rs ──► services/ ──► db/                        │
│       │              │                                    │
│       │              ├─ llm.rs        (Claude API)         │
│       │              ├─ pty.rs        (portable-pty)       │
│       │              ├─ cache.rs      (LRU)                │
│       │              ├─ crypto.rs     (AES-256-GCM)        │
│       │              ├─ agent.rs      (Agent 探测)          │
│       │              ├─ validation.rs (校验)               │
│       │              └─ usage_reader.rs (JSONL 解析)       │
│       └─ system_monitor.rs (sysinfo)                       │
│                                                            │
│  AppState { db, llm_client, cache, pty_manager }           │
│    └─ 注意：llm_client 初始化为 None 且无读写（未使用）      │
└──────────────────────────────────────────────────────────┘
                          │
                          ▼
                  SQLite (database.db, WAL)
```

### 9.3 Service 层职责

| Service | 文件 | 职责 | 缓存 |
|---------|------|------|------|
| `RequirementService` | `services/requirement_service.rs` | 需求 CRUD（扩展字段二次 UPDATE）、缓存失效 | 列表/详情 30s |
| `TaskService` | `services/task_service.rs` | 任务创建/列表/状态更新（含自动启动 Agent 联动）/删除 | 列表 30s |
| `ProjectService` | `services/project_service.rs` | 项目 CRUD + 项目 Skill/MCP 增删查 | 无 |
| `TokenService` | `services/token_service.rs` | Token 查询/汇总/趋势/模型/预算/本地同步 | 60s / 300s |
| `StatsService` | `services/stats_service.rs` | 仪表盘聚合（任务/需求/Token） | 300s |
| `SettingService` | `services/setting_service.rs` | KV 设置读写 | 300s |
| `PrdService` | `services/prd_service.rs` | PRD 生成编排（需求→Skill→API→落库→日志） | — |
| `PrdSkillService` | `services/prd_skill_service.rs` | Skill CRUD、派生自定义 | 无 |
| `AgentService<'a>` | `services/agent_service.rs` | Agent 发现/启动/停止/从任务建会话/会话列表 | 无 |
| `GitService` | `services/git_service.rs` | git status/diff/commit/log（命令行实现，无 db） | 无 |
| `PtyService<'a>` | `services/pty_service.rs` | `PtyManager` 薄封装 | 无 |

### 9.4 缓存策略

| 缓存 key | 内容 | TTL |
|----------|------|-----|
| `statistics:main` | 仪表盘聚合 | 300s |
| `token:summary` | Token 汇总 | 60s |
| `token:trend:{days}` | 每日趋势 | 60s |
| `token:by_model` | 模型分布 | 60s |
| `token_budget:global` | 预算（字面量 key） | 300s |
| `setting:{key}` | 设置项 | 300s |
| `requirement:{id}` | 需求详情 | 30s |
| `tasks:list:{status}:{type}` | 任务列表 | 30s |

> 需求**列表**缓存键（`requirements:list:*`）及其对应的 `RequirementService::list_requirements` 从未被调用（命令层走分页查询），已一并删除，避免"列表已缓存"的误解。

**失效策略**：写操作后按 key 或前缀（`delete_pattern`）主动失效，例如需求写操作失效单条 + `statistics`。

**已知限制**：

- 单层 LRU，命名 `l1_cache` 属遗留；`CacheConfig` 默认 `l1_max_size=1000`、`default_ttl=300s`、`enabled=true`、`key_prefix="ai_platform:"`
- `CacheStats.evictions` 定义但从不自增
- 条目以 `bincode` 序列化，无 schema 版本号，类型变更后反序列化可能失败

### 9.5 数据流概览

```
用户操作
    │
    ▼
React UI (pages/components)
    │
    ▼
Zustand store 或 直接调用
    │
    ▼
invoke() IPC ──► Tauri Command (commands.rs)
                       │
                       ▼
                  Service 层（业务编排）
                    ├──► db/ (SQLite CRUD)
                    ├──► llm.rs (Claude API)      ──► token_usage / skill_execution_logs
                    ├──► pty.rs (portable-pty)    ──► 事件 pty:event ──┐
                    ├──► git_service.rs (git CLI)                     │
                    ├──► agent.rs (探测)                              │
                    └──► cache.rs (LRU, 读写时失效)                    │
                                                                      │
                          ┌───────────────────────────────────────────┘
                          ▼
              前端 listen("pty:event") → xterm.write() 实时渲染

旁路通道（只读聚合）：
    token_usage ──► get_token_usage_* ──► 成本分析 / 统计报表
    skill_execution_logs ──► get_skill_usage_stats ──► 使用统计
    requirement_history ──► get_audit_logs ──► 审计日志（当前数据源为空）
```

---

## 十、验收标准汇总

> 每个功能点的完整验收标准（Given/When/Then）见第四章对应条目；本节按模块给出**高层验收要点**，用于测试计划编排。

| 模块 | 验收要点 |
|------|----------|
| 1 项目管理 | 项目 CRUD 正常；名称长度与 Key 长度限制生效；Skill/MCP 绑定与解绑生效；环节配置受唯一索引保护不重复；项目删除不静默破坏关联需求 |
| 2 需求管理 | 需求 CRUD 与分页正常；状态流转严格遵循 8 状态机；扩展字段可存可读；PRD 生成写入 `prd_content` 并产生 Token 与 Skill 日志；未配置 Key 时给出明确错误；表单校验拦截非法输入 |
| 3 任务管理 | 任务 CRUD 正常；状态流转遵循 4 状态机；`session_mode='auto'` 时进入 `in_progress` 触发 Agent 启动；评论可增删且随任务级联删除；按项目/需求筛选结果正确；表单校验（标题/工时/截止日期）生效 |
| 4 Agent 管理 | 检测幂等且不重复注册；版本号可按 3 种格式解析；状态三值正确；PTY 可 spawn 且输出经 `pty:event` 实时到达前端；启动 Agent 会创建会话记录 |
| 5 会话管理 | 会话列表可按状态筛选；详情页正确展示元信息与日志；从任务创建会话成功；终端可展示实时输出；停止操作更新会话状态 |
| 6 Skill 管理 | 4 个内置模板存在且不可改删；可基于内置派生自定义；自定义可 CRUD（软删除）；更新时版本号递增；使用统计按 `skill_name` 聚合正确 |
| 7 MCP 配置管理 | MCP 可增删查；默认标记可设置；执行日志与统计口径正确（依赖后续补齐写入） |
| 8 统计分析 | Token 汇总/趋势/模型分布数据正确；预算可读写且缓存按 TTL 生效；缓存统计可读可清空；仪表盘聚合正确并在写操作后及时刷新 |
| 9 LLM 集成 | Claude API 调用成功返回文本与 TokenStats；超时/鉴权错误可读；本地用量同步幂等；Token 记录写入正确 |
| 10 安全 | 加解密往返正确、错误口令失败且不 panic；前后端双层校验均生效 |
| 11 审计日志 | 需求变更产生可查询的审计记录；操作类型可区分（依赖补齐写入链路） |
| 12 Git 集成 | 非 Git 目录被拒；状态/Diff/Log/Commit 结果正确；提交可关联任务（依赖补齐落库） |
| 13 系统配置 | KV 读写与 upsert 正确；缓存按 TTL 生效；API Key 配置后 PRD 可用 |
| 14 UI 与交互 | 首页统计正确渲染；导航切换与高亮正确；分页 >7 页折叠；徽章/骨架/Toast/动画/错误边界/主题均按要求表现 |
| 15 数据管理 | 数据库损坏时生成 `.bak` 并重建成功；导入导出可用（依赖新增后端命令） |

### 10.1 关键回归用例（建议纳入 CI）

| 编号 | 用例 | 关联 |
|------|------|------|
| R01 | 需求 `draft → completed` 被拒绝；`in_development → testing` 成功 | F007 |
| R02 | 任务 `pending → completed` 被拒绝；`in_progress → completed` 成功 | F014 |
| R03 | 同一 `binary_path` 重复检测后 `agents` 表仍只有 1 行 | F019 |
| R04 | 同一环节重复 upsert 后 `stage_skill_configs` 仍只有 1 行 | F005 |
| R05 | 内置 Skill 修改/删除被拒绝 | F033 |
| R06 | 一次修改 3 个字段后 Skill `version` **只增加 1**（按操作递增，单一 UPDATE） | F034 |
| R07 | `sync_local_usage` 连续调用两次，第二次返回 0 | F048 |
| R08 | 非 Git 目录调用 `git_get_status` 返回错误 | F056 |
| R09 | 加密→解密往返一致；错误口令解密失败 | F051 |
| R10 | 删除任务后其评论被级联删除 | F015 |

---

## 十一、实现差距与纠错清单

> 本节记录 v1.0 文档与代码实况的差异，以及需要补齐的实现项，作为后续迭代的输入。

### 11.1 未实现功能（❌，0 项 — 原 8 项已全部修复）

| 功能 | 原问题 | 修复方式 |
|------|--------|----------|
| F011 / F053 / F054 / F055 审计链路 | `requirement_history` 无写入，审计页恒空 | `RequirementService` 在 create / update / status_change / delete 时写入历史；`get_audit_logs` 支持 `entity_type` 过滤，并支持删除场景标题快照回退 |
| F039 / F040 MCP 日志与统计 | `mcp_execution_logs` 无写入 | 会话启动时为项目绑定的 MCP 端点写入 `connected`/`session_start` 记录 |
| F060 提交记录落库 | `git_commits` 无写入、`_task_id`/`_session_id` 被忽略 | `git_create_commit` 使用任务/会话参数并落库；新增 `list_git_commits` 查询命令 |
| F073 / F074 数据导入导出 | 伪实现（导出空数组、导入不落库却提示成功） | 新增 `export_data` / `import_data`（含 dry-run 预检、事务导入、外键保护、导入前自动备份）；前端改为真实数据链路 |

### 11.2 部分实现 / 未接线（⚠️ 1 项）

| 功能 | 缺口 | 状态 |
|------|------|:---:|
| F035 Skill 执行日志 | `task_execution` 阶段仍无写入点（平台尚无"任务执行阶段调用 Skill"的链路）；`prd_generation`（成功与失败）与 `session_start` 两个阶段已真实写入，`skill_type` 取自 Skill 记录，不再硬编码 | ⚠️ |

> **第三轮已修复的原 ⚠️ / ⚪ 项**：
> - **F021 Agent 状态管理**：新增 `verify_agents` 命令（二进制有效性复核 + 进程存活对账）、新增 `agentsStore`，`Agents` 页面接入 store 并新增「状态复核」入口；
> - **F030 会话停止**：`stop_agent` 先经 `validate_session_status_transition` 校验，再真实 kill PTY 进程，并以 `finish_agent_session` 回写状态 / 结束时间 / 时长；
> - **F034 Skill 版本控制**：改为单条 `UPDATE ... version = version + 1`，版本按**操作**递增（一次改 3 个字段只 +1）；
> - **F049 流式响应**：`generate_prd_streaming` 改为 `bytes_stream()` 真实增量读取并注册为 IPC 命令，前端 `RequirementDetail` 订阅 `prd:chunk` 实时渲染，完成时收到 `prd:done`。
>
> 说明：F045 缓存统计本身功能可用（✅），其单层 LRU 属**实现局限**；`evictions` 恒为 0 与 `bincode` 无版本号两项已修复（见 11.4）。

### 11.3 文档纠错（v1.0 → v2.0）

| 项 | v1.0 记述 | 实际 |
|----|-----------|------|
| 需求管理命令数 | "6 个"（却列出 7 条） | **7 条** |
| 任务管理命令数 | "8 个"（却列出 7 条） | **8 条** |
| 系统命令数 | "6 个" | 实际含 `get_statistics`、`get_cache_stats`、`clear_cache`、`get_setting`、`set_setting`、`encrypt_api_key`、`decrypt_api_key`、`get_audit_logs` 等 **8 条**；另有 `get_monitor_data`、`check_alerts` **未在原文列出** |
| 任务列表路由 | `/tasks` | **`/board`** |
| 审计路由 | `/audit` | **`/audit-log`** |
| 监控路由 | `/monitor` | **`/system-monitor`** |
| 工作台路由 | `/workspace` | **`/`**（index） |
| 会话表 `work_dir` | 列为字段 | **该字段不存在**于 `agent_sessions`，仅存在于 PTY 内存态 |
| 会话状态取值 | 提及 created/launching/terminated | DB CHECK 仅 **running/completed/failed** |
| PTY 事件名 | `pty:data` | 实际为 **`pty:event`**（数据以 `type="data"` 载荷下发） |
| 数据导入导出状态 | ✅ | **❌ 未实现** |
| 审计日志状态 | ✅ | **❌ 未生效**（无写入） |
| 提交记录存储状态 | ✅ | **❌ 未实现**（无写入） |
| MCP 执行日志状态 | ✅ | **❌ 未实现**（无写入） |
| 需求管理"6 个"命令中的 `get_requirement` | 未列出 | **存在**，应计入 |
| 命令总数 | 未给出 | **77 条**（v1.0 时点）；现已补齐至 **82 条** |
| 数据表总数 | 18 张 | **18 张（一致）** |
| 前端路由总数 | 19 条 | **18 条已注册 + 1 条未挂载（`Dashboard`）** |

### 11.4 已知数据风险

| 风险 | 位置 | 说明 | 状态 |
|------|------|------|:---:|
| Token 列错位 | `db/tokens.rs::get_token_usage` | 现已改为显式列清单（不再使用 `SELECT *` + 固定下标），并补充"旧库列追加到表尾"回归测试 | ✅ 已修复 |
| 迁移双轨 | `db/migration.rs` vs `db/mod.rs` | 已统一为**单一运行时路径**：`init_tables` → `migrate_tables` → `create_supplementary_indexes` → `migrate_unique_constraints` → `seed_data`；版本登记收敛到 `migration::ensure_baseline()` + `SCHEMA_BASELINE_VERSION`；死代码 `apply_migration` / `get_version_info` 与 12 个从未加载的 `db/migrations/*.sql` 已移除 | ✅ 已修复 |
| 索引缺失 | `010_additional_indexes.sql` | 新增 `create_supplementary_indexes()`，在 `migrate_tables()` 之后创建 6 个补充索引 | ✅ 已修复 |
| API Key 明文 | `settings.claude_api_key` | 已接入本地密钥库（`keystore.key`）自动加解密，兼容历史明文 | ✅ 已修复 |
| `llm_client` 未使用 | `AppState` | 初始化为 `None` 且全代码库无读写，属死字段 | ⬜ 待清理 |
| 无唯一约束 | `projects.key`、`agents.binary_path`、`project_skills(project_id, skill_id)`、`project_mcps` 每项目仅一个默认 | 新增 `migrate_unique_constraints()`：先安全去重再建 4 个唯一索引；`add_project_skill` 幂等、`add_project_mcp` 自动清理旧默认 | ✅ 已修复 |
| 会话僵尸 | `agent_sessions` vs PTY 内存态 | 启动时 `reconcile_running_sessions()` 将残留 `running` 标记为 failed 并补齐时长 | ✅ 已修复 |
| 需求删除失败 | `requirement_history` 外键 | 外键使"写审计后删除需求"必然失败；已移除该外键并新增表重建迁移，审计在需求删除后仍保留 | ✅ 已修复（新增测试发现） |
| 备份不含 WAL | `database.db.bak` | 备份前执行 `PRAGMA wal_checkpoint(TRUNCATE)`，并轮转保留最近 5 份 | ✅ 已修复 |
| Token 成本恒 0 | `token_usage.total_cost` | 新增模型价格表与 `estimate_cost`（含缓存/思考口径），PRD 生成与需求完善两条路径均记账 | ✅ 已修复 |
| 预算用量不累计 | `token_budgets.current_usage` | 写入 Token 时自动累加，跨月自动重置；`get_token_budget` 返回 `usage_ratio` / `exceeded` 供前端告警 | ✅ 已修复 |

---

## 附录 A：文档结构导航

| 章节 | 内容 | 主要读者 |
|------|------|----------|
| 一、文档说明 | 定位、阅读指南、状态标记、术语 | 全部 |
| 二、系统概览 | 产品定位、用户、边界、业务流程、功能域、流水线环节 | 产品/评审 |
| 三、功能总览 | 75 项功能总表 + 编号号段 + 模块统计 | 全部 |
| 四、功能详细设计 | 15 模块 / 75 项逐功能结构化规格 | 开发/测试 |
| 五、数据模型设计 | 18 张表字段级设计、索引、外键、枚举、迁移、种子数据 | 开发/DBA |
| 六、IPC 命令清单 | 82 条命令入参/出参/错误场景、事件通道、前端封装映射 | 开发 |
| 七、前端页面与交互 | 18 条路由、页面交互、UI 组件、Store、表单校验 | 前端/测试 |
| 八、状态机汇总 | 需求/任务/会话/Agent/项目状态机与转换规则 | 开发/测试 |
| 九、技术特性与数据流 | 技术栈、架构分层、Service 职责、缓存策略、数据流 | 架构/开发 |
| 十、验收标准汇总 | 分模块验收要点 + 关键回归用例 | 测试 |
| 十一、实现差距与纠错清单 | 未实现/部分实现项、文档纠错、数据风险 | 产品/开发 |

---

## 附录 B：修订记录

| 版本 | 日期 | 作者 | 变更说明 |
|------|------|------|----------|
| v1.0 | 2026-09-21 | — | 初版：15 个模块功能清单（表格）、20 张表简表、IPC 命令名列表、路由列表、技术特性、数据流 |
| v2.0 | 2026-09-21 | — | 重写为逐功能结构化详细设计文档。新增：文档说明与术语、系统概览（用户/边界/流程图/功能域/环节）、F001–F075 编号体系与功能总览总表、75 项逐功能规格（目标用户/场景/前置条件/主流程/异常/边界/输入输出/依赖/约束/状态变化/权限/验收标准）、18 张表字段级设计（含索引/外键/枚举/迁移/种子数据）、77 条 IPC 命令出入参与错误场景、事件通道与前端封装映射、18 条路由与页面交互设计、UI 组件与 Zustand Store 清单、前端校验规则汇总、5 组状态机、技术栈与架构分层、缓存策略、数据流、验收标准汇总与回归用例、实现差距与纠错清单。修正：路由名（`/board`、`/audit-log`、`/system-monitor`、`/`）、命令数量、`agent_sessions.work_dir` 不存在、PTY 事件名、4 项功能状态由 ✅ 更正为 ❌ |
| v2.2 | 2026-09-21 | — | 按全量问题修复计划 v2.0 完成第二轮实现。修复：Token 明细列错位（改显式列清单）、敏感设置静态加密（本地密钥库）、`stop_agent` 联动终止 PTY 进程、Git 提交支持未跟踪新文件、备份含 WAL 检查点且保留 5 份、预算用量自动累计与跨月重置、启动时会话对账、Skill 版本按操作递增、需求完善写入 Token 记录、模型成本估算与可配置、缺失索引与唯一约束（含安全去重迁移）、**需求删除因 `requirement_history` 外键必然失败**（新增测试发现）。工程化：新增 ASCII 路径测试脚本（`npm run test:win`）、`isolate: true` 修复跨文件 mock 泄漏、后端默认执行单测 2 → **29**、新增真实访问 SQLite 的 14 个链路测试、`AgentTerminal`/`stores` 统一导出等清理。实测：`cargo test` 29 + 71 + 3 全通过；前端 222 用例全通过；L6 IPC 覆盖 80/80 = 100% |
| v2.1 | 2026-09-21 | — | 按差距分析报告（`docs/analysis/gap-analysis-report.md`）与开发计划（`docs/plan-fix-v3.md`）完成代码补齐后的状态同步。修复：审计写入链路（F011/F053/F054/F055）、MCP 执行日志与统计（F039/F040）、PTY 真实输入/尺寸/终止（F022/F023/F029）、会话结束回写（F027/F030）、提交记录落库与查询（F060）、数据导入导出真实实现（F073/F074）、F010/F036/F040/F045 前端入口补齐、`update_requirement` 支持扩展字段。新增 3 条 IPC 命令（`list_git_commits`、`export_data`、`import_data`），命令总数 77 → **80**，IPC 测试覆盖率 71.4% → **100%（80/80）**。功能状态：✅ 56 → **70**，⚠️ 10 → **4**，❌ 8 → **0** |
| v2.3 | 2026-09-21 | — | 第四轮（问题清单 23 项）修复与收口。**功能**：F060 前端闭环（`Commits` 支持关联任务/会话并展示平台提交记录）、F021（新增 `verify_agents` + `agentsStore` + 「状态复核」入口）、F030（`stop_agent` 状态流转校验 + 真实 kill + 时长回写）、F034（版本按操作递增）、F035（执行日志参数化，成功/失败/`session_start` 三处真实写入）、F049（`bytes_stream()` 真流式 + 命令注册 + 前端订阅）。**架构债务**：迁移机制统一（消除 `apply_migration` 死代码与 12 个死 `.sql` 文件，版本登记收敛到 `ensure_baseline`）、种子逻辑去重、`AppState.llm_client` 死字段移除、缓存 `evictions` 自增与条目 schema 版本化、删除形同虚设的列表缓存、**Rust 告警 15 → 0**。**测试**：新增 19 个页面渲染冒烟用例与 F060/F021 回归用例（前端 222 → **245** 全通过）、L2 E2E 14 → **24 例**、L4 CDP 改为按 `TAURI_DEVTOOLS_SERVER_PORT` 自动启用的真实用例并新增 `npm run test:l4` / `vitest.l4.config.ts`。**文档**：`full-test-report.md` 数字刷新、`full-test-plan.md` / `CLAUDE.md` 命令同步。新增 2 条 IPC 命令（`verify_agents`、`generate_prd_streaming`），命令总数 80 → **82**。功能状态：✅ 70 → **74**，⚠️ 4 → **1**，⚪ 1 → **0** |
| v2.4 | 2026-09-21 | — | **新增模块 16「会话交互与消息」（F076–F086，📋 设计中）**：会话内以消息方式下发指令、结构化回复（正文/思考/工具）、思考内容落库并默认折叠、双视图（消息 + 终端）、多 Agent 适配（claude/codex）、任务/需求上下文绑定。详细设计见 `docs/features/session-interaction-design.md`（v1.2）。**状态标记新增 📋**（设计完成、代码未实现）。功能总数 75 → **86**；实现后需同步：数据表 18 → 19、IPC 命令 82 → 87、Tauri 事件 3 → 9。**附带的实测结论**（真实调用，成本约 $0.49）：① `--input-format stream-json` 用户消息 schema 已锁定；② `--include-partial-messages` 可产出 `thinking_delta`/`text_delta` 增量；③ **该 CLI 发行版不支持 host 逐次权限确认**（`--permission-mode manual --permission-prompts host` 下 `control_request` 出现 0 次，直接下发 `system/permission_denied`，且无独立 `--permission-prompt-tool` flag）→ 权限方案改为「预设默认 `acceptEdits`（实测可放行写入）+ `--allowedTools` 白名单 + 被拒卡片（含"允许并重试"）+ 终端视图作为唯一逐次确认路径」；④ `permission_denied` 字段已锁定（`tool_name`/`tool_use_id`/`message`）。**环境风险**：本机 `claude` 为第三方包装（QClaw + `mimo-v2.5` 模型 + 用户级 MCP 工具注入），官方发行版需复测权限行为 |
| v2.5 | 2026-09-21 | — | **模块 16 P0 实现完成（"结构化会话可用闭环"）**。**数据层**：新增 `session_messages` 表（17 字段 + 4 索引，`session_id+seq` 唯一）、`agent_sessions` 9 列（`cli_session_id`/`work_dir`/`agent_path`/`mode`/`view`/`permission_mode`/`model`/`permission_allowlist`/`last_event_at`），数据表 18 → **19**。**后端**：新增 `agent_adapter.rs`（Claude 启动参数/用户消息编码/事件归一化，含 9 个基于实测载荷的解析用例）与 `session_driver.rs`（子进程生命周期、逐行归一化、落库与 6 类事件推送、中断、退出收尾）；新增 6 条命令（命令总数 82 → **88**）；`verify_agents` 的进程对账并入结构化会话存活集（避免误杀消息模式会话）；启动时收尾全部残留 `streaming` 消息。**前端**：`SessionChat`/`SessionInput`/`MessageBubble`（思考折叠、工具块、被拒卡片）、`sessionMessagesStore`（事件订阅 + 增量缓冲 + 落库幂等）、`SessionDetail` 双视图（消息主 / 终端兜底）、`Agents` 页新增「消息模式启动」入口；`lib/tauri.ts` 新增 6 个封装。**测试**：后端 31 → **42**（适配层 9 + 数据层 2 新增）、L6 IPC 82 → **88**（100%）、前端 247 → **259**（新增 `MessageBubble` 6 用例），全部通过；Rust 告警保持 0。**范围裁剪**：F078 ⚠️（正文 + 围栏代码块 + 行内 code，完整 Markdown 待引入渲染库）、F083 ⚠️（被拒卡片 + 白名单沉淀已实现，逐次授权确认依赖终端视图）；多 Agent 适配（codex）移至 P1。功能状态：📋 11 → **0**，✅ 74 → **83**，⚠️ 1 → **3** |

---

*文档版本：v2.3*
*最后更新：2026-09-21*
*事实来源：`FEATURES.md` v1.0 + `src/`、`src-tauri/` 只读代码核对 + 第三/四轮修复后的实现*
