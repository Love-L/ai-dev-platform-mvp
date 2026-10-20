# ACE - AI Collaboration Engine — 需求文档 & 技术开发方案

> 版本：v2.4 | 更新日期：2026-09-19 | 基于评审报告 v1.0 + 用户确认调整
> 系统名称：AI Collaboration Engine (ACE)

---

## 一、产品定位与范围

### 1.1 目标用户

**单人开发者**，使用 AI Agent（如 Claude Code / Codex）辅助编码的桌面场景。非多人协作 SaaS，无用户认证/授权体系。

### 1.2 迭代规划（MVP → 后续版本）

以下功能 MVP 阶段不实现，纳入后续版本迭代：

| 项目 | MVP | v2.2 | v3.0+ |
|------|-----|------|-------|
| 多用户/多角色 | ✗ | ✗ | 视需求 |
| 移动端/Web端 | ✗ | ✗ | 视需求 |
| 云端同步 | ✗ | ✗ | 视需求 |
| 实时多人协作 | ✗ | ✗ | 视需求 |
| CI/CD 集成 | ✗ | ✓ 视需求 | — |
| 自动更新 | ✗ | ✓ | — |

**MVP 已纳入的功能（原属Out of Scope，已调整）：**

| 项目 | MVP方案 |
|------|---------|
| 富文本编辑器 | 引入 TipTap 编辑器用于需求描述/验收标准 |
| 附件上传 | MVP做文件路径引用（不上传文件），存储路径到DB |

### 1.3 非功能需求

| 维度 | 要求 |
|------|------|
| 数据库上限 | 单库不超过 100MB，超过时提示归档 |
| 并发会话 | 最大同时运行 3 个 Agent 会话 |
| 页面加载 | 列表页 < 500ms（1000条数据内） |
| 状态栏轮询 | 2秒间隔，页面不可见时暂停（document.visibilitychange） |
| 离线可用 | 完全离线运行，无需网络（除 LLM API 调用） |
| 数据备份 | 启动时自动备份 db 文件到 `backups/` 目录，保留最近 5 份 |
| API Key安全 | AES-256-GCM加密存储，密钥从硬件ID派生 |
| 会话日志清理 | 已终止超过30天的会话自动清空 output_log |
| 国际化 | 暂不支持，仅中文 |
| 可访问性 | 暂不做 WCAG 适配 |

---

## 二、需求清单（8项）

---

### 需求1：需求管理增强

#### 目标
对标 TAPD 需求管理，补齐字段，完善详情页功能。

#### 新增字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `module` | TEXT 下拉 | 否 | 所属模块：前端/后端/算法/数据/运维 |
| `owner` | TEXT | 否 | 需求负责人 |
| `developer` | TEXT | 否 | 开发负责人 |
| `tester` | TEXT | 否 | 测试负责人 |
| `source` | TEXT 下拉 | 否 | 需求来源：客户/内部/竞品/技术驱动 |
| `current_node` | TEXT | 是 | 当前节点（替代简单status） |
| `estimated_cost` | INTEGER | 否 | 预估Token消耗 |
| `actual_cost` | INTEGER | 否 | 实际Token消耗（自动累计） |
| `acceptance_criteria` | TEXT | 否 | 验收标准 |
| `iteration` | TEXT | 否 | 迭代版本 |
| `parent_id` | TEXT | 否 | 父需求ID（支持需求拆分） |
| `attachment_paths` | TEXT | 否 | 附件文件路径（JSON数组，逗号分隔） |

#### 状态流转规则

```
draft → reviewing → approved → in_development → testing → completed
                  ↘ rejected
         ↘ frozen（可从任何状态冻结，可从冻结恢复到冻结前状态）
```

不允许的流转：completed → 任何状态（已完成不可回退）

#### 详情页功能

| 功能 | 说明 | 优先级 |
|------|------|--------|
| 基本信息编辑 | 标题/描述/优先级/状态/模块/负责人等 | P0 |
| 富文本描述 | TipTap编辑器，支持标题/列表/加粗/代码块/链接 | P0 |
| 附件管理 | 文件路径引用（选择本地文件路径，存DB不上传） | P0 |
| 关联任务列表 | 展示该需求下的所有任务，支持创建 | P0 |
| PRD文档 | Markdown渲染 + AI生成 | P0 |
| 操作历史 | 展示 `requirement_history` 记录 | P1 |
| 需求树 | 展示父子需求关系 | P2 |

#### 验收标准

```
Given 需求列表有5个需求
When 点击"新建需求"并填写标题、模块、负责人、优先级
Then 创建成功，列表刷新显示新需求
And 需求详情页展示所有填写的字段

Given 需求处于 "reviewing" 状态
When 将状态改为 "approved"
Then 允许流转，操作历史记录变更

Given 需求处于 "completed" 状态
When 尝试改回 "draft"
Then 拒绝操作，提示"已完成的需求不可回退"

Given 需求下有3个关联任务
When 进入需求详情页
Then 展示3个任务的标题、状态、执行人
And 可从详情页直接创建新任务
```

#### 异常流程

| 异常 | 处理 |
|------|------|
| 标题为空 | 前端拦截，按钮置灰 |
| 标题超200字符 | 前端截断提示 |
| 非法状态流转 | 后端校验，返回错误码 `INVALID_TRANSITION` |
| 关联任务已删除 | 需求详情展示"已删除"标记，不报错 |

---

### 需求2：任务管理增强

#### 目标
对标 TAPD 任务管理，补齐字段，新增详情页。

#### 新增字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `project_id` | TEXT | 否 | 所属项目（已有，UI未使用） |
| `requirement_id` | TEXT | 否 | 关联需求（已有，UI未使用） |
| `assignee` | TEXT | 否 | 执行人（AI Agent 或人工） |
| `description` | TEXT | 否 | 任务描述 |
| `estimated_tokens` | INTEGER | 否 | 预估Token |
| `due_date` | TEXT | 否 | 截止日期 |
| `tags` | TEXT | 否 | 标签（逗号分隔） |
| `skill_config` | TEXT | 否 | 使用的Skill ID |
| `mcp_config` | TEXT | 否 | 使用的MCP配置 |
| `acceptance_criteria` | TEXT | 否 | 验收标准 |
| `blocked_by` | TEXT | 否 | 前置任务ID |

#### 详情页功能

| 功能 | 说明 | 优先级 |
|------|------|--------|
| 基本信息编辑 | 全字段可编辑 | P0 |
| 关联需求展示 | 显示所属需求标题，可跳转 | P0 |
| 执行终端 | 内嵌 AgentTerminal 组件 | P0 |
| 操作日志 | 状态变更历史 | P1 |
| 评论区 | 开发备注，支持文本输入，按时间排序 | P1 |

#### 评论功能设计

```
task_comments 表：
  id          TEXT PRIMARY KEY
  task_id     TEXT NOT NULL (FK → tasks)
  content     TEXT NOT NULL
  created_at  TEXT

前端：任务详情页底部评论列表 + 输入框
后端：create_task_comment / list_task_comments 命令
```

#### 看板增强

| 增强项 | 说明 |
|--------|------|
| 卡片信息 | 增加执行人、截止日、关联需求 |
| 筛选器 | 按项目/执行人/优先级/标签筛选 |
| 新增列 | 增加 "blocked" 列 |
| 点击卡片 | 打开任务详情页（非弹窗） |

#### 验收标准

```
Given 看板有5个任务
When 点击"新建任务"并选择关联需求、项目、Skill
Then 任务创建成功，看板对应列显示新卡片
And 卡片显示执行人、优先级、关联需求标题

Given 任务处于 "pending" 状态
When 拖拽到 "in_progress" 列
Then 状态更新成功，如果是自动会话模式则自动创建会话

Given 任务处于 "completed" 状态
When 点击卡片进入详情页
Then 显示完整信息、执行日志、终端历史输出
```

#### 异常流程

| 异常 | 处理 |
|------|------|
| 标题为空 | 前端拦截 |
| 关联的需求已被删除 | 显示"需求已删除"标记，不阻塞 |
| 截止日期早于创建日期 | 前端校验拒绝 |
| 前置任务不存在 | 后端校验，返回 `DEPENDENCY_NOT_FOUND` |

---

### 需求3：项目-需求-任务联动

#### 目标
三级数据全链路打通，任何入口都能穿透到关联数据。

#### 联动规则

```
项目 (1) ──→ (N) 需求 (1) ──→ (N) 任务
  │                                │
  └──────── (1) ──→ (N) 任务 ─────┘（任务可直接归属项目，不经过需求）
```

#### 项目详情页新增 Tab

| Tab | 内容 |
|-----|------|
| 概览 | 已有：基本信息编辑 |
| 需求 | 新增：该项目下所有需求列表，支持筛选/新建 |
| 任务 | 新增：该项目下所有任务列表，支持筛选/新建 |
| Skill | 已有：Skill配置 |
| MCP | 已有：MCP配置 |

#### 全局项目选择器

- 创建需求/任务时，下拉选择所属项目
- 默认选中最近使用的项目（存 settings 表）
- 看板顶部增加项目筛选器

#### 验收标准

```
Given 项目A下有3个需求、10个任务
When 进入项目A详情页，点击"需求" Tab
Then 展示3个需求的标题/状态/优先级
And 点击某个需求可跳转到需求详情

Given 看板顶部选择"项目A"
Then 只展示项目A的10个任务
And 切换为"全部"时展示所有任务

Given 新建需求时选择"项目A"
Then 需求自动关联到项目A
And 项目A的需求列表自动刷新
```

---

### 需求4：CLI终端双模式

#### 目标
支持自动/手动两种会话开启模式，会话输出可查看。

#### 两种模式

| 模式 | 触发条件 | 行为 |
|------|---------|------|
| **自动模式** | 任务状态变为 `in_progress` | 自动创建AgentSession并启动终端 |
| **手动模式** | 任务状态变为 `in_progress` | 任务详情页显示"开启会话"按钮，由人工决定 |

#### 设置开关

在 Settings 页面增加：
```
会话模式：[● 自动模式  ○ 手动模式]
```
存储到 `settings` 表：`session_mode = "auto" | "manual"`

#### 会话输出方案

| 维度 | 方案 |
|------|------|
| 实时输出 | PTY 事件通过 Tauri event 系统推送到前端（修复 T24） |
| 输出存储 | **SQLite blob**：`agent_sessions.output_log` BLOB 字段 |
| 历史回放 | 读取 blob 数据，逐行渲染到终端组件 |
| 存储上限 | 单会话日志最大 10MB（约10万行），超过时截断头部 |
| 定期清理 | 已终止超过 30 天的会话自动清空 output_log（保留元数据） |
| 清理策略 | 启动时执行 `DELETE FROM agent_sessions WHERE status IN ('completed','failed','terminated') AND ended_at < datetime('now','-30 days') AND output_log IS NOT NULL` |

**选择SQLite blob的理由：**
- 与现有数据模型一致，无需额外文件系统管理
- 单机场景下blob查询效率可接受
- 便于事务一致性（会话元数据+日志在同一DB）

#### 会话与任务绑定

- 创建会话时写入 `task_id` 和 `project_id`
- 任务详情页展示关联会话列表
- 会话详情页展示关联任务信息

#### 验收标准

```
Given 设置为"自动模式"
When 将任务从 "pending" 拖到 "in_progress"
Then 自动创建AgentSession
And 任务详情页显示内嵌终端，实时输出Agent执行内容

Given 设置为"手动模式"
When 任务状态变为 "in_progress"
Then 任务详情页显示"开启会话"按钮
And 点击按钮后创建会话并打开终端

Given 会话已运行10分钟
When 关闭终端弹窗后重新打开
Then 从日志文件回放历史输出
And 可继续实时查看新输出
```

---

### 需求5：Skill替换

#### 目标
内置Skill可派生自定义版本，支持编辑和切换。

#### Skill版本模型

```
prd_skills 表新增字段：
  version      INTEGER DEFAULT 1    -- 版本号
  parent_id    TEXT                  -- 父Skill ID（自定义版本指向内置版本）
  is_deleted   INTEGER DEFAULT 0    -- 软删除
```

#### 替换流程

```
内置Skill (is_builtin=1, version=1)
    │
    ├─→ "基于此创建自定义版本"
    │     创建新记录：name="自定义xxx", parent_id=内置ID, is_builtin=0, version=1
    │
    └─→ 自定义版本可编辑 name/description/template
          编辑后 version+1（保留历史版本）
```

#### 项目级Skill选择

- 项目详情 → Skill配置 Tab
- 添加Skill时可选择：内置版本 或 自定义版本
- 任务创建时继承项目默认Skill

#### 验收标准

```
Given 有4个内置Skill
When 点击"功能需求 PRD"的"创建自定义版本"
Then 创建一个名为"自定义 功能需求 PRD"的新Skill
And 可编辑其模板内容
And 原内置Skill不受影响

Given 项目A配置了自定义Skill
When 创建新任务
Then 任务默认使用项目A的自定义Skill
And 可在任务详情中切换为其他Skill
```

---

### 需求6：Skill管理后台

#### 目标
独立管理页，查看Skill使用情况和执行日志。

#### Skill管理页功能

| 功能 | 说明 | 优先级 |
|------|------|--------|
| Skill列表 | 展示所有Skill（内置+自定义），支持筛选 | P0 |
| 新建Skill | 从零创建自定义Skill | P0 |
| 编辑Skill | 编辑模板内容（CodeMirror Markdown编辑器，语法高亮） | P0 |
| 删除Skill | 自定义可删，内置不可删 | P0 |
| 执行日志 | 查看 `skill_execution_logs`，按Skill分组 | P1 |
| 使用统计 | 调用次数/成功率/Token消耗 | P1 |
| 迭代建议 | 基于执行日志自动生成优化建议 | P1 |
| 导入/导出 | JSON格式导入导出Skill模板 | P2 |

#### luban-skill 模型格式

Skill模板遵循以下结构：

```markdown
# {skill_name}

## 输入参数
- title: 需求标题 (必填)
- description: 需求描述 (必填)
- context: 上下文信息 (选填)

## 输出格式
- format: markdown
- sections: [背景, 方案, 验收标准, 风险]

## 约束条件
- max_tokens: 4000
- temperature: 0.7

## Prompt模板
（用户自定义的Prompt内容）
```

#### 迭代升级建议（MVP实现）

基于执行日志自动分析，生成优化建议：

| 规则 | 条件 | 建议内容 |
|------|------|---------|
| 成功率低 | 成功率 < 80%（且执行次数 ≥ 5） | "该Skill成功率偏低，建议检查模板Prompt" |
| Token消耗高 | 平均Token > 5000 | "Token消耗较高，建议精简Prompt内容" |
| 长期未使用 | 最近30天无执行记录 | "该Skill超过30天未使用，建议归档" |
| 执行缓慢 | 平均执行时间 > 60秒 | "执行时间较长，建议优化模板复杂度" |

建议展示位置：Skill管理页每个Skill卡片下方，带图标提示

#### 验收标准

```
Given 有4个内置Skill和2个自定义Skill
When 进入Skill管理页
Then 展示6个Skill，标记"内置"/"自定义"
And 自定义Skill可编辑/删除

Given 某Skill被执行了10次，成功8次
When 查看该Skill的使用统计
Then 显示：调用10次，成功率80%，总Token 12000

Given 导出一个Skill为JSON文件
When 在另一个环境导入
Then Skill模板完整恢复
```

---

### 需求7：仪表盘→工作台

#### 目标
首页从统计概览改为可操作的工作台。

#### 工作台模块

| 模块 | 内容 | 优先级 | 数据源 |
|------|------|--------|--------|
| 我的待办 | 分配给我的需求+任务 | P0 | tasks WHERE assignee=me |
| 快速操作 | 新建需求/新建任务/创建会话 | P0 | 无 |
| 进行中 | 正在执行的任务+会话状态 | P0 | tasks + agent_sessions |
| 系统资源 | 内存/会话数/任务数（状态栏数据） | P0 | get_system_info |
| 最近活动 | 操作时间线 | P1 | requirement_history + task变更 |
| 项目概览 | 各项目进度条 | P1 | projects + task stats |
| Token趋势 | 近7天消耗曲线 | P2 | token_usage |
| Agent状态 | 各Agent当前任务数 | P2 | agents + sessions |

#### 验收标准

```
Given 有2个分配给我的任务
When 打开工作台
Then "我的待办"区域展示2个任务，可点击跳转

Given 有1个运行中的会话
When 打开工作台
Then "进行中"区域展示会话状态+关联任务
And 系统资源显示当前内存占用和活跃会话数

Given 点击"新建需求"
Then 弹出需求创建表单
And 创建成功后自动刷新待办列表
```

---

### 需求8：系统资源监控

#### 目标
应用级资源监控 + 状态栏实时展示。

#### 采集指标

| 指标 | 采集方式 | 频率 | 存储 |
|------|---------|------|------|
| 应用内存 (RSS) | sysinfo crate | 2秒 | 不存储（实时） |
| 应用CPU% | sysinfo crate | 2秒 | 不存储 |
| 应用启动时间 | std::time::Instant | 启动时 | 内存 |
| 活跃会话数 | DB查询 | 2秒 | 不存储 |
| 待处理任务数 | DB查询 | 5秒 | 不存储 |
| 会话峰值内存 | sysinfo 追踪子进程 | 按需 | agent_sessions 表 |
| 会话平均CPU | sysinfo 追踪子进程 | 按需 | agent_sessions 表 |

#### 状态栏设计

```
┌──────────────────────────────────────────────────────────────┐
│ ● 就绪           📊 215MB  >_ 2  ⚙ 3  v2.0.0               │
└──────────────────────────────────────────────────────────────┘
     左侧               右侧（从左到右）
```

| 元素 | 含义 | 交互 |
|------|------|------|
| `📊 215MB` | 应用内存占用 | 悬停tooltip：CPU%、启动时间、运行时长 |
| `>_ 2` | 活跃会话数 | 悬停tooltip：各会话名称+状态；点击跳转Sessions |
| `⚙ 3` | 待处理任务数 | 悬停tooltip：任务列表；点击跳转Board |
| `v2.0.0` | 版本号 | 静态 |

#### 资源告警（MVP实现）

| 告警条件 | 展示方式 | 交互 |
|---------|---------|------|
| 内存 > 1GB | 状态栏内存数字变红 | 悬停tooltip增加"内存占用过高"提示 |
| 会话数 ≥ 3 | 状态栏会话数变红 | 悬停tooltip提示"会话已满，需关闭后才能新建" |
| 待处理任务 > 20 | 任务数数字变橙 | 悬停tooltip提示"任务积压过多" |

告警阈值存储在 settings 表，支持后续用户自定义：
```
alert_memory_threshold = 1024    (MB)
alert_session_threshold = 3
alert_task_threshold = 20
```

#### 验收标准

```
Given 应用启动后
When 查看状态栏右侧
Then 显示实时内存占用（如 "📊 215MB"）
And 数值每2秒刷新

Given 有2个运行中的会话
When 查看状态栏
Then 显示 ">_ 2"
And 悬停显示2个会话的名称和状态

Given 鼠标悬停在内存数字上
Then 显示tooltip：CPU 5.2%、启动于 10:30、已运行 23分钟
```

---

### 需求9：系统托盘图标

#### 目标
应用最小化到系统托盘，右键菜单提供快捷操作和退出功能。

#### 现状

| 维度 | 状态 |
|------|------|
| Cargo.toml | ✅ `tray-icon` feature 已启用 |
| main.rs | ❌ 无 TrayIcon 创建代码 |
| tauri.conf.json | ❌ 无 tray 权限配置 |
| capabilities | ❌ 无 capabilities 目录 |
| 图标资源 | ✅ icon.ico 已生成（32x32 适配托盘） |

#### 功能设计

**右键菜单结构：**

```
┌─────────────────────────┐
│  AI研发协作平台 v2.0    │  ← 灰色标题（不可点击）
├─────────────────────────┤
│  📋 显示主窗口          │  ← P0
│  📌 最小化到托盘        │  ← P0（关闭窗口时自动执行）
├─────────────────────────┤
│  📊 运行中 · 2个会话    │  ← 灰色状态文字（不可点击）
│  💾 内存: 215MB         │  ← 灰色资源信息（不可点击）
├─────────────────────────┤
│  ➕ 新建需求            │  ← P1
│  ➕ 新建任务            │  ← P1
├─────────────────────────┤
│  ⚙️ 设置               │  ← P1
├─────────────────────────┤
│  🚪 退出               │  ← P0
└─────────────────────────┘
```

**菜单项详情：**

| 菜单项 | 类型 | 动作 | 优先级 |
|--------|------|------|--------|
| AI研发协作平台 v2.0 | 标题（灰） | 无 | P0 |
| 显示主窗口 | 可点击 | `window.show()` + `window.set_focus()` | P0 |
| 最小化到托盘 | 可点击 | `window.hide()` | P0 |
| 状态/资源信息 | 禁用文字 | 显示运行状态+内存 | P1 |
| 新建需求 | 可点击 | 发送事件 → 前端路由到 `/requirements` 新建弹窗 | P1 |
| 新建任务 | 可点击 | 发送事件 → 前端路由到 `/board` 新建弹窗 | P1 |
| 设置 | 可点击 | 发送事件 → 前端路由到 `/settings` | P1 |
| 退出 | 可点击 | `process::exit(0)` 或 `app.exit(0)` | P0 |

#### 窗口行为变更

| 行为 | 当前 | 改为 |
|------|------|------|
| 点击窗口关闭按钮 | 退出应用 | 隐藏到系统托盘 |
| 点击任务栏图标 | 显示/隐藏切换 | 同左（不变） |
| 托盘右键"显示主窗口" | — | 显示并聚焦窗口 |
| 托盘右键"退出" | — | 真正退出应用 |
| 应用启动时 | 直接显示窗口 | 显示窗口 + 创建托盘图标 |

#### 技术实现

**1. capabilities 权限配置**

```json
// src-tauri/capabilities/default.json
{
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:tray:default",
    "core:window:default",
    "core:event:default"
  ]
}
```

**2. 托盘模块 (src-tauri/src/tray.rs)**

```rust
use tauri::{
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    menu::{Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Manager,
};

pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 状态信息（灰色禁用项）
    let status_item = MenuItem::with_id(app, "status", "就绪", false, None::<&str>)?;
    let resource_item = MenuItem::with_id(app, "resource", "内存: --", false, None::<&str>)?;

    // 可操作菜单项
    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "最小化到托盘", true, None::<&str>)?;
    let new_req_item = MenuItem::with_id(app, "new_req", "新建需求", true, None::<&str>)?;
    let new_task_item = MenuItem::with_id(app, "new_task", "新建任务", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    // 组装菜单
    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &resource_item,
            &PredefinedMenuItem::separator(app)?,
            &show_item,
            &hide_item,
            &PredefinedMenuItem::separator(app)?,
            &new_req_item,
            &new_task_item,
            &PredefinedMenuItem::separator(app)?,
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;

    // 创建托盘图标
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("AI研发协作平台")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
                "new_req" => {
                    let _ = app.emit("tray-action", "new_requirement");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "new_task" => {
                    let _ = app.emit("tray-action", "new_task");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "settings" => {
                    let _ = app.emit("tray-action", "settings");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            // 左键点击托盘图标：显示/隐藏切换
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
```

**3. 窗口关闭事件拦截 (main.rs)**

```rust
// 在 setup 中拦截窗口关闭事件
let app_handle = app.handle().clone();
app.listen("window-close-requested", move |event| {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();  // 隐藏而非退出
    }
});
```

**4. 前端监听托盘事件**

```typescript
// App.tsx 或 Layout.tsx
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "react-router-dom";

useEffect(() => {
    const unlisten = listen("tray-action", (event) => {
        switch (event.payload) {
            case "new_requirement": navigate("/requirements"); break;
            case "new_task": navigate("/board"); break;
            case "settings": navigate("/settings"); break;
        }
    });
    return () => { unlisten.then(fn => fn()); };
}, [navigate]);
```

**5. 托盘状态更新IPC**

```rust
#[tauri::command]
fn update_tray_status(app: AppHandle, status: String, memory: String, sessions: u32) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_menu_item_text("status", &status);
        let _ = tray.set_menu_item_text("resource", &format!("内存: {} · {}个会话", memory, sessions));
    }
}
```

#### 验收标准

```
Given 应用启动
When 查看系统托盘区域
Then 出现应用图标
And 右键显示菜单，包含"显示主窗口"、"退出"等选项

Given 应用窗口可见
When 点击窗口关闭按钮（X）
Then 窗口隐藏，应用继续运行
And 托盘图标仍在

Given 应用窗口隐藏
When 点击托盘图标（左键）
Then 窗口显示并聚焦

Given 应用窗口隐藏
When 右键托盘图标，点击"显示主窗口"
Then 窗口显示并聚焦

Given 应用窗口隐藏
When 右键托盘图标，点击"退出"
Then 应用真正退出

Given 有2个运行中的会话
When 右键托盘图标
Then 状态区域显示"运行中 · 2个会话"
And 资源区域显示当前内存
```

#### 异常流程

| 异常 | 处理 |
|------|------|
| 系统不支持托盘（极少见） | 降级为普通窗口模式，关闭按钮直接退出 |
| 托盘图标不显示 | 检查 icon.ico 尺寸（需32x32），日志记录错误 |
| 多次点击退出 | 进程直接终止，无确认弹窗 |

---

## 三、技术开发方案

### 3.1 总体架构（调整后）

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri 2.0 Desktop                     │
├──────────────────────┬──────────────────────────────────┤
│     Frontend (React) │        Backend (Rust)             │
│                      │                                   │
│  ┌────────────────┐  │  ┌─────────────────────────────┐ │
│  │ Zustand Stores │  │  │ Commands (IPC)              │ │
│  │ (统一数据层)    │──│──│ typed Result<T, AppError>   │ │
│  └────────────────┘  │  └──────────┬──────────────────┘ │
│                      │             │                     │
│  ┌────────────────┐  │  ┌──────────▼──────────────────┐ │
│  │ Page Components│  │  │ Service Layer               │ │
│  │ (100% Store)   │  │  │ (全部通过Service访问DB)      │ │
│  └────────────────┘  │  └──────────┬──────────────────┘ │
│                      │             │                     │
│  ┌────────────────┐  │  ┌──────────▼──────────────────┐ │
│  │ Shared Components│ │  │ Database (SQLite + WAL)     │ │
│  │ Modal/Badge/... │  │  │ 版本化迁移系统              │ │
│  └────────────────┘  │  └─────────────────────────────┘ │
│                      │                                   │
│  ┌────────────────┐  │  ┌─────────────────────────────┐ │
│  │ System Monitor │  │  │ SystemMonitor (sysinfo)     │ │
│  │ (状态栏轮询)    │──│──│ 后台线程2秒刷新             │ │
│  └────────────────┘  │  └─────────────────────────────┘ │
└──────────────────────┴──────────────────────────────────┘
```

### 3.2 DB迁移系统重构（解决 T4/T5）

#### 设计：版本化迁移

```rust
// 新增 db/migration.rs

pub struct Migration {
    pub version: i32,
    pub description: String,
    pub up: &'static str,    // SQL
}

pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "初始schema".into(),
            up: include_str!("migrations/001_init.sql"),
        },
        Migration {
            version: 2,
            description: "需求管理增强字段".into(),
            up: include_str!("migrations/002_requirement_fields.sql"),
        },
        Migration {
            version: 3,
            description: "任务管理增强字段".into(),
            up: include_str!("migrations/003_task_fields.sql"),
        },
        // ...
    ]
}
```

#### schema_version 表

```sql
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    description TEXT,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### 迁移执行流程

```
启动 → 读取 schema_version 最大版本号
     → 对比代码中 get_migrations() 的最新版本
     → 逐个执行未应用的迁移（每个迁移独立事务）
     → 失败时记录错误版本号，下次启动重试
     → 迁移前自动备份 db 文件
```

#### 迁移文件组织

```
src-tauri/src/db/migrations/
├── 001_init.sql                 -- 初始schema（从init_tables提取）
├── 002_requirement_fields.sql   -- 需求增强字段
├── 003_task_fields.sql          -- 任务增强字段
├── 004_session_fields.sql       -- 会话扩展字段
├── 005_skill_versioning.sql     -- Skill版本管理
├── 006_indexes.sql              -- 补充索引
└── ...
```

### 3.3 AppError 统一错误枚举（解决 T3/T11）

```rust
// 新增 errors.rs

use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum AppError {
    NotFound(String),           // 资源不存在
    InvalidInput(String),       // 输入校验失败
    InvalidTransition(String),  // 状态流转非法
    DatabaseError(String),      // 数据库错误
    InternalError(String),      // 内部错误
    DependencyError(String),    // 依赖错误（如前置任务未完成）
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "NOT_FOUND: {}", msg),
            Self::InvalidInput(msg) => write!(f, "INVALID_INPUT: {}", msg),
            Self::InvalidTransition(msg) => write!(f, "INVALID_TRANSITION: {}", msg),
            Self::DatabaseError(msg) => write!(f, "DATABASE_ERROR: {}", msg),
            Self::InternalError(msg) => write!(f, "INTERNAL_ERROR: {}", msg),
            Self::DependencyError(msg) => write!(f, "DEPENDENCY_ERROR: {}", msg),
        }
    }
}

// Tauri command 返回 Result<T, AppError>
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
```

前端可按前缀判断错误类型：
```typescript
catch (e) {
  if (e.startsWith("NOT_FOUND:")) { /* 资源不存在 */ }
  else if (e.startsWith("INVALID_TRANSITION:")) { /* 状态错误 */ }
  // ...
}
```

### 3.4 分页系统（解决 T10/T19）

#### 后端

```rust
// 新增分页参数
pub struct PaginationParams {
    pub page: u32,        // 默认 1
    pub page_size: u32,   // 默认 20，最大 100
}

pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}
```

所有 `list_*` 命令统一改为分页查询：

```
list_requirements(page, page_size, status_filter, project_filter)
list_tasks(page, page_size, status_filter, type_filter, project_filter)
list_projects(page, page_size)
list_sessions(page, page_size, status_filter)
```

前端列表组件统一使用分页组件。

### 3.5 时间戳统一（解决 T21）

**方案：全部使用 ISO 8601 格式**

```rust
// 统一时间戳生成
fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()  // "2026-09-19T12:34:56.789+00:00"
}

// SQL DEFAULT 改为：
// created_at TEXT DEFAULT (datetime('now'))
// 或在 Rust 层统一生成，SQL 不设 DEFAULT
```

迁移时批量转换旧格式：
```sql
-- 将 "YYYY-MM-DD HH:MM:SS" 转为 ISO 8601
UPDATE requirements SET created_at = created_at || 'Z'
WHERE created_at NOT LIKE '%T%';
```

### 3.6 删除保护策略（解决 T22）

**策略：有关联数据时禁止删除，提示用户先清理**

```rust
// 后端删除前检查
fn check_cannot_delete_project(db: &Database, project_id: &str) -> Result<(), AppError> {
    let req_count = db.get_requirement_count_by_project(project_id)?;
    let task_count = db.get_task_count_by_project(project_id)?;
    if req_count > 0 || task_count > 0 {
        return Err(AppError::InvalidInput(format!(
            "项目下有 {} 个需求和 {} 个任务，请先清理",
            req_count, task_count
        )));
    }
    Ok(())
}

fn check_cannot_delete_requirement(db: &Database, req_id: &str) -> Result<(), AppError> {
    let task_count = db.get_task_count_by_requirement(req_id)?;
    if task_count > 0 {
        return Err(AppError::InvalidInput(format!(
            "需求下有 {} 个关联任务，请先删除或解绑",
            task_count
        )));
    }
    Ok(())
}
```

**前端交互：** 删除时弹出确认框，显示关联数据数量，用户确认后执行。

**外键约束保持 NO ACTION**（不做CASCADE），通过应用层显式检查保护数据完整性。

### 3.7 PTY事件通道修复（解决 T24）

```rust
// main.rs 中保留事件通道

let (pty_event_sender, pty_event_receiver) = tokio::sync::mpsc::channel(100);

// 保留 sender 给 PtyManager 使用
// 将 receiver 转为 Tauri 事件推送到前端

let app_handle = app.handle().clone();
tokio::spawn(async move {
    let mut receiver = pty_event_receiver;
    while let Some(event) = receiver.recv().await {
        let _ = app_handle.emit("pty-output", event);
    }
});
```

前端 AgentTerminal 监听：
```typescript
import { listen } from "@tauri-apps/api/event";
const unlisten = await listen("pty-output", (event) => {
    // 处理 PTY 输出
});
```

### 3.8 前端统一数据层（解决 T27）

**原则：所有页面通过 Store 获取数据，禁止直接调 IPC**

当前需迁移的7个页面：

| 页面 | 当前方式 | 迁移到 |
|------|---------|--------|
| Cost | 直接调 tauri.ts | 新增 useCostStore |
| Statistics | 直接调 tauri.ts | 新增 useStatisticsStore |
| Commits | 直接调 tauri.ts | 新增 useCommitsStore |
| Agents | 直接调 tauri.ts | 新增 useAgentsStore |
| RequirementDetail | 直接调 tauri.ts | 复用 useRequirementsStore |
| SessionDetail | 直接调 tauri.ts | 复用 useSessionsStore |
| ProjectDetail | 直接调 tauri.ts | 复用 useProjectsStore |

### 3.9 会话输出持久化方案（解决 C2）

```
存储方式：SQLite BLOB 字段 (agent_sessions.output_log)
数据格式：JSON Lines（每行一条事件）
  {"ts":"2026-09-19T10:30:00Z","type":"output","data":"hello"}
  {"ts":"2026-09-19T10:30:01Z","type":"input","data":"npm install"}
  {"ts":"2026-09-19T10:30:05Z","type":"exit","code":0}

写入：PTY事件回调中追加到 BLOB（定期 batch 写入，每5秒或每100条）
读取：会话详情页打开时读取 BLOB → 按行解析
截断：超过10MB时截取前5MB（保留开头+结尾）
清理：启动时清理30天前已终止会话的 output_log
      SQL: UPDATE agent_sessions SET output_log = NULL
           WHERE status IN ('completed','failed','terminated')
           AND ended_at < datetime('now','-30 days')
           AND output_log IS NOT NULL
```

### 3.10 数据库备份策略（解决备份需求）

```rust
// 启动时自动备份
fn auto_backup(db_path: &Path) {
    let backup_dir = db_path.parent().unwrap().join("backups");
    std::fs::create_dir_all(&backup_dir).ok();

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("app_{}.db", timestamp));

    std::fs::copy(db_path, &backup_path).ok();

    // 保留最近5份，删除旧的
    let mut backups: Vec<_> = std::fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "db"))
        .collect();
    backups.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).unwrap());
    while backups.len() > 5 {
        if let Some(old) = backups.first() {
            std::fs::remove_file(old.path()).ok();
            backups.remove(0);
        }
    }
}
```

### 3.11 useEffect cleanup（解决 T17/T18）

所有包含轮询的页面统一模式：

```typescript
useEffect(() => {
    let cancelled = false;

    async function load() {
        const data = await fetchXxx();
        if (!cancelled) setData(data);
    }

    load();
    const timer = setInterval(load, interval);

    return () => {
        cancelled = true;
        clearInterval(timer);
    };
}, []);
```

状态栏轮询使用全局单例：

```typescript
// stores/systemMonitorStore.ts
let pollTimer: number | null = null;

export const useSystemMonitorStore = create((set) => ({
    memory: 0,
    sessions: 0,
    tasks: 0,
    startPolling: () => {
        if (pollTimer) return;
        pollTimer = window.setInterval(async () => {
            if (document.hidden) return; // 页面不可见时暂停
            const info = await getSystemInfo();
            set({ memory: info.memory_mb, sessions: info.session_count, tasks: info.pending_tasks });
        }, 2000);
    },
    stopPolling: () => {
        if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
    },
}));
```

### 3.12 Input校验（解决 T12/T13）

```rust
// 后端校验函数
fn validate_requirement(title: &str, priority: &str) -> Result<(), AppError> {
    if title.trim().is_empty() || title.len() > 200 {
        return Err(AppError::InvalidInput("标题长度1-200字符".into()));
    }
    if !["P0","P1","P2","P3"].contains(&priority) {
        return Err(AppError::InvalidInput("无效的优先级".into()));
    }
    Ok(())
}

fn validate_task(title: &str, task_type: &str, priority: &str) -> Result<(), AppError> {
    if title.trim().is_empty() || title.len() > 200 {
        return Err(AppError::InvalidInput("标题长度1-200字符".into()));
    }
    if !["requirement","development","testing","bug_fix","review"].contains(&task_type) {
        return Err(AppError::InvalidInput("无效的任务类型".into()));
    }
    if !["P0","P1","P2","P3"].contains(&priority) {
        return Err(AppError::InvalidInput("无效的优先级".into()));
    }
    Ok(())
}
```

前端防重复提交：
```typescript
const [submitting, setSubmitting] = useState(false);

async function handleCreate() {
    if (submitting) return;
    setSubmitting(true);
    try { await createXxx(); }
    finally { setSubmitting(false); }
}
```

### 3.13 死代码清理 + 新增依赖（解决 T25/T16/T31 + 用户确认）

#### 死代码清理

| 项目 | 处理 |
|------|------|
| AppState.llm_client | 删除字段，PrdService自行创建client |
| requirement_history表 | 需求状态变更时写入，详情页展示 |
| Requirements.tsx FileText | 替换为lucide-react导入 |

#### 新增前端依赖

| 依赖 | 版本 | 用途 | 所属需求 |
|------|------|------|---------|
| `@tiptap/react` + `@tiptap/starter-kit` | ^2.x | 富文本编辑器（需求描述/验收标准） | 需求1 |
| `@uiw/react-codemirror` + `@uiw/codemirror-theme-github` | ^4.x | Markdown编辑器（Skill模板编辑） | 需求6 |

#### API Key AES加密

```
加密方案：
- 密钥派生：从机器硬件ID + 固定盐值通过 PBKDF2 派生32字节密钥
- 加密算法：AES-256-GCM
- 存储格式：base64(IV + ciphertext + tag)
- 加密/解密：Rust aes-gcm crate
- 前端不持有密钥，仅通过IPC调用 encrypt/decrypt 命令

新增 Cargo 依赖：
  aes-gcm = "0.10"
  pbkdf2 = { version = "0.12", features = ["hmac"] }
  hmac = "0.12"

新增 commands：
  encrypt_api_key(plaintext) -> String
  decrypt_api_key(ciphertext) -> String

Settings 页面：
  输入框调用 encrypt_api_key 后存储密文
  读取时调用 decrypt_api_key 解密展示（密码框内显示●●●）
```

#### 新增DB表

```sql
-- 任务评论表
CREATE TABLE IF NOT EXISTS task_comments (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);
CREATE INDEX IF NOT EXISTS idx_task_comments_task ON task_comments(task_id);

-- agent_sessions 表新增 output_log 字段
-- (通过迁移脚本 ALTER TABLE)
```

### 3.14 Toast通知系统（解决 T28）

```typescript
// 新增 components/Toast.tsx
// 使用简单的状态管理，不引入额外依赖

// 在 App.tsx 根级别提供 Toast Context
// 各页面操作成功/失败时调用 showToast("保存成功", "success")
```

---

## 四、分阶段执行计划（调整后）

### Phase 0：基础设施修复（7.5天）

**目标：** 解决所有CRITICAL和HIGH级技术债务

| 天数 | 任务 | 解决问题 |
|------|------|---------|
| 1 | DB迁移系统重构（migration.rs + schema_version） | T4/T5 |
| 1 | 时间戳统一 + 删除保护策略 + 索引补充 | T21/T22/T8/T20 |
| 0.5 | AgentSession结构体同步 + output_log字段 | T23/需求4 |
| 1 | PTY事件通道修复 + 备份策略 | T24/备份 |
| 0.5 | AppError枚举定义 + Input校验框架 | T3/T11/T12 |
| 1 | API Key AES加密（新增aes-gcm依赖） | T14 |
| 1 | 系统托盘图标（tray.rs + capabilities + 窗口关闭拦截） | 需求9 |
| 0.5 | 死代码清理（llm_client等） | T25/T31 |
| 0.5 | 新增前端依赖（TipTap + CodeMirror）+ 编译验证 | 需求1/6 |
| 0.5 | 编译验证 + 回归测试 | — |

**交付物：** 健壮的基础设施 + 系统托盘，可支撑后续所有功能开发

### Phase 1：需求/任务增强（7-8天）

**目标：** TAPD级字段 + 详情页 + 富文本 + 评论

| 天数 | 任务 |
|------|------|
| 1 | DB迁移：requirements/tasks/agent_sessions 新增字段 + task_comments表 |
| 1 | 后端：扩展 create/update 命令 + 分页 + 评论CRUD + 删除保护校验 |
| 1 | 前端：RequirementForm（含TipTap富文本编辑器 + 附件路径选择） |
| 1 | 前端：RequirementDetail 重构（关联任务+操作历史+富文本展示） |
| 1 | 前端：TaskForm 组件 + TaskDetail 新增（含评论区） |
| 1 | 前端：Board 看板增强（卡片+筛选+新列blocked） |
| 0.5 | Store迁移：统一数据层（T27） |
| 0.5 | 需求状态流转校验 + requirement_history写入 |

**交付物：** 完整的需求/任务管理功能（含富文本+评论）

### Phase 2：三级联动（3-4天）

**目标：** 项目→需求→任务全链路打通

| 天数 | 任务 |
|------|------|
| 1 | ProjectDetail 增加需求/任务 Tab |
| 0.5 | 全局 ProjectSelector 组件 |
| 1 | 后端：list_by_project 查询命令 |
| 0.5 | Board 按项目筛选 |
| 0.5 | 需求→任务创建联动 |

**交付物：** 三级数据完全联动

### Phase 3：会话双模式（3-4天）

**目标：** 自动/手动模式 + 输出持久化

| 天数 | 任务 |
|------|------|
| 0.5 | Settings 增加 session_mode 开关 |
| 1 | AgentService：自动模式（任务状态变更→创建会话） |
| 0.5 | TaskDetail：手动模式（"开启会话"按钮） |
| 1 | 会话输出持久化（日志文件+回放） |
| 0.5 | TaskDetail 内嵌终端 + 会话资源面板 |

**交付物：** 完整的双模式会话系统

### Phase 4：Skill管理（4-5天）

**目标：** 独立管理页 + 替换 + 日志 + 迭代建议

| 天数 | 任务 |
|------|------|
| 0.5 | DB迁移：Skill版本字段 (migrations/005) |
| 1 | SkillManagement 页面（CRUD + CodeMirror模板编辑器） |
| 0.5 | Skill替换功能（创建自定义版本） |
| 0.5 | SkillLogs 页面（执行日志+使用统计） |
| 1 | 迭代建议引擎（成功率/Token/未使用/耗时规则） |
| 0.5 | 导入/导出功能 |
| 0.5 | 导航菜单 + 路由更新 |

**交付物：** 完整的Skill生命周期管理（含智能建议）

### Phase 5：工作台 + 系统监控（4-5天）

**目标：** Dashboard→Workspace + 资源监控 + 告警

| 天数 | 任务 |
|------|------|
| 1 | Workspace 页面（我的待办+快速操作+进行中+系统资源） |
| 0.5 | system_monitor.rs + sysinfo集成 |
| 0.5 | 状态栏重构 + 全局轮询单例 |
| 0.5 | 资源告警逻辑（内存/会话数/任务积压 + 阈值可配置） |
| 0.5 | Toast通知系统 |
| 0.5 | useEffect cleanup 全量修复 |
| 0.5 | Commits/Sessions 样式统一 |

**交付物：** 完整的工作台 + 系统监控 + 资源告警

### 总排期

```
Phase 0  基础设施修复   ████████████████████  7.5天
Phase 1  需求/任务增强   ░░░░░░░░████████████████  7-8天
Phase 2  三级联动       ░░░░░░░░░░░░░░░░████████  3-4天
Phase 3  会话双模式     ░░░░░░░░░░░░░░░░████████  3-4天（可与P2并行）
Phase 4  Skill管理      ░░░░░░░░░░░░████████░░░░  4-5天（可与P3并行）
Phase 5  工作台+监控    ░░░░░░░░░░░░░░░░░░░░████  4-5天
                         ────────────────────────
                         总计约 29-34 天
```

---

## 五、问题追踪对照表

| 原编号 | 严重程度 | 问题 | 解决方案 | 解决阶段 |
|--------|---------|------|---------|---------|
| T1 | HIGH | commands绕过Service | Phase 1统一迁移 | P1 |
| T2 | MEDIUM | Database职责过重 | 迁移逻辑抽取到migration.rs | P0 |
| T3 | MEDIUM | 无统一错误处理 | AppError枚举 | P0 |
| T4 | CRITICAL | 迁移无事务保护 | 版本化迁移系统 | P0 |
| T5 | HIGH | migrate_tables膨胀 | 迁移文件拆分 | P0 |
| T6 | MEDIUM | UUID v4无序 | MVP保持，后续可换ULID | — |
| T7 | MEDIUM | create_project返回类型 | 修复返回类型 | P0 |
| T8 | MEDIUM | tasks缺project_id索引 | 补充索引 | P0 |
| T9 | LOW | 时间戳TEXT类型 | MVP可接受 | — |
| T10 | HIGH | list无分页 | 统一分页系统 | P0 |
| T11 | HIGH | 无typed error | AppError枚举 | P0 |
| T12 | MEDIUM | 无输入校验 | validate函数 | P0 |
| T13 | MEDIUM | 无幂等设计 | 前端防重复+后端校验 | P1 |
| T14 | HIGH | API Key明文 | AES-256-GCM加密存储 | P0 |
| T15 | MEDIUM | project_path无校验 | 增加路径校验 | P1 |
| T16 | LOW | 审计日志未使用 | 需求状态变更时写入 | P1 |
| T17 | HIGH | useEffect未cleanup | 全量修复 | P5 |
| T18 | MEDIUM | 轮询多实例 | 全局单例 | P5 |
| T19 | MEDIUM | 全量查询+前端过滤 | 后端分页 | P0 |
| T20 | LOW | 无busy_timeout | 添加PRAGMA | P0 |
| T21 | HIGH | 时间戳格式不一致 | 统一ISO 8601 | P0 |
| T22 | HIGH | 无CASCADE | 重建表+应用层保护 | P0 |
| T23 | MEDIUM | AgentSession字段缺失 | 同步结构体 | P0 |
| T24 | HIGH | PTY事件通道断裂 | Tauri event转发 | P0 |
| T25 | MEDIUM | llm_client死代码 | 删除 | P0 |
| T26 | MEDIUM | without_db()风险 | 移除，改用trait | P1 |
| T27 | HIGH | 7页面绕过Store | 统一Store | P1 |
| T28 | MEDIUM | error未渲染 | Toast系统 | P5 |
| T29 | MEDIUM | 样式不一致 | 统一组件类 | P5 |
| T30 | MEDIUM | JSON.parse未保护 | 加try-catch | P1 |
| T31 | LOW | 本地FileText | lucide导入 | P0 |
| C1 | HIGH | 需求1缺附件/富文本 | TipTap编辑器 + 文件路径引用 | P0/P1 |
| C2 | HIGH | 会话输出未定义 | 文件系统+日志方案 | P3 |
| C3 | HIGH | luban-skill模型未定义 | Markdown模板格式 | P4 |
| C4 | MEDIUM | 资源告警未定义 | 内存/会话/任务积压告警 + 可配置阈值 | P5 |
| D1 | LOW | CPU监控价值有限 | 保留，成本低 | — |
