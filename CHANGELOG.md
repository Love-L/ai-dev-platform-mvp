# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/) 格式。

## [2.3.1] - 2026-09-23

### Added

- **内置依赖工具环境**: 安装包自动携带企业微信官方 CLI `wecom-cli`（原生 win32-x64，免 Node 运行时），位于安装目录 `tools/`
- **安装时自动注册 PATH**: NSIS 安装钩子（`hooks.nsh`）将 `tools/` 写入当前用户 PATH 并广播环境变更，卸载时自动移除
- **打包脚本**: `scripts/bundle-tools.ps1`（`npm run bundle:tools`）幂等拉取并冒烟验证工具，已挂入 `beforeBuildCommand`
- **版本号**: 统一升级至 2.3.1

## [2.3.0] - 2026-09-20

### Added

- **Agent数据持久化**: Agent检测后自动保存到数据库，下次启动直接加载
- **系统资源监控修复**: 状态栏现在显示应用内存(RSS)而非系统内存
- **Skill管理CRUD**: 完整的Skill创建、编辑、删除功能
- **Skill版本管理**: 支持基于内置Skill创建自定义版本
- **会话双模式自动创建**: 自动模式下任务状态变更时自动创建会话
- **分页系统**: 需求列表支持分页查询
- **删除保护策略**: 删除项目/需求前检查关联数据
- **Toast通知系统**: 操作成功/失败显示通知提示
- **.gitignore**: 添加标准忽略文件配置
- **CLAUDE.md**: 添加AI助手项目上下文文档
- **CHANGELOG.md**: 添加变更日志

### Fixed

- **Agent检测后数据丢失**: 修复每次进入Agent页面需重新刷新的问题
- **状态栏内存显示错误**: 从系统总内存改为应用RSS内存
- **Skill管理无法编辑/删除**: 实现完整的CRUD功能
- **CMD弹窗问题**: Windows下进程创建不再显示控制台窗口

### Changed

- **统一版本号**: 所有版本号统一为2.3.0
- **PrdSkill结构体**: 新增version、parent_id、is_deleted字段

## [2.2.0] - 2026-09-19

### Added

- 系统监控页面
- 暗色主题支持
- 需求字段扩展（11个新字段）
- 任务字段扩展（8个新字段）

### Fixed

- 桌面快捷方式创建问题
- 窗口关闭拦截

## [2.1.0] - 2026-09-18

### Added

- 初始版本发布
- 项目管理
- 需求管理
- 任务看板
- Agent管理
- 会话管理
- Skill管理
- Token消耗统计
