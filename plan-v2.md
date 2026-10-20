# ACE - AI Collaboration Engine 开发计划 v2

> 基于 feature-spec.md v1.1 审计报告 | 总工期：38-45天
> 审计发现：20个现有功能 + 15个缺口 + 10个建议 + 19个自洽性问题

---

## 一、总览

```
现有功能：20个（17已实现 / 3部分实现）
功能缺口：15个（4 Must / 4 Should / 7 Could）
建议新增：10个（4 Must / 3 Should / 3 Could）
自洽性问题：19个（2 High / 9 Medium / 8 Low）
技术债务：10个死代码命令 + 3个死代码模块 + 版本不一致 + 零测试 + 无CI/CD
```

### 优先级说明

```
Must   = 阻塞核心功能，必须实现
Should = 提升用户体验，强烈建议
Could  = 锦上添花，资源允许时实现
```

### 里程碑

```
M0  Day 5    技术债务清零 — 版本统一、死代码清理、.gitignore、基础测试
M1  Day 12   核心增强完成 — 需求/任务字段扩展、Skill CRUD
M2  Day 18   编辑器集成 — TipTap富文本、分页系统
M3  Day 25   系统完善 — 操作审计、附件管理、暗色主题
M4  Day 32   基础设施 — CI/CD、文档、导入导出、通知
M5  Day 38-45 收尾 — 全面测试、性能优化、发布
```

---

## 二、Phase 0：技术债务清零

**周期：** Day 1 — Day 5（5天）
**目标：** 解决所有High/Medium严重度的自洽性问题，建立质量基线

### Day 1：版本统一 + 基础设施

| 时间 | 任务              | 文件                  | 产出                | 对应问题 |
| ---- | ----------------- | --------------------- | ------------------- | -------- |
| 上午 | 统一版本号为2.1.0 | `package.json:4`      | version: "2.1.0"    | C011     |
| 上午 | 创建.gitignore    | `.gitignore` **新增** | 标准Tauri gitignore | C014     |
| 下午 | 创建CLAUDE.md     | `CLAUDE.md` **新增**  | AI助手项目上下文    | C019     |
| 下午 | 创建LICENSE       | `LICENSE` **新增**    | MIT许可证           | C019     |

### Day 2：死代码清理

| 时间 | 任务                        | 文件                            | 产出                  | 对应问题   |
| ---- | --------------------------- | ------------------------------- | --------------------- | ---------- |
| 上午 | 移除10个未调用的后端命令    | `commands.rs`                   | 删除get_token_usage等 | C015       |
| 上午 | 清理migration.rs死代码      | `migration.rs`                  | 删除3个未调用函数     | C018       |
| 下午 | 集成validation.rs到命令处理 | `commands.rs` + `validation.rs` | 6个验证函数被调用     | C016       |
| 下午 | 集成crypto.rs到Settings     | `Settings.tsx` + `crypto.rs`    | API Key加密生效       | C002, C017 |

### Day 3-4：基础测试框架

| 时间       | 任务                     | 文件                                        | 产出            | 对应问题 |
| ---------- | ------------------------ | ------------------------------------------- | --------------- | -------- |
| Day 3 上午 | 配置Vitest               | `vitest.config.ts` **新增**                 | 测试框架就绪    | C012     |
| Day 3 下午 | 后端单元测试：validation | `src-tauri/src/validation_test.rs` **新增** | 6个验证函数测试 | C012     |
| Day 4 上午 | 后端单元测试：crypto     | `src-tauri/src/crypto_test.rs` **新增**     | 加解密测试      | C012     |
| Day 4 下午 | 前端单元测试：stores     | `src/stores/__tests__/*.test.ts` **新增**   | Store测试       | C012     |

### Day 5：CI/CD基础

| 时间 | 任务               | 文件                                | 产出                   | 对应问题 |
| ---- | ------------------ | ----------------------------------- | ---------------------- | -------- |
| 上午 | GitHub Actions配置 | `.github/workflows/ci.yml` **新增** | 自动构建+测试          | C013     |
| 下午 | 全量编译验证       | —                                   | `npx tauri build` 成功 | —        |

**Phase 0 交付物：**

```
新增文件：
  .gitignore
  CLAUDE.md
  LICENSE
  vitest.config.ts
  src-tauri/src/validation_test.rs
  src-tauri/src/crypto_test.rs
  src/stores/__tests__/*.test.ts
  .github/workflows/ci.yml

修改文件：
  package.json (版本号)
  src-tauri/src/commands.rs (删除死代码)
  src-tauri/src/db/migration.rs (清理死代码)
  src-tauri/src/validation.rs (集成到命令)
  src-tauri/src/crypto.rs (集成到Settings)
  src/pages/Settings.tsx (加密集成)
```

---

## 三、Phase 1：需求/任务字段扩展

**周期：** Day 6 — Day 12（7天）
**目标：** 实现Must级字段扩展，提升需求/任务管理能力

### Day 6-7：数据库迁移

| 时间  | 任务                           | 文件                                      | 产出                            | 对应缺口   |
| ----- | ------------------------------ | ----------------------------------------- | ------------------------------- | ---------- |
| Day 6 | 迁移：requirements新增12个字段 | `migrations/011_req_fields.sql` **新增**  | module/owner/developer/source等 | G001, S001 |
| Day 7 | 迁移：tasks新增11个字段        | `migrations/012_task_fields.sql` **新增** | assignee/due_date/tags等        | G002, S002 |

### Day 8-9：后端命令扩展

| 时间  | 任务                            | 文件                        | 产出           |
| ----- | ------------------------------- | --------------------------- | -------------- |
| Day 8 | 更新create/update命令支持新字段 | `commands.rs` + `services/` | 全字段CRUD     |
| Day 9 | 更新类型定义                    | `types.ts` + `models.rs`    | 前后端类型同步 |

### Day 10-11：前端表单

| 时间   | 任务                | 文件                           | 产出           |
| ------ | ------------------- | ------------------------------ | -------------- |
| Day 10 | RequirementForm组件 | `RequirementForm.tsx` **新增** | 12个新字段表单 |
| Day 11 | TaskForm组件        | `TaskForm.tsx` **新增**        | 11个新字段表单 |

### Day 12：详情页增强 + 验证

| 时间 | 任务                        | 文件                             | 产出         |
| ---- | --------------------------- | -------------------------------- | ------------ |
| 上午 | RequirementDetail展示新字段 | `RequirementDetail.tsx` **修改** | 完整字段展示 |
| 下午 | TaskDetail展示新字段        | `TaskDetail.tsx` **修改**        | 完整字段展示 |

**Phase 1 交付物：**

```
新增文件：
  src-tauri/src/db/migrations/011_req_fields.sql
  src-tauri/src/db/migrations/012_task_fields.sql
  src/components/RequirementForm.tsx
  src/components/TaskForm.tsx

修改文件：
  src-tauri/src/commands.rs
  src-tauri/src/services/*.rs
  src-tauri/src/db/models.rs
  src/pages/RequirementDetail.tsx
  src/pages/TaskDetail.tsx
  src/lib/tauri.ts
  src/lib/types.ts
```

---

## 四、Phase 2：Skill CRUD + 评论增强

**周期：** Day 6 — Day 14（与Phase 1并行，9天）
**目标：** 实现Must级Skill管理，完善任务评论

### Day 6-7：Skill CRUD后端

| 时间  | 任务           | 文件                                     | 产出                 | 对应缺口   |
| ----- | -------------- | ---------------------------------------- | -------------------- | ---------- |
| Day 6 | Skill CRUD命令 | `commands.rs`                            | create/update/delete | G006, S006 |
| Day 7 | Skill服务层    | `services/prd_skill_service.rs` **修改** | CRUD逻辑             | G006       |

### Day 8-9：Skill管理前端

| 时间  | 任务                          | 文件                           | 产出           |
| ----- | ----------------------------- | ------------------------------ | -------------- |
| Day 8 | SkillManagement重构           | `SkillManagement.tsx` **修改** | 完整CRUD界面   |
| Day 9 | SkillEditor组件（CodeMirror） | `SkillEditor.tsx` **新增**     | Markdown编辑器 |

### Day 10-11：评论增强

| 时间   | 任务                     | 文件                             | 产出       | 对应问题 |
| ------ | ------------------------ | -------------------------------- | ---------- | -------- |
| Day 10 | 评论长度限制（5000字符） | `TaskDetail.tsx` + `commands.rs` | 前后端校验 | Q001     |
| Day 11 | 评论编辑功能             | `TaskDetail.tsx` **修改**        | 编辑/回复  | —        |

### Day 12-14：Skill日志 + 统计

| 时间   | 任务          | 文件                     | 产出         |
| ------ | ------------- | ------------------------ | ------------ |
| Day 12 | SkillLogs页面 | `SkillLogs.tsx` **新增** | 执行日志列表 |
| Day 13 | Skill使用统计 | `commands.rs`            | 统计命令     |
| Day 14 | 集成测试      | —                        | 全流程验证   |

**Phase 2 交付物：**

```
新增文件：
  src/pages/SkillLogs.tsx
  src/components/SkillEditor.tsx

修改文件：
  src-tauri/src/commands.rs
  src-tauri/src/services/prd_skill_service.rs
  src/pages/SkillManagement.tsx
  src/pages/TaskDetail.tsx
  src/lib/tauri.ts
  src/lib/types.ts
```

---

## 五、Phase 3：富文本编辑器

**周期：** Day 13 — Day 18（6天）
**目标：** 集成TipTap，实现Must级富文本编辑

### Day 13-14：依赖安装 + 基础集成

| 时间   | 任务           | 文件                          | 产出        | 对应缺口   |
| ------ | -------------- | ----------------------------- | ----------- | ---------- |
| Day 13 | 安装TipTap依赖 | `package.json`                | npm install | G003, S003 |
| Day 14 | TipTap基础组件 | `RichTextEditor.tsx` **新增** | 编辑器组件  | G003       |

### Day 15-16：功能集成

| 时间   | 任务                        | 文件                             | 产出       |
| ------ | --------------------------- | -------------------------------- | ---------- |
| Day 15 | RequirementForm集成TipTap   | `RequirementForm.tsx` **修改**   | 富文本编辑 |
| Day 16 | RequirementDetail展示富文本 | `RequirementDetail.tsx` **修改** | HTML渲染   |

### Day 17-18：样式 + 测试

| 时间   | 任务            | 文件                 | 产出                 |
| ------ | --------------- | -------------------- | -------------------- |
| Day 17 | 编辑器样式定制  | `RichTextEditor.tsx` | 工具栏+样式          |
| Day 18 | 降级方案 + 测试 | `RichTextEditor.tsx` | 加载失败降级为纯文本 |

**Phase 3 交付物：**

```
新增文件：
  src/components/RichTextEditor.tsx

修改文件：
  package.json
  src/components/RequirementForm.tsx
  src/pages/RequirementDetail.tsx
```

---

## 六、Phase 4：分页系统 + 操作审计

**周期：** Day 19 — Day 25（7天）
**目标：** 实现Should级分页和操作历史

### Day 19-20：分页后端

| 时间   | 任务                       | 文件                     | 产出             | 对应缺口   |
| ------ | -------------------------- | ------------------------ | ---------------- | ---------- |
| Day 19 | PagedResult类型 + 分页参数 | `pagination.rs` **新增** | 分页框架         | G007, S007 |
| Day 20 | 各命令支持分页             | `commands.rs`            | limit/offset参数 | G007       |

### Day 21-22：分页前端

| 时间   | 任务             | 文件                              | 产出     |
| ------ | ---------------- | --------------------------------- | -------- |
| Day 21 | Pagination组件   | `Pagination.tsx` **新增**         | 页码导航 |
| Day 22 | 各列表页集成分页 | `Requirements.tsx` `Board.tsx` 等 | 分页显示 |

### Day 23-24：操作审计

| 时间   | 任务              | 文件                                                    | 产出         | 对应缺口   |
| ------ | ----------------- | ------------------------------------------------------- | ------------ | ---------- |
| Day 23 | 审计日志表 + 命令 | `migrations/013_audit_log.sql` **新增** + `commands.rs` | 审计记录     | G005, S005 |
| Day 24 | 审计日志页面      | `AuditLog.tsx` **新增**                                 | 变更历史展示 | G005       |

### Day 25：集成测试

| 时间 | 任务                | 产出     |
| ---- | ------------------- | -------- |
| 全天 | 分页+审计全流程测试 | 测试报告 |

**Phase 4 交付物：**

```
新增文件：
  src/lib/pagination.ts
  src/components/Pagination.tsx
  src-tauri/src/db/migrations/013_audit_log.sql
  src/pages/AuditLog.tsx

修改文件：
  src-tauri/src/commands.rs
  src/pages/Requirements.tsx
  src/pages/Board.tsx
  src/pages/Sessions.tsx
```

---

## 七、Phase 5：附件管理 + 暗色主题

**周期：** Day 26 — Day 32（7天）
**目标：** 实现Should级附件管理和Could级暗色主题

### Day 26-27：附件管理

| 时间   | 任务          | 文件                                                      | 产出          | 对应缺口   |
| ------ | ------------- | --------------------------------------------------------- | ------------- | ---------- |
| Day 26 | 附件表 + 命令 | `migrations/014_attachments.sql` **新增** + `commands.rs` | 附件CRUD      | G004, S004 |
| Day 27 | 附件UI        | `AttachmentList.tsx` **新增**                             | 附件展示+上传 | G004       |

### Day 28-30：暗色主题

| 时间   | 任务             | 文件                          | 产出              | 对应缺口         |
| ------ | ---------------- | ----------------------------- | ----------------- | ---------------- |
| Day 28 | Tailwind暗色配置 | `tailwind.config.js` **修改** | darkMode: 'class' | G010, S010, Q004 |
| Day 29 | 主题切换Store    | `themeStore.ts` **新增**      | 主题状态管理      | G010             |
| Day 30 | 各组件适配暗色   | 多个组件文件                  | 暗色样式          | G010             |

### Day 31-32：通知系统

| 时间   | 任务         | 文件                 | 产出       | 对应缺口         |
| ------ | ------------ | -------------------- | ---------- | ---------------- |
| Day 31 | 托盘通知命令 | `tray.rs` **修改**   | 系统通知   | G009, S009, Q006 |
| Day 32 | Toast组件    | `Toast.tsx` **新增** | 应用内通知 | G009             |

**Phase 5 交付物：**

```
新增文件：
  src-tauri/src/db/migrations/014_attachments.sql
  src/components/AttachmentList.tsx
  src/stores/themeStore.ts
  src/components/Toast.tsx

修改文件：
  tailwind.config.js
  src-tauri/src/tray.rs
  多个组件文件（暗色适配）
```

---

## 八、Phase 6：数据导入导出 + 系统监控

**周期：** Day 33 — Day 38（6天）
**目标：** 实现Could级导入导出和系统监控

### Day 33-34：数据导入导出

| 时间   | 任务                 | 文件                                        | 产出              | 对应缺口         |
| ------ | -------------------- | ------------------------------------------- | ----------------- | ---------------- |
| Day 33 | 导出命令（JSON格式） | `commands.rs`                               | 全量/增量导出     | G008, S008, Q005 |
| Day 34 | 导入命令 + UI        | `commands.rs` + `ImportExport.tsx` **新增** | 导入验证+冲突处理 | G008             |

### Day 35-36：系统监控

| 时间   | 任务                         | 文件                         | 产出              | 对应缺口   |
| ------ | ---------------------------- | ---------------------------- | ----------------- | ---------- |
| Day 35 | system_monitor.rs（sysinfo） | `system_monitor.rs` **新增** | CPU/内存/会话采集 | G012, Q008 |
| Day 36 | 状态栏实时监控               | `Layout.tsx` **修改**        | 实时数据展示      | G012       |

### Day 37-38：会话双模式完善

| 时间   | 任务         | 文件                                 | 产出              | 对应问题 |
| ------ | ------------ | ------------------------------------ | ----------------- | -------- |
| Day 37 | 自动模式逻辑 | `services/agent_service.rs` **修改** | 任务→自动创建会话 | C008     |
| Day 38 | 内嵌终端     | `TaskDetail.tsx` **修改**            | 任务内直接看终端  | —        |

**Phase 6 交付物：**

```
新增文件：
  src/pages/ImportExport.tsx
  src-tauri/src/system_monitor.rs

修改文件：
  src-tauri/src/commands.rs
  src-tauri/src/services/agent_service.rs
  src/components/Layout.tsx
  src/pages/TaskDetail.tsx
```

---

## 九、Phase 7：收尾 + 发布

**周期：** Day 39 — Day 45（7天）
**目标：** 全面测试、性能优化、文档完善、发布

### Day 39-40：全面测试

| 时间   | 任务         | 产出     |
| ------ | ------------ | -------- |
| Day 39 | 功能回归测试 | 测试报告 |
| Day 40 | 边界条件测试 | 测试报告 |

### Day 41-42：性能优化

| 时间   | 任务                   | 文件       | 产出         |
| ------ | ---------------------- | ---------- | ------------ |
| Day 41 | 虚拟滚动（大数据列表） | 各列表组件 | 性能提升     |
| Day 42 | 懒加载 + 代码分割      | `App.tsx`  | 首屏加载优化 |

### Day 43：文档完善

| 时间 | 任务                          | 文件    | 产出     |
| ---- | ----------------------------- | ------- | -------- |
| 全天 | API文档 + 用户指南 + 变更日志 | `docs/` | 完整文档 |

### Day 44-45：发布准备

| 时间   | 任务               | 产出       |
| ------ | ------------------ | ---------- |
| Day 44 | Release构建 + 打包 | 安装包     |
| Day 45 | 最终验证 + 发布    | v2.1.0发布 |

**Phase 7 交付物：**

```
新增文件：
  docs/api.md
  docs/user-guide.md
  CHANGELOG.md

修改文件：
  多个列表组件（虚拟滚动）
  src/App.tsx（代码分割）
```

---

## 十、并行策略

```
         Day 1-5   Day 6-12  Day 13-18  Day 19-25  Day 26-32  Day 33-38  Day 39-45
         ────────  ────────  ────────   ────────   ────────   ────────   ────────
Phase 0  ████████
Phase 1             ████████
Phase 2             ████████████████
Phase 3                       ████████
Phase 4                                 ████████
Phase 5                                           ████████
Phase 6                                                     ████████
Phase 7                                                               ████████

并行点：
  P1 + P2 可同时进行（需求/任务 vs Skill，不同模块）
  P3 等 P1 完成后进行（依赖需求表单）
  P4 等 P1/P2 完成后进行（依赖列表页面）
  P5 可与 P4 并行（附件/主题 vs 分页/审计）
  P6 等 P4/P5 完成后进行（依赖基础设施）
  P7 等全部完成后进行
```

**并行分工建议：**

| 并行窗口  | 工作流A                  | 工作流B               |
| --------- | ------------------------ | --------------------- |
| Day 6-12  | Phase 1（需求/任务字段） | Phase 2（Skill CRUD） |
| Day 19-25 | Phase 4（分页+审计）     | Phase 5（附件+主题）  |

---

## 十一、文件变更总览

### 新增文件（约20个）

| 文件                                      | 阶段        | 说明             |
| ----------------------------------------- | ----------- | ---------------- |
| `.gitignore`                              | P0          | Git忽略规则      |
| `CLAUDE.md`                               | P0          | AI助手上下文     |
| `LICENSE`                                 | P0          | 许可证           |
| `vitest.config.ts`                        | P0          | 测试配置         |
| `.github/workflows/ci.yml`                | P0          | CI/CD            |
| `src-tauri/src/db/migrations/011-014.sql` | P1/P4/P5/P6 | 4个迁移文件      |
| `src/components/RequirementForm.tsx`      | P1          | 需求表单         |
| `src/components/TaskForm.tsx`             | P1          | 任务表单         |
| `src/components/SkillEditor.tsx`          | P2          | CodeMirror编辑器 |
| `src/pages/SkillLogs.tsx`                 | P2          | Skill日志        |
| `src/components/RichTextEditor.tsx`       | P3          | TipTap编辑器     |
| `src/lib/pagination.ts`                   | P4          | 分页工具         |
| `src/components/Pagination.tsx`           | P4          | 分页组件         |
| `src/pages/AuditLog.tsx`                  | P4          | 审计日志         |
| `src/components/AttachmentList.tsx`       | P5          | 附件列表         |
| `src/stores/themeStore.ts`                | P5          | 主题Store        |
| `src/components/Toast.tsx`                | P5          | Toast通知        |
| `src/pages/ImportExport.tsx`              | P6          | 导入导出         |
| `src-tauri/src/system_monitor.rs`         | P6          | 系统监控         |
| `docs/api.md`                             | P7          | API文档          |

### 修改文件（约25个）

| 文件                                          | 阶段     | 变更类型       |
| --------------------------------------------- | -------- | -------------- |
| `package.json`                                | P0/P3    | 版本+依赖      |
| `src-tauri/src/commands.rs`                   | P0-P6    | 持续扩展       |
| `src-tauri/src/db/migration.rs`               | P0       | 清理死代码     |
| `src-tauri/src/validation.rs`                 | P0       | 集成到命令     |
| `src-tauri/src/crypto.rs`                     | P0       | 集成到Settings |
| `src-tauri/src/services/prd_skill_service.rs` | P2       | CRUD           |
| `src-tauri/src/services/agent_service.rs`     | P6       | 自动模式       |
| `src-tauri/src/tray.rs`                       | P5       | 通知           |
| `src/pages/Settings.tsx`                      | P0       | 加密集成       |
| `src/pages/RequirementDetail.tsx`             | P1/P3    | 字段+富文本    |
| `src/pages/TaskDetail.tsx`                    | P1/P2/P6 | 字段+评论+终端 |
| `src/pages/SkillManagement.tsx`               | P2       | CRUD           |
| `src/pages/Requirements.tsx`                  | P4       | 分页           |
| `src/pages/Board.tsx`                         | P4       | 分页           |
| `src/pages/Sessions.tsx`                      | P4       | 分页           |
| `src/components/Layout.tsx`                   | P5/P6    | 暗色+监控      |
| `src/App.tsx`                                 | P5/P7    | 路由+分割      |
| `tailwind.config.js`                          | P5       | 暗色模式       |
| `src/lib/tauri.ts`                            | P1-P6    | 持续扩展       |
| `src/lib/types.ts`                            | P1/P2    | 类型扩展       |
| `src/stores/*.ts`                             | P1       | Store扩展      |

---

## 十二、风险与缓冲

### 时间缓冲分配

| 阶段     | 乐观     | 基准     | 悲观     | 缓冲      |
| -------- | -------- | -------- | -------- | --------- |
| P0       | 4天      | 5天      | 7天      | +2天      |
| P1       | 5天      | 7天      | 9天      | +2天      |
| P2       | 7天      | 9天      | 12天     | +3天      |
| P3       | 5天      | 6天      | 8天      | +2天      |
| P4       | 5天      | 7天      | 9天      | +2天      |
| P5       | 5天      | 7天      | 9天      | +2天      |
| P6       | 4天      | 6天      | 8天      | +2天      |
| P7       | 5天      | 7天      | 9天      | +2天      |
| **总计** | **40天** | **54天** | **71天** | **+17天** |

### 关键风险

| 风险               | 概率 | 影响 | 缓解                         |
| ------------------ | ---- | ---- | ---------------------------- |
| TipTap集成兼容性   | 中   | 高   | 先验证基础集成，准备降级方案 |
| DB迁移破坏旧数据   | 中   | 高   | 迁移前自动备份+逐版本执行    |
| 暗色主题适配工作量 | 中   | 中   | 优先适配核心页面，渐进式完善 |
| 分页影响现有功能   | 低   | 中   | 分页参数可选，默认不分页     |
| 操作审计性能开销   | 低   | 低   | 异步写入，不影响主流程       |

### 每日站会检查点

```
每日结束前检查：
□ 今日任务是否完成？
□ 编译是否通过？
□ 测试是否通过？
□ 有无阻塞问题需要调整计划？
□ 明天的任务是否就绪（依赖是否满足）？
```

---

## 十三、交付标准

### 每个Phase完成时

```
□ 编译通过（npx tauri build 无error）
□ 核心功能可演示
□ 无P0/P1级Bug
□ 新增代码有对应测试
□ 设计文档已同步更新
```

### 最终交付

```
□ 全部20个现有功能正常
□ 全部4个Must级缺口已实现
□ 全部4个Should级缺口已实现
□ 全部3个Could级缺口已实现（资源允许时）
□ 全部19个自洽性问题已解决
□ 版本号统一为2.1.0
□ 测试覆盖率 > 60%
□ CI/CD管道正常
□ 文档完整（API/用户指南/变更日志）
□ Release构建成功
□ 安装包可正常安装运行
□ 无已知P0/P1级Bug
```

---

## 十四、与旧计划的差异

| 维度     | 旧计划 (plan.md)   | 新计划 (plan-v2.md)               |
| -------- | ------------------ | --------------------------------- |
| 基础     | design.md v2.3     | feature-spec.md v1.1 审计报告     |
| 范围     | 9项需求 + 31项修复 | 20功能 + 15缺口 + 10建议 + 19问题 |
| 优先级   | 按Phase线性推进    | 按Must/Should/Could分级           |
| 测试     | 无                 | P0阶段建立测试框架                |
| CI/CD    | 无                 | P0阶段配置GitHub Actions          |
| 文档     | 无                 | P7阶段完善文档                    |
| 技术债务 | 部分涉及           | P0阶段集中清理                    |
| 并行     | P2+P3+P4并行       | P1+P2并行，P4+P5并行              |
| 工期     | 29-34天            | 38-45天（更现实）                 |

---

**变更记录：**

| 版本 | 日期       | 变更内容                             | 作者        |
| ---- | ---------- | ------------------------------------ | ----------- |
| v2.0 | 2026-09-20 | 基于feature-spec.md v1.1审计报告创建 | Claude Code |
