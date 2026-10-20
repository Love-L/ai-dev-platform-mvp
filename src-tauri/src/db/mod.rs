mod models;
mod requirements;
mod tasks;
mod agents;
mod tokens;
mod projects;
mod settings;
mod skills;
mod git;
mod backup;
pub mod session_messages;
pub mod migration;

#[cfg(test)]
mod integration_tests;

pub use models::*;
pub use agents::NewAgentSession;

use rusqlite::{Connection, Result, params};
use std::path::Path;
use tracing::{info, debug};

/// 需要加密存储的设置项（写入时自动加密，读取时自动解密）
const SENSITIVE_SETTING_KEYS: &[&str] = &["claude_api_key"];

pub struct Database {
    conn: Connection,
    /// 本地密钥：用于敏感设置项的静态加密。
    /// 不可用时**拒绝**敏感项的读写（写报错、读返回 None），绝不降级为明文。
    secret_key: Option<[u8; 32]>,
}

impl Database {
    /// 创建数据库连接并初始化表结构
    pub fn new(db_path: &Path) -> Result<Self> {
        debug!("创建数据库连接: {:?}", db_path);

        let conn = Connection::open(db_path)?;

        // 启用WAL模式提升并发性能
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch("PRAGMA busy_timeout=5000;")?;

        info!("数据库连接创建成功");

        // 读取/创建本地加密密钥（与数据库同目录）
        let secret_key = match db_path.parent() {
            Some(dir) => match crate::crypto::load_or_create_key(dir) {
                Ok(key) => Some(key),
                Err(e) => {
                    tracing::warn!("加载本地密钥失败，敏感设置项将拒绝读写（不降级明文）: {}", e);
                    None
                }
            },
            None => None,
        };

        let db = Self { conn, secret_key };
        db.init_tables()?;
        db.migrate_tables()?; // 运行数据库迁移，添加缺失的列
        db.create_supplementary_indexes()?; // 补充索引（依赖迁移后的列/表）
        db.migrate_unique_constraints()?; // 去重并建立唯一约束（依赖迁移后的列/表）
        db.seed_data()?;

        // 登记 schema 基线版本（运行时唯一的迁移入口为 init_tables + migrate_tables）
        migration::ensure_baseline(&db.conn)?;

        Ok(db)
    }

    /// 初始化种子数据
    fn seed_data(&self) -> Result<()> {
        debug!("检查种子数据...");

        // 初始化默认项目
        let project_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM projects",
            [],
            |row| row.get::<_, i64>(0),
        )? > 0;

        if !project_exists {
            self.conn.execute(
                "INSERT INTO projects (id, name, description) VALUES (?, ?, ?)",
                params![
                    "default-project",
                    "默认项目",
                    "ACE的默认项目"
                ],
            )?;
        }

        // 初始化默认 Token 预算
        let budget_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM token_budgets",
            [],
            |row| row.get::<_, i64>(0),
        )? > 0;

        if !budget_exists {
            let budget_id = uuid::Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO token_budgets (id, scope, monthly_limit, alert_threshold) VALUES (?, 'global', 2000000, 0.8)",
                params![budget_id],
            )?;
        }

        // 初始化内置 PRD Skill 模板
        let skill_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM prd_skills WHERE is_builtin = 1",
            [],
            |row| row.get::<_, i64>(0),
        )? > 0;

        if !skill_exists {
            self.init_builtin_skills()?;
        }

        info!("种子数据初始化完成");
        Ok(())
    }

    /// 初始化数据库表
    fn init_tables(&self) -> Result<()> {
        debug!("初始化数据库表...");

        self.conn.execute_batch("
            -- 项目表
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- 需求表
            CREATE TABLE IF NOT EXISTS requirements (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                priority TEXT CHECK(priority IN ('P0', 'P1', 'P2', 'P3')) DEFAULT 'P1',
                status TEXT CHECK(status IN ('draft', 'reviewing', 'approved', 'rejected', 'frozen', 'in_development', 'testing', 'completed')) DEFAULT 'draft',
                prd_content TEXT,
                requirement_type TEXT DEFAULT 'feature',
                target_version TEXT,
                due_date TEXT,
                tags TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects(id)
            );

            -- 任务表
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                requirement_id TEXT,
                title TEXT NOT NULL,
                description TEXT,
                task_type TEXT CHECK(task_type IN ('requirement', 'development', 'testing', 'bug_fix', 'review')) DEFAULT 'development',
                status TEXT CHECK(status IN ('pending', 'in_progress', 'completed', 'blocked')) DEFAULT 'pending',
                priority TEXT CHECK(priority IN ('P0', 'P1', 'P2', 'P3')) DEFAULT 'P1',
                agent_type TEXT,
                token_usage INTEGER DEFAULT 0,
                assignee TEXT,
                due_date TEXT,
                tags TEXT,
                estimated_tokens INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (requirement_id) REFERENCES requirements(id)
            );

            -- Agent配置表
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                binary_path TEXT NOT NULL,
                version TEXT,
                status TEXT CHECK(status IN ('available', 'running', 'unavailable')) DEFAULT 'available',
                capabilities TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Agent会话表
            CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                task_id TEXT,
                status TEXT CHECK(status IN ('running', 'completed', 'failed')) DEFAULT 'running',
                started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                ended_at DATETIME,
                exit_code INTEGER,
                FOREIGN KEY (agent_id) REFERENCES agents(id),
                FOREIGN KEY (task_id) REFERENCES tasks(id)
            );

            -- Token消耗记录表
            CREATE TABLE IF NOT EXISTS token_usage (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                task_id TEXT,
                agent_type TEXT,
                model TEXT,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                cache_creation_tokens INTEGER DEFAULT 0,
                cache_read_tokens INTEGER DEFAULT 0,
                thinking_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0,
                source TEXT DEFAULT 'api',
                recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id),
                FOREIGN KEY (task_id) REFERENCES tasks(id)
            );

            -- Token预算表
            CREATE TABLE IF NOT EXISTS token_budgets (
                id TEXT PRIMARY KEY,
                scope TEXT NOT NULL DEFAULT 'global',
                monthly_limit INTEGER NOT NULL DEFAULT 2000000,
                alert_threshold REAL NOT NULL DEFAULT 0.8,
                current_usage INTEGER DEFAULT 0,
                reset_date TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- PRD Skill模板表
            CREATE TABLE IF NOT EXISTS prd_skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                skill_type TEXT NOT NULL,
                template TEXT NOT NULL,
                is_builtin INTEGER DEFAULT 0,
                version INTEGER DEFAULT 1,
                parent_id TEXT,
                is_deleted INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- 需求变更历史表
            -- 需求变更历史（审计）：requirement_id 刻意不加外键，
            -- 以便需求被删除后审计记录仍然保留
            CREATE TABLE IF NOT EXISTS requirement_history (
                id TEXT PRIMARY KEY,
                requirement_id TEXT NOT NULL,
                action TEXT NOT NULL,
                old_value TEXT,
                new_value TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Git提交记录表
            CREATE TABLE IF NOT EXISTS git_commits (
                id TEXT PRIMARY KEY,
                task_id TEXT,
                session_id TEXT,
                commit_hash TEXT NOT NULL,
                branch_name TEXT,
                message TEXT,
                files_changed INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (task_id) REFERENCES tasks(id)
            );

            -- 系统配置表
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- 创建索引
            CREATE INDEX IF NOT EXISTS idx_requirements_project ON requirements(project_id);
            CREATE INDEX IF NOT EXISTS idx_requirements_status ON requirements(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_requirement ON tasks(requirement_id);
            CREATE INDEX IF NOT EXISTS idx_agent_sessions_agent ON agent_sessions(agent_id);
            CREATE INDEX IF NOT EXISTS idx_token_usage_session ON token_usage(session_id);
            CREATE INDEX IF NOT EXISTS idx_token_usage_recorded ON token_usage(recorded_at);
            CREATE INDEX IF NOT EXISTS idx_requirement_history_req ON requirement_history(requirement_id);
            CREATE INDEX IF NOT EXISTS idx_git_commits_task ON git_commits(task_id);

            -- 任务评论表
            CREATE TABLE IF NOT EXISTS task_comments (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                author TEXT NOT NULL DEFAULT 'user',
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_task_comments_task ON task_comments(task_id);
            CREATE INDEX IF NOT EXISTS idx_task_comments_created ON task_comments(created_at);

            -- 会话消息块表（模块 16 / F076）
            -- seq 为会话内单调递增序号，是回放顺序的唯一依据
            CREATE TABLE IF NOT EXISTS session_messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                role TEXT NOT NULL,
                block_type TEXT NOT NULL,
                content TEXT,
                tool_name TEXT,
                tool_use_id TEXT,
                parent_tool_use_id TEXT,
                cli_message_id TEXT,
                raw_json TEXT,
                tokens INTEGER,
                status TEXT DEFAULT 'completed',
                related_task_id TEXT,
                related_requirement_id TEXT,
                project_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
            );
        ")?;

        // 迁移：确保 prd_skills 表包含新字段
        self.migrate_prd_skills()?;

        // 迁移：确保 requirements 表包含新字段
        self.migrate_requirements()?;

        // 迁移：移除 requirement_history 的外键约束（审计需在需求删除后保留）
        self.migrate_requirement_history()?;

        // 说明：种子数据（默认项目 / 默认 Token 预算 / 内置 Skill 模板）统一由
        // `seed_data()` 负责，此处不再重复插入，避免同一份数据存在两套守卫与文案。

        info!("数据库表初始化完成");
        Ok(())
    }

    /// 初始化内置 PRD Skill 模板
    fn init_builtin_skills(&self) -> Result<()> {
        let skills = vec![
            (
                "skill-feature",
                "功能需求 PRD",
                "适用于新功能开发的需求文档生成",
                "feature",
                r#"# PRD: {title}

## 1. 需求背景与目标
{description}

## 2. 用户故事
- 作为 [用户角色]，我想 [功能]，以便 [价值]

## 3. 功能描述
### 3.1 功能范围
### 3.2 详细设计
### 3.3 交互流程

## 4. 非功能需求
- 性能要求
- 安全要求
- 兼容性要求

## 5. 验收标准
- [ ] 标准 1
- [ ] 标准 2

## 6. 影响范围分析

## 7. 风险评估

## 8. 里程碑计划
"#,
            ),
            (
                "skill-bugfix",
                "Bug修复 PRD",
                "适用于 Bug 修复的需求文档生成",
                "bugfix",
                r#"# Bug修复: {title}

## 1. Bug 描述
{description}

## 2. 复现步骤
1. 步骤 1
2. 步骤 2

## 3. 期望行为 vs 实际行为

## 4. 影响范围
- 受影响的模块
- 受影响的用户

## 5. 修复方案
### 5.1 根因分析
### 5.2 修复思路
### 5.3 代码变更预估

## 6. 回归测试用例

## 7. 风险评估
"#,
            ),
            (
                "skill-optimization",
                "技术优化 PRD",
                "适用于性能优化、重构等技术改进",
                "optimization",
                r#"# 技术优化: {title}

## 1. 优化背景
{description}

## 2. 现状分析
### 2.1 当前性能数据
### 2.2 瓶颈识别

## 3. 优化方案
### 3.1 方案描述
### 3.2 预期效果
### 3.3 实现步骤

## 4. 性能对比
- 优化前:
- 优化后:

## 5. 验收标准

## 6. 回滚方案

## 7. 风险评估
"#,
            ),
            (
                "skill-refactor",
                "架构重构 PRD",
                "适用于代码重构、架构调整",
                "refactor",
                r#"# 架构重构: {title}

## 1. 重构背景
{description}

## 2. 现状分析
### 2.1 当前架构
### 2.2 问题识别

## 3. 重构方案
### 3.1 目标架构
### 3.2 迁移路径
### 3.3 分阶段计划

## 4. 影响范围
- 受影响的模块
- API 变更
- 数据库变更

## 5. 验收标准

## 6. 回滚方案

## 7. 风险评估
"#,
            ),
        ];

        for (id, name, desc, skill_type, template) in skills {
            self.conn.execute(
                "INSERT INTO prd_skills (id, name, description, skill_type, template, is_builtin) VALUES (?, ?, ?, ?, ?, 1)",
                params![id, name, desc, skill_type, template],
            )?;
        }

        Ok(())
    }

    /// 迁移数据库表结构（添加缺失的列和表）
    fn migrate_tables(&self) -> Result<()> {
        debug!("检查数据库迁移...");

        // === requirements 表迁移 ===
        let req_columns: Vec<String> = self.conn.prepare("SELECT name FROM pragma_table_info('requirements')")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
                rows.collect::<std::result::Result<Vec<_>, _>>()
            })
            .unwrap_or_default();

        let req_missing: Vec<&str> = vec!["requirement_type", "target_version", "due_date", "tags"]
            .into_iter()
            .filter(|col| !req_columns.contains(&col.to_string()))
            .collect();

        if !req_missing.is_empty() {
            info!("添加 requirements 表缺失的列: {:?}", req_missing);
            for col in &req_missing {
                match *col {
                    "requirement_type" => { self.conn.execute("ALTER TABLE requirements ADD COLUMN requirement_type TEXT DEFAULT 'feature'", [])?; }
                    "target_version" => { self.conn.execute("ALTER TABLE requirements ADD COLUMN target_version TEXT", [])?; }
                    "due_date" => { self.conn.execute("ALTER TABLE requirements ADD COLUMN due_date TEXT", [])?; }
                    "tags" => { self.conn.execute("ALTER TABLE requirements ADD COLUMN tags TEXT", [])?; }
                    _ => {}
                }
            }
            info!("requirements 表迁移完成");
        }

        // === tasks 表迁移 ===
        let task_columns: Vec<String> = self.conn.prepare("SELECT name FROM pragma_table_info('tasks')")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
                rows.collect::<std::result::Result<Vec<_>, _>>()
            })
            .unwrap_or_default();

        let task_missing: Vec<&str> = vec!["assignee", "due_date", "tags", "estimated_tokens", "estimated_hours", "actual_hours", "blocked_reason", "related_requirement_id", "related_project_id", "priority_level", "notes", "attachments", "project_id", "skill_config", "mcp_config"]
            .into_iter()
            .filter(|col| !task_columns.contains(&col.to_string()))
            .collect();

        if !task_missing.is_empty() {
            info!("添加 tasks 表缺失的列: {:?}", task_missing);
            for col in &task_missing {
                match *col {
                    "assignee" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN assignee TEXT", [])?; }
                    "due_date" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN due_date TEXT", [])?; }
                    "tags" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN tags TEXT", [])?; }
                    "estimated_tokens" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN estimated_tokens INTEGER", [])?; }
                    "estimated_hours" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN estimated_hours REAL", [])?; }
                    "actual_hours" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN actual_hours REAL", [])?; }
                    "blocked_reason" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN blocked_reason TEXT", [])?; }
                    "related_requirement_id" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN related_requirement_id TEXT", [])?; }
                    "related_project_id" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN related_project_id TEXT", [])?; }
                    "priority_level" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN priority_level TEXT", [])?; }
                    "notes" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN notes TEXT", [])?; }
                    "attachments" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN attachments TEXT", [])?; }
                    "project_id" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN project_id TEXT", [])?; }
                    "skill_config" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN skill_config TEXT", [])?; }
                    "mcp_config" => { self.conn.execute("ALTER TABLE tasks ADD COLUMN mcp_config TEXT", [])?; }
                    _ => {}
                }
            }
            info!("tasks 表迁移完成");
        }

        // === token_usage 表迁移 ===
        let has_cache_column: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('token_usage') WHERE name = 'cache_creation_tokens'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !has_cache_column {
            info!("添加 token_usage 表缺失的列...");
            self.conn.execute_batch("
                ALTER TABLE token_usage ADD COLUMN cache_creation_tokens INTEGER DEFAULT 0;
                ALTER TABLE token_usage ADD COLUMN cache_read_tokens INTEGER DEFAULT 0;
                ALTER TABLE token_usage ADD COLUMN thinking_tokens INTEGER DEFAULT 0;
            ")?;
            info!("token_usage 表迁移完成");
        }

        // === agent_sessions 表迁移 ===
        let has_duration_column: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('agent_sessions') WHERE name = 'duration_seconds'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !has_duration_column {
            info!("添加 agent_sessions 表缺失的列...");
            self.conn.execute_batch("
                ALTER TABLE agent_sessions ADD COLUMN duration_seconds INTEGER;
                ALTER TABLE agent_sessions ADD COLUMN total_tokens INTEGER DEFAULT 0;
                ALTER TABLE agent_sessions ADD COLUMN total_cost REAL DEFAULT 0;
            ")?;
            info!("agent_sessions 表迁移完成");
        }

        // === 新增：项目 Skill/MCP 配置和执行日志表 ===

        // 检查 project_skills 表是否存在
        let has_project_skills: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_skills'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !has_project_skills {
            info!("创建 project_skills 和 project_mcps 表...");
            self.conn.execute_batch("
                CREATE TABLE IF NOT EXISTS project_skills (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    skill_id TEXT NOT NULL,
                    is_default INTEGER DEFAULT 1,
                    custom_params TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (project_id) REFERENCES projects(id),
                    FOREIGN KEY (skill_id) REFERENCES prd_skills(id)
                );

                CREATE TABLE IF NOT EXISTS project_mcps (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    mcp_name TEXT NOT NULL,
                    endpoint TEXT NOT NULL,
                    description TEXT,
                    is_default INTEGER DEFAULT 1,
                    auth_config TEXT,
                    custom_params TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (project_id) REFERENCES projects(id)
                );

                CREATE TABLE IF NOT EXISTS skill_execution_logs (
                    id TEXT PRIMARY KEY,
                    task_id TEXT,
                    session_id TEXT,
                    requirement_id TEXT,
                    skill_id TEXT NOT NULL,
                    skill_name TEXT NOT NULL,
                    skill_type TEXT,
                    execution_phase TEXT,
                    input_params TEXT,
                    output_summary TEXT,
                    token_usage INTEGER DEFAULT 0,
                    status TEXT DEFAULT 'success',
                    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (task_id) REFERENCES tasks(id),
                    FOREIGN KEY (session_id) REFERENCES agent_sessions(id),
                    FOREIGN KEY (requirement_id) REFERENCES requirements(id),
                    FOREIGN KEY (skill_id) REFERENCES prd_skills(id)
                );

                CREATE TABLE IF NOT EXISTS mcp_execution_logs (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    mcp_name TEXT NOT NULL,
                    endpoint TEXT,
                    action TEXT,
                    status TEXT DEFAULT 'connected',
                    error_message TEXT,
                    duration_ms INTEGER,
                    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (session_id) REFERENCES agent_sessions(id)
                );

                CREATE INDEX IF NOT EXISTS idx_project_skills_project ON project_skills(project_id);
                CREATE INDEX IF NOT EXISTS idx_project_mcps_project ON project_mcps(project_id);
                CREATE INDEX IF NOT EXISTS idx_skill_exec_task ON skill_execution_logs(task_id);
                CREATE INDEX IF NOT EXISTS idx_skill_exec_session ON skill_execution_logs(session_id);
                CREATE INDEX IF NOT EXISTS idx_mcp_exec_session ON mcp_execution_logs(session_id);
            ")?;
            info!("Skill/MCP 配置和执行日志表创建完成");
        }

        // === Skill 自进化：执行日志扩展列 + 评分/建议/版本快照表 ===
        {
            // 1) skill_execution_logs 可能尚未创建（has_project_skills 为真但日志表缺失的边缘场景）
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS skill_execution_logs (
                    id TEXT PRIMARY KEY,
                    task_id TEXT,
                    session_id TEXT,
                    requirement_id TEXT,
                    skill_id TEXT NOT NULL,
                    skill_name TEXT NOT NULL,
                    skill_type TEXT,
                    execution_phase TEXT,
                    input_params TEXT,
                    output_summary TEXT,
                    token_usage INTEGER DEFAULT 0,
                    status TEXT DEFAULT 'success',
                    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );",
            )?;

            // 2) 旧库补列（幂等）
            let has_duration: bool = self.conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('skill_execution_logs') WHERE name = 'duration_ms'",
                [],
                |row| row.get::<_, i64>(0),
            ).unwrap_or(0) > 0;
            if !has_duration {
                info!("添加 skill_execution_logs 表缺失的列（duration_ms / error_message）...");
                self.conn.execute_batch(
                    "ALTER TABLE skill_execution_logs ADD COLUMN duration_ms INTEGER;
                     ALTER TABLE skill_execution_logs ADD COLUMN error_message TEXT;",
                )?;
            }

            // 3) 自进化三张新表
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS skill_scores (
                    id TEXT PRIMARY KEY,
                    skill_id TEXT NOT NULL,
                    version INTEGER NOT NULL DEFAULT 1,
                    structure_score INTEGER NOT NULL,
                    effect_score INTEGER,
                    total_score INTEGER NOT NULL,
                    dimensions_json TEXT NOT NULL DEFAULT '[]',
                    judge_reason TEXT,
                    judged_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS skill_suggestions (
                    id TEXT PRIMARY KEY,
                    skill_id TEXT NOT NULL,
                    rule_type TEXT NOT NULL,
                    severity TEXT NOT NULL DEFAULT 'MEDIUM',
                    message TEXT NOT NULL,
                    dimension TEXT,
                    proposed_template TEXT,
                    status TEXT NOT NULL DEFAULT 'pending',
                    applied_version INTEGER,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS skill_versions (
                    id TEXT PRIMARY KEY,
                    skill_id TEXT NOT NULL,
                    version INTEGER NOT NULL,
                    template TEXT NOT NULL,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE INDEX IF NOT EXISTS idx_skill_scores_skill ON skill_scores(skill_id);
                CREATE INDEX IF NOT EXISTS idx_skill_suggestions_skill ON skill_suggestions(skill_id);
                CREATE INDEX IF NOT EXISTS idx_skill_versions_skill ON skill_versions(skill_id);",
            )?;
            info!("Skill 自进化表（skill_scores / skill_suggestions / skill_versions）就绪");
        }

        // === 新增：tasks 表扩展字段 ===
        let has_task_project_id: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'project_id'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !has_task_project_id {
            info!("添加 tasks 表扩展字段...");
            self.conn.execute_batch("
                ALTER TABLE tasks ADD COLUMN project_id TEXT;
                ALTER TABLE tasks ADD COLUMN skill_config TEXT;
                ALTER TABLE tasks ADD COLUMN mcp_config TEXT;
            ")?;
            info!("tasks 表扩展完成");
        }

        // === 新增：agent_sessions 表扩展字段 ===
        let has_session_project_id: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('agent_sessions') WHERE name = 'project_id'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !has_session_project_id {
            info!("添加 agent_sessions 表扩展字段...");
            self.conn.execute_batch("
                ALTER TABLE agent_sessions ADD COLUMN project_id TEXT;
                ALTER TABLE agent_sessions ADD COLUMN source TEXT DEFAULT 'native';
                ALTER TABLE agent_sessions ADD COLUMN source_path TEXT;
                ALTER TABLE agent_sessions ADD COLUMN title TEXT;
                ALTER TABLE agent_sessions ADD COLUMN description TEXT;
                ALTER TABLE agent_sessions ADD COLUMN related_task_id TEXT;
                ALTER TABLE agent_sessions ADD COLUMN related_requirement_id TEXT;
                ALTER TABLE agent_sessions ADD COLUMN skills TEXT;
                ALTER TABLE agent_sessions ADD COLUMN mcps TEXT;
                ALTER TABLE agent_sessions ADD COLUMN skill_execution_summary TEXT;
                ALTER TABLE agent_sessions ADD COLUMN mcp_execution_summary TEXT;
            ")?;
            info!("agent_sessions 表扩展完成");
        }

        // === 新增：projects 表扩展字段 ===
        let has_project_key: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'key'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !has_project_key {
            info!("添加 projects 表扩展字段...");
            self.conn.execute_batch("
                ALTER TABLE projects ADD COLUMN key TEXT;
                ALTER TABLE projects ADD COLUMN status TEXT DEFAULT 'active';
                ALTER TABLE projects ADD COLUMN owner_id TEXT;
                ALTER TABLE projects ADD COLUMN start_date TEXT;
                ALTER TABLE projects ADD COLUMN end_date TEXT;
            ")?;
            info!("projects 表扩展完成");
        }

        // === 新增：环节 Skill 配置表 ===
        let has_stage_skill_configs: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stage_skill_configs'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if !has_stage_skill_configs {
            info!("创建 stage_skill_configs 表...");
            self.conn.execute_batch("
                CREATE TABLE IF NOT EXISTS stage_skill_configs (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    stage TEXT NOT NULL,
                    skill_id TEXT NOT NULL,
                    is_default INTEGER DEFAULT 1,
                    custom_params TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (project_id) REFERENCES projects(id),
                    FOREIGN KEY (skill_id) REFERENCES prd_skills(id)
                );

                CREATE INDEX IF NOT EXISTS idx_stage_skill_project ON stage_skill_configs(project_id);
                CREATE INDEX IF NOT EXISTS idx_stage_skill_stage ON stage_skill_configs(stage);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_stage_skill_unique ON stage_skill_configs(project_id, stage);
            ")?;
            info!("stage_skill_configs 表创建完成");
        }

        info!("数据库迁移检查完成");
        Ok(())
    }

    // ==================== 需求操作 ====================

    pub fn create_requirement(&self, project_id: &str, title: &str, description: Option<&str>, priority: &str) -> Result<Requirement> {
        requirements::create_requirement(&self.conn, project_id, title, description, priority)
    }

    pub fn list_requirements(&self, status_filter: Option<&str>) -> Result<Vec<Requirement>> {
        requirements::list_requirements(&self.conn, status_filter)
    }

    pub fn list_requirements_paged(&self, page: u32, limit: u32, status_filter: Option<&str>) -> Result<crate::pagination::PagedResult<Requirement>> {
        requirements::list_requirements_paged(&self.conn, page, limit, status_filter)
    }

    pub fn get_requirement(&self, id: &str) -> Result<Option<Requirement>> {
        requirements::get_requirement(&self.conn, id)
    }

    pub fn update_requirement(&self, id: &str, title: Option<&str>, description: Option<&str>, priority: Option<&str>, status: Option<&str>, prd_content: Option<&str>) -> Result<()> {
        requirements::update_requirement(&self.conn, id, title, description, priority, status, prd_content)
    }

    pub fn update_requirement_fields(
        &self,
        id: &str,
        module: Option<&str>,
        source: Option<&str>,
        source_detail: Option<&str>,
        product_owner: Option<&str>,
        developer: Option<&str>,
        tester: Option<&str>,
        business_priority: Option<&str>,
        estimated_effort: Option<f64>,
        actual_effort: Option<f64>,
        attachments: Option<&str>,
        tags: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        requirements::update_requirement_fields(&self.conn, id, module, source, source_detail, product_owner, developer, tester, business_priority, estimated_effort, actual_effort, attachments, tags, notes)
    }

    pub fn delete_requirement(&self, id: &str) -> Result<()> {
        requirements::delete_requirement(&self.conn, id)
    }

    // ==================== 需求变更历史（审计） ====================

    /// 写入一条需求变更历史
    pub fn insert_requirement_history(
        &self,
        requirement_id: &str,
        action: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) -> Result<()> {
        requirements::insert_requirement_history(
            &self.conn,
            requirement_id,
            action,
            old_value,
            new_value,
        )
    }

    /// 查询指定需求的历史记录
    pub fn list_requirement_history(
        &self,
        requirement_id: &str,
        limit: i64,
    ) -> Result<Vec<requirements::RequirementHistoryRow>> {
        requirements::list_requirement_history(&self.conn, requirement_id, limit)
    }

    // ==================== 任务操作 ====================

    pub fn create_task(&self, title: &str, requirement_id: Option<&str>, task_type: &str, priority: &str, description: Option<&str>) -> Result<Task> {
        tasks::create_task(&self.conn, title, requirement_id, task_type, priority, description)
    }

    pub fn list_tasks(&self, status_filter: Option<&str>, type_filter: Option<&str>) -> Result<Vec<Task>> {
        tasks::list_tasks(&self.conn, status_filter, type_filter)
    }

    pub fn update_task_status(&self, id: &str, status: &str) -> Result<()> {
        tasks::update_task_status(&self.conn, id, status)
    }

    pub fn delete_task(&self, id: &str) -> Result<()> {
        tasks::delete_task(&self.conn, id)
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>> {
        tasks::get_task(&self.conn, id)
    }

    pub fn update_task(&self, id: &str, title: Option<&str>, description: Option<&str>, task_type: Option<&str>, priority: Option<&str>, assignee: Option<&str>, due_date: Option<&str>, tags: Option<&str>, project_id: Option<&str>) -> Result<()> {
        tasks::update_task(&self.conn, id, title, description, task_type, priority, assignee, due_date, tags, project_id)
    }

    pub fn update_task_fields(
        &self,
        id: &str,
        assignee: Option<&str>,
        due_date: Option<&str>,
        estimated_hours: Option<f64>,
        actual_hours: Option<f64>,
        priority_level: Option<&str>,
        blocked_reason: Option<&str>,
        related_requirement_id: Option<&str>,
        related_project_id: Option<&str>,
        tags: Option<&str>,
        notes: Option<&str>,
        attachments: Option<&str>,
    ) -> Result<()> {
        tasks::update_task_fields(&self.conn, id, assignee, due_date, estimated_hours, actual_hours, priority_level, blocked_reason, related_requirement_id, related_project_id, tags, notes, attachments)
    }

    pub fn list_tasks_by_project(&self, project_id: &str) -> Result<Vec<Task>> {
        tasks::list_tasks_by_project(&self.conn, project_id)
    }

    pub fn list_tasks_by_requirement(&self, requirement_id: &str) -> Result<Vec<Task>> {
        tasks::list_tasks_by_requirement(&self.conn, requirement_id)
    }

    // ==================== 任务评论操作 ====================

    pub fn create_task_comment(&self, task_id: &str, author: &str, content: &str) -> Result<TaskComment> {
        tasks::create_task_comment(&self.conn, task_id, author, content)
    }

    pub fn list_task_comments(&self, task_id: &str) -> Result<Vec<TaskComment>> {
        tasks::list_task_comments(&self.conn, task_id)
    }

    pub fn delete_task_comment(&self, id: &str) -> Result<()> {
        tasks::delete_task_comment(&self.conn, id)
    }

    // ==================== Agent操作 ====================

    /// 写入 Agent 并返回**实际生效的记录 id**（binary_path 命中已有记录时为该记录的 id）
    pub fn upsert_agent(&self, agent: &Agent) -> Result<String> {
        agents::upsert_agent(&self.conn, agent)
    }

    pub fn get_agents(&self) -> Result<Vec<Agent>> {
        agents::get_agents(&self.conn)
    }

    /// 更新 Agent 状态
    pub fn set_agent_status(&self, id: &str, status: &str) -> Result<()> {
        agents::set_agent_status(&self.conn, id, status)
    }

    // ==================== 会话消息块（模块 16 / F076） ====================

    /// 取会话内下一个消息序号
    pub fn next_session_message_seq(&self, session_id: &str) -> Result<i64> {
        session_messages::next_seq(&self.conn, session_id)
    }

    /// 追加一条完整消息块
    pub fn append_session_message(
        &self,
        session_id: &str,
        ctx: &session_messages::MessageContext,
        spec: session_messages::BlockSpec,
    ) -> Result<SessionMessage> {
        session_messages::append_block(&self.conn, session_id, ctx, spec)
    }

    /// 分页读取消息块（返回按 seq 升序）
    pub fn list_session_messages(
        &self,
        session_id: &str,
        before_seq: Option<i64>,
        limit: u32,
    ) -> Result<Vec<SessionMessage>> {
        session_messages::list(&self.conn, session_id, before_seq, limit)
    }

    /// 会话消息总数
    pub fn count_session_messages(&self, session_id: &str) -> Result<i64> {
        session_messages::count(&self.conn, session_id)
    }

    /// 把残留的 streaming 块收尾为 failed
    pub fn fail_streaming_session_messages(&self, session_id: &str) -> Result<usize> {
        session_messages::mark_streaming_as_failed(&self.conn, session_id)
    }

    /// 启动时收尾全部残留 streaming 块
    pub fn fail_all_streaming_session_messages(&self) -> Result<usize> {
        session_messages::mark_all_streaming_as_failed(&self.conn)
    }

    /// 按块类型统计
    pub fn session_message_stats(&self, session_id: &str) -> Result<Vec<(String, i64)>> {
        session_messages::count_by_block_type(&self.conn, session_id)
    }

    // ==================== 结构化会话元信息（模块 16） ====================

    /// 读取单个会话
    pub fn get_agent_session(&self, id: &str) -> Result<Option<AgentSession>> {
        agents::get_agent_session(&self.conn, id)
    }

    /// 创建会话（支持模式与上下文关联）
    pub fn create_agent_session_full(
        &self,
        spec: &agents::NewAgentSession,
    ) -> Result<AgentSession> {
        agents::create_agent_session_full(&self.conn, spec)
    }

    /// 回填 CLI 会话 id
    pub fn set_session_cli_id(&self, id: &str, cli_session_id: &str) -> Result<()> {
        agents::set_session_cli_id(&self.conn, id, cli_session_id)
    }

    /// 关联/取消关联会话与开发任务
    pub fn link_session_task(
        &self,
        session_id: &str,
        task_id: Option<&str>,
        requirement_id: Option<&str>,
    ) -> Result<()> {
        agents::link_session_task(&self.conn, session_id, task_id, requirement_id)
    }

    /// 写入会话已加载的 Skill 列表（JSON）
    pub fn set_session_skills(&self, session_id: &str, skills_json: &str) -> Result<()> {
        agents::set_session_skills(&self.conn, session_id, skills_json)
    }

    /// 复位会话为运行态（重建会话时使用）
    pub fn restart_agent_session(&self, id: &str) -> Result<()> {
        agents::restart_session(&self.conn, id)
    }

    /// 更新会话运行期元信息
    #[allow(clippy::too_many_arguments)]
    pub fn update_session_meta(
        &self,
        id: &str,
        mode: Option<&str>,
        permission_mode: Option<&str>,
        model: Option<&str>,
        view: Option<&str>,
        work_dir: Option<&str>,
        cli_session_id: Option<&str>,
        allowed_tools: Option<&str>,
    ) -> Result<()> {
        agents::update_session_meta(
            &self.conn,
            id,
            mode,
            permission_mode,
            model,
            view,
            work_dir,
            cli_session_id,
            allowed_tools,
        )
    }

    /// 刷新会话的最后事件时间
    pub fn touch_agent_session(&self, id: &str) -> Result<()> {
        agents::touch_session(&self.conn, id)
    }

    /// 累加会话用量
    pub fn add_session_usage(&self, id: &str, tokens: i64, cost: f64) -> Result<()> {
        agents::add_session_usage(&self.conn, id, tokens, cost)
    }

    /// 结构化会话的消息级记账（F048）。
    ///
    /// 复用 `record_token_usage_ext`：写入 `token_usage`（可按会话/任务归因），
    /// 并自动累计 Token 预算用量。`source = "session"` 与 API 调用记账区分。
    #[allow(clippy::too_many_arguments)]
    pub fn record_session_token_usage(
        &self,
        session_id: &str,
        task_id: Option<&str>,
        agent_type: &str,
        model: Option<&str>,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cost: f64,
    ) -> Result<TokenUsage> {
        tokens::record_token_usage_ext(
            &self.conn,
            Some(session_id),
            task_id,
            Some(agent_type),
            model,
            input_tokens,
            output_tokens,
            0,
            cache_read_tokens,
            0,
            cost,
            "session",
        )
    }

    pub fn create_agent_session(&self, agent_id: &str, task_id: Option<&str>) -> Result<AgentSession> {
        agents::create_agent_session(&self.conn, agent_id, task_id)
    }

    /// 会话结束时回写状态、退出码、结束时间与时长
    pub fn finish_agent_session(
        &self,
        id: &str,
        status: &str,
        exit_code: Option<i32>,
    ) -> Result<()> {
        agents::finish_agent_session(&self.conn, id, status, exit_code)
    }

    /// 启动时对账：把无真实进程的 `running` 会话标记为 failed
    pub fn reconcile_running_sessions(&self) -> Result<usize> {
        agents::reconcile_running_sessions(&self.conn)
    }

    pub fn get_agent_sessions(&self, agent_id: Option<&str>) -> Result<Vec<AgentSession>> {
        agents::get_agent_sessions(&self.conn, agent_id)
    }

    // ==================== Token追踪 ====================

    pub fn record_token_usage(&self, session_id: Option<&str>, task_id: Option<&str>, agent_type: Option<&str>, model: Option<&str>, input_tokens: i64, output_tokens: i64, total_cost: f64) -> Result<TokenUsage> {
        tokens::record_token_usage(&self.conn, session_id, task_id, agent_type, model, input_tokens, output_tokens, total_cost)
    }

    pub fn get_token_usage(&self, task_id: Option<&str>) -> Result<Vec<TokenUsage>> {
        tokens::get_token_usage(&self.conn, task_id)
    }

    /// 记录 Token 使用（完整字段，含缓存/思考 Token 与来源），并累计预算用量
    #[allow(clippy::too_many_arguments)]
    pub fn record_token_usage_ext(
        &self,
        session_id: Option<&str>,
        task_id: Option<&str>,
        agent_type: Option<&str>,
        model: Option<&str>,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_read_tokens: i64,
        thinking_tokens: i64,
        total_cost: f64,
        source: &str,
    ) -> Result<TokenUsage> {
        tokens::record_token_usage_ext(
            &self.conn,
            session_id,
            task_id,
            agent_type,
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            thinking_tokens,
            total_cost,
            source,
        )
    }

    /// 估算一次调用的成本（美元）
    pub fn estimate_cost(
        &self,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_read_tokens: i64,
        thinking_tokens: i64,
    ) -> f64 {
        tokens::estimate_cost(
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            thinking_tokens,
        )
    }

    /// 预算告警状态（用量比例、是否超阈值）
    pub fn get_budget_alert(&self) -> Result<Option<serde_json::Value>> {
        tokens::get_budget_alert(&self.conn)
    }

    // ==================== Token 预算 ====================

    pub fn get_token_budget(&self) -> Result<Option<TokenBudget>> {
        tokens::get_token_budget(&self.conn)
    }

    pub fn set_token_budget(&self, monthly_limit: i64, alert_threshold: f64) -> Result<()> {
        tokens::set_token_budget(&self.conn, monthly_limit, alert_threshold)
    }

    // ==================== Token 统计 ====================

    pub fn get_token_usage_summary(&self) -> Result<TokenUsageSummary> {
        tokens::get_token_usage_summary(&self.conn)
    }

    pub fn get_token_usage_trend(&self, days: i32) -> Result<Vec<DailyUsage>> {
        tokens::get_token_usage_trend(&self.conn, days)
    }

    pub fn get_token_usage_by_model(&self) -> Result<Vec<ModelUsage>> {
        tokens::get_token_usage_by_model(&self.conn)
    }

    // ==================== PRD Skill 操作 ====================

    /// 迁移 prd_skills 表，确保包含新字段
    fn migrate_prd_skills(&self) -> Result<()> {
        // 检查 version 字段是否存在
        let has_version: bool = self.conn.prepare(
            "SELECT COUNT(*) FROM pragma_table_info('prd_skills') WHERE name = 'version'"
        ).and_then(|mut stmt| {
            stmt.query_row([], |row| row.get::<_, i64>(0)).map(|count| count > 0)
        }).unwrap_or(false);

        if !has_version {
            info!("迁移 prd_skills 表: 添加 version, parent_id, is_deleted 字段");
            self.conn.execute_batch("ALTER TABLE prd_skills ADD COLUMN version INTEGER DEFAULT 1")?;
            self.conn.execute_batch("ALTER TABLE prd_skills ADD COLUMN parent_id TEXT")?;
            self.conn.execute_batch("ALTER TABLE prd_skills ADD COLUMN is_deleted INTEGER DEFAULT 0")?;
        }

        Ok(())
    }

    /// 迁移 requirements 表，确保包含新字段
    fn migrate_requirements(&self) -> Result<()> {
        // 检查 module 字段是否存在
        let has_module: bool = self.conn.prepare(
            "SELECT COUNT(*) FROM pragma_table_info('requirements') WHERE name = 'module'"
        ).and_then(|mut stmt| {
            stmt.query_row([], |row| row.get::<_, i64>(0)).map(|count| count > 0)
        }).unwrap_or(false);

        if !has_module {
            info!("迁移 requirements 表: 添加 module, source, source_detail 等字段");
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN module TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN source TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN source_detail TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN product_owner TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN developer TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN tester TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN business_priority TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN estimated_effort REAL")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN actual_effort REAL")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN attachments TEXT")?;
            self.conn.execute_batch("ALTER TABLE requirements ADD COLUMN notes TEXT")?;
        }

        // === agent_sessions 表迁移（模块 16：结构化会话） ===
        let session_columns: Vec<String> = self
            .conn
            .prepare("SELECT name FROM pragma_table_info('agent_sessions')")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
                rows.collect::<std::result::Result<Vec<_>, _>>()
            })
            .unwrap_or_default();

        let session_missing: Vec<&str> = vec![
            "cli_session_id",
            "work_dir",
            "agent_path",
            "mode",
            "view",
            "permission_mode",
            "model",
            "permission_allowlist",
            "last_event_at",
        ]
        .into_iter()
        .filter(|col| !session_columns.contains(&col.to_string()))
        .collect();

        if !session_missing.is_empty() {
            info!("添加 agent_sessions 表缺失的列: {:?}", session_missing);
            for col in &session_missing {
                let ddl = match *col {
                    "cli_session_id" => {
                        "ALTER TABLE agent_sessions ADD COLUMN cli_session_id TEXT"
                    }
                    "work_dir" => "ALTER TABLE agent_sessions ADD COLUMN work_dir TEXT",
                    "agent_path" => "ALTER TABLE agent_sessions ADD COLUMN agent_path TEXT",
                    "mode" => {
                        "ALTER TABLE agent_sessions ADD COLUMN mode TEXT DEFAULT 'structured'"
                    }
                    "view" => "ALTER TABLE agent_sessions ADD COLUMN view TEXT DEFAULT 'messages'",
                    "permission_mode" => {
                        "ALTER TABLE agent_sessions ADD COLUMN permission_mode TEXT"
                    }
                    "model" => "ALTER TABLE agent_sessions ADD COLUMN model TEXT",
                    "permission_allowlist" => {
                        "ALTER TABLE agent_sessions ADD COLUMN permission_allowlist TEXT"
                    }
                    "last_event_at" => {
                        "ALTER TABLE agent_sessions ADD COLUMN last_event_at DATETIME"
                    }
                    _ => continue,
                };
                self.conn.execute(ddl, [])?;
            }
            info!("agent_sessions 表迁移完成");
        }

        Ok(())
    }

    /// 补充索引。
    ///
    /// 这些索引原先只写在未被程序加载的迁移脚本中（该脚本及同目录下的 12 个
    /// `db/migrations/*.sql` 均已确认为死文件并移除），从未生效；
    /// 且部分索引列（如 `tasks.project_id`）由 `migrate_tables()` 通过
    /// `ALTER TABLE` 追加，因此必须在迁移之后再创建。
    fn create_supplementary_indexes(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);
             CREATE INDEX IF NOT EXISTS idx_agent_sessions_task ON agent_sessions(task_id);
             CREATE INDEX IF NOT EXISTS idx_agent_sessions_status ON agent_sessions(status);
             CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
             CREATE INDEX IF NOT EXISTS idx_git_commits_session ON git_commits(session_id);
             CREATE INDEX IF NOT EXISTS idx_requirement_history_created ON requirement_history(created_at);
             -- 会话消息块（模块 16 / F076）
             CREATE UNIQUE INDEX IF NOT EXISTS idx_session_messages_seq ON session_messages(session_id, seq);
             CREATE INDEX IF NOT EXISTS idx_session_messages_time ON session_messages(session_id, created_at);
             CREATE INDEX IF NOT EXISTS idx_session_messages_type ON session_messages(session_id, block_type);
             CREATE INDEX IF NOT EXISTS idx_session_messages_req ON session_messages(related_requirement_id);",
        )?;

        Ok(())
    }

    /// 迁移 requirement_history：移除 `requirement_id` 的外键约束。
    ///
    /// 审计记录必须在需求被删除后依然保留（"删除"这一动作本身也要留痕），
    /// 因此不能受 FK 约束限制——否则删除需求时会因历史记录引用而失败。
    /// SQLite 不支持直接删除外键，需要重建表；本方法幂等。
    fn migrate_requirement_history(&self) -> Result<()> {
        let has_fk: bool = self
            .conn
            .prepare("SELECT COUNT(*) FROM pragma_foreign_key_list('requirement_history')")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);

        if !has_fk {
            return Ok(());
        }

        info!("迁移 requirement_history 表：移除外键约束以保留删除后的审计记录");

        self.conn.execute_batch(
            "PRAGMA foreign_keys=OFF;
             BEGIN;
             ALTER TABLE requirement_history RENAME TO requirement_history_old;
             CREATE TABLE requirement_history (
                 id TEXT PRIMARY KEY,
                 requirement_id TEXT NOT NULL,
                 action TEXT NOT NULL,
                 old_value TEXT,
                 new_value TEXT,
                 created_at DATETIME DEFAULT CURRENT_TIMESTAMP
             );
             INSERT INTO requirement_history (id, requirement_id, action, old_value, new_value, created_at)
                 SELECT id, requirement_id, action, old_value, new_value, created_at
                 FROM requirement_history_old;
             DROP TABLE requirement_history_old;
             CREATE INDEX IF NOT EXISTS idx_requirement_history_req
                 ON requirement_history(requirement_id);
             COMMIT;
             PRAGMA foreign_keys=ON;",
        )?;

        Ok(())
    }

    /// 去重并建立唯一约束（幂等）。
    ///
    /// 历史版本缺少唯一约束，可能已经存在重复数据；这里先做一次安全去重，
    /// 再创建唯一索引。索引已存在时，前面的去重语句为无副作用的空操作。
    fn migrate_unique_constraints(&self) -> Result<()> {
        // 1. agents.binary_path：把重复 Agent 的会话迁移到保留的那一条，再删除重复
        let dup_agent_groups: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM (
                     SELECT binary_path FROM agents GROUP BY binary_path HAVING COUNT(*) > 1
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if dup_agent_groups > 0 {
            info!(
                "检测到 {} 组重复 Agent（binary_path），执行去重后再建唯一索引",
                dup_agent_groups
            );

            // 保留每组最早的一条，其余重复项：先迁移会话引用，再删除
            self.conn.execute_batch(
                "BEGIN;
                 UPDATE agent_sessions
                    SET agent_id = (
                        SELECT a.id FROM agents a
                         WHERE a.binary_path = (
                             SELECT b.binary_path FROM agents b WHERE b.id = agent_sessions.agent_id
                         )
                         ORDER BY a.created_at ASC, a.id ASC
                         LIMIT 1
                    )
                  WHERE agent_id IN (
                        SELECT a2.id FROM agents a2
                         WHERE a2.id <> (
                             SELECT a3.id FROM agents a3
                              WHERE a3.binary_path = a2.binary_path
                              ORDER BY a3.created_at ASC, a3.id ASC
                              LIMIT 1
                         )
                  );
                 DELETE FROM agents WHERE id NOT IN (
                     SELECT MIN(id) FROM agents GROUP BY binary_path
                 );
                 COMMIT;",
            )?;
        }

        // 2. project_skills(project_id, skill_id) 去重
        let removed_skills = self.conn.execute(
            "DELETE FROM project_skills WHERE rowid NOT IN (
                 SELECT MIN(rowid) FROM project_skills GROUP BY project_id, skill_id
             )",
            [],
        )?;
        if removed_skills > 0 {
            info!("清理了 {} 条重复的项目 Skill 绑定", removed_skills);
        }

        // 3. project_mcps：每个项目仅保留一个默认 MCP
        let cleared_defaults = self.conn.execute(
            "UPDATE project_mcps SET is_default = 0
              WHERE is_default = 1 AND rowid NOT IN (
                 SELECT MIN(rowid) FROM project_mcps WHERE is_default = 1 GROUP BY project_id
              )",
            [],
        )?;
        if cleared_defaults > 0 {
            info!("清理了 {} 条重复的默认 MCP 标记", cleared_defaults);
        }

        // 4. projects.key：重复的 Key 置空，仅保留最早一条
        let cleared_keys = self.conn.execute(
            "UPDATE projects SET key = NULL
              WHERE key IS NOT NULL AND rowid NOT IN (
                 SELECT MIN(rowid) FROM projects WHERE key IS NOT NULL GROUP BY key
              )",
            [],
        )?;
        if cleared_keys > 0 {
            info!("清理了 {} 个重复的项目 Key", cleared_keys);
        }

        // 5. 建立唯一索引（幂等）
        self.conn.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS uq_agents_binary_path ON agents(binary_path);
             CREATE UNIQUE INDEX IF NOT EXISTS uq_project_skills ON project_skills(project_id, skill_id);
             CREATE UNIQUE INDEX IF NOT EXISTS uq_project_mcps_default ON project_mcps(project_id) WHERE is_default = 1;
             CREATE UNIQUE INDEX IF NOT EXISTS uq_projects_key ON projects(key) WHERE key IS NOT NULL;",
        )?;

        Ok(())
    }

    pub fn list_prd_skills(&self) -> Result<Vec<PrdSkill>> {
        skills::list_prd_skills(&self.conn)
    }

    pub fn get_prd_skill(&self, id: &str) -> Result<Option<PrdSkill>> {
        skills::get_prd_skill(&self.conn, id)
    }

    pub fn create_prd_skill(&self, name: &str, skill_type: &str, description: Option<&str>, template: &str) -> Result<PrdSkill> {
        skills::create_prd_skill(&self.conn, name, skill_type, description, template)
    }

    pub fn update_prd_skill(&self, id: &str, name: Option<&str>, description: Option<&str>, template: Option<&str>) -> Result<()> {
        skills::update_prd_skill(&self.conn, id, name, description, template)
    }

    pub fn delete_prd_skill(&self, id: &str) -> Result<()> {
        skills::delete_prd_skill(&self.conn, id)
    }

    pub fn create_custom_skill_from_builtin(&self, parent_id: &str, name: &str, template: Option<&str>) -> Result<PrdSkill> {
        skills::create_custom_skill_from_builtin(&self.conn, parent_id, name, template)
    }

    // ==================== 设置操作 ====================

    /// 读取设置项。
    ///
    /// 敏感项（如 `claude_api_key`）若为密文则自动解密；若解密失败
    /// （例如数据库来自其它机器、密钥不匹配）返回 `None`，提示用户重新配置。
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let raw = settings::get_setting(&self.conn, key)?;
        Ok(raw.and_then(|value| self.unseal_setting(key, value)))
    }

    /// 写入设置项。敏感项**必须**加密落盘：
    /// - 本地 keystore 不可用或加密失败时返回错误，**拒绝明文写入**（禁止静默降级）；
    /// - 历史明文数据在下一次写入时自动完成封存迁移。
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        if SENSITIVE_SETTING_KEYS.contains(&key) && !crate::crypto::is_encrypted_secret(value) {
            let secret = self.secret_key.as_ref().ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    "本地加密密钥不可用，拒绝明文写入敏感设置项".to_string(),
                )
            })?;
            let sealed = crate::crypto::encrypt_secret(value, secret).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                    format!("敏感设置加密失败: {e}"),
                )))
            })?;
            return settings::set_setting(&self.conn, key, &sealed);
        }

        settings::set_setting(&self.conn, key, value)
    }

    /// 敏感设置项解密。
    ///
    /// 返回 `None` 表示密文无法解开（如数据库从其它机器导入、keystore 丢失），
    /// 调用方应提示用户重新配置，而不是把密文当明文使用。
    fn unseal_setting(&self, key: &str, value: String) -> Option<String> {
        if !SENSITIVE_SETTING_KEYS.contains(&key) || !crate::crypto::is_encrypted_secret(&value) {
            // 历史明文（或历史"前端固定口令加密层"）兼容路径
            return Some(self.migrate_legacy_secret(key, value));
        }

        let Some(ref secret) = self.secret_key else {
            tracing::warn!("设置项 {} 无法解密：本地加密密钥不可用", key);
            return None;
        };

        match crate::crypto::decrypt_secret(&value, secret) {
            Some(plain) => Some(self.migrate_legacy_secret(key, plain)),
            None => {
                tracing::warn!("设置项 {} 解密失败（密钥不匹配），请重新配置", key);
                None
            }
        }
    }

    /// 历史数据透明迁移：
    ///
    /// 旧版前端用固定口令做了 PBKDF2+AES-GCM 加密（base64(salt+nonce+ct) 形态），
    /// 后端拿到它无法直接当 API Key 使用。此处识别该形态 → 用历史口令解开 →
    /// 立即以当前 keystore 重新封存 → 返回明文。解不开则按明文原样返回
    /// （随机明文恰好撞上 base64 形态时的兜底，避免误伤有效 Key）。
    fn migrate_legacy_secret(&self, key: &str, value: String) -> String {
        if !crate::crypto::is_encrypted_format(&value) {
            return value;
        }
        match crate::crypto::decrypt_api_key(&value, crate::crypto::LEGACY_SETTING_PASSWORD) {
            Ok(plain) => {
                info!("设置项 {} 历史加密层已解封，正在以本地 keystore 重新封存", key);
                if let Some(ref secret) = self.secret_key {
                    if let Ok(sealed) = crate::crypto::encrypt_secret(&plain, secret) {
                        let _ = settings::set_setting(&self.conn, key, &sealed);
                    }
                }
                plain
            }
            Err(_) => value,
        }
    }

    // ==================== 项目 Skill 配置 ====================

    pub fn list_project_skills(&self, project_id: &str) -> Result<Vec<ProjectSkill>> {
        projects::list_project_skills(&self.conn, project_id)
    }

    pub fn add_project_skill(&self, project_id: &str, skill_id: &str, is_default: bool, custom_params: Option<&str>) -> Result<ProjectSkill> {
        projects::add_project_skill(&self.conn, project_id, skill_id, is_default, custom_params)
    }

    pub fn remove_project_skill(&self, id: &str) -> Result<()> {
        projects::remove_project_skill(&self.conn, id)
    }

    // ==================== 项目 MCP 配置 ====================

    pub fn list_project_mcps(&self, project_id: &str) -> Result<Vec<ProjectMcp>> {
        projects::list_project_mcps(&self.conn, project_id)
    }

    pub fn add_project_mcp(&self, project_id: &str, mcp_name: &str, endpoint: &str, description: Option<&str>, is_default: bool, auth_config: Option<&str>, custom_params: Option<&str>) -> Result<ProjectMcp> {
        projects::add_project_mcp(&self.conn, project_id, mcp_name, endpoint, description, is_default, auth_config, custom_params)
    }

    pub fn remove_project_mcp(&self, id: &str) -> Result<()> {
        projects::remove_project_mcp(&self.conn, id)
    }

    // ==================== 环节 Skill 配置 ====================

    pub fn list_stage_skill_configs(&self, project_id: &str) -> Result<Vec<models::StageSkillConfig>> {
        projects::list_stage_skill_configs(&self.conn, project_id)
    }

    pub fn get_stage_skill_config(&self, project_id: &str, stage: &str) -> Result<Option<models::StageSkillConfig>> {
        projects::get_stage_skill_config(&self.conn, project_id, stage)
    }

    pub fn upsert_stage_skill_config(&self, project_id: &str, stage: &str, skill_id: &str, is_default: bool, custom_params: Option<&str>) -> Result<models::StageSkillConfig> {
        projects::upsert_stage_skill_config(&self.conn, project_id, stage, skill_id, is_default, custom_params)
    }

    pub fn remove_stage_skill_config(&self, id: &str) -> Result<()> {
        projects::remove_stage_skill_config(&self.conn, id)
    }

    pub fn get_stage_skill_for_requirement(&self, project_id: &str, stage: &str) -> Result<Option<String>> {
        projects::get_stage_skill_for_requirement(&self.conn, project_id, stage)
    }

    // ==================== Skill 执行日志 ====================

    #[allow(clippy::too_many_arguments)]
    pub fn record_skill_execution_log(
        &self,
        task_id: Option<&str>,
        session_id: Option<&str>,
        requirement_id: Option<&str>,
        skill_id: &str,
        skill_name: &str,
        skill_type: &str,
        execution_phase: &str,
        input_params: Option<&str>,
        output_summary: Option<&str>,
        token_usage: i64,
        status: &str,
        duration_ms: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<()> {
        skills::record_skill_execution_log(
            &self.conn, task_id, session_id, requirement_id, skill_id, skill_name, skill_type, execution_phase, input_params, output_summary, token_usage, status, duration_ms, error_message,
        )
    }

    pub fn get_skill_execution_logs_by_skill(&self, skill_id: &str, limit: i64) -> Result<Vec<SkillExecutionLog>> {
        skills::get_skill_execution_logs_by_skill(&self.conn, skill_id, limit)
    }

    pub fn get_skill_log_aggregate(&self, skill_id: &str) -> Result<serde_json::Value> {
        skills::get_skill_log_aggregate(&self.conn, skill_id)
    }

    // ==================== Skill 自进化：评分 / 建议 / 版本 ====================

    pub fn insert_skill_score(
        &self,
        skill_id: &str,
        version: i32,
        structure_score: i32,
        effect_score: Option<i32>,
        total_score: i32,
        dimensions_json: &str,
        judge_reason: Option<&str>,
    ) -> Result<SkillScore> {
        skills::insert_skill_score(&self.conn, skill_id, version, structure_score, effect_score, total_score, dimensions_json, judge_reason)
    }

    pub fn get_skill_score_history(&self, skill_id: &str) -> Result<Vec<SkillScore>> {
        skills::get_skill_score_history(&self.conn, skill_id)
    }

    pub fn get_latest_skill_score(&self, skill_id: &str) -> Result<Option<SkillScore>> {
        skills::get_latest_skill_score(&self.conn, skill_id)
    }

    pub fn upsert_skill_suggestion(
        &self,
        skill_id: &str,
        rule_type: &str,
        severity: &str,
        message: &str,
        dimension: Option<&str>,
        proposed_template: Option<&str>,
    ) -> Result<String> {
        skills::upsert_skill_suggestion(&self.conn, skill_id, rule_type, severity, message, dimension, proposed_template)
    }

    pub fn list_skill_suggestions(&self, skill_id: Option<&str>) -> Result<Vec<SkillSuggestion>> {
        skills::list_skill_suggestions(&self.conn, skill_id)
    }

    pub fn get_skill_suggestion(&self, id: &str) -> Result<Option<SkillSuggestion>> {
        skills::get_skill_suggestion(&self.conn, id)
    }

    pub fn update_suggestion_status(&self, id: &str, status: &str, applied_version: Option<i32>) -> Result<()> {
        skills::update_suggestion_status(&self.conn, id, status, applied_version)
    }

    pub fn dismiss_pending_suggestions(&self, skill_id: &str, rule_type: &str) -> Result<()> {
        skills::dismiss_pending_suggestions(&self.conn, skill_id, rule_type)
    }

    pub fn insert_skill_version(&self, skill_id: &str, version: i32, template: &str, note: Option<&str>) -> Result<String> {
        skills::insert_skill_version(&self.conn, skill_id, version, template, note)
    }

    pub fn list_skill_versions(&self, skill_id: &str) -> Result<Vec<SkillVersion>> {
        skills::list_skill_versions(&self.conn, skill_id)
    }

    pub fn get_skill_version(&self, skill_id: &str, version: i32) -> Result<Option<SkillVersion>> {
        skills::get_skill_version(&self.conn, skill_id, version)
    }

    pub fn get_skill_execution_logs_by_task(&self, task_id: &str) -> Result<Vec<SkillExecutionLog>> {
        skills::get_skill_execution_logs_by_task(&self.conn, task_id)
    }

    pub fn get_skill_execution_logs_by_session(&self, session_id: &str) -> Result<Vec<SkillExecutionLog>> {
        skills::get_skill_execution_logs_by_session(&self.conn, session_id)
    }

    // ==================== MCP 执行日志 ====================

    pub fn get_mcp_execution_logs(&self, session_id: &str) -> Result<Vec<McpExecutionLog>> {
        skills::get_mcp_execution_logs(&self.conn, session_id)
    }

    /// 写入 MCP 执行日志（会话启动时按项目绑定的 MCP 记录连接事件）
    #[allow(clippy::too_many_arguments)]
    pub fn insert_mcp_execution_log(
        &self,
        session_id: &str,
        mcp_name: &str,
        endpoint: Option<&str>,
        action: Option<&str>,
        status: &str,
        error_message: Option<&str>,
        duration_ms: Option<i64>,
    ) -> Result<String> {
        skills::insert_mcp_execution_log(
            &self.conn,
            session_id,
            mcp_name,
            endpoint,
            action,
            status,
            error_message,
            duration_ms,
        )
    }

    // ==================== Skill/MCP 使用统计 ====================

    pub fn get_skill_usage_stats(&self) -> Result<Vec<serde_json::Value>> {
        skills::get_skill_usage_stats(&self.conn)
    }

    pub fn get_mcp_usage_stats(&self) -> Result<Vec<serde_json::Value>> {
        skills::get_mcp_usage_stats(&self.conn)
    }

    // ==================== 项目操作 ====================

    pub fn create_project(&self, name: &str, description: Option<&str>, key: Option<&str>) -> Result<Project> {
        projects::create_project(&self.conn, name, description, key)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        projects::list_projects(&self.conn)
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        projects::get_project(&self.conn, id)
    }

    pub fn update_project(&self, id: &str, name: Option<&str>, description: Option<&str>, key: Option<&str>, status: Option<&str>) -> Result<()> {
        projects::update_project(&self.conn, id, name, description, key, status)
    }

    pub fn delete_project(&self, id: &str) -> Result<()> {
        projects::delete_project(&self.conn, id)
    }

    // ==================== 统计聚合 ====================

    /// 获取任务统计 (total, completed, in_progress, pending, development, testing, bug_fix)
    pub fn get_task_stats(&self) -> Result<(i64, i64, i64, i64, i64, i64, i64)> {
        let mut stmt = self.conn.prepare(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
                SUM(CASE WHEN status = 'in_progress' THEN 1 ELSE 0 END) as in_progress,
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN task_type = 'development' THEN 1 ELSE 0 END) as development,
                SUM(CASE WHEN task_type = 'testing' THEN 1 ELSE 0 END) as testing,
                SUM(CASE WHEN task_type = 'bug_fix' THEN 1 ELSE 0 END) as bug_fix
            FROM tasks"
        )?;
        stmt.query_row([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
    }

    /// 获取需求总数
    pub fn get_requirement_count(&self) -> Result<i64> {
        self.conn.query_row("SELECT COUNT(*) FROM requirements", [], |row| row.get(0))
    }

    /// 获取项目下的需求数量
    pub fn get_requirement_count_by_project(&self, project_id: &str) -> Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM requirements WHERE project_id = ?",
            params![project_id],
            |row| row.get(0),
        )
    }

    /// 获取项目下的任务数量
    pub fn get_task_count_by_project(&self, project_id: &str) -> Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?",
            params![project_id],
            |row| row.get(0),
        )
    }

    /// 获取需求下的任务数量
    pub fn get_task_count_by_requirement(&self, requirement_id: &str) -> Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE requirement_id = ?",
            params![requirement_id],
            |row| row.get(0),
        )
    }

    /// 获取审计日志（需求变更历史）
    ///
    /// `entity_type`：可选实体类型过滤。当前仅支持 `requirement`；
    /// 传入其它值时返回空列表（避免前端误以为有数据）。
    pub fn list_audit_logs(
        &self,
        limit: Option<i64>,
        entity_type: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        if let Some(t) = entity_type {
            if t != "requirement" {
                return Ok(Vec::new());
            }
        }

        let limit = limit.unwrap_or(200);
        let mut stmt = self.conn.prepare(
            "SELECT h.id, h.requirement_id, h.action, h.old_value, h.new_value, h.created_at,
                    COALESCE(r.title, '') as requirement_title
             FROM requirement_history h
             LEFT JOIN requirements r ON h.requirement_id = r.id
             ORDER BY h.created_at DESC
             LIMIT ?"
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let action: String = row.get(2)?;
            let old_value: Option<String> = row.get(3)?;
            let new_value: Option<String> = row.get(4)?;
            let joined_title: String = row.get(6)?;

            // 需求被删除后 LEFT JOIN 取不到标题，此时回退到删除时快照的 old_value
            let entity_title = if !joined_title.is_empty() {
                joined_title
            } else if action == "delete" {
                old_value.clone().unwrap_or_default()
            } else {
                String::new()
            };

            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "entity_type": "requirement",
                "entity_id": row.get::<_, String>(1)?,
                "action": action,
                "old_value": old_value,
                "new_value": new_value,
                "changes": format!(
                    "{} → {}",
                    old_value.clone().unwrap_or_default(),
                    new_value.clone().unwrap_or_default()
                ),
                "created_at": row.get::<_, String>(5)?,
                "entity_title": entity_title
            }))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    // ==================== Git 提交记录 ====================

    /// 写入 Git 提交记录，返回记录 id
    pub fn insert_git_commit(
        &self,
        task_id: Option<&str>,
        session_id: Option<&str>,
        commit_hash: &str,
        branch_name: Option<&str>,
        message: Option<&str>,
        files_changed: i64,
    ) -> Result<String> {
        git::insert_git_commit(
            &self.conn,
            task_id,
            session_id,
            commit_hash,
            branch_name,
            message,
            files_changed,
        )
    }

    /// 查询 Git 提交记录
    pub fn list_git_commits(
        &self,
        task_id: Option<&str>,
        session_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<serde_json::Value>> {
        git::list_git_commits(&self.conn, task_id, session_id, limit.unwrap_or(100))
    }

    // ==================== 数据导入导出 ====================

    /// 导出全量数据快照
    pub fn export_snapshot(&self) -> Result<serde_json::Value> {
        backup::export_snapshot(&self.conn)
    }

    /// 导入数据快照（事务 + 外键保护）
    pub fn import_snapshot(
        &self,
        payload: &serde_json::Value,
        replace: bool,
    ) -> Result<serde_json::Value> {
        backup::import_snapshot(&self.conn, payload, replace)
    }

    /// 统计各表行数
    pub fn table_counts(&self) -> Result<serde_json::Value> {
        backup::table_counts(&self.conn)
    }

    /// 备份数据库文件（含 WAL 检查点与轮转），返回备份路径
    pub fn backup_to(&self, db_path: &std::path::Path, keep: usize) -> Result<std::path::PathBuf> {
        backup::backup_database_file(&self.conn, db_path, keep)
    }

    /// 列出已有备份文件
    pub fn list_backups(&self, dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        backup::list_backups(dir)
    }
}
