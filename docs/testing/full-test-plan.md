# 全面自动化测试计划

> 基于 design.md v2.4、plan.md、plan-v2.md、feature-spec.md v1.1、requirement-gap-report.md、fix-plan.md、analysis-report.md
> 六层测试体系 | 126个测试用例 | 2026-09-19

---

## 一、需求提取

### 1.1 功能需求（R001-R009）

| 编号 | 需求名称 | 优先级 | 核心内容 |
|------|---------|--------|---------|
| R001 | 需求管理增强 | P0 | 12个新字段(module/owner/developer/source/type/acceptance_criteria/priority_level/tags/attachments/estimated_tokens/actual_tokens/version)、TipTap富文本、附件路径、状态流转规则 |
| R002 | 任务管理增强 | P0 | 11个新字段(assignee/due_date/tags/estimated_tokens/estimated_hours/actual_hours/blocked_reason/related_requirement_id/related_project_id/priority_level/notes/attachments)、详情页、评论区、看板增强 |
| R003 | 项目-需求-任务联动 | P0 | 三级数据全链路打通、项目详情增加需求/任务 Tab、全局项目选择器、看板项目筛选 |
| R004 | CLI终端双模式 | P0 | 自动/手动会话模式、会话输出持久化(SQLite BLOB)、历史回放、会话与任务绑定 |
| R005 | Skill替换 | P0 | Skill版本模型(version/parent_id/is_deleted)、自定义版本创建编辑、项目级Skill选择 |
| R006 | Skill管理后台 | P0 | Skill CRUD、CodeMirror模板编辑器(Markdown)、执行日志、使用统计、迭代建议(4条规则)、导入/导出 |
| R007 | 工作台(Dashboard→Workspace) | P0 | 8模块：我的待办、快速操作、进行中、系统资源、最近活动、项目概览、Token趋势、Agent状态 |
| R008 | 系统资源监控 | P0 | sysinfo采集(内存/CPU)、状态栏实时展示(2秒刷新)、资源告警(内存>1GB/会话≥3/任务>20) |
| R009 | 系统托盘图标 | P0 | 右键菜单(8项)、最小化到托盘、退出、窗口关闭拦截、状态更新IPC |

### 1.2 非功能需求（NFR001-NFR007）

| 编号 | 需求名称 | 指标 |
|------|---------|------|
| NFR001 | 数据库大小 | < 100MB |
| NFR002 | 并发会话数 | 最大 3 个 |
| NFR003 | 页面加载时间 | < 500ms |
| NFR004 | 轮询间隔 | 2秒 |
| NFR005 | 离线能力 | 网络断开时本地功能正常 |
| NFR006 | 自动备份 | 启动时备份，保留最近5份 |
| NFR007 | API Key加密 | AES-256-GCM，密文存储 |

### 1.3 技术问题（TOP 10 HIGH/CRITICAL）

| 编号 | 严重度 | 问题 |
|------|--------|------|
| T001 | CRITICAL | DB迁移无事务保护 |
| T002 | CRITICAL | list_requirements返回类型不匹配(PagedResult vs Requirement[]) |
| T003 | HIGH | commands绕过Service层 |
| T004 | HIGH | API Key明文存储 |
| T005 | HIGH | useEffect未cleanup导致内存泄漏 |
| T006 | HIGH | 7个页面绕过Store直接调IPC |
| T007 | HIGH | 无统一错误处理(AppError) |
| T008 | HIGH | list无分页 |
| T009 | HIGH | 时间戳格式不一致 |
| T010 | HIGH | 无删除保护策略 |

---

## 二、六层测试体系

> ⚠️ **实现说明（第四轮对齐）**：计划中的 L2 原定为 WebdriverIO，实际实现为 Playwright 浏览器结构冒烟；
> L3 原定为 Victauri MCP 全栈，实际实现为 **mock IPC 的契约测试**，真实后端链路验证已下沉到
> `src-tauri/src/db/integration_tests.rs`（默认可执行，真实 SQLite）与 `src-tauri/tests/integration.rs`（Victauri 门控）。
> 各层实测用例数见下表"实测"列。

| 层级 | 名称 | 工具 | 计划 | 实测 | 覆盖范围 |
|------|------|------|:---:|:---:|---------|
| L1 | Mock Runtime（组件 + Store + 页面） | Vitest + jsdom | 38 | 122 | 组件渲染、Store 逻辑、表单校验、**19 个页面渲染冒烟** |
| L2 | E2E 结构冒烟 | Playwright | 22 | **24** | 路由可达、导航高亮、主内容区渲染 |
| L3 | IPC 契约测试（mock IPC） | Vitest + `vi.mock` | 20 | 33 | 前端封装 → 命令名 / 参数契约（**不含真实后端**） |
| L4 | CDP（WebView2 真实运行） | Playwright over CDP | 18 | 5 | 控制台错误、请求健康、性能指标、IPC 拦截与耗时 |
| L5 | Chaos Engineering | 故障注入 | 12 | 10 | 异常输入、并发、资源耗尽、网络异常 |
| L6 | IPC Coverage | 覆盖率分析 | 16 | **88** | IPC 命令覆盖率 **88/88 = 100%**（v2.5 新增 6 条会话交互命令 L6-083~088） |

**运行命令**：`npm run test:win`（前端全量，路径含中文时使用） · `npm run test:rust`（后端） ·
`npm run test:e2e`（L2） · `npm run test:l4`（L4，需先启动应用并设置 `TAURI_DEVTOOLS_SERVER_PORT`）

### 2.1 L1 Mock Runtime 测试用例（38个）

| 用例编号 | 对应需求 | 测试类型 | 测试描述 | 优先级 |
|---------|---------|---------|---------|--------|
| L1-001 | R001 | 组件渲染 | RequirementForm渲染所有12个新字段 | P0 |
| L1-002 | R001 | 表单验证 | 标题为空时提交被拦截 | P0 |
| L1-003 | R001 | 表单验证 | 标题超过200字符时提交被拦截 | P0 |
| L1-004 | R001 | 状态流转 | 需求状态从draft→in_progress→completed合法流转 | P0 |
| L1-005 | R001 | 状态流转 | 需求状态非法流转被拒绝(如completed→draft) | P0 |
| L1-006 | R001 | Store逻辑 | useRequirementsStore加载/创建/更新需求 | P0 |
| L1-007 | R002 | 组件渲染 | TaskForm渲染所有11个新字段 | P0 |
| L1-008 | R002 | 表单验证 | 任务标题为空时提交被拦截 | P0 |
| L1-009 | R002 | 表单验证 | 无效任务类型被拒绝 | P0 |
| L1-010 | R002 | 表单验证 | 无效优先级被拒绝 | P0 |
| L1-011 | R002 | Store逻辑 | useTasksStore加载/创建/更新任务 | P0 |
| L1-012 | R002 | 组件渲染 | TaskDetail显示评论区 | P0 |
| L1-013 | R003 | Store逻辑 | useProjectsStore加载/创建项目 | P0 |
| L1-014 | R003 | 组件渲染 | ProjectSelector渲染项目列表 | P0 |
| L1-015 | R003 | 联动逻辑 | 选择项目后需求/任务列表过滤 | P0 |
| L1-016 | R003 | 组件渲染 | ProjectDetail显示需求/任务 Tab | P0 |
| L1-017 | R004 | Store逻辑 | useSessionsStore加载会话列表 | P0 |
| L1-018 | R004 | 设置逻辑 | session_mode切换(auto/manual) | P0 |
| L1-019 | R004 | 组件渲染 | AgentTerminal组件渲染 | P1 |
| L1-020 | R005 | Store逻辑 | Skill版本创建和切换 | P0 |
| L1-021 | R005 | 组件渲染 | Skill选择器显示内置/自定义 | P0 |
| L1-022 | R005 | 业务逻辑 | 基于内置Skill创建自定义版本 | P0 |
| L1-023 | R006 | Store逻辑 | Skill CRUD操作 | P0 |
| L1-024 | R006 | 组件渲染 | SkillManagement页面渲染 | P0 |
| L1-025 | R006 | 业务逻辑 | 迭代建议规则触发(成功率<80%) | P1 |
| L1-026 | R006 | 业务逻辑 | 迭代建议规则触发(Token>5000) | P1 |
| L1-027 | R007 | 组件渲染 | Workspace页面8模块渲染 | P0 |
| L1-028 | R007 | Store逻辑 | useSystemMonitorStore数据加载 | P0 |
| L1-029 | R007 | 组件渲染 | 我的待办模块显示任务列表 | P0 |
| L1-030 | R008 | Store逻辑 | 系统监控数据采集mock | P0 |
| L1-031 | R008 | 组件渲染 | 状态栏显示内存/会话/任务数 | P0 |
| L1-032 | R008 | 告警逻辑 | 内存>1GB时状态栏变红 | P0 |
| L1-033 | R009 | 组件渲染 | 托盘菜单项渲染 | P0 |
| L1-034 | R009 | 事件逻辑 | 托盘菜单事件分发 | P0 |
| L1-035 | NFR007 | 加密逻辑 | API Key加密/解密函数 | P0 |
| L1-036 | NFR003 | 性能 | 页面组件懒加载 | P1 |
| L1-037 | T005 | 内存泄漏 | useEffect cleanup正确执行 | P0 |
| L1-038 | T007 | 错误处理 | AppError枚举正确序列化 | P0 |

### 2.2 L2 WebDriver E2E 测试用例（22个）

| 用例编号 | 对应需求 | 测试描述 | 优先级 |
|---------|---------|---------|--------|
| L2-001 | R001 | 创建需求：填写表单→提交→列表刷新 | P0 |
| L2-002 | R001 | 编辑需求：修改字段→保存→详情更新 | P0 |
| L2-003 | R001 | 需求状态流转：选择新状态→确认→列表更新 | P0 |
| L2-004 | R002 | 创建任务：填写表单→选择类型/优先级→提交 | P0 |
| L2-005 | R002 | 任务看板拖拽：拖动卡片→新列→状态更新 | P0 |
| L2-006 | R002 | 任务评论：输入评论→提交→显示 | P0 |
| L2-007 | R003 | 项目选择器：切换项目→列表过滤 | P0 |
| L2-008 | R003 | 项目详情Tab：点击需求/任务Tab→正确内容 | P0 |
| L2-009 | R003 | 联动创建：需求关联项目→任务关联需求 | P0 |
| L2-010 | R004 | 会话模式切换：设置→自动/手动→生效 | P0 |
| L2-011 | R004 | 手动开启会话：任务详情→开启会话→终端显示 | P0 |
| L2-012 | R005 | Skill创建自定义版本：选择内置→创建→编辑 | P0 |
| L2-013 | R006 | Skill管理：列表→新建→编辑模板→保存 | P0 |
| L2-014 | R006 | Skill导入导出：导出JSON→导入→验证 | P1 |
| L2-015 | R007 | 工作台加载：首页→8模块显示→数据正确 | P0 |
| L2-016 | R007 | 快速操作：点击新建需求/任务→弹窗→创建 | P0 |
| L2-017 | R008 | 状态栏刷新：等待2秒→数值更新 | P0 |
| L2-018 | R008 | 状态栏tooltip：悬停→显示详细信息 | P1 |
| L2-019 | R009 | 托盘菜单：右键→菜单项→操作执行 | P0 |
| L2-020 | NFR005 | 离线操作：断网→创建需求→成功→联网→同步 | P1 |
| L2-021 | NFR003 | 页面加载性能：导航→500ms内加载完成 | P1 |
| L2-022 | T002 | 分页返回类型：list_requirements返回PagedResult | P0 |

### 2.3 L3 AI MCP Full-stack 测试用例（20个）

| 用例编号 | 对应需求 | 测试描述 | 优先级 |
|---------|---------|---------|--------|
| L3-001 | R001 | IPC: create_requirement全字段→DB验证 | P0 |
| L3-002 | R001 | IPC: list_requirements分页→返回PagedResult | P0 |
| L3-003 | R001 | IPC: update_requirement→字段更新验证 | P0 |
| L3-004 | R002 | IPC: create_task全字段→DB验证 | P0 |
| L3-005 | R002 | IPC: list_tasks分页→返回PagedResult | P0 |
| L3-006 | R002 | IPC: create_task_comment→评论存储 | P0 |
| L3-007 | R003 | IPC: list_requirements_by_project→过滤正确 | P0 |
| L3-008 | R003 | IPC: list_tasks_by_project→过滤正确 | P0 |
| L3-009 | R004 | IPC: create_session→会话创建+任务绑定 | P0 |
| L3-010 | R004 | IPC: get_session_output→日志读取 | P0 |
| L3-011 | R005 | IPC: create_skill_version→版本创建 | P0 |
| L3-012 | R006 | IPC: create_skill→Skill创建 | P0 |
| L3-013 | R006 | IPC: get_skill_stats→统计数据正确 | P1 |
| L3-014 | R007 | IPC: get_system_info→返回内存/会话/任务数 | P0 |
| L3-015 | R008 | IPC: get_system_monitor→实时数据 | P0 |
| L3-016 | NFR006 | DB: 自动备份→backups目录生成 | P0 |
| L3-017 | NFR007 | IPC: encrypt_api_key→密文存储 | P0 |
| L3-018 | NFR007 | IPC: decrypt_api_key→解密还原 | P0 |
| L3-019 | T001 | DB: 迁移执行→schema_version正确递增 | P0 |
| L3-020 | T010 | IPC: delete_project有关联→返回错误 | P0 |

### 2.4 L4 Playwright CDP 测试用例（18个）

| 用例编号 | 对应需求 | 测试描述 | 优先级 |
|---------|---------|---------|--------|
| L4-001 | R001 | 需求创建全流程：表单→提交→列表→详情 | P0 |
| L4-002 | R001 | 需求富文本：TipTap编辑→保存→渲染 | P0 |
| L4-003 | R001 | 需求筛选：按状态/模块/负责人筛选 | P0 |
| L4-004 | R002 | 任务看板：拖拽→状态更新→排序 | P0 |
| L4-005 | R002 | 任务详情：字段展示→评论→历史 | P0 |
| L4-006 | R002 | 任务筛选：按类型/优先级/状态筛选 | P0 |
| L4-007 | R003 | 三级导航：项目→需求→任务穿透 | P0 |
| L4-008 | R003 | 全局筛选：项目选择器→看板过滤 | P0 |
| L4-009 | R004 | 会话终端：输出渲染→历史回放 | P0 |
| L4-010 | R005 | Skill切换：项目配置→选择Skill→任务继承 | P0 |
| L4-011 | R006 | Skill编辑：CodeMirror→语法高亮→保存 | P0 |
| L4-012 | R007 | 工作台交互：点击待办→跳转→返回 | P0 |
| L4-013 | R008 | 状态栏交互：悬停tooltip→点击跳转 | P0 |
| L4-014 | R009 | 窗口行为：关闭→隐藏→托盘恢复 | P0 |
| L4-015 | NFR003 | 性能：页面切换<500ms | P1 |
| L4-016 | T005 | 内存：反复导航无内存泄漏 | P0 |
| L4-017 | T006 | Store一致性：页面数据来源统一 | P0 |
| L4-018 | T002 | 类型一致性：前后端类型匹配 | P0 |

### 2.5 L5 Chaos Engineering 测试用例（12个）

| 用例编号 | 对应需求 | 测试描述 | 优先级 |
|---------|---------|---------|--------|
| L5-001 | R001 | 畸形输入：SQL注入/XSS/超长字符串 | P0 |
| L5-002 | R002 | 并发创建：同时提交10个任务 | P0 |
| L5-003 | R003 | 数据一致性：删除项目后关联数据处理 | P0 |
| L5-004 | R004 | 资源耗尽：创建100个会话 | P1 |
| L5-005 | R006 | Skill删除：删除被引用的Skill | P0 |
| L5-006 | R008 | 高负载：系统监控在高CPU下的表现 | P1 |
| L5-007 | NFR001 | 数据库膨胀：插入10万条记录后性能 | P1 |
| L5-008 | NFR002 | 并发限制：超过3个会话时的行为 | P0 |
| L5-009 | NFR005 | 网络断开：RPC调用超时处理 | P0 |
| L5-010 | T001 | 迁移失败：中断迁移后恢复 | P0 |
| L5-011 | T004 | 加密异常：损坏密文解密 | P0 |
| L5-012 | R009 | 托盘异常：系统不支持托盘时降级 | P1 |

### 2.6 L6 IPC Coverage 测试用例（16个 + 模块 16 增量 6 个）

| 用例编号 | 对应需求 | 测试描述 | 优先级 |
|---------|---------|---------|--------|
| L6-001 | 全局 | list_requirements: 正常/空/分页边界 | P0 |
| L6-002 | 全局 | create_requirement: 全字段/缺省/非法 | P0 |
| L6-003 | 全局 | update_requirement: 部分更新/全量更新 | P0 |
| L6-004 | 全局 | delete_requirement: 正常/有关联/不存在 | P0 |
| L6-005 | 全局 | list_tasks: 正常/空/分页边界 | P0 |
| L6-006 | 全局 | create_task: 全字段/缺省/非法 | P0 |
| L6-007 | 全局 | update_task: 部分更新/全量更新 | P0 |
| L6-008 | 全局 | delete_task: 正常/有评论/不存在 | P0 |
| L6-009 | 全局 | list_projects: 正常/空 | P0 |
| L6-010 | 全局 | create_project: 正常/重复名/空名 | P0 |
| L6-011 | 全局 | delete_project: 正常/有需求/有任务 | P0 |
| L6-012 | 全局 | create_session: 正常/重复/上限 | P0 |
| L6-013 | 全局 | list_sessions: 正常/按状态过滤 | P0 |
| L6-014 | 全局 | get_system_info: 正常返回 | P0 |
| L6-015 | 全局 | encrypt/decrypt_api_key: 正常/空/特殊字符 | P0 |
| L6-016 | 全局 | 所有IPC命令总覆盖率目标>80% | P0 |
| L6-083 | F077 | `start_structured_session`：以消息模式启动会话 | P0 |
| L6-084 | F077 | `send_session_message`：下发指令 | P0 |
| L6-085 | F076/F085 | `list_session_messages`：分页参数（beforeSeq/limit） | P0 |
| L6-086 | F086 | `interrupt_session`：中断执行 | P0 |
| L6-087 | F081/F083 | `set_session_mode`：白名单沉淀 + resendLast | P0 |
| L6-088 | F082 | `resume_session`：续接 CLI 会话 | P0 |

> 模块 16 组件级用例：`MessageBubble` 6 例（思考折叠/工具块/被拒卡片/代码块/单轮结果/上下文卡片）、
> `SessionInput` 7 例（Enter/Shift+Enter/禁用态/中断/只读）、`SessionChat` 7 例（契约拉取、
> 增量→落库幂等、事件幂等、思考计数、被拒卡片联动、多会话过滤、卸载退订）。

---

## 三、执行顺序

```
Phase 1 → 测试计划（本文档）
Phase 2 → L1 Mock Runtime 单元/集成测试
Phase 3 → L2 WebDriver E2E 测试
Phase 4 → L3 AI MCP 全栈测试（victauri-test Rust crate）
Phase 5 → L4 Playwright CDP 测试
Phase 6 → L5 Chaos Engineering 混沌工程测试
Phase 7 → L6 IPC 覆盖率分析 + CI集成
Phase 8 → 完整测试报告 docs/testing/full-test-report.md
```

## 四、关键说明

1. **Victauri 测试**：`victauri-test` 是 Rust crate，通过 MCP 桥接或 `cargo test` 运行，**不是 npm 包**
2. **Windows 限制**：exe 运行时无法 `cargo test`（文件锁定），需停止应用后运行 Rust 测试
3. **不确定标记**：无法自动验证的测试结果标记为"待确认"
4. **需求可追溯**：每个测试用例必须映射到需求编号

---

## 五、测试数据准备

### 5.1 基础数据

- 至少1个项目（名称："测试项目"，Key："TP"）
- 至少3个需求（不同状态：draft/in_progress/completed）
- 至少5个任务（不同类型：requirement/development/testing/bug_fix/review）
- 至少2个Skill（1内置+1自定义）

### 5.2 边界数据

- 空标题/超长标题(>200字符)
- 无效枚举值（priority="invalid", task_type="unknown"）
- 特殊字符（SQL注入 `' OR 1=1 --`、XSS `<script>alert(1)</script>`）
- 超大文本（10MB评论内容）

---

*计划生成时间: 2026-09-19*
*需求来源: design.md v2.4, plan.md, plan-v2.md, feature-spec.md v1.1*
