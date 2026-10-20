# ACE — AI Collaboration Engine

[![CI](https://github.com/Love-L/ai-dev-platform-mvp/actions/workflows/ci.yml/badge.svg)](https://github.com/Love-L/ai-dev-platform-mvp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://www.rust-lang.org)
[![Local First](https://img.shields.io/badge/Data-100%25%20Local-green)](#设计理念)

> 把 AI 研发协作从"网页对话框"搬进真正的桌面工作台：
> **需求 → PRD → 任务 → Agent 执行 → 成本核算**，一个窗口闭环，数据一个字节都不上云。

📖 **当前实现状态与已知缺口**见 [`docs/STATUS.md`](docs/STATUS.md)（唯一事实来源）。

<p align="center">
  <img src="docs/assets/philosophy-loop.svg" alt="ACE 设计理念：需求 → PRD → 任务 → 执行 → 提交，全流程本地闭环" width="100%">
</p>

**上图是 ACE 的工作流全景**——五个阶段首尾相接，一个窗口走完：

- **一条链路** — 需求、PRD、任务、会话、提交互相关联：任何一步都能向上回溯、向下下钻，
  不再在十个标签页之间搬运上下文
- **一个反馈环** — 执行统计回流到 Skill 模板自进化，用得越久，模板越贴合你的项目
  （见「设计理念 §3」）
- **一条边界线** — 图中灰色边框就是你的机器：SQLite、keystore、备份、会话日志全部不出机；
  框外唯一的网络调用是 Claude API，且只发提示词、只经你自己的 API Key

---

## 设计理念

### 1. 一站式管理：研发全流程只在一个窗口里

用 AI 写代码的日常是割裂的：在浏览器里给 Claude 提需求、复制 PRD 到文档工具、再手动拆成任务卡片、
做完任务还要翻账单看烧了多少 Token——流程是断的，上下文在十个标签页之间反复横跳。

ACE 的回答是：**把研发资产收进一条可追溯的链路**。

```
项目 (projects)
  └── 需求 (requirements)  ← AI 生成 PRD、字段智能完善、变更历史全留痕
        └── 任务 (tasks)   ← 看板拖拽：待开发 → 开发中 → 已完成
              └── 会话 (agent_sessions)  ← 真实 Agent 在 PTY 终端里执行
                    └── 统计 (statistics)  ← Token 消耗、Skill 用量、成本分析
```

每一层都向下关联、向上汇总：点开一个任务能看到它的执行会话，点开一个会话能回溯它用了哪个 Skill、
花了多少 Token，点开统计能定位到是哪个项目、哪个环节在消耗预算。

### 2. 流程即流水线：6 个环节的标准化研发管线

ACE 预置了覆盖研发全周期的 6 个流水线环节（Stage），每个环节可以在项目维度绑定默认 Skill 模板，
实现"**按环节自动选用模板**"：

```
需求分析 → PRD 生成 → 任务规划 → 代码生成 → 测试 → 评审
requirement_analysis  prd_generation  task_planning  code_generation  testing  review
```

新建项目时不必从零配置——流水线配置随项目走，每个项目可以有自己的一套最佳实践。

### 3. Skill 自动升级演进：模板是活的，会跟着你的项目一起成长

这是 ACE 区别于"一次性提示词"的核心设计。Skill（PRD 生成模板）不是写死的文本，而是一个**有血缘、
有版本、有使用反馈的资产**，并且内置了一套**自动化的自进化引擎**（「Skill 自进化」页）：

**资产层——模板家族树：**

- **内置起步** — 系统预置多套 PRD 模板（功能需求、技术方案等），开箱即用
- **派生演进** — 一键「基于内置创建自定义 Skill」，新模板通过 `parent_id` 记录血缘，通过 `version`
  字段演进版本，形成一棵可回溯的"模板家族树"
- **反馈闭环** — 每次生成 PRD 都会写入 `skill_execution_logs`（哪个环节、成功与否）并在统计页汇总
  使用次数——哪个模板被高频使用、哪个环节生成效果不佳，数据一目了然

**引擎层——让模板自我诊断、自我评分、自动刹车：**

<p align="center">
  <img src="docs/assets/skill-evolution.svg" alt="Skill 自进化引擎：执行日志 → L1 规则诊断 → L2 优化草稿 → LLM-as-Judge → 棘轮判定 → 人在回路" width="100%">
</p>

五个阶段串成一条反馈流水线：**日志沉淀 → 规则诊断 → 优化草稿 → 独立评分 → 棘轮判定**。
中间任何产出都停在"草稿"态，绝不直接落库；只有你在「人在回路」环节显式点了应用或回滚，
变更才会生效并进入下一轮迭代。

> 模板越用越贴合你的项目风格，而不是每次都在和 AI 重新解释"我要什么样的 PRD"。
> 更重要的是：改坏了会被棘轮拦住——**进化有油门，也有刹车**。

### 4. Agent 真实执行，而非玩具级对话框

ACE 自动发现机器上已安装的 CLI Agent（Claude Code、Codex CLI），在内置 **PTY 伪终端** 中启动并
承载真实的交互式会话——Agent 在会话里跑的就是你本地的开发环境，产生的会话记录、Token 消耗、
提交归属全部持久化到本地数据库。

### 5. 成本可见：AI 超支前先收到通知

- 按**模型 / Agent / 项目**多维度统计 Token 消耗与成本
- 月度 **Token 预算**与告警阈值，接近上限提前预警
- 每次 PRD 生成、字段完善都精确记账到对应需求

### 6. Local-First：数据不出机

- 所有数据存储在本地 **SQLite（WAL 模式）**，启动时自动备份、保留最近 5 份
- API Key 以 **AES-256-GCM** 加密落盘，keystore 经 **Windows DPAPI** 绑定当前用户，缺失时拒绝明文降级
- 完全离线可用（仅 LLM API 调用需要网络）
- 无账号、无服务器、无遥测

---

## 架构总览

前端、IPC、命令、服务、存储五层职责单一；**所有出站依赖显式可审计**，除 Claude API（仅提示词）
与本地 Agent CLI / Git 外没有任何网络通信：

<p align="center">
  <img src="docs/assets/architecture.svg" alt="ACE 系统架构：UI 层 → Tauri IPC → 命令层 → 服务层 → 存储层，右侧为可审计的外部依赖" width="100%">
</p>

图中左侧自上而下是本机内的完整调用链：**React 页面**发起调用，经 **Tauri IPC 边界**
（101 个命令，错误统一带 `{code, message}`）进入 **Rust 命令层**，由**服务层**处理业务，
最终落到**本机存储**——双向箭头表示调用与结构化错误的往返。右侧蓝色总线是全部出站依赖：
每一条都由你在设置或项目配置中显式声明，没有隐藏的网络调用，可在执行日志中逐条审计。

## 功能总览

| 模块       | 能力                                                                                                                                         |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 项目管理   | 项目 CRUD、Skill/MCP 绑定、6 环节流水线配置                                                                                                  |
| 需求管理   | 优先级/状态流转、富文本描述（TipTap）、**AI 生成 PRD**、字段智能完善、变更历史审计                                                           |
| 任务管理   | 拖拽看板（待开发/开发中/已完成）、任务字段扩展、从任务一键创建会话                                                                           |
| Agent 管理 | PATH 自动发现 Claude Code / Codex、启停监控、检测结果持久化                                                                                  |
| 会话管理   | PTY 终端交互、双模式自动创建会话、执行日志                                                                                                   |
| Skill 管理 | 内置模板、派生自定义版本（血缘+版本树）、使用统计、**自进化引擎**（L1 规则诊断 / L2 优化草稿 / LLM-as-Judge 评分 / 棘轮回滚判定 / 振荡检测） |
| MCP 配置   | 按项目绑定 Model Context Protocol 端点                                                                                                       |
| 统计分析   | Token 多维统计、预算告警、Skill 使用排行、系统监控                                                                                           |
| Git 集成   | 内置 status / diff / commit / log 面板                                                                                                       |
| 基础设施   | 暗色主题、Toast 通知、删除保护策略、系统托盘                                                                                                 |

## 技术栈

| 层级     | 技术                                                                 |
| -------- | -------------------------------------------------------------------- |
| 桌面框架 | Tauri 2.0（安装包 ~10MB，对比 Electron 体量降 90%+）                 |
| 后端     | Rust + rusqlite（WAL）+ AES-256-GCM 密钥库                           |
| 前端     | React 19 + TypeScript + TailwindCSS + Zustand                        |
| 终端     | xterm.js + PTY 伪终端                                                |
| LLM      | Claude API（PRD 生成、字段完善、Token 记账）                         |
| 测试     | Vitest 四层金字塔（组件/契约/混沌/IPC）+ Playwright E2E + cargo test |

## 开发环境要求

- Node.js >= 18
- Rust >= 1.70
- npm

## 快速开始

### 1. 安装依赖

```bash
npm install
```

### 2. 启动开发服务器

```bash
# 启动 Tauri 开发模式
npm run tauri dev
```

### 3. 首次使用配置

1. 启动应用后，进入「系统设置」页面
2. 输入 Claude API Key（从 [Anthropic 控制台](https://console.anthropic.com/) 获取）
3. 保存设置

### 4. 推荐上手路径

1. 创建一个项目 → 在「流水线配置」里为 PRD 生成环节绑定一个 Skill
2. 录入一条需求 → 点击「生成 PRD」体验 Skill 模板驱动的 AI 生成
3. 在 Skill 管理页「基于内置创建自定义 Skill」，再到「Skill 自进化」页跑一次
   L1 诊断 + LLM 评分，生成优化草稿后显式应用——感受"进化有油门也有刹车"
4. 安装 [Claude Code](https://docs.anthropic.com/claude-code) 后回到 Agent 管理页，
   从任务一键发起真实会话
5. 在统计页回顾本月 Token 消耗，设置预算告警

## 项目结构

```
ai-dev-platform-mvp/
├── src-tauri/                 # Tauri Rust 后端
│   ├── src/
│   │   ├── main.rs            # 入口（命令注册、托盘、启动容错）
│   │   ├── commands/          # Tauri Commands（14 个业务域模块，全类型安全 IPC）
│   │   ├── db/                # 数据库操作（SQLite WAL、迁移框架 v9+、keystore）
│   │   ├── services/          # 业务服务（PRD、自进化引擎、会话驱动、Git、监控、备份）
│   │   ├── crypto.rs          # AES-256-GCM + DPAPI 密钥库
│   │   └── fatal.rs           # 启动致命错误对话框
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                       # React 前端
│   ├── pages/                 # 页面组件
│   ├── components/            # 通用组件
│   ├── lib/                   # 工具函数、类型定义、i18n
│   └── stores/                # Zustand 状态管理
├── tests/                     # 契约 / 混沌 / IPC / E2E
├── docs/                      # STATUS.md（唯一事实来源）+ 历史文档 + 配图
├── .github/workflows/         # CI + Release（v* 标签三平台构建）
└── package.json
```

## 测试

```bash
npm test              # 前端全量（L1 组件/Store + L3 契约 + L5 混沌 + L6 IPC）
npm run test:e2e      # Playwright 端到端
npm run test:rust     # Rust 后端单元测试
npm run verify        # CI 同款校验链
```

> Windows 下项目路径含中文时，使用 `npm run test:win`（内部会 subst 映射为 ASCII 盘符）。

## 常见问题

### Q: 未检测到 Agent 怎么办？

请确保已安装以下工具之一，并已加入系统 PATH：

- [Claude Code](https://docs.anthropic.com/claude-code)
- [Codex CLI](https://github.com/openai/codex)

### Q: PRD 生成失败怎么办？

1. 检查 API Key 是否正确配置
2. 检查网络连接
3. 确认 API Key 有足够额度

### Q: 数据存储在哪里？

所有数据存储在应用数据目录的 SQLite 数据库中（启动时自动备份到 `backups/`）：

- Windows: `%APPDATA%/ai-dev-platform-mvp/`
- macOS: `~/Library/Application Support/ai-dev-platform-mvp/`
- Linux: `~/.local/share/ai-dev-platform-mvp/`

## 路线图

- [x] ~~Skill 自动演进建议~~ — 已实现：L1 规则诊断 / L2 优化草稿 / LLM-as-Judge 评分 / 棘轮回滚 / 振荡检测
- [x] ~~自动更新~~ — 已接入 tauri-plugin-updater，`v*` 标签自动构建三平台 Release（含签名产物）
- [x] ~~Skill 模板导入 / 导出与分享~~ — 版本化 JSON 定义，跨机器复用（设置外的 Skill 页内「导入 / 导出」）
- [x] ~~i18n 国际化~~ — 渐进式框架（中文即源文案，缺译回退），侧边栏 / 设置已双语
- [ ] Agent 终端面板增强：多 Agent 并行管理（并发上限现已可配置）
- [ ] 统计报表导出
- [ ] 业务页面英文字典补全

## License

[MIT](LICENSE) © ACE - AI Collaboration Engine
