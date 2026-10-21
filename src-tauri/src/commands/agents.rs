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
        db.get_agent_sessions(None, true)
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
    //    - 二进制存在但 `--version` 探测失败（文件损坏/不可执行）→ unavailable（P1-A2 真探测）
    //    - 二进制可用且该 Agent 仍有 running 会话 → running
    //    - 否则 → available
    //
    //    这样既能纠正"二进制已删除但状态仍是 available"，也能发现
    //    "文件在但已损坏"以及"Agent 被标记为 running 但其会话已失败"的脏状态。
    let agents = {
        let db = state.db.lock().await;
        let agents = db.get_agents().map_err(|e| e.to_string())?;

        let running_agent_ids: std::collections::HashSet<String> = db
            .get_agent_sessions(None, true)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|s| s.status == "running")
            .map(|s| s.agent_id)
            .collect();

        for agent in &agents {
            let exists = std::path::Path::new(&agent.binary_path).exists();
            // 真实执行 `--version`（带 5 秒超时），文件在但跑不起来同样判不可用
            let probed = exists && crate::agent::probe_binary(&agent.binary_path).is_some();
            let expected = if !probed {
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

/// 手动添加 Agent（P1-A2：支持自定义路径，不依赖自动检测）。
///
/// 真实执行 `--version` 验证后落库；路径无效直接报错。
#[tauri::command]
pub async fn add_agent(
    state: State<'_, AppState>,
    binary_path: String,
    display_name: Option<String>,
) -> Result<Agent, AppError> {
    let path = binary_path.trim();
    if path.is_empty() {
        return Err(AppError::legacy("路径不能为空"));
    }
    if !std::path::Path::new(path).exists() {
        return Err(AppError::legacy(format!("文件不存在: {}", path)));
    }

    // 真实探测：能跑 `--version` 才允许添加
    let version = crate::agent::probe_binary(path)
        .ok_or_else(|| AppError::legacy(format!("该二进制无法执行 --version（可能已损坏或无权限）: {}", path)))?;

    let name = display_name
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            std::path::Path::new(path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "agent".to_string())
        });

    let agent = Agent {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        display_name: name,
        binary_path: path.to_string(),
        version: Some(version),
        status: "available".to_string(),
        capabilities: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let db = state.db.lock().await;
    db.upsert_agent(&agent).map_err(|e| AppError::legacy(format!("保存 Agent 失败: {}", e)))?;
    info!("手动添加 Agent: {}", path);

    // 回读真实记录（upsert 可能复用既有 id）
    db.get_agents()
        .map_err(|e| AppError::legacy(format!("查询 Agent 失败: {}", e)))?
        .into_iter()
        .find(|a| a.binary_path == path)
        .ok_or_else(|| AppError::legacy("保存后未找到该 Agent"))
}

/// 删除 Agent 记录（有运行中会话时拒绝）。
#[tauri::command]
pub async fn delete_agent(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    let running: i64 = {
        let sessions = db
            .get_agent_sessions(None, true)
            .map_err(|e| AppError::legacy(format!("查询会话失败: {}", e)))?;
        sessions
            .iter()
            .filter(|s| s.agent_id == id && s.status == "running")
            .count() as i64
    };
    if running > 0 {
        return Err(AppError::legacy(format!(
            "该 Agent 有 {} 个运行中的会话，请先停止会话再删除",
            running
        )));
    }
    db.delete_agent(&id).map_err(|e| AppError::legacy(format!("删除 Agent 失败: {}", e)))?;
    info!("删除 Agent 记录: {}", id);
    Ok(())
}

#[tauri::command]
pub async fn start_agent(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    agent_path: String,
    task_id: Option<String>,
    work_dir: Option<String>,
    model: Option<String>,
) -> Result<AgentSession, AppError> {
    // 1. 创建数据库会话记录
    let svc = AgentService::new(&state.db);
    let mut session = svc.start_agent(&agent_path, task_id.as_deref()).await?;

    // 无标题会话自动命名（P1：列表以可辨识标题展示，而非 UUID）
    if session.title.as_deref().map(str::trim).unwrap_or("").is_empty() {
        let agent_label = std::path::Path::new(&agent_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Agent".to_string());
        let ts = chrono::Local::now().format("%m-%d %H:%M").to_string();
        let title = format!("{} · {}", agent_label, ts);
        {
            let db = state.db.lock().await;
            if let Err(e) = db.rename_session(&session.id, &title) {
                tracing::warn!("自动命名会话失败: {}", e);
            }
        }
        session.title = Some(title);
    }

    // 2. 解析工作目录（P0 修复：此前固定为进程 cwd，Agent 在错误目录执行且用户无感知）。
    //    优先级：显式传入 → 「项目路径」设置项 → 进程当前目录兜底。
    let work_dir = match work_dir.filter(|s| !s.trim().is_empty()) {
        Some(dir) => Some(dir),
        None => read_string_setting(state.inner(), "project_path").await,
    };
    let work_dir = work_dir
        .filter(|s| std::path::Path::new(s).is_dir())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string())
        });

    // 元信息落库（工作目录 / 模型），供前端展示与后续重建复用
    {
        let db = state.db.lock().await;
        if let Err(e) = db.update_session_meta(
            &session.id,
            None,
            None,
            model.as_deref().filter(|s| !s.trim().is_empty()),
            None,
            Some(&work_dir),
            None,
            None,
        ) {
            tracing::warn!("回写终端会话元信息失败: {}", e);
        }
    }
    session.work_dir = Some(work_dir.clone());
    if let Some(m) = model.as_deref().filter(|s| !s.trim().is_empty()) {
        session.model = Some(m.to_string());
    }

    // 3. 启动 PTY 进程（实际运行 Agent）。
    //    模型仅作元数据记录：终端模式下 Agent 是全交互 CLI，
    //    用户可在 PTY 内用 `/model` 等命令自行切换，无需注入启动参数。
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
    session_driver::emit_session_status(&app, &session.id, "running");
    watch_terminal_session_exit(state.inner(), app, pty_session_id, session.id.clone());

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
    include_archived: Option<bool>,
) -> Result<Vec<AgentSession>, AppError> {
    let svc = AgentService::new(&state.db);
    svc.get_agent_sessions(agent_id.as_deref(), include_archived.unwrap_or(false))
        .await
        .map_err(AppError::legacy)
}

/// 单查会话（详情页避免全量拉取列表再 find）
#[tauri::command]
pub async fn get_agent_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<AgentSession>, AppError> {
    let db = state.db.lock().await;
    db.get_agent_session(&session_id)
        .map_err(|e| AppError::Database(e.to_string()))
}
