# AI研发协作平台 — 开发计划

> 基于 design.md v2.3 | 总工期：29-34天 | 9项需求 + 31项技术修复

---

## 一、总览

```
需求覆盖：9项（需求管理/任务管理/联动/会话/Skill/工作台/监控/托盘）
技术修复：31项（2 CRITICAL / 10 HIGH / 14 MEDIUM / 5 LOW）
新增文件：约 15 个
修改文件：约 25 个
新增依赖：sysinfo / aes-gcm / pbkdf2 / @tiptap/react / @uiw/react-codemirror
```

### 里程碑

```
M0  Day 7.5   基础设施就绪 — 可编译运行，托盘可用，加密可用
M1  Day 15    需求/任务增强完成 — TAPD级功能可用
M2  Day 19    三级联动完成 — 项目→需求→任务全链路打通
M3  Day 23    会话双模式+Skill管理完成 — AI协作全流程可用
M4  Day 29-34 全部完成 — 工作台+监控+告警+收尾
```

---

## 二、Phase 0：基础设施修复

**周期：** Day 1 — Day 7.5（7.5天）
**目标：** 解决所有CRITICAL/HIGH技术债务，建立可扩展的基础设施

### Day 1-2：DB迁移系统重构

| 时间       | 任务                                     | 文件                                  | 产出                        |
| ---------- | ---------------------------------------- | ------------------------------------- | --------------------------- |
| Day 1 上午 | 创建 migration.rs 框架                   | `db/migration.rs` **新增**            | Migration结构体 + 执行引擎  |
| Day 1 下午 | 创建 schema_version 表 + 001_init.sql    | `db/migrations/001_init.sql` **新增** | 从init_tables提取初始schema |
| Day 2 上午 | 拆分现有迁移为独立SQL文件                | `db/migrations/002-006.sql` **新增**  | 6个迁移文件                 |
| Day 2 下午 | 重构 mod.rs：init_tables → migration驱动 | `db/mod.rs` **修改**                  | 启动时自动执行迁移          |

**验收：** 旧数据库自动迁移到最新schema，新数据库从001开始

### Day 3-4：数据层修复

| 时间       | 任务                                         | 文件                                       | 产出                         |
| ---------- | -------------------------------------------- | ------------------------------------------ | ---------------------------- |
| Day 3 上午 | 时间戳统一为ISO 8601                         | 各 create 函数                             | `now_iso8601()` 工具函数     |
| Day 3 下午 | 迁移脚本转换旧时间戳                         | `migrations/007_timestamp_fix.sql`         | 批量UPDATE                   |
| Day 4 上午 | 删除保护策略（项目/需求/任务）               | `db/mod.rs` + 各 service                   | `check_cannot_delete_*` 函数 |
| Day 4 下午 | 补充索引（tasks.project_id等）+ busy_timeout | `migrations/008_indexes.sql` + `db/mod.rs` | 索引+PRAGMA                  |

**验收：** 删除有关联数据的项目时提示错误；时间戳格式一致

### Day 4-5：PTY + 会话基础

| 时间       | 任务                             | 文件                                         | 产出            |
| ---------- | -------------------------------- | -------------------------------------------- | --------------- |
| Day 5 上午 | AgentSession结构体同步           | `db/models.rs` **修改**                      | 补全缺失字段    |
| Day 5 下午 | PTY事件通道修复 + output_log字段 | `main.rs` + `migrations/009_session_log.sql` | 事件转发+DB字段 |

**验收：** PTY输出可推送到前端；会话有output_log字段

### Day 5-6：AppError + 校验

| 时间       | 任务                  | 文件                     | 产出                                 |
| ---------- | --------------------- | ------------------------ | ------------------------------------ |
| Day 5 下午 | AppError枚举定义      | `errors.rs` **新增**     | 6种错误类型                          |
| Day 6 上午 | Input校验框架         | `validation.rs` **新增** | validate_requirement / validate_task |
| Day 6 下午 | 死代码清理 + 备份策略 | `main.rs` + `db/mod.rs`  | 删除llm_client + 自动备份            |

**验收：** 错误有类型前缀；无效输入返回明确错误

### Day 6-7：API Key 加密

| 时间       | 任务                           | 文件                                 | 产出              |
| ---------- | ------------------------------ | ------------------------------------ | ----------------- |
| Day 6 下午 | Cargo.toml 新增 aes-gcm/pbkdf2 | `Cargo.toml`                         | 依赖安装          |
| Day 7 上午 | encrypt/decrypt 命令实现       | `crypto.rs` **新增** + `commands.rs` | AES-256-GCM加解密 |
| Day 7 下午 | Settings页面对接加密           | `Settings.tsx` **修改**              | 输入/读取使用加密 |

**验收：** API Key在DB中以密文存储；解密后正确显示

### Day 7-8：系统托盘

| 时间       | 任务                            | 文件                                 | 产出                  |
| ---------- | ------------------------------- | ------------------------------------ | --------------------- |
| Day 7 下午 | capabilities 权限配置           | `capabilities/default.json` **新增** | tray 权限声明         |
| Day 8 上午 | tray.rs 实现                    | `tray.rs` **新增**                   | 菜单创建+事件处理     |
| Day 8 下午 | main.rs 集成托盘 + 窗口关闭拦截 | `main.rs` **修改**                   | 启动托盘+最小化到托盘 |

**验收：** 右键托盘菜单可用；关闭窗口隐藏到托盘

### Day 8（收尾）：前端依赖 + 编译验证

| 时间       | 任务                     | 文件           | 产出                   |
| ---------- | ------------------------ | -------------- | ---------------------- |
| Day 8 下午 | 安装 TipTap + CodeMirror | `package.json` | npm install            |
| Day 8 下午 | 全量编译验证 + 回归测试  | —              | `npx tauri build` 成功 |

**Phase 0 交付物清单：**

```
新增文件：
  src-tauri/src/db/migration.rs
  src-tauri/src/db/migrations/001-009.sql（9个迁移文件）
  src-tauri/src/errors.rs
  src-tauri/src/validation.rs
  src-tauri/src/crypto.rs
  src-tauri/src/tray.rs
  src-tauri/capabilities/default.json

修改文件：
  src-tauri/src/db/mod.rs
  src-tauri/src/db/models.rs
  src-tauri/src/main.rs
  src-tauri/src/commands.rs
  src-tauri/Cargo.toml
  src/pages/Settings.tsx
  package.json
```

---

## 三、Phase 1：需求/任务增强

**周期：** Day 9 — Day 16（7-8天）
**目标：** TAPD级字段 + 详情页 + 富文本 + 评论

### Day 9-10：DB迁移 + 后端命令

| 时间   | 任务                                 | 文件                                                    | 产出              |
| ------ | ------------------------------------ | ------------------------------------------------------- | ----------------- |
| Day 9  | 迁移：requirements/tasks新增10+8字段 | `migrations/010_req_fields.sql` + `011_task_fields.sql` | 18个新字段        |
| Day 9  | 迁移：task_comments表                | `migrations/012_task_comments.sql`                      | 评论表+索引       |
| Day 10 | 后端：扩展 create/update 命令        | `commands.rs` + `services/`                             | 支持全字段        |
| Day 10 | 后端：分页系统 + 评论CRUD            | `pagination.rs` **新增** + `commands.rs`                | 分页参数+评论接口 |

### Day 11-12：需求管理前端

| 时间   | 任务                                      | 文件                             | 产出                         |
| ------ | ----------------------------------------- | -------------------------------- | ---------------------------- |
| Day 11 | RequirementForm 组件（TipTap + 附件路径） | `RequirementForm.tsx` **新增**   | 富文本编辑器+文件选择        |
| Day 12 | RequirementDetail 重构                    | `RequirementDetail.tsx` **修改** | 关联任务+操作历史+富文本展示 |
| Day 12 | 需求列表增强（分页+筛选）                 | `Requirements.tsx` **修改**      | 模块/负责人/来源筛选         |

### Day 13-14：任务管理前端

| 时间   | 任务            | 文件                      | 产出                    |
| ------ | --------------- | ------------------------- | ----------------------- |
| Day 13 | TaskForm 组件   | `TaskForm.tsx` **新增**   | 关联需求/项目/Skill选择 |
| Day 13 | TaskDetail 新增 | `TaskDetail.tsx` **新增** | 完整详情+评论区         |
| Day 14 | Board 看板增强  | `Board.tsx` **修改**      | 卡片信息+筛选+blocked列 |

### Day 15-16：状态流转 + Store迁移

| 时间   | 任务                           | 文件                               | 产出          |
| ------ | ------------------------------ | ---------------------------------- | ------------- |
| Day 15 | 需求状态流转校验 + history写入 | `services/` + `db/requirements.rs` | 流转规则+审计 |
| Day 16 | Store迁移：7个页面统一到Store  | `stores/*.ts` + 7个页面            | 统一数据层    |
| Day 16 | 编译验证 + 功能测试            | —                                  | 全流程验证    |

**Phase 1 交付物清单：**

```
新增文件：
  src/components/RequirementForm.tsx
  src/components/TaskForm.tsx
  src/pages/TaskDetail.tsx
  src/lib/pagination.ts
  src-tauri/src/db/migrations/010-012.sql

修改文件：
  src/pages/Requirements.tsx
  src/pages/RequirementDetail.tsx
  src/pages/Board.tsx
  src/stores/*.ts（5个Store扩展）
  src-tauri/src/commands.rs
  src-tauri/src/services/*.rs
  src/lib/tauri.ts
  src/lib/types.ts
```

---

## 四、Phase 2：三级联动

**周期：** Day 17 — Day 20（3-4天）
**目标：** 项目→需求→任务全链路打通

### Day 17-18：项目详情扩展

| 时间   | 任务                      | 文件                           | 产出              |
| ------ | ------------------------- | ------------------------------ | ----------------- |
| Day 17 | ProjectDetail 增加需求Tab | `ProjectDetail.tsx` **修改**   | 需求列表+新建     |
| Day 17 | ProjectDetail 增加任务Tab | `ProjectDetail.tsx` **修改**   | 任务列表+新建     |
| Day 18 | 全局 ProjectSelector 组件 | `ProjectSelector.tsx` **新增** | 下拉选择+最近使用 |

### Day 19-20：后端联动 + 看板筛选

| 时间   | 任务                       | 文件                             | 产出                |
| ------ | -------------------------- | -------------------------------- | ------------------- |
| Day 19 | 后端：list_by_project 命令 | `commands.rs` + `db/`            | 按项目查询需求/任务 |
| Day 20 | Board 按项目筛选           | `Board.tsx` **修改**             | 顶部项目筛选器      |
| Day 20 | 需求→任务创建联动          | `RequirementDetail.tsx` **修改** | 创建任务自动关联    |

**Phase 2 交付物清单：**

```
新增文件：
  src/components/ProjectSelector.tsx

修改文件：
  src/pages/ProjectDetail.tsx
  src/pages/Board.tsx
  src/pages/RequirementDetail.tsx
  src-tauri/src/commands.rs
  src/lib/tauri.ts
```

---

## 五、Phase 3：会话双模式

**周期：** Day 17 — Day 21（与Phase 2并行，3-4天）
**目标：** 自动/手动模式 + 输出持久化

### Day 17-18：设置 + 自动模式

| 时间   | 任务                            | 文件                                 | 产出                          |
| ------ | ------------------------------- | ------------------------------------ | ----------------------------- |
| Day 17 | Settings 增加 session_mode 开关 | `Settings.tsx` **修改**              | 自动/手动切换                 |
| Day 18 | AgentService：自动模式逻辑      | `services/agent_service.rs` **修改** | 任务→in_progress 自动创建会话 |

### Day 19-20：手动模式 + 输出持久化

| 时间   | 任务                            | 文件                                         | 产出          |
| ------ | ------------------------------- | -------------------------------------------- | ------------- |
| Day 19 | TaskDetail："开启会话"按钮      | `TaskDetail.tsx` **修改**                    | 手动创建会话  |
| Day 20 | 会话输出持久化（BLOB写入+读取） | `services/agent_service.rs` + `db/agents.rs` | 日志存储+回放 |

### Day 21：内嵌终端 + 资源面板

| 时间   | 任务                          | 文件                         | 产出             |
| ------ | ----------------------------- | ---------------------------- | ---------------- |
| Day 21 | TaskDetail 内嵌 AgentTerminal | `TaskDetail.tsx` **修改**    | 任务内直接看终端 |
| Day 21 | 会话资源面板（内存/CPU）      | `SessionDetail.tsx` **修改** | 资源消耗展示     |

**Phase 3 交付物清单：**

```
修改文件：
  src/pages/Settings.tsx
  src/pages/TaskDetail.tsx
  src/pages/SessionDetail.tsx
  src-tauri/src/services/agent_service.rs
  src-tauri/src/db/agents.rs
```

---

## 六、Phase 4：Skill管理

**周期：** Day 17 — Day 22（与Phase 2/3并行，4-5天）
**目标：** 独立管理页 + 替换 + 日志 + 迭代建议

### Day 17-18：DB迁移 + 管理页框架

| 时间   | 任务                     | 文件                               | 产出                         |
| ------ | ------------------------ | ---------------------------------- | ---------------------------- |
| Day 17 | 迁移：Skill版本字段      | `migrations/013_skill_version.sql` | version/parent_id/is_deleted |
| Day 18 | SkillManagement 页面框架 | `SkillManagement.tsx` **新增**     | 列表+新建+编辑               |

### Day 19-20：CRUD + CodeMirror编辑器

| 时间   | 任务                  | 文件                                            | 产出                 |
| ------ | --------------------- | ----------------------------------------------- | -------------------- |
| Day 19 | Skill CRUD 后端命令   | `commands.rs` + `services/prd_skill_service.rs` | create/update/delete |
| Day 20 | CodeMirror 模板编辑器 | `SkillEditor.tsx` **新增**                      | Markdown语法高亮     |

### Day 21：替换 + 日志 + 统计

| 时间        | 任务                            | 文件                     | 产出              |
| ----------- | ------------------------------- | ------------------------ | ----------------- |
| Day 21 上午 | Skill替换功能（创建自定义版本） | `SkillManagement.tsx`    | 基于内置创建副本  |
| Day 21 下午 | SkillLogs 页面 + 使用统计       | `SkillLogs.tsx` **新增** | 日志列表+统计图表 |

### Day 22：迭代建议 + 导入导出

| 时间        | 任务                        | 文件                                     | 产出              |
| ----------- | --------------------------- | ---------------------------------------- | ----------------- |
| Day 22 上午 | 迭代建议引擎                | `services/prd_skill_service.rs`          | 4条规则+建议展示  |
| Day 22 下午 | 导入/导出 + 导航菜单 + 路由 | `commands.rs` + `Layout.tsx` + `App.tsx` | JSON导入导出+菜单 |

**Phase 4 交付物清单：**

```
新增文件：
  src/pages/SkillManagement.tsx
  src/pages/SkillLogs.tsx
  src/components/SkillEditor.tsx
  src-tauri/src/db/migrations/013_skill_version.sql

修改文件：
  src-tauri/src/commands.rs
  src-tauri/src/services/prd_skill_service.rs
  src-tauri/src/db/skills.rs
  src/components/Layout.tsx
  src/App.tsx
  src/lib/tauri.ts
  src/lib/types.ts
```

---

## 七、Phase 5：工作台 + 系统监控

**周期：** Day 23 — Day 29-34（4-5天）
**目标：** Dashboard→Workspace + 资源监控 + 告警 + 收尾

### Day 23-24：Workspace 页面

| 时间   | 任务                         | 文件                                      | 产出       |
| ------ | ---------------------------- | ----------------------------------------- | ---------- |
| Day 23 | Workspace 页面框架           | `Workspace.tsx` **新增**（替代Dashboard） | 4模块布局  |
| Day 24 | 我的待办 + 快速操作 + 进行中 | `Workspace.tsx`                           | P0模块完成 |

### Day 25：系统监控

| 时间        | 任务                            | 文件                                            | 产出              |
| ----------- | ------------------------------- | ----------------------------------------------- | ----------------- |
| Day 25 上午 | system_monitor.rs + sysinfo集成 | `system_monitor.rs` **新增**                    | 内存/CPU/会话采集 |
| Day 25 下午 | 状态栏重构 + 全局轮询单例       | `Layout.tsx` + `systemMonitorStore.ts` **新增** | 实时状态栏        |

### Day 26：告警 + Toast

| 时间        | 任务          | 文件                               | 产出               |
| ----------- | ------------- | ---------------------------------- | ------------------ |
| Day 26 上午 | 资源告警逻辑  | `system_monitor.rs` + `Layout.tsx` | 内存/会话/任务告警 |
| Day 26 下午 | Toast通知系统 | `Toast.tsx` **新增** + `App.tsx`   | 全局Toast          |

### Day 27-28：收尾修复

| 时间   | 任务                            | 文件                           | 产出         |
| ------ | ------------------------------- | ------------------------------ | ------------ |
| Day 27 | useEffect cleanup 全量修复      | 所有含轮询的页面               | 内存泄漏修复 |
| Day 27 | Commits/Sessions 样式统一       | `Commits.tsx` + `Sessions.tsx` | 统一组件类   |
| Day 28 | 路由更新（Dashboard→Workspace） | `App.tsx` + `Layout.tsx`       | 首页切换     |
| Day 28 | JSON.parse保护                  | `SessionDetail.tsx`            | try-catch    |

### Day 29-30（缓冲）：全面测试 + 构建

| 时间   | 任务               | 产出       |
| ------ | ------------------ | ---------- |
| Day 29 | 全功能回归测试     | 测试报告   |
| Day 30 | Release构建 + 打包 | 最终安装包 |

**Phase 5 交付物清单：**

```
新增文件：
  src/pages/Workspace.tsx（替代Dashboard.tsx）
  src-tauri/src/system_monitor.rs
  src/stores/systemMonitorStore.ts
  src/components/Toast.tsx

修改文件：
  src/components/Layout.tsx
  src/App.tsx
  src/pages/Commits.tsx
  src/pages/Sessions.tsx
  src/pages/SessionDetail.tsx
  src/lib/tauri.ts
  所有含useEffect轮询的页面
```

---

## 八、并行策略

```
         Day 1-8    Day 9-16   Day 17-20  Day 21-22  Day 23-30
         ─────────  ─────────  ─────────  ─────────  ─────────
Phase 0  ████████
Phase 1             ████████
Phase 2                        ████
Phase 3                        ████
Phase 4                        ████████
Phase 5                                   ████████

并行点：
  P2 + P3 可同时进行（无代码冲突，分属不同模块）
  P4 可与 P2/P3 同时进行（Skill模块独立）
  P5 等 P2-P4 完成后进行（依赖前面的数据）
```

**并行分工建议：**

| 并行窗口  | 工作流A         | 工作流B          |
| --------- | --------------- | ---------------- |
| Day 17-20 | Phase 2（联动） | Phase 3（会话）  |
| Day 17-22 | —               | Phase 4（Skill） |

---

## 九、文件变更总览

### 新增文件（约15个）

| 文件                                      | 阶段     | 说明             |
| ----------------------------------------- | -------- | ---------------- |
| `src-tauri/src/db/migration.rs`           | P0       | 版本化迁移引擎   |
| `src-tauri/src/db/migrations/001-013.sql` | P0/P1/P4 | 13个迁移文件     |
| `src-tauri/src/errors.rs`                 | P0       | AppError枚举     |
| `src-tauri/src/validation.rs`             | P0       | 输入校验         |
| `src-tauri/src/crypto.rs`                 | P0       | AES加解密        |
| `src-tauri/src/tray.rs`                   | P0       | 系统托盘         |
| `src-tauri/src/system_monitor.rs`         | P5       | 系统监控         |
| `src-tauri/capabilities/default.json`     | P0       | Tauri权限        |
| `src/lib/pagination.ts`                   | P1       | 分页组件         |
| `src/components/RequirementForm.tsx`      | P1       | 需求表单         |
| `src/components/TaskForm.tsx`             | P1       | 任务表单         |
| `src/pages/TaskDetail.tsx`                | P1       | 任务详情         |
| `src/pages/Workspace.tsx`                 | P5       | 工作台           |
| `src/pages/SkillManagement.tsx`           | P4       | Skill管理        |
| `src/pages/SkillLogs.tsx`                 | P4       | Skill日志        |
| `src/components/SkillEditor.tsx`          | P4       | CodeMirror编辑器 |
| `src/components/ProjectSelector.tsx`      | P2       | 项目选择器       |
| `src/components/Toast.tsx`                | P5       | Toast通知        |
| `src/stores/systemMonitorStore.ts`        | P5       | 系统监控Store    |

### 修改文件（约25个）

| 文件                                          | 阶段     | 变更类型      |
| --------------------------------------------- | -------- | ------------- |
| `src-tauri/src/db/mod.rs`                     | P0       | 迁移系统重构  |
| `src-tauri/src/db/models.rs`                  | P0       | 结构体同步    |
| `src-tauri/src/db/requirements.rs`            | P1       | 全字段CRUD    |
| `src-tauri/src/db/tasks.rs`                   | P1       | 全字段CRUD    |
| `src-tauri/src/db/agents.rs`                  | P3       | output_log    |
| `src-tauri/src/db/skills.rs`                  | P4       | 版本管理      |
| `src-tauri/src/commands.rs`                   | P0-P5    | 持续扩展      |
| `src-tauri/src/main.rs`                       | P0       | 托盘+备份+PTY |
| `src-tauri/src/services/agent_service.rs`     | P3       | 自动/手动模式 |
| `src-tauri/src/services/prd_skill_service.rs` | P4       | CRUD+建议     |
| `src-tauri/Cargo.toml`                        | P0       | 新增依赖      |
| `src/pages/Requirements.tsx`                  | P1       | 列表增强      |
| `src/pages/RequirementDetail.tsx`             | P1/P2    | 富文本+联动   |
| `src/pages/Board.tsx`                         | P1/P2    | 看板+筛选     |
| `src/pages/ProjectDetail.tsx`                 | P2       | 需求/任务Tab  |
| `src/pages/Settings.tsx`                      | P0/P3    | 加密+会话模式 |
| `src/pages/Sessions.tsx`                      | P5       | 样式统一      |
| `src/pages/SessionDetail.tsx`                 | P3/P5    | 资源面板+修复 |
| `src/pages/Commits.tsx`                       | P5       | 样式统一      |
| `src/components/Layout.tsx`                   | P0/P2/P5 | 托盘事件+导航 |
| `src/App.tsx`                                 | P2/P4/P5 | 路由+Toast    |
| `src/stores/*.ts`                             | P1       | 5个Store扩展  |
| `src/lib/tauri.ts`                            | P0-P5    | 持续扩展      |
| `src/lib/types.ts`                            | P1/P4    | 类型扩展      |
| `package.json`                                | P0       | 新增前端依赖  |

---

## 十、风险与缓冲

### 时间缓冲分配

| 阶段     | 乐观     | 基准     | 悄观     | 缓冲     |
| -------- | -------- | -------- | -------- | -------- |
| P0       | 6天      | 7.5天    | 9天      | +1.5天   |
| P1       | 6天      | 7.5天    | 9天      | +1.5天   |
| P2       | 3天      | 3.5天    | 5天      | +1.5天   |
| P3       | 3天      | 3.5天    | 5天      | +1.5天   |
| P4       | 4天      | 4.5天    | 6天      | +1.5天   |
| P5       | 4天      | 4.5天    | 6天      | +1.5天   |
| **总计** | **26天** | **31天** | **40天** | **+5天** |

### 关键风险

| 风险                            | 概率 | 影响 | 缓解                      |
| ------------------------------- | ---- | ---- | ------------------------- |
| DB迁移破坏旧数据                | 中   | 高   | 迁移前自动备份+逐版本执行 |
| PTY兼容性（Windows/Linux）      | 中   | 中   | 先Windows验证             |
| TipTap/CodeMirror集成问题       | 低   | 中   | 已有成熟npm包             |
| 状态栏轮询性能                  | 低   | 低   | 页面不可见时暂停          |
| 托盘图标在某些Windows版本不显示 | 低   | 低   | 降级为普通窗口模式        |

### 每日站会检查点

```
每日结束前检查：
□ 今日任务是否完成？
□ 编译是否通过？
□ 有无阻塞问题需要调整计划？
□ 明天的任务是否就绪（依赖是否满足）？
```

---

## 十一、交付标准

### 每个Phase完成时

```
□ 编译通过（npx tauri build 无error）
□ 核心功能可演示
□ 无P0/P1级Bug
□ 代码无新增warning
□ 设计文档已同步更新
```

### 最终交付

```
□ 全部9项需求功能可用
□ 全部31项技术修复已落地
□ Release构建成功
□ 安装包可正常安装运行
□ 系统托盘功能正常
□ API Key加密存储
□ 数据库自动迁移
□ 自动备份正常
□ 无已知P0/P1级Bug
```
