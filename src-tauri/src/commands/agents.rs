//! 域命令：agents（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== Agent管理命令 ====================

#[tauri::command]
pub async fn discover_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, AppError> {
    let svc = AgentService::new(&state.db);
    svc.discover_agents().await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, AppError> {
    let db = state.db.lock().await;
    db.get_agents().map_err(|e| e.to_string()).map_err(AppError::legacy)
}

/// Agent 状态治理（F021）：
///
/// 1. **进程存活对账**：数据库中处于 `running`、但已无真实 PTY 进程的会话，纠正为 `failed`
///    （应用重启、进程崩溃、PTY 启动成功但子进程已退出等场景）；
/// 2. **二进制有效性复核**：逐个检查 `binary_path` 是否仍存在，失效标记 `unavailable`，
///    恢复可用则标记 `available`（`running` 的 Agent 交由会话生命周期管理，不在此处改动）。
///
/// 返回复核后的 Agent 列表。
#[tauri::command]
pub async fn verify_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, AppError> {
    // 1. 进程存活对账
    //
    // 注意：结构化会话（模块 16）不经过 PTY，其存活状态在 session_registry 中，
    // 若只统计 PTY 会把正在运行的消息模式会话误判为"僵尸"。两者取并集。
    let mut live_db_sessions = state.pty_manager.live_db_session_ids().await;
    live_db_sessions.extend(session_driver::active_session_ids(&state.session_registry).await);
    let stale_sessions: Vec<String> = {
        let db = state.db.lock().await;
        db.get_agent_sessions(None)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|s| s.status == "running" && !live_db_sessions.contains(&s.id))
            .map(|s| s.id)
            .collect()
    };

    for session_id in &stale_sessions {
        let db = state.db.lock().await;
        if let Err(e) = db.finish_agent_session(session_id, "failed", Some(-1)) {
            tracing::warn!("会话对账回写失败: session={}, err={}", session_id, e);
        }
    }
    if !stale_sessions.is_empty() {
        info!("Agent 状态复核：{} 个无进程会话已标记为 failed", stale_sessions.len());
    }

    // 2. 二进制有效性复核 + Agent 状态对账
    //
    //    expected 判定依据真实事实而非历史状态：
    //    - 二进制不存在 → unavailable（优先级最高）
    //    - 二进制存在且该 Agent 仍有 running 会话 → running
    //    - 否则 → available
    //
    //    这样既能纠正"二进制已删除但状态仍是 available"，也能修复
    //    "Agent 被标记为 running 但其会话已失败/不存在"的脏状态。
    let agents = {
        let db = state.db.lock().await;
        let agents = db.get_agents().map_err(|e| e.to_string())?;

        let running_agent_ids: std::collections::HashSet<String> = db
            .get_agent_sessions(None)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|s| s.status == "running")
            .map(|s| s.agent_id)
            .collect();

        for agent in &agents {
            let exists = std::path::Path::new(&agent.binary_path).exists();
            let expected = if !exists {
                "unavailable"
            } else if running_agent_ids.contains(&agent.id) {
                "running"
            } else {
                "available"
            };

            if agent.status != expected {
                info!(
                    "Agent 状态纠正: id={}, {} → {}",
                    agent.id, agent.status, expected
                );
                if let Err(e) = db.set_agent_status(&agent.id, expected) {
                    tracing::warn!("更新 Agent 状态失败: agent={}, err={}", agent.id, e);
                }
            }
        }

        db.get_agents().map_err(|e| e.to_string())?
    };

    Ok(agents)
}

#[tauri::command]
pub async fn start_agent(
    state: State<'_, AppState>,
    agent_path: String,
    task_id: Option<String>,
) -> Result<AgentSession, AppError> {
    // 1. 创建数据库会话记录
    let svc = AgentService::new(&state.db);
    let session = svc.start_agent(&agent_path, task_id.as_deref()).await?;

    // 2. 启动 PTY 进程（实际运行 Agent）
    let work_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let pty_session_id = match state.pty_manager.spawn_agent(&agent_path, &work_dir, vec![]).await {
        Ok(id) => {
            info!("Agent {} 已启动，PTY session: {}", agent_path, id);
            id
        }
        Err(e) => {
            // PTY 启动失败时把会话置为 failed，避免出现"有会话无进程"的僵尸记录
            error!("PTY 启动失败: {}", e);
            {
                let db = state.db.lock().await;
                if let Err(err) = db.finish_agent_session(&session.id, "failed", Some(-1)) {
                    error!("回写会话失败状态出错: {}", err);
                }
            }
            return Err(AppError::legacy(format!("启动 Agent 进程失败: {}", e)));
        }
    };

    // 3. 建立 PTY 会话与数据库会话的双向关联，供 stop_agent 反查并终止进程
    state
        .pty_manager
        .bind_db_session(&pty_session_id, &session.id)
        .await;

    // 4. 记录会话启动时的 MCP 连接事件（使 MCP 日志/统计具备真实数据）
    record_mcp_session_start(state.inner(), &session.id, task_id.as_deref()).await;

    // 4. 后台等待进程退出并回写会话状态/时长
    watch_terminal_session_exit(state.inner(), pty_session_id, session.id.clone());

    Ok(session)
}

#[tauri::command]
pub async fn stop_agent(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), AppError> {
    // 1. 若存在对应的 PTY 进程，先真实终止，避免进程泄漏（F030）
    if let Some(pty_id) = state.pty_manager.find_pty_by_db_session(&session_id).await {
        match state.pty_manager.kill(&pty_id).await {
            Ok(_) => info!("已终止 PTY 进程: pty={}, db_session={}", pty_id, session_id),
            Err(e) => tracing::warn!("终止 PTY 进程失败（继续更新会话状态）: {}", e),
        }
    }

    // 2. 更新数据库会话状态
    let svc = AgentService::new(&state.db);
    svc.stop_agent(&session_id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_agent_sessions(
    state: State<'_, AppState>,
    agent_id: Option<String>,
) -> Result<Vec<AgentSession>, AppError> {
    let svc = AgentService::new(&state.db);
    svc.get_agent_sessions(agent_id.as_deref()).await.map_err(AppError::legacy)
}
