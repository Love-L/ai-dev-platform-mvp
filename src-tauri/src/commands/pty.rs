//! 域命令：pty（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== PTY 终端命令 ====================

#[tauri::command]
pub async fn pty_spawn(
    state: State<'_, AppState>,
    agent_path: String,
    work_dir: String,
    env_vars: Option<Vec<(String, String)>>,
) -> Result<String, AppError> {
    let svc = PtyService::new(&state.pty_manager);
    let env = env_vars.unwrap_or_default();
    svc.spawn(&agent_path, &work_dir, env).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn pty_write(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<(), AppError> {
    let svc = PtyService::new(&state.pty_manager);
    svc.write(&session_id, &input).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn pty_resize(
    state: State<'_, AppState>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), AppError> {
    let svc = PtyService::new(&state.pty_manager);
    svc.resize(&session_id, rows, cols).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn pty_kill(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), AppError> {
    let svc = PtyService::new(&state.pty_manager);
    svc.kill(&session_id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn pty_get_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::pty::PtySessionInfo>, AppError> {
    let svc = PtyService::new(&state.pty_manager);
    Ok(svc.get_sessions().await)
}

#[tauri::command]
pub async fn pty_get_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<crate::pty::PtySessionInfo>, AppError> {
    let svc = PtyService::new(&state.pty_manager);
    Ok(svc.get_session(&session_id).await)
}

/// 附着到终端会话：按数据库会话 id 解析真实 PTY id，并回放既有输出。
///
/// PTY id 与数据库会话 id **不同源**（`spawn_agent` 自行生成 uuid，仅靠
/// `bind_db_session` 关联），前端必须先经此换取真实 PTY id 才能收发
/// `pty:event` / 调用 `pty_write|resize|kill`；`replay` 补齐挂载前的历史输出，
/// 避免打开终端只见一片黑。进程不存在或已退出时返回 Err，由前端展示重启入口。
#[tauri::command]
pub async fn pty_attach(
    state: State<'_, AppState>,
    db_session_id: String,
) -> Result<PtyAttach, AppError> {
    let (pty_id, replay) = state
        .pty_manager
        .attach_by_db_session(&db_session_id)
        .await?;
    Ok(PtyAttach { pty_id, replay })
}

/// 为终端会话重启 Agent 进程（进程已退出/应用重启后恢复可交互终端）。
///
/// 复用既有数据库会话记录（不新建会话）：spawn 新 PTY → 重新绑定 db_session_id
/// → 复位会话状态为 running → 挂后台退出回写。已有存活进程时直接返回其 PTY id。
#[tauri::command]
pub async fn pty_reconnect(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    db_session_id: String,
) -> Result<String, AppError> {
    if let Some(pty_id) = state
        .pty_manager
        .find_pty_by_db_session(&db_session_id)
        .await
    {
        return Ok(pty_id);
    }

    let session = {
        let db = state.db.lock().await;
        db.get_agent_session(&db_session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "会话不存在".to_string())?
    };
    if session.mode.as_deref() == Some(session_mode::STRUCTURED) {
        return Err(AppError::legacy("结构化会话不支持终端进程重启".to_string()));
    }

    // 二进制路径：优先会话记录，回退 agents 表
    let agent_path = match session
        .agent_path
        .clone()
        .filter(|s| !s.trim().is_empty())
    {
        Some(p) => p,
        None => {
            let db = state.db.lock().await;
            let agents = db.get_agents().map_err(|e| e.to_string())?;
            let agent_id = session.agent_id.clone();
            agents
                .into_iter()
                .find(|a| a.id == agent_id)
                .map(|a| a.binary_path)
                .ok_or_else(|| "Agent 记录不存在".to_string())?
        }
    };
    if !crate::agent::check_agent_available(&agent_path).await {
        return Err(AppError::legacy(format!("Agent 不可用: {}", agent_path)));
    }

    // 工作目录：会话记录 → 项目路径设置 → 进程 cwd
    let work_dir = match session
        .work_dir
        .clone()
        .filter(|s| !s.trim().is_empty())
    {
        Some(w) => w,
        None => match read_string_setting(&state, "project_path")
            .await
            .filter(|s| !s.trim().is_empty())
        {
            Some(p) => p,
            None => std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        },
    };

    let pty_id = state
        .pty_manager
        .spawn_agent(&agent_path, &work_dir, vec![])
        .await?;
    state
        .pty_manager
        .bind_db_session(&pty_id, &db_session_id)
        .await;

    {
        let db = state.db.lock().await;
        db.restart_agent_session(&db_session_id)
            .map_err(|e| format!("更新会话状态失败: {}", e))?;
    }
    session_driver::emit_session_status(&app, &db_session_id, "running");

    watch_terminal_session_exit(state.inner(), app, pty_id.clone(), db_session_id);
    Ok(pty_id)
}
