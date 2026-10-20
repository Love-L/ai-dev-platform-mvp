# 快速启动指南

## 前置条件

确保已安装以下工具：

### 1. Node.js (>= 18)

```bash
node --version
npm --version
```

### 2. Rust (>= 1.70)

```bash
rustc --version
cargo --version
```

如果未安装，请访问 https://rustup.rs/ 安装。

### 3. Tauri CLI

```bash
cargo install tauri-cli
```

## 启动步骤

### 步骤 1: 进入项目目录

```bash
cd "E:/orca/AI自动化开发流程/单机版/ai-dev-platform-mvp"
```

### 步骤 2: 安装前端依赖

```bash
npm install
```

### 步骤 3: 启动开发模式

```bash
npm run tauri dev
```

首次启动会：

1. 编译Rust后端（可能需要几分钟）
2. 启动Vite开发服务器
3. 打开桌面应用窗口

### 步骤 4: 配置API Key

1. 在应用中点击左侧导航"系统设置"
2. 在"Claude API配置"部分输入你的API Key
3. 点击"保存设置"

## 构建发布版本

### Windows

```bash
npm run tauri build
```

生成的安装包位于：`src-tauri/target/release/bundle/`

## 常见问题

### Q: 编译Rust时出错

确保已安装Visual Studio Build Tools（Windows）或对应的C++编译器。

### Q: 端口被占用

如果5173端口被占用，修改 `vite.config.ts` 中的端口号：

```typescript
server: {
  port: 5174,  // 改为其他端口
  strictPort: true,
},
```

### Q: 数据库错误

删除应用数据目录中的 `database.db` 文件，重启应用会自动重建。

## 项目文件说明

```
ai-dev-platform-mvp/
├── src-tauri/           # Rust后端代码
│   └── src/
│       ├── main.rs     # 应用入口
│       ├── db.rs       # 数据库操作
│       ├── llm.rs      # Claude API集成
│       ├── agent.rs    # Agent检测
│       └── commands.rs # Tauri命令接口
├── src/                # React前端代码
│   ├── pages/          # 页面组件
│   │   ├── Dashboard.tsx      # 仪表盘
│   │   ├── Requirements.tsx   # 需求列表
│   │   ├── RequirementDetail.tsx # 需求详情+PRD
│   │   ├── Board.tsx          # 任务看板
│   │   ├── Agents.tsx         # Agent管理
│   │   └── Settings.tsx       # 系统设置
│   ├── components/     # 通用组件
│   ├── lib/            # 工具库
│   │   ├── tauri.ts    # Tauri IPC封装
│   │   └── types.ts    # TypeScript类型
│   └── App.tsx         # 应用入口
└── package.json        # 项目配置
```
