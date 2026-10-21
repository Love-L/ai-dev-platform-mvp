use crate::db::{Agent, AgentSession, Database};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, debug};

/// Agent 服务 - 处理 Agent 发现、启动、停止等业务逻辑
pub struct AgentService<'a> {
    db: Option<&'a Arc<Mutex<Database>>>,
}

impl<'a> AgentService<'a> {
    /// 创建 Agent 服务（需要数据库）
    pub fn new(db: &'a Arc<Mutex<Database>>) -> Self {
        Self { db: Some(db) }
    }

    /// 发现系统上所有可用的 Agent
    pub async fn discover_agents(&self) -> Result<Vec<Agent>, String> {
        debug!("发现系统Agent...");
        let detected = crate::agent::discover_agents().await;

        let mut agents: Vec<Agent> = detected
            .into_iter()
            .map(|d| Agent {
                id: uuid::Uuid::new_v4().to_string(),
                name: d.name,
                display_name: d.display_name,
                binary_path: d.path,
                version: d.version,
                status: "available".to_string(),
                capabilities: serde_json::to_string(&d.capabilities).ok(),
                created_at: chrono::Utc::now().to_rfc3339(),
            })
            .collect();

        // 持久化到数据库，并用"实际生效 id"回填：
        // binary_path 命中已有记录时执行的是 UPDATE，库里保留的是原 id，
        // 若不同步回填，后续用该 id 建会话会触发外键失败。
        if let Some(db_ref) = self.db {
            let db = db_ref.lock().await;
            for agent in agents.iter_mut() {
                match db.upsert_agent(agent) {
                    Ok(id) => agent.id = id,
                    Err(e) => error!("保存Agent失败: {}", e),
                }
            }
        }

        info!("发现 {} 个Agent", agents.len());
        Ok(agents)
    }

    /// 启动 Agent 会话
    pub async fn start_agent(
        &self,
        agent_path: &str,
        task_id: Option<&str>,
    ) -> Result<AgentSession, String> {
        let db_ref = self.db.ok_or("数据库未初始化")?;
        info!("启动Agent: path={}", agent_path);

        if !crate::agent::check_agent_available(agent_path).await {
            error!("Agent不可用: {}", agent_path);
            return Err("Agent不可用".to_string());
        }

        // 1. 解析目标 Agent 记录（复用已注册记录，否则补登记）
        let agent_id = self.resolve_agent_id(agent_path).await?;

        // 2. 创建会话（agent_id 必定存在于 agents 表）
        let session = {
            let db = db_ref.lock().await;
            db.create_agent_session(&agent_id, task_id)
                .map_err(|e| {
                    error!("创建会话失败: {}", e);
                    format!("创建会话失败: {}", e)
                })?
        };

        // 3. 不再把 agents.status 置为 running：
        //    运行状态由 agent_sessions（是否存在 running 会话）唯一表达，
        //    agents.status 只表示注册可用性（available/unavailable）。
        //    此前写入 running 且无人回收，导致 Agent 永久卡在 running（前端误显示"已失效"）。

        // 4. 记录"会话启动"阶段的 Skill 关联（F035）：
        //    使 skill_execution_logs.execution_phase = session_start 具备真实数据。
        //    仅在带任务上下文时记录（Skill 经任务→项目解析）。
        if let Some(tid) = task_id {
            self.record_session_start_skill(&session.id, tid).await;
        }

        info!("Agent启动成功: session_id={}", session.id);
        Ok(session)
    }

    /// 解析（或补登记）目标 Agent 记录，返回**实际生效的 agents.id**。
    ///
    /// 优先复用已注册记录（保留其名称/版本/能力，避免被临时构造值覆盖）；
    /// 从未注册过时按二进制文件名补登记。无论哪条路径都返回库中真实存在的 id，
    /// 以保证 `agent_sessions.agent_id` 的外键成立。
    pub async fn resolve_agent_id(&self, agent_path: &str) -> Result<String, String> {
        let db_ref = self.db.ok_or("数据库未初始化")?;
        let db = db_ref.lock().await;

        if let Some(agent) = db
            .get_agents()
            .map_err(|e| format!("获取Agent列表失败: {}", e))?
            .into_iter()
            .find(|a| a.binary_path == agent_path)
        {
            info!("复用已注册 Agent: id={}, name={}", agent.id, agent.name);
            return Ok(agent.id);
        }

        let stem = std::path::Path::new(agent_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("agent")
            .to_string();
        let new_agent = Agent {
            id: uuid::Uuid::new_v4().to_string(),
            name: stem.clone(),
            display_name: stem,
            binary_path: agent_path.to_string(),
            version: None,
            status: "available".to_string(),
            capabilities: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        db.upsert_agent(&new_agent).map_err(|e| {
            error!("保存Agent失败: {}", e);
            format!("保存Agent失败: {}", e)
        })
    }

    /// 停止 Agent 会话
    pub async fn stop_agent(&self, session_id: &str) -> Result<(), String> {
        let db_ref = self.db.ok_or("数据库未初始化")?;
        info!("停止Agent: session_id={}", session_id);
        let db = db_ref.lock().await;

        // 校验状态流转：仅 running 可终止为 completed，避免对已终态会话重复操作
        let current = db
            .get_agent_session(session_id)
            .map_err(|e| format!("获取会话失败: {}", e))?
            .map(|s| s.status)
            .ok_or_else(|| format!("会话不存在: {}", session_id))?;

        crate::validation::validate_session_status_transition(&current, "completed")
            .map_err(|e| e.to_string())?;

        // 回写状态、结束时间与执行时长
        db.finish_agent_session(session_id, "completed", Some(0))
            .map_err(|e| {
                error!("停止Agent失败: {}", e);
                format!("停止Agent失败: {}", e)
            })?;

        info!("Agent停止成功: session_id={}", session_id);
        Ok(())
    }

    /// 记录"会话启动"阶段的 Skill 关联（F035）。
    ///
    /// 解析链路：任务 → 项目（优先 `tasks.project_id`，其次所属需求的 `project_id`）
    /// → 项目默认 Skill。未匹配到 Skill 时**不写占位记录**，避免污染统计口径。
    ///
    /// 注意：这是**推定绑定**（会话启动时项目默认 Skill 会注入上下文），
    /// 并非 Agent 实际使用证明；`output_summary` 已标注，避免 use_count 被误读。
    async fn record_session_start_skill(&self, session_id: &str, task_id: &str) {
        let Some(db_ref) = self.db else { return };
        let db = db_ref.lock().await;

        let Ok(Some(task)) = db.get_task(task_id) else {
            return;
        };

        let project_id = match task.project_id.clone() {
            Some(pid) if !pid.is_empty() => Some(pid),
            _ => task
                .requirement_id
                .as_deref()
                .and_then(|rid| db.get_requirement(rid).ok().flatten())
                .map(|req| req.project_id)
                .filter(|pid| !pid.is_empty()),
        };

        let Some(project_id) = project_id else {
            return;
        };

        let Ok(project_skills) = db.list_project_skills(&project_id) else {
            return;
        };
        let Some(default_skill) = project_skills.into_iter().find(|ps| ps.is_default) else {
            return;
        };
        let Ok(Some(skill)) = db.get_prd_skill(&default_skill.skill_id) else {
            return;
        };

        if let Err(e) = db.record_skill_execution_log(
            Some(task_id),
            Some(session_id),
            task.requirement_id.as_deref(),
            &skill.id,
            &skill.name,
            &skill.skill_type,
            "session_start",
            Some(&serde_json::json!({
                "task_id": task_id,
                "inferred": true, // 推定绑定，非实际使用证明（P2-S4）
            }).to_string()),
            Some("会话启动时推定绑定项目默认 Skill（注入上下文，非实际使用证明）"),
            0,
            "success",
            None,
            None,
        ) {
            tracing::warn!("记录会话启动 Skill 日志失败: {}", e);
        }
    }

    /// 获取 Agent 会话列表
    pub async fn get_agent_sessions(
        &self,
        agent_id: Option<&str>,
        include_archived: bool,
    ) -> Result<Vec<AgentSession>, String> {
        let db_ref = self.db.ok_or("数据库未初始化")?;
        debug!("获取Agent会话: agent_id={:?}", agent_id);
        let db = db_ref.lock().await;
        db.get_agent_sessions(agent_id, include_archived)
            .map_err(|e| {
                error!("获取Agent会话失败: {}", e);
                format!("获取Agent会话失败: {}", e)
            })
    }
}
