# AI研发协作平台 - 单机版 MVP

基于 Tauri 2.0 + React + SQLite 构建的AI驱动软件研发协作平台单机版本。

## 功能特性

### ✅ 已实现

- **需求管理**: 创建、编辑、删除需求，支持优先级和状态管理
- **PRD生成**: 集成Claude API，一键生成结构化PRD文档
- **任务看板**: 拖拽式任务管理，支持待开发/开发中/已完成三列看板
- **Agent管理**: 自动检测本地CLI Agent（Claude Code、Codex），支持启动和监控
- **Token追踪**: 记录和统计LLM API调用的Token消耗
- **本地存储**: 所有数据存储在本地SQLite数据库

### 🚧 待实现

- Agent终端面板（PTY集成）
- 多Agent并行管理
- 统计报表
- 审计日志

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.0 |
| 前端 | React 18 + TypeScript |
| UI样式 | TailwindCSS |
| 状态管理 | Zustand |
| 本地存储 | SQLite (via rusqlite) |
| LLM | Claude API (Anthropic) |

## 开发环境要求

- Node.js >= 18
- Rust >= 1.70
- npm 或 pnpm

## 快速开始

### 1. 安装依赖

```bash
cd ai-dev-platform-mvp

# 安装前端依赖
npm install
```

### 2. 启动开发服务器

```bash
# 启动Tauri开发模式
npm run tauri dev
```

### 3. 首次使用配置

1. 启动应用后，进入"系统设置"页面
2. 输入Claude API Key（从 [Anthropic控制台](https://console.anthropic.com/) 获取）
3. 保存设置

## 项目结构

```
ai-dev-platform-mvp/
├── src-tauri/                 # Tauri Rust后端
│   ├── src/
│   │   ├── main.rs           # 入口
│   │   ├── db.rs             # 数据库操作
│   │   ├── llm.rs            # Claude API客户端
│   │   ├── agent.rs          # Agent发现和管理
│   │   └── commands.rs       # Tauri Commands
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                       # React前端
│   ├── pages/                 # 页面组件
│   ├── components/            # 通用组件
│   ├── lib/                   # 工具函数和类型定义
│   └── stores/                # 状态管理
├── package.json
└── README.md
```

## 使用流程

### 1. 创建需求

1. 点击侧边栏"需求管理"
2. 点击"创建需求"按钮
3. 填写需求标题、描述和优先级
4. 保存需求

### 2. 生成PRD

1. 进入需求详情页
2. 点击"生成PRD"按钮
3. AI将根据需求内容自动生成PRD文档
4. 生成的PRD会自动保存

### 3. 创建任务

1. 点击侧边栏"任务看板"
2. 点击"创建任务"按钮
3. 填写任务信息
4. 任务会出现在"待开发"列

### 4. 管理Agent

1. 点击侧边栏"Agent管理"
2. 系统会自动检测本地已安装的Agent
3. 点击"启动"按钮启动Agent
4. Agent状态会实时显示

## API Key获取

1. 访问 [Anthropic Console](https://console.anthropic.com/)
2. 注册/登录账号
3. 创建API Key
4. 复制Key并在应用中配置

**注意**: API Key会存储在本地SQLite数据库中，请妥善保管。

## 常见问题

### Q: 未检测到Agent怎么办？

A: 请确保已安装以下工具之一：
- [Claude Code](https://docs.anthropic.com/claude-code)
- [Codex CLI](https://github.com/openai/codex)

安装后确保命令行可以访问（添加到系统PATH）。

### Q: PRD生成失败怎么办？

A: 请检查：
1. API Key是否正确配置
2. 网络连接是否正常
3. API Key是否有足够的额度

### Q: 数据存储在哪里？

A: 所有数据存储在应用数据目录的SQLite数据库中：
- Windows: `%APPDATA%/ai-dev-platform-mvp/`
- macOS: `~/Library/Application Support/ai-dev-platform-mvp/`
- Linux: `~/.local/share/ai-dev-platform-mvp/`

## License

MIT
