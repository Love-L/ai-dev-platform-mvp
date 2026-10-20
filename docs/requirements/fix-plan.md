# 全部问题修复计划

> **制定日期：** 2026-09-20
> **基于文档：** requirement-gap-report.md
> **目标：** 修复所有需求vs实现差异，实现率达到90%+
> **总预估工时：** 约15-20天

---

## 修复策略

采用**分阶段、按优先级**的修复策略：

```
Phase 1 (P0紧急) → Phase 2 (P1重要) → Phase 3 (P2优化) → Phase 4 (P3补充)
     3-4天              4-5天              3-4天              5-7天
```

---

## Phase 1：P0紧急修复（3-4天）

### 目标
修复影响核心功能的关键问题

### 任务1.1：修复Agent数据持久化（0.5天）

**问题描述：** Agent检测后未正确持久化，每次进入页面需重新刷新

**涉及文件：**
- `src-tauri/src/services/agent_service.rs`
- `src-tauri/src/db/agents.rs`
- `src/pages/Agents.tsx`

**修复内容：**

1. **检查agent_service.rs的discover_agents函数**
   - 确认是否正确调用db.upsert_agent()
   - 确认upsert逻辑是否正确（INSERT OR REPLACE）

2. **检查db/agents.rs的get_agents函数**
   - 确认SQL查询是否正确
   - 确认返回的Agent结构体是否完整

3. **检查Agents.tsx的加载逻辑**
   - 确认是否优先从数据库读取
   - 确认discover_agents是否仅在刷新时调用

**验收标准：**
- 首次检测Agent后，下次进入页面直接显示已有数据
- 无需每次刷新重新检测

---

### 任务1.2：修复系统资源监控显示（0.5天）

**问题描述：** 状态栏显示系统内存(18113/32374MB)而非应用内存

**涉及文件：**
- `src-tauri/src/system_monitor.rs`
- `src/components/Layout.tsx`

**修复内容：**

1. **修改system_monitor.rs的get_data函数**
   ```rust
   // 当前代码（系统级内存）
   let memory = sys.total_memory();
   let memory_used = sys.used_memory();

   // 修改为（应用级内存）
   let pid = sysinfo::get_current_pid().unwrap();
   if let Some(process) = sys.process(pid) {
       let memory_used = process.memory(); // RSS in bytes
       let memory_used_mb = memory_used / (1024 * 1024);
   }
   ```

2. **修改SystemMonitorData结构体**
   - 移除memory_total_mb（应用无法获取系统总内存）
   - 保留memory_used_mb（应用RSS内存）
   - 保留cpu_usage（应用CPU使用率）

3. **修改Layout.tsx的状态栏显示**
   - 显示格式：`📊 215MB`（应用内存）
   - 移除系统内存显示

**验收标准：**
- 状态栏显示应用内存（如215MB）而非系统内存
- 内存数值每2秒刷新

---

### 任务1.3：实现Skill管理后台CRUD（1.5天）

**问题描述：** SkillManagement.tsx仅有只读列表，无CRUD功能

**涉及文件：**
- `src-tauri/src/commands.rs`（新增命令）
- `src-tauri/src/services/prd_skill_service.rs`（新增方法）
- `src-tauri/src/db/skills.rs`（新增方法）
- `src/pages/SkillManagement.tsx`（重构）
- `src/lib/tauri.ts`（新增IPC调用）
- `src/lib/types.ts`（更新类型）

**修复内容：**

1. **后端：新增CRUD命令**
   ```rust
   // commands.rs新增
   #[tauri::command]
   pub async fn create_skill(
       state: State<'_, AppState>,
       name: String,
       skill_type: String,
       description: Option<String>,
       template: String,
   ) -> Result<PrdSkill, String>

   #[tauri::command]
   pub async fn update_skill(
       state: State<'_, AppState>,
       id: String,
       name: Option<String>,
       description: Option<String>,
       template: Option<String>,
   ) -> Result<(), String>

   #[tauri::command]
   pub async fn delete_skill(
       state: State<'_, AppState>,
       id: String,
   ) -> Result<(), String>
   ```

2. **后端：实现Service方法**
   - prd_skill_service.rs新增create_skill, update_skill, delete_skill
   - 内置Skill不可删除检查

3. **后端：实现DB方法**
   - db/skills.rs新增insert_skill, update_skill, delete_skill

4. **前端：重构SkillManagement.tsx**
   - 新增"新建Skill"弹窗
   - 新增"编辑Skill"弹窗（含CodeMirror编辑器）
   - 新增"删除Skill"确认弹窗
   - 内置Skill禁用编辑/删除按钮

5. **前端：集成CodeMirror编辑器**
   - 安装依赖：`@uiw/react-codemirror` + `@uiw/codemirror-theme-github`
   - 创建SkillEditor组件

6. **前端：更新tauri.ts和types.ts**
   - 新增createSkill, updateSkill, deleteSkill函数
   - 更新PrdSkill类型

**验收标准：**
- 可新建自定义Skill
- 可编辑Skill模板（CodeMirror编辑器）
- 可删除自定义Skill（内置不可删）
- 操作后列表自动刷新

---

### 任务1.4：实现Skill替换/版本管理（1天）

**问题描述：** 无版本管理字段，无自定义版本创建功能

**涉及文件：**
- `src-tauri/src/db/migration.rs`（新增迁移）
- `src-tauri/src/db/skills.rs`（新增方法）
- `src-tauri/src/services/prd_skill_service.rs`（新增方法）
- `src-tauri/src/commands.rs`（新增命令）
- `src/pages/SkillManagement.tsx`（新增UI）

**修复内容：**

1. **数据库迁移：添加版本字段**
   ```sql
   -- 新增迁移文件 005_skill_versioning.sql
   ALTER TABLE prd_skills ADD COLUMN version INTEGER DEFAULT 1;
   ALTER TABLE prd_skills ADD COLUMN parent_id TEXT;
   ALTER TABLE prd_skills ADD COLUMN is_deleted INTEGER DEFAULT 0;
   ```

2. **后端：实现版本管理**
   - 新增create_custom_skill命令（基于内置Skill创建自定义版本）
   - 新增list_skill_versions命令（查看版本历史）
   - 新增restore_skill_version命令（恢复到指定版本）

3. **后端：更新PrdSkill结构体**
   ```rust
   pub struct PrdSkill {
       // 现有字段...
       pub version: i32,
       pub parent_id: Option<String>,
       pub is_deleted: bool,
   }
   ```

4. **前端：SkillManagement.tsx新增功能**
   - "基于此创建自定义版本"按钮
   - 版本历史查看
   - 版本恢复功能

**验收标准：**
- 可基于内置Skill创建自定义版本
- 可查看版本历史
- 可恢复到指定版本
- 原内置Skill不受影响

---

## Phase 2：P1重要修复（4-5天）

### 目标
完善核心业务流程

### 任务2.1：实现会话双模式自动创建逻辑（1天）

**问题描述：** Settings有UI设置，但自动创建逻辑未实现

**涉及文件：**
- `src-tauri/src/services/task_service.rs`（修改）
- `src-tauri/src/services/agent_service.rs`（新增方法）
- `src-tauri/src/commands.rs`（新增命令）
- `src/pages/TaskDetail.tsx`（新增按钮）

**修复内容：**

1. **后端：监听任务状态变更**
   ```rust
   // task_service.rs
   pub async fn update_task_status(&self, task_id: &str, new_status: &str) -> Result<(), AppError> {
       // 1. 更新任务状态
       // 2. 检查session_mode设置
       // 3. 如果是auto模式且状态变为in_progress，自动创建会话
   }
   ```

2. **后端：实现自动创建会话**
   ```rust
   // agent_service.rs
   pub async fn create_session_from_task(&self, task_id: &str, project_id: &str) -> Result<AgentSession, AppError> {
       // 1. 获取任务信息
       // 2. 获取可用Agent
       // 3. 创建AgentSession
       // 4. 启动PTY
   }
   ```

3. **后端：新增命令**
   ```rust
   #[tauri::command]
   pub async fn create_session_from_task(
       state: State<'_, AppState>,
       task_id: String,
   ) -> Result<AgentSession, String>
   ```

4. **前端：TaskDetail.tsx**
   - 手动模式下显示"开启会话"按钮
   - 点击按钮调用create_session_from_task

**验收标准：**
- 自动模式：任务状态变为in_progress时自动创建会话
- 手动模式：任务详情页显示"开启会话"按钮
- 创建的会话正确关联task_id

---

### 任务2.2：实现分页系统（1天）

**问题描述：** Pagination.tsx组件已创建，但未集成到列表

**涉及文件：**
- `src-tauri/src/commands.rs`（修改所有list_*命令）
- `src-tauri/src/services/*.rs`（修改所有list方法）
- `src-tauri/src/db/*.rs`（修改所有查询）
- `src/pages/Requirements.tsx`（集成分页）
- `src/pages/Board.tsx`（集成分页）
- `src/pages/Sessions.tsx`（集成分页）
- `src/pages/Projects.tsx`（集成分页）

**修复内容：**

1. **后端：定义分页结构体**
   ```rust
   // 新增 src-tauri/src/pagination.rs
   pub struct PaginationParams {
       pub page: u32,
       pub page_size: u32,
   }

   pub struct PagedResult<T> {
       pub items: Vec<T>,
       pub total: u64,
       pub page: u32,
       pub page_size: u32,
       pub total_pages: u32,
   }
   ```

2. **后端：修改list_*命令**
   ```rust
   #[tauri::command]
   pub async fn list_requirements(
       state: State<'_, AppState>,
       page: Option<u32>,
       page_size: Option<u32>,
       status: Option<String>,
   ) -> Result<PagedResult<Requirement>, String>
   ```

3. **后端：修改Service和DB层**
   - 添加LIMIT和OFFSET到SQL查询
   - 添加COUNT查询获取总数
   - 计算总页数

4. **前端：列表页面集成分页**
   - 使用Pagination组件
   - 管理currentPage状态
   - 切换页面时重新请求数据

**验收标准：**
- 列表显示分页组件
- 可切换页码
- 每页显示指定数量（默认20条）
- 总数和总页数正确显示

---

### 任务2.3：实现删除保护策略（0.5天）

**问题描述：** 无前置检查，可直接删除关联数据

**涉及文件：**
- `src-tauri/src/commands.rs`（修改delete_*命令）
- `src-tauri/src/services/*.rs`（新增检查方法）
- `src-tauri/src/db/*.rs`（新增计数查询）

**修复内容：**

1. **后端：实现检查函数**
   ```rust
   // project_service.rs
   pub async fn check_can_delete_project(&self, project_id: &str) -> Result<(), AppError> {
       let req_count = self.db.get_requirement_count_by_project(project_id)?;
       let task_count = self.db.get_task_count_by_project(project_id)?;
       if req_count > 0 || task_count > 0 {
           return Err(AppError::InvalidInput(format!(
               "项目下有 {} 个需求和 {} 个任务，请先清理",
               req_count, task_count
           )));
       }
       Ok(())
   }

   // requirement_service.rs
   pub async fn check_can_delete_requirement(&self, req_id: &str) -> Result<(), AppError> {
       let task_count = self.db.get_task_count_by_requirement(req_id)?;
       if task_count > 0 {
           return Err(AppError::InvalidInput(format!(
               "需求下有 {} 个关联任务，请先删除或解绑",
               task_count
           )));
       }
       Ok(())
   }
   ```

2. **后端：修改delete_*命令**
   ```rust
   #[tauri::command]
   pub async fn delete_project(
       state: State<'_, AppState>,
       id: String,
   ) -> Result<(), String> {
       let svc = ProjectService::new(&state.db);
       svc.check_can_delete_project(&id).await?; // 新增检查
       svc.delete_project(&id).await
   }
   ```

3. **后端：新增计数查询**
   ```rust
   // db/projects.rs
   pub fn get_requirement_count_by_project(&self, project_id: &str) -> Result<i64> {
       let count = self.conn.query_row(
           "SELECT COUNT(*) FROM requirements WHERE project_id = ?1",
           [project_id],
           |row| row.get::<_, i64>(0),
       )?;
       Ok(count)
   }
   ```

**验收标准：**
- 删除有关联数据的项目时，返回错误提示
- 删除有关联任务的需求时，返回错误提示
- 错误信息包含关联数据数量

---

### 任务2.4：实现全局ProjectSelector组件（0.5天）

**问题描述：** 创建需求/任务时无法选择所属项目

**涉及文件：**
- `src/components/ProjectSelector.tsx`（新增组件）
- `src/pages/Requirements.tsx`（集成）
- `src/pages/Board.tsx`（集成）
- `src/pages/Workspace.tsx`（集成）

**修复内容：**

1. **新建ProjectSelector组件**
   ```tsx
   interface ProjectSelectorProps {
       value?: string;
       onChange?: (projectId: string) => void;
       showAll?: boolean; // 是否显示"全部"选项
   }

   export default function ProjectSelector({ value, onChange, showAll }: ProjectSelectorProps) {
       // 1. 加载项目列表
       // 2. 下拉选择
       // 3. 默认选中最近使用的项目
   }
   ```

2. **集成到需求/任务创建表单**
   - RequirementForm添加项目选择
   - TaskForm添加项目选择

3. **集成到看板筛选**
   - Board顶部添加ProjectSelector
   - 选择项目后筛选任务

**验收标准：**
- 创建需求/任务时可选择所属项目
- 看板顶部可按项目筛选
- 默认选中最近使用的项目

---

### 任务2.5：实现附件管理（0.5天）

**问题描述：** AttachmentList.tsx组件已创建，但未接入后端

**涉及文件：**
- `src-tauri/src/commands.rs`（新增命令）
- `src-tauri/src/services/requirement_service.rs`（新增方法）
- `src/components/AttachmentList.tsx`（修改）
- `src/components/RequirementForm.tsx`（集成）

**修复内容：**

1. **后端：实现附件存储**
   - 附件存储为文件路径（不复制文件）
   - 数据库存储路径列表（JSON数组）

2. **后端：新增命令**
   ```rust
   #[tauri::command]
   pub async fn add_attachment(
       state: State<'_, AppState>,
       requirement_id: String,
       file_path: String,
   ) -> Result<(), String>

   #[tauri::command]
   pub async fn remove_attachment(
       state: State<'_, AppState>,
       requirement_id: String,
       file_path: String,
   ) -> Result<(), String>
   ```

3. **前端：集成AttachmentList**
   - RequirementForm显示附件列表
   - 支持添加/删除附件
   - 使用Tauri对话框选择文件

**验收标准：**
- 可为需求添加附件（文件路径）
- 可删除附件
- 附件列表正确显示

---

### 任务2.6：实现操作历史审计（0.5天）

**问题描述：** AuditLog.tsx页面已创建，但无数据来源

**涉及文件：**
- `src-tauri/src/commands.rs`（新增命令）
- `src-tauri/src/services/requirement_service.rs`（新增方法）
- `src-tauri/src/services/task_service.rs`（新增方法）
- `src/pages/AuditLog.tsx`（修改）

**修复内容：**

1. **后端：记录操作历史**
   - 需求状态变更时写入requirement_history表
   - 任务状态变更时写入task_history表（需新建）

2. **后端：新增查询命令**
   ```rust
   #[tauri::command]
   pub async fn list_audit_logs(
       state: State<'_, AppState>,
       entity_type: Option<String>, // requirement/task
       entity_id: Option<String>,
       page: Option<u32>,
   ) -> Result<PagedResult<AuditLog>, String>
   ```

3. **前端：AuditLog.tsx显示日志**
   - 按时间排序显示操作记录
   - 显示操作类型、操作人、变更内容

**验收标准：**
- 需求/任务状态变更时自动记录
- AuditLog页面显示操作历史
- 可按实体类型/ID筛选

---

## Phase 3：P2优化修复（3-4天）

### 目标
提升代码质量和用户体验

### 任务3.1：完成前端统一数据层（1天）

**问题描述：** 5个Store已创建，但部分页面仍直接调IPC

**涉及文件：**
- `src/stores/costStore.ts`（新增）
- `src/stores/statisticsStore.ts`（新增）
- `src/stores/commitsStore.ts`（新增）
- `src/stores/agentsStore.ts`（新增）
- `src/pages/Cost.tsx`（修改）
- `src/pages/Statistics.tsx`（修改）
- `src/pages/Commits.tsx`（修改）
- `src/pages/Agents.tsx`（修改）

**修复内容：**

1. **新建缺失的Store**
   ```typescript
   // stores/costStore.ts
   export const useCostStore = create((set) => ({
       summary: null,
       dailyUsage: [],
       loading: false,
       fetchSummary: async () => { /* ... */ },
       fetchDailyUsage: async () => { /* ... */ },
   }));
   ```

2. **修改页面使用Store**
   - Cost.tsx使用useCostStore
   - Statistics.tsx使用useStatisticsStore
   - Commits.tsx使用useCommitsStore
   - Agents.tsx使用useAgentsStore

3. **更新stores/index.ts**
   - 导出所有新Store

**验收标准：**
- 所有页面通过Store获取数据
- 无直接IPC调用（除Store外）
- 数据加载状态统一管理

---

### 任务3.2：修复useEffect cleanup（1天）

**问题描述：** 多个页面无cleanup，可能存在内存泄漏

**涉及文件：**
- `src/pages/*.tsx`（所有含useEffect的页面）
- `src/components/Layout.tsx`

**修复内容：**

1. **统一useEffect模式**
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

2. **逐页面修复**
   - Workspace.tsx
   - Requirements.tsx
   - Board.tsx
   - Sessions.tsx
   - Projects.tsx
   - Agents.tsx
   - Cost.tsx
   - Statistics.tsx
   - Commits.tsx
   - Settings.tsx

3. **状态栏轮询单例**
   ```typescript
   // stores/systemMonitorStore.ts
   let pollTimer: number | null = null;

   export const useSystemMonitorStore = create((set) => ({
       // ...
       startPolling: () => {
           if (pollTimer) return;
           pollTimer = window.setInterval(async () => {
               if (document.hidden) return; // 页面不可见时暂停
               const info = await getMonitorData();
               set({ memory: info.memory_used_mb, sessions: info.active_sessions });
           }, 2000);
       },
       stopPolling: () => {
           if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
       },
   }));
   ```

**验收标准：**
- 所有useEffect有cleanup函数
- 组件卸载时清除定时器
- 状态栏轮询使用单例模式
- 页面不可见时暂停轮询

---

### 任务3.3：集成Toast通知系统（0.5天）

**问题描述：** Toast.tsx已创建，但未集成到操作流程

**涉及文件：**
- `src/components/Toast.tsx`（完善）
- `src/App.tsx`（添加Provider）
- `src/pages/*.tsx`（集成调用）

**修复内容：**

1. **完善Toast组件**
   ```typescript
   // components/Toast.tsx
   interface ToastContextType {
       showToast: (message: string, type: 'success' | 'error' | 'warning') => void;
   }

   export const ToastContext = createContext<ToastContextType>({});
   export const useToast = () => useContext(ToastContext);
   ```

2. **App.tsx添加Provider**
   ```tsx
   function App() {
       return (
           <ToastProvider>
               <Router>
                   {/* ... */}
               </Router>
           </ToastProvider>
       );
   }
   ```

3. **各页面集成Toast**
   ```typescript
   const { showToast } = useToast();

   async function handleSave() {
       try {
           await saveData();
           showToast('保存成功', 'success');
       } catch (error) {
           showToast('保存失败: ' + error.message, 'error');
       }
   }
   ```

**验收标准：**
- 操作成功显示绿色Toast
- 操作失败显示红色Toast
- Toast自动消失（3秒）
- 可手动关闭

---

### 任务3.4：实现前端防重复提交（0.5天）

**问题描述：** 无submitting状态管理

**涉及文件：**
- `src/pages/Requirements.tsx`
- `src/pages/Board.tsx`
- `src/pages/Projects.tsx`
- `src/pages/Settings.tsx`
- `src/pages/SkillManagement.tsx`

**修复内容：**

1. **统一防重复提交模式**
   ```typescript
   const [submitting, setSubmitting] = useState(false);

   async function handleCreate() {
       if (submitting) return;
       setSubmitting(true);
       try {
           await createXxx();
           showToast('创建成功', 'success');
       } catch (error) {
           showToast('创建失败', 'error');
       } finally {
           setSubmitting(false);
       }
   }
   ```

2. **按钮状态管理**
   ```tsx
   <button onClick={handleCreate} disabled={submitting}>
       {submitting ? '提交中...' : '创建'}
   </button>
   ```

**验收标准：**
- 提交过程中按钮显示"提交中..."
- 按钮禁用防止重复点击
- 提交完成后恢复按钮状态

---

### 任务3.5：统一版本号（0.5天）

**问题描述：** package.json版本过旧(0.1.0)

**涉及文件：**
- `package.json`

**修复内容：**

```json
{
  "version": "2.3.0"
}
```

**验收标准：**
- package.json版本与tauri.conf.json一致
- 所有版本号统一为2.3.0

---

## Phase 4：P3补充修复（5-7天）

### 目标
补充基础设施和文档

### 任务4.1：补充单元测试（2天）

**涉及文件：**
- `src-tauri/src/validation.rs`（测试）
- `src-tauri/src/crypto.rs`（测试）
- `src-tauri/src/services/*.rs`（测试）
- `src-tauri/src/db/*.rs`（测试）
- `src/**/*.test.ts`（前端测试）

**修复内容：**

1. **后端单元测试**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_validate_requirement() {
           assert!(validate_requirement("标题", "P0").is_ok());
           assert!(validate_requirement("", "P0").is_err());
       }
   }
   ```

2. **前端单元测试**
   ```typescript
   // src/components/__tests__/Pagination.test.tsx
   import { render, screen } from '@testing-library/react';
   import Pagination from '../Pagination';

   test('renders pagination', () => {
       render(<Pagination currentPage={1} totalPages={5} onPageChange={() => {}} />);
       expect(screen.getByText('1')).toBeInTheDocument();
   });
   ```

**验收标准：**
- 核心函数有单元测试
- 测试覆盖率达到30%+
- 所有测试通过

---

### 任务4.2：添加.gitignore（0.5天）

**涉及文件：**
- `.gitignore`（新增）

**修复内容：**

```gitignore
# Dependencies
node_modules/
.pnp
.pnp.js

# Build
dist/
dist-ssr/
*.local

# Tauri
src-tauri/target/

# IDE
.vscode/*
!.vscode/extensions.json
.idea

# OS
.DS_Store
Thumbs.db

# Env
.env
.env.local
```

**验收标准：**
- 构建产物不被提交
- 依赖目录不被提交
- 敏感信息不被提交

---

### 任务4.3：添加CLAUDE.md（0.5天）

**涉及文件：**
- `CLAUDE.md`（新增）

**修复内容：**

```markdown
# CLAUDE.md

## 项目概述
AI Collaboration Engine (ACE) - 基于Tauri 2.0的桌面应用

## 技术栈
- 前端：React + TypeScript + Tailwind CSS
- 后端：Rust + Tauri 2.0
- 数据库：SQLite
- 状态管理：Zustand

## 开发命令
- 安装依赖：`npm install`
- 开发模式：`npm run tauri dev`
- 构建：`npm run tauri build`
- 测试：`npm test`

## 代码规范
- Rust：rustfmt + clippy
- TypeScript：eslint + prettier
- 提交信息：遵循Conventional Commits

## 项目结构
- `src/` - 前端代码
- `src-tauri/` - 后端代码
- `docs/` - 文档
```

**验收标准：**
- AI助手可读取项目上下文
- 开发者可快速了解项目结构

---

### 任务4.4：添加变更日志（0.5天）

**涉及文件：**
- `CHANGELOG.md`（新增）

**修复内容：**

```markdown
# Changelog

## [2.3.0] - 2026-09-20

### Added
- Agent数据持久化
- 系统资源监控（应用内存）
- Skill管理CRUD功能
- Skill版本管理
- 会话双模式自动创建
- 分页系统
- 删除保护策略
- 全局项目选择器
- 附件管理
- 操作历史审计
- Toast通知系统

### Fixed
- Agent检测后数据丢失
- 状态栏显示系统内存而非应用内存
- Skill管理无法编辑/删除

### Changed
- 统一版本号为2.3.0
```

**验收标准：**
- 记录所有版本变更
- 按Added/Fixed/Changed分类

---

### 任务4.5：添加API文档（1天）

**涉及文件：**
- `docs/api.md`（新增）

**修复内容：**

1. **整理所有Tauri命令**
   - 命令名称
   - 参数说明
   - 返回类型
   - 示例

2. **整理所有数据模型**
   - Requirement
   - Task
   - Project
   - Agent
   - AgentSession
   - PrdSkill

**验收标准：**
- 覆盖所有后端命令
- 参数和返回类型清晰
- 包含使用示例

---

### 任务4.6：添加用户指南（1天）

**涉及文件：**
- `docs/user-guide.md`（新增）

**修复内容：**

1. **快速开始**
   - 安装说明
   - 首次配置
   - 基本操作

2. **功能说明**
   - 工作台使用
   - 项目管理
   - 需求管理
   - 任务管理
   - Agent管理
   - Skill管理

3. **常见问题**
   - Agent检测失败
   - API Key配置
   - 数据备份

**验收标准：**
- 覆盖所有核心功能
- 包含截图示例
- 解答常见问题

---

## 依赖关系

```
Phase 1
├── 1.1 Agent持久化 (独立)
├── 1.2 系统监控修复 (独立)
├── 1.3 Skill CRUD (依赖1.1)
└── 1.4 Skill版本管理 (依赖1.3)

Phase 2
├── 2.1 会话双模式 (依赖1.1)
├── 2.2 分页系统 (独立)
├── 2.3 删除保护 (独立)
├── 2.4 ProjectSelector (独立)
├── 2.5 附件管理 (独立)
└── 2.6 操作历史 (独立)

Phase 3
├── 3.1 前端数据层 (依赖2.2)
├── 3.2 useEffect cleanup (独立)
├── 3.3 Toast通知 (独立)
├── 3.4 防重复提交 (独立)
└── 3.5 版本号统一 (独立)

Phase 4
├── 4.1 单元测试 (依赖Phase 1-3)
├── 4.2 .gitignore (独立)
├── 4.3 CLAUDE.md (独立)
├── 4.4 变更日志 (依赖Phase 1-3)
├── 4.5 API文档 (依赖Phase 1-3)
└── 4.6 用户指南 (依赖Phase 1-3)
```

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|-----|------|---------|
| 数据库迁移失败 | 数据丢失 | 迁移前自动备份 |
| Skill版本管理复杂 | 开发延期 | 先实现基础CRUD，版本管理可简化 |
| 分页系统影响性能 | 用户体验差 | 先实现后端分页，前端渐进集成 |
| 测试覆盖率不足 | 质量风险 | 优先测试核心功能 |

---

## 验收检查清单

### Phase 1验收
- [ ] Agent检测后持久化，下次直接加载
- [ ] 状态栏显示应用内存（如215MB）
- [ ] Skill管理可新建/编辑/删除
- [ ] Skill可创建自定义版本

### Phase 2验收
- [ ] 自动模式下任务状态变更自动创建会话
- [ ] 列表显示分页组件，可切换页码
- [ ] 删除有关联数据时返回错误提示
- [ ] 创建需求/任务时可选择项目
- [ ] 可为需求添加附件
- [ ] AuditLog显示操作历史

### Phase 3验收
- [ ] 所有页面通过Store获取数据
- [ ] 所有useEffect有cleanup
- [ ] 操作成功/失败显示Toast
- [ ] 提交按钮防重复点击
- [ ] 版本号统一为2.3.0

### Phase 4验收
- [ ] 核心函数有单元测试
- [ ] .gitignore正确配置
- [ ] CLAUDE.md已添加
- [ ] CHANGELOG.md已添加
- [ ] API文档已添加
- [ ] 用户指南已添加

---

**计划制定完成时间：** 2026-09-20
**下一步：** 按Phase顺序执行修复任务
