# 前端+后端业务问题分析报告

**分析日期**: 2026-09-20

---

## 问题 1: Agent 检测后重新进入页面会消失

### 严重度: 高

### 现象

点击"刷新检测"后 Agent 出现，但切换到其他页面再回来，检测到的 Agent 消失，需要重新检测。

### 根因分析

**文件**: `src/pages/Agents.tsx`

```typescript
// 第 19-31 行: loadData() 只调用 getAgents()
async function loadData() {
  const savedAgents = await getAgents(); // ← 从数据库读取
  setAgents(savedAgents);
}

// 第 33-43 行: handleRefresh() 调用 discoverAgents()
async function handleRefresh() {
  const agentsData = await discoverAgents(); // ← 只在内存中，不保存到数据库
  setAgents(agentsData); // ← 设置到组件 state
}
```

**核心问题**: `discoverAgents()` 只在内存中检测，**不保存到数据库**。`Agents` 组件没有 agent store，使用 `useState` 管理状态。当页面重新加载时，`loadData()` 调用 `getAgents()` 从数据库读取，但 `discoverAgents()` 检测到的结果从未写入数据库。

### 后端行为

**文件**: `src-tauri/src/services/agent_service.rs`

`discover_agents()` 方法会检测并调用 `upsert_agent` 保存到数据库（之前已修复为 binary_path 唯一键）。但问题在于 `Agents.tsx` 的 `loadData` 函数在每次进入页面时只调用 `getAgents()` 读取数据库，如果数据库中没有记录（因为从未调用过 `discover_agents`），就会显示空列表。

**等等** — 实际上 `discover_agents` 的后端实现是调用 `upsert_agent` 保存的。让我重新检查：

**文件**: `src-tauri/src/services/agent_service.rs`

```rust
pub async fn discover_agents(&self) -> Result<Vec<Agent>, String> {
    // ... 检测逻辑 ...
    // 检测到后调用 upsert_agent 保存
    agent_service.upsert_agent(&detected)?;
}
```

所以如果 `discover_agents` 确实保存了，那 `getAgents()` 应该能读到。让我再检查...

**实际根因**: 后端 `discover_agents` 在 agent_service 中是独立创建的 service 实例，它使用 `AgentService::new(&state.db)`，而 `get_agents` 直接调用 `db.get_agents()`。问题是 `AgentService` 的 `discover_agents` 方法创建了独立的 db 连接锁。

**重新分析**: 实际上 `discover_agents` 和 `get_agents` 都通过同一个 `state.db`。`discover_agents` 的 upsert 保存了数据，`get_agents` 应该能读到。

**真正的根因** — `Agents.tsx` 中的 `handleRefresh` 调用 `discoverAgents()` 返回结果后只设置到 `useState`，而**不调用 `loadData()` 重新从数据库加载**。如果 `discover_agents` 后端确实保存了数据库，那重新进入页面调用 `loadData()` -> `getAgents()` 应该能读到。

但如果 `discover_agents` 后端实现**没有正确保存**（比如 upsert 失败），那数据库中就没有记录。

**最终根因总结**:

1. `Agents.tsx` 没有使用 store，用 `useState` 管理状态，页面卸载后状态丢失
2. `handleRefresh()` 调用 `discoverAgents()` 后只设置内存状态，需确认后端是否正确保存
3. 进入页面时 `loadData()` 从数据库加载，但如果数据库没有数据就显示空

### 修复方案

1. 在 `Agents.tsx` 的 `handleRefresh()` 成功后调用 `loadData()` 确保从数据库重新加载
2. 增加 agent 的 `is_active` 字段，检测时验证 binary_path 是否仍然存在
3. 前端显示"已失效"标志

---

## 问题 2: 项目创建按钮失效

### 严重度: 高

### 根因分析

**文件**: `src/pages/Projects.tsx` 第 19-30 行

```typescript
async function handleCreate() {
  if (!newName.trim()) return;  // ← 如果名称为空则静默返回
  try {
    await addProject(newName, ...);  // ← 调用 store 的 addProject
    setShowCreateModal(false);
  } catch (error) {
    console.error("创建项目失败:", error);  // ← 错误只打印到控制台，无用户反馈
  }
}
```

**文件**: `src/stores/projectsStore.ts` 第 29-33 行

```typescript
addProject: async (name, description, key) => {
  const project = await createProject(name, description, key);  // ← 调用 tauri
  await get().fetchProjects();  // ← 刷新列表
  return project;
},
```

**文件**: `src/lib/tauri.ts` 第 344-350 行

```typescript
export async function createProject(name: string, description?: string, key?: string): Promise<Project> {
  return invoke("create_project", { name, description, key });
}
```

**后端命令**: `src-tauri/src/commands.rs` 第 584-596 行

```rust
pub async fn create_project(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    key: Option<String>,
) -> Result<Project, String> {
    validate_project(&name).map_err(|e| e.to_string())?;
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.create_project(&name, description.as_deref(), key.as_deref()).await
}
```

**分析**: 代码逻辑看起来是正确的。按钮点击 → handleCreate → addProject → createProject (invoke) → 后端 create_project → 返回。

可能的问题：

1. **验证失败静默** — `validate_project` 失败时错误被 catch 但只 console.error，用户看不到
2. **Modal overlay 问题** — Modal 的 onClick 覆盖了按钮点击
3. **后端错误未传播** — 后端返回的错误被 catch 但无 toast 提示

**最可能的原因**: 后端验证失败（如 `validate_project` 返回错误），或者 `ProjectService` 的实现有问题，但错误被静默吞掉，用户看不到任何反馈。

---

## 问题 3: 任务创建按钮失效

### 严重度: 高

### 根因分析

**文件**: `src/pages/Board.tsx` 第 58-78 行

```typescript
async function handleCreate(data: any) {
  try {
    await createTask(
      data.title,
      undefined,
      data.task_type,
      data.priority,
      data.assignee,
      data.due_date,
      data.estimated_hours,
      data.priority_level,
      data.blocked_reason,
      data.tags,
      data.notes,
    );
    setShowCreateModal(false);
    await fetchTasks();
  } catch (error) {
    console.error("创建任务失败:", error); // ← 无用户反馈
  }
}
```

**与项目相同的问题**: 错误被静默 catch，用户看不到任何反馈。

**文件**: `src/components/TaskForm.tsx` — 需要检查表单的 onSubmit 是否正确触发。

---

## 问题 4: 需求创建按钮失效

### 严重度: 高 — **这是最严重的问题**

### 根因分析

**关键 BUG**: `listRequirements` 返回类型不匹配

**文件**: `src/lib/tauri.ts` 第 55-59 行

```typescript
export async function listRequirements(status?: string): Promise<Requirement[]> {
  // ← 声明返回 Requirement[] (数组)
  return invoke("list_requirements", { status });
}
```

**文件**: `src-tauri/src/commands.rs` 第 48-61 行

```rust
pub async fn list_requirements(
    state: State<'_, AppState>,
    page: Option<u32>,
    page_size: Option<u32>,
    status: Option<String>,
) -> Result<crate::pagination::PagedResult<Requirement>, String> {
    // ← 实际返回 PagedResult<Requirement>，包含 { items, total, page, ... }
}
```

**后端返回的是 `PagedResult<Requirement>`**，结构为：

```json
{ "items": [...], "total": 10, "page": 1, "page_size": 20, "total_pages": 1 }
```

**但前端声明为 `Requirement[]`**，store 中直接赋值：

```typescript
const data = await listRequirements(status);
set({ requirements: data, loading: false }); // ← data 是对象而非数组！
```

**后果**:

- `requirements` 变成了 `{ items: [...], total: ..., page: ... }` 对象
- `requirements.length` 为 `undefined`
- `requirements.filter(...)` 抛出运行时错误
- 需求页面无法渲染任何内容
- 创建需求后刷新列表也会失败

### 修复方案

**方案 A**（推荐）: 前端适配分页返回格式

```typescript
export async function listRequirements(status?: string) {
  const result = await invoke("list_requirements", { status });
  return result.items; // ← 提取 items 数组
}
```

**方案 B**: 后端改为返回数组（去掉分页）

---

## 问题 5: 创建 Skill 失败

### 严重度: 高

### 根因分析

**文件**: `src/pages/SkillManagement.tsx` 第 42-62 行

```typescript
async function handleCreate() {
  if (!formData.name.trim() || !formData.template.trim()) return;
  setSaving(true);
  try {
    await createPrdSkill(formData.name, formData.skill_type, formData.description || null, formData.template);
    // ...
  } catch (error) {
    console.error("创建Skill失败:", error);
    showToast("error", "创建Skill失败");
  }
}
```

**文件**: `src/lib/tauri.ts` 第 255-262 行

```typescript
export async function createPrdSkill(
  name: string,
  skillType: string,
  description: string | null,
  template: string,
): Promise<PrdSkill> {
  return invoke("create_prd_skill", { name, skillType, description, template });
}
```

**文件**: `src-tauri/src/commands.rs` 第 408-418 行

```rust
pub async fn create_prd_skill(
    state: State<'_, AppState>,
    name: String,
    skill_type: String,     // ← Rust 参数名: skill_type
    description: Option<String>,
    template: String,
) -> Result<PrdSkill, String> {
```

**参数名映射**: 前端传 `skillType`，后端接收 `skill_type`。Tauri 的 `invoke` 使用 camelCase 到 snake_case 的自动映射，所以 `skillType` → `skill_type` 是正确的。

**数据库插入**: `src-tauri/src/db/skills.rs` 第 51-54 行

```rust
conn.execute(
    "INSERT INTO prd_skills (id, name, description, skill_type, template, is_builtin, version, parent_id, is_deleted, created_at)
     VALUES (?, ?, ?, ?, ?, 0, 1, NULL, 0, ?)",
    params![id, name, description, skill_type, template, now],
)?;
```

**可能的问题**:

1. 数据库表 `prd_skills` 可能缺少新字段（version, parent_id, is_deleted）— 需要检查 migration
2. SkillManagement 页面使用的是旧版 `PrdSkill` 类型（没有 version/parent_id/is_deleted 字段）

**关键发现**: SkillManagement.tsx 直接导入 `PrdSkill` from `../lib/types`，但 types.ts 中的 `PrdSkill` 可能缺少 `version`、`parent_id`、`is_deleted` 字段。如果数据库返回了这些字段但前端类型没有声明，可能会导致反序列化问题。

---

## 问题汇总

| #   | 问题             | 严重度   | 根因                                                                  | 修复难度 |
| --- | ---------------- | -------- | --------------------------------------------------------------------- | -------- |
| 1   | Agent 检测后消失 | 高       | `useState` 状态不持久化 + `handleRefresh` 不刷新数据库                | 中       |
| 2   | 项目创建按钮失效 | 高       | 错误静默 + 可能的验证失败                                             | 低       |
| 3   | 任务创建按钮失效 | 高       | 错误静默 + 可能的验证失败                                             | 低       |
| 4   | 需求创建按钮失效 | **严重** | `listRequirements` 返回类型 `PagedResult` vs 前端期望 `Requirement[]` | **高**   |
| 5   | Skill 创建失败   | 高       | 可能是数据库 migration 缺少新字段                                     | 中       |

---

## 统一修复建议

### 必须修复（阻断性问题）

1. **需求列表返回类型** — `listRequirements` 在 `tauri.ts` 中需要适配 `PagedResult` 返回格式
2. **错误反馈** — 所有创建操作的 catch 块需要添加 `showToast("error", ...)` 提示
3. **Agent 状态持久化** — `handleRefresh` 成功后应调用 `loadData()` 从数据库重新加载

### 建议修复

4. **Agent 失效检测** — 启动前验证 binary_path 是否存在，添加"已失效"标志
5. **数据库 Migration** — 确保 prd_skills 表包含 version, parent_id, is_deleted 字段
6. **PrdSkill 类型同步** — 前端 types.ts 需包含所有数据库字段
