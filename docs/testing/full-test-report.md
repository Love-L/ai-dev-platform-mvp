# AI研发协作平台 - 全栈自动化测试报告

**首次生成**: 2026-09-20 03:35 UTC
**最近实测**: 2026-09-21（第四轮修复后）
**测试环境**: Windows 11 Pro, Node.js, Vitest 1.6.0, Playwright
**应用版本**: v2.3.0

> ⚠️ **数据说明**：下方各分层章节保留首次生成时的历史明细。
> 其中"文件清单 / 用例数 / 覆盖率"一律以本节的 **实测汇总** 为准（第一轮报告中存在失实的文件清单与覆盖率，已按实测更正）。

---

## 📊 实测汇总（权威）

| 测试层                                   | 运行命令                 | 实测用例             | 通过         | 失败 | 状态    |
| ---------------------------------------- | ------------------------ | -------------------- | ------------ | ---- | ------- |
| 前端全套（L1 组件+Store / L3 / L5 / L6） | `npm run test:win`       | **245**（89 个套件） | 245          | 0    | ✅      |
| 后端默认可执行单测                       | `npm run test:rust`      | **29**               | 29           | 0    | ✅      |
| 后端门控集成用例（需 Victauri 运行环境） | 同上（未开 E2E 时 skip） | 71                   | —            | —    | ⏭️ 跳过 |
| 后端门控冒烟用例                         | 同上                     | 3                    | —            | —    | ⏭️ 跳过 |
| L2 E2E (Playwright)                      | `npm run test:e2e`       | **24**               | 需浏览器环境 | —    | ▶️ 手动 |
| L4 CDP (WebView2)                        | `npm run test:l4`        | **5**                | 需启动应用   | —    | ▶️ 手动 |

**运行命令说明**

| 命令                          | 说明                                                                                     |
| ----------------------------- | ---------------------------------------------------------------------------------------- |
| `npm test` / `npm run verify` | 常规 Vitest 全量（含 L3 / L5 / L6 与页面单测）                                           |
| `npm run test:win`            | 项目路径含中文时使用（内部 `subst` 映射为 ASCII 盘符，规避 Vitest 1.6 的用例不注册问题） |
| `npm run test:pages`          | 页面级单测（19 个页面冒烟 + F060 / F021 回归）                                           |
| `npm run test:l4`             | 连接**运行中**的 Tauri 应用（需 `$env:TAURI_DEVTOOLS_SERVER_PORT=9222`）                 |
| `npm run test:rust`           | 后端测试（默认执行 29 个单测；门控用例需 Victauri 运行环境）                             |

**第四轮新增测试**

| 文件                                       | 说明                                                                       |
| ------------------------------------------ | -------------------------------------------------------------------------- |
| `src/pages/__tests__/Pages.smoke.test.tsx` | 18 个路由页面 + 未挂载 `Dashboard` 的渲染冒烟（19 例）                     |
| `src/pages/__tests__/Commits.test.tsx`     | F060 回归（提交携带任务/会话、平台记录展示）+ F021 回归（`verify_agents`） |
| `tests/e2e/pages-coverage.test.ts`         | L2 页面覆盖（10 例，E2E 合计 24 例）                                       |
| `tests/l4-cdp/webview2-coverage.test.ts`   | L4 改写为按端口自动启用的真实 CDP 用例（5 例）                             |

---

## L1: Mock Runtime 测试

### 测试文件清单（实测）

> 以下为仓库中实际存在的测试文件（原报告的 `AgentCard` / `ChatMessage` / `StatusBadge` / `EmptyState` / `SearchInput` 五个文件在仓库中并不存在，其对应组件亦不存在，属历史失实数据，已移除）。

| 文件                                                    | 覆盖范围                            |
| ------------------------------------------------------- | ----------------------------------- |
| `components/__tests__/RequirementForm.test.tsx`         | 需求表单渲染 / 校验 / 提交          |
| `components/__tests__/RequirementForm.minimal.test.tsx` | 需求表单最小渲染                    |
| `components/__tests__/TaskForm.test.tsx`                | 任务表单、类型选择、编辑模式        |
| `components/__tests__/ErrorBoundary.test.tsx`           | 错误捕获、UI 显示、重试             |
| `components/__tests__/Skeleton.test.tsx`                | 骨架屏渲染                          |
| `components/__tests__/AnimatedNumber.test.tsx`          | 数字动画                            |
| `components/__tests__/AgentTerminal.test.tsx`           | 终端初始化                          |
| `components/__tests__/RichTextEditor.test.tsx`          | 编辑器初始化与内容操作              |
| `components/__tests__/Layout.test.tsx`                  | 布局与导航                          |
| `components/__tests__/AttachmentList.test.tsx`          | 附件列表                            |
| `components/__tests__/Badges.test.tsx`                  | 状态 / 优先级 / 类型徽章            |
| `components/__tests__/Pagination.test.tsx`              | 分页控件（>7 页省略号折叠）         |
| `components/__tests__/Toast.test.tsx`                   | 全局通知                            |
| `stores/__tests__/requirementsStore.test.ts`            | 需求 Store                          |
| `stores/__tests__/tasksStore.test.ts`                   | 任务 Store                          |
| `stores/__tests__/projectsStore.test.ts`                | 项目 Store                          |
| `stores/__tests__/sessionsStore.test.ts`                | 会话 Store                          |
| `stores/__tests__/settingsStore.test.ts`                | 设置 Store                          |
| `stores/__tests__/themeStore.test.ts`                   | 主题 Store（含 `persist`）          |
| `pages/__tests__/Pages.smoke.test.tsx`                  | **19 个页面渲染冒烟（第四轮新增）** |
| `pages/__tests__/Commits.test.tsx`                      | **F060 / F021 回归（第四轮新增）**  |
| `test/example.test.ts`                                  | 框架自检                            |

### Mock 策略

- **@tauri-apps/api/core**: `vi.mock` 返回 `invoke: vi.fn()`
- **@tiptap/react**: Mock `useEditor` 返回完整编辑器对象
- **@xterm/xterm**: Mock Terminal 类含 `open/write/onData/onResize/resize`
- **react-countup**: Mock 返回固定数值 span
- **framer-motion**: Mock `motion.div` 返回普通 div
- **react-router-dom**: Mock `NavLink`/`Link` 返回子元素

### 关键技术决策

- `src/test/setup.ts` **不包含** `vi.mock` 调用（会导致 Vitest 挂起）
- 每个测试文件独立管理自己的 mock
- `isolate: false` 提升执行速度

---

## L2: E2E 结构冒烟测试 (24 tests)

### 测试文件清单（实测）

| 文件                     | 测试数 | 覆盖范围                                                                                                     |
| ------------------------ | ------ | ------------------------------------------------------------------------------------------------------------ |
| `navigation.test.ts`     | 4      | L2-007~010: 页面导航、首页加载                                                                               |
| `requirements.test.ts`   | 3      | L2-001~003: 需求页面加载、列表、导航                                                                         |
| `tasks.test.ts`          | 3      | L2-004~006: 看板加载、导航、内容                                                                             |
| `workspace.test.ts`      | 2      | L2-015~016: 工作台、统计卡片                                                                                 |
| `statusbar.test.ts`      | 2      | L2-017~018: 状态栏、版本号                                                                                   |
| `pages-coverage.test.ts` | **10** | 第四轮新增：Skill / Agent / 会话 / 提交 / 成本 / 统计 / 审计 / 设置 / 监控 / 导入导出 的路由可达与主内容渲染 |

### 技术方案

- **框架**: Playwright (替代 @wdio/tauri-service)
- **目标**: Vite dev server (http://localhost:5174)
- **路由**: Hash 路由 (`#/requirements`, `#/board`)
- **选择器**: `getByRole`, `a[href="#/path"]`, CSS class selectors

### 限制说明

- 测试针对 Vite dev server 运行，非 Tauri WebView2
- IPC 调用在浏览器环境中不可用，仅测试前端渲染和导航
- 完整 E2E 需启用 Tauri `devtools` feature 连接 WebView2

---

## L3: IPC 契约测试 (mock IPC, 33 tests)

> **命名说明**：本层原名为 "AI MCP Full-stack"，但实现方式为 `vi.mock('@tauri-apps/api/core')`，
> 仅断言"前端封装是否传对了命令名与参数"，**不连接 Rust 后端、不访问 SQLite**。
> 真实全栈链路验证见 `src-tauri/src/db/integration_tests.rs`（默认执行，真实临时 SQLite）
> 与 `src-tauri/tests/integration.rs`（Victauri 门控，71 例）。

### 测试文件清单

| 文件                       | 测试数 | 覆盖范围                                |
| -------------------------- | ------ | --------------------------------------- |
| `requirement-flow.test.ts` | 5      | L3-001~005: 需求 CRUD + PRD 生成        |
| `task-flow.test.ts`        | 7      | L3-006~012: 任务 CRUD + 评论 + 状态流转 |
| `project-flow.test.ts`     | 7      | L3-013~019: 项目 CRUD + Skill 配置      |
| `settings-flow.test.ts`    | 5      | L3-020~024: 设置读写 + AES-256-GCM 加密 |
| `git-flow.test.ts`         | 3      | L3-025~027: Git Log/Status/Diff         |
| `statistics-flow.test.ts`  | 6      | L3-028~033: 统计 + Token 追踪 + 预算    |

### 测试策略

- Mock `@tauri-apps/api/core` 的 `invoke` 函数
- 验证每个 IPC 调用的参数和返回值
- 测试 PagedResult 解析逻辑
- 测试加密/解密往返一致性

---

## L4: Playwright CDP 测试 (5 tests)

> **第四轮更新**：`devtools` feature 已启用（`Cargo.toml`），用例已改写为**真实 CDP 断言**
> （控制台无错误、请求无 4xx/5xx、性能指标、IPC 拦截与往返耗时），
> 并按 `TAURI_DEVTOOLS_SERVER_PORT` 自动启用/跳过。

### 运行方式（需本机手动执行）

```powershell
# 终端 1：启动应用并暴露 CDP 端口
$env:TAURI_DEVTOOLS_SERVER_PORT=9222; npm run tauri dev

# 终端 2：连接并运行 L4 用例
npm run test:l4
```

未设置该环境变量时，5 个用例自动 `skip`（不影响常规流水线）。

### 启用步骤

1. 修改 `src-tauri/Cargo.toml`:
   ```toml
   tauri = { version = "2", features = ["tray-icon", "image-ico", "devtools"] }
   ```
2. 重新构建: `cargo build`
3. 启动应用时设置: `TAURI_DEVTOOLS_SERVER_PORT=9222`
4. 获取 WebSocket URL: `curl http://localhost:9222`

### 待实现测试

- L4-001: WebView2 控制台日志捕获
- L4-002: 网络请求监控
- L4-003: 性能指标收集
- L4-004: Tauri IPC 拦截
- L4-005: IPC 响应时间测量

---

## L5: Chaos Engineering 测试 (10 tests)

### 测试文件清单

| 测试   | 覆盖范围                    |
| ------ | --------------------------- |
| L5-001 | IPC 超时异常                |
| L5-002 | invoke 返回 null            |
| L5-003 | invoke 返回空数组           |
| L5-004 | PagedResult 缺少 items 字段 |
| L5-005 | 网络错误模拟                |
| L5-006 | 并发 IPC 调用               |
| L5-007 | 大数据量 (10000 条)         |
| L5-008 | XSS 特殊字符输入            |
| L5-009 | Unicode 字符处理            |
| L5-010 | 空字符串输入                |

---

## L6: IPC Coverage 测试

### 覆盖范围（实测）

| IPC 命令分类       | 命令数 | 测试数 |
| ------------------ | ------ | ------ |
| 需求管理           | 8      | 8      |
| 任务管理           | 8      | 8      |
| 任务评论           | 3      | 3      |
| Agent 管理         | 7      | 7      |
| Token 追踪         | 7      | 7      |
| PRD Skill          | 6      | 6      |
| 流水线环节         | 5      | 5      |
| 执行日志与统计     | 4      | 4      |
| PTY 终端           | 6      | 6      |
| Git（含提交落库）  | 5      | 5      |
| 项目管理           | 5      | 5      |
| 项目 Skill / MCP   | 6      | 6      |
| 数据导入导出       | 2      | 2      |
| 缓存               | 2      | 2      |
| 审计               | 1      | 1      |
| 系统监控           | 2      | 2      |
| 统计 / 设置 / 加密 | 5      | 5      |
| **总计**           | **82** | **82** |

### 覆盖说明（82/82 = 100%）

全部 82 条 Tauri IPC 命令均有对应测试用例，验证：

- 正确的 IPC 命令名称
- 参数传递正确性
- 返回值处理

> 本层为**前端封装契约测试**（mock IPC），不连接 Rust 后端；
> 后端真实链路由 `src-tauri/src/db/integration_tests.rs`（默认执行）与
> `src-tauri/tests/integration.rs`（Victauri 门控）保证。

---

## 🛠️ 运行指南

### L1 Mock Runtime

```bash
npx vitest run src/
```

### L2 E2E (需要 Vite dev server 运行)

```bash
# 终端1: 启动 dev server
npm run dev

# 终端2: 运行测试
npx playwright test
```

### L3 + L5 + L6 (IPC 测试)

```bash
npx vitest run tests/l3-fullstack tests/l5-chaos tests/l6-ipc
```

### 全部测试 (分组执行)

```bash
npx vitest run src/ && npx vitest run tests/l3-fullstack tests/l5-chaos tests/l6-ipc && npx playwright test
```

---

## 📋 需求覆盖矩阵

| 需求编号 | 需求描述   | 测试覆盖                           |
| -------- | ---------- | ---------------------------------- |
| R001     | 需求管理   | L1-001~006, L2-001~003, L3-001~005 |
| R002     | 任务看板   | L1-007~012, L2-004~006, L3-006~012 |
| R003     | Agent 管理 | L1-020~022, L3-018~019             |
| R004     | 会话管理   | L1-048~052                         |
| R005     | PRD 生成   | L3-005                             |
| R006     | 项目管理   | L1-043~047, L3-013~019             |
| R007     | 统计报表   | L3-028~033                         |
| R008     | 设置管理   | L1-053~057, L3-020~024             |
| R009     | Git 集成   | L3-025~027                         |

---

## ⚠️ 已知限制

1. **L4 CDP 测试**: 需要启用 Tauri devtools feature 才能连接 WebView2
2. **L2 E2E**: 测试针对 Vite dev server，非完整 Tauri 应用
3. **Mock 隔离**: L1 和 L3/L6 测试不能同时运行（vi.mock 冲突）
4. **Victauri MCP**: 未配置完整的 MCP bridge 工具，L3 通过 mock IPC 实现

---

## 📁 文件结构

```
tests/
├── l3-fullstack/          # L3 全栈测试
│   ├── requirement-flow.test.ts
│   ├── task-flow.test.ts
│   ├── project-flow.test.ts
│   ├── settings-flow.test.ts
│   ├── git-flow.test.ts
│   └── statistics-flow.test.ts
├── l4-cdp/                # L4 CDP 测试 (待启用)
│   └── webview2-coverage.test.ts
├── l5-chaos/              # L5 混沌工程
│   └── store-fault-injection.test.ts
├── l6-ipc/                # L6 IPC 覆盖
│   └── ipc-coverage.test.ts
└── e2e/                   # L2 E2E 测试
    ├── navigation.test.ts
    ├── requirements.test.ts
    ├── tasks.test.ts
    ├── workspace.test.ts
    └── statusbar.test.ts

src/
├── components/__tests__/  # L1 组件测试 (19 files)
├── stores/__tests__/      # L1 Store 测试 (5 files)
└── test/setup.ts          # 测试 setup
```
