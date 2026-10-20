//! 域命令：sessions（由 commands.rs 拆分生成，P2-4）

use super::*;

/// 基于任务启动 Claude 会话（修复：此前只建数据库记录、不启动进程，
/// 且前端无调用点）。现在直接以**结构化（消息）模式**启动真实 Claude 进程，
/// 任务作为上下文注入（上下文卡片 + `--append-system-prompt`）。
#[tauri::command]
pub async fn create_session_from_task(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    task_id: String,
) -> Result<AgentSession, AppError> {
    let agent_path = first_available_agent(state.inner())
        .await
        .ok_or_else(|| "未检测到可用的 Agent，请先在 Agent 页面执行「刷新检测」".to_string())?;

    info!(
        "基于任务启动 Claude 会话: task_id={}, agent={}",
        task_id, agent_path
    );

    start_structured_session_inner(
        state.inner(),
        &app,
        agent_path,
        Some(task_id),
        None,
        None,
        None,
        None,
    )
    .await.map_err(AppError::legacy)
}

/// 关联/取消关联会话与开发任务（task_id 为 None 时解除关联）
#[tauri::command]
pub async fn link_session_task(
    state: State<'_, AppState>,
    session_id: String,
    task_id: Option<String>,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    if db
        .get_agent_session(&session_id)
        .map_err(|e| format!("查询会话失败: {}", e))?
        .is_none()
    {
        return Err(AppError::legacy(format!("会话 {} 不存在", session_id)));
    }
    // 关联时联动任务所属需求，保持上下文链路一致
    let requirement_id = match task_id.as_deref() {
        Some(tid) => {
            let task = db
                .get_task(tid)
                .map_err(|e| format!("查询任务失败: {}", e))?
                .ok_or_else(|| format!("任务 {} 不存在", tid))?;
            task.requirement_id.filter(|s| !s.is_empty())
        }
        None => None,
    };
    db.link_session_task(&session_id, task_id.as_deref(), requirement_id.as_deref())
        .map_err(|e| format!("关联任务失败: {}", e)).map_err(AppError::legacy)
}

/// 以**结构化（消息）模式**启动会话（F077）。
///
/// 与 `start_agent`（终端模式）并列：本命令启动 `stream-json` 双向流进程，
/// 支持指令下发、结构化回复、思考内容与工具调用的落库与展示。
/// 以**结构化（消息）模式**启动会话（F077）。
///
/// 与 `start_agent`（终端模式）并列：本命令启动 `stream-json` 双向流进程，
/// 支持指令下发、结构化回复、思考内容与工具调用的落库与展示。
#[tauri::command]
pub async fn start_structured_session(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    agent_path: String,
    task_id: Option<String>,
    requirement_id: Option<String>,
    work_dir: Option<String>,
    permission_mode: Option<String>,
    model: Option<String>,
) -> Result<AgentSession, AppError> {
    start_structured_session_inner(
        state.inner(),
        &app,
        agent_path,
        task_id,
        requirement_id,
        work_dir,
        permission_mode,
        model,
    )
    .await.map_err(AppError::legacy)
}

/// 下发一条用户指令（F077）
#[tauri::command]
pub async fn send_session_message(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
    content: String,
) -> Result<SessionMessage, AppError> {
    let session = {
        let db = state.db.lock().await;
        db.get_agent_session(&session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("会话不存在: {}", session_id))?
    };

    if !session_driver::is_active(&state.session_registry, &session_id).await {
        return Err(AppError::legacy("会话进程未在运行，请重建或续接会话".to_string()));
    }

    let ctx = build_message_context(state.inner(), &session).await;
    session_driver::send_user_message(
        &state.db,
        &app,
        &state.session_registry,
        &session_id,
        &ctx,
        &content,
    )
    .await.map_err(AppError::legacy)
}

/// 保存会话附件（图片粘贴 / 文件导入）到应用数据目录。
///
/// 返回 `{ name, path, size }`；前端把路径注入消息文本或终端输入行，
/// 由 Agent 通过文件读取完成内容识别（图片与文档通用）。
#[tauri::command]
pub async fn save_session_attachment(
    app: tauri::AppHandle,
    session_id: Option<String>,
    file_name: String,
    contents: Vec<u8>,
) -> Result<serde_json::Value, AppError> {
    if contents.is_empty() {
        return Err(AppError::legacy("附件内容为空".to_string()));
    }
    const MAX_SIZE: usize = 20 * 1024 * 1024;
    if contents.len() > MAX_SIZE {
        return Err(AppError::legacy(format!("附件不能超过 {}MB", MAX_SIZE / 1024 / 1024)));
    }

    // 文件名清洗：只取 basename 并过滤非法字符，防路径穿越
    let raw_name = std::path::Path::new(&file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "attachment".to_string());
    let safe_name: String = raw_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe_name = safe_name.trim().trim_matches('.').to_string();
    let safe_name = if safe_name.is_empty() {
        "attachment".to_string()
    } else {
        safe_name
    };

    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    let scope = session_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("general");
    let scope = scope.replace(['/', '\\', ':'], "_");
    let dir = base.join("attachments").join(scope);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建附件目录失败: {}", e))?;

    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S%.3f");
    let path = dir.join(format!("{}_{}", ts, safe_name));
    std::fs::write(&path, &contents).map_err(|e| format!("写入附件失败: {}", e))?;
    info!("会话附件已保存: session={:?}, path={:?}", session_id, path);

    Ok(serde_json::json!({
        "name": raw_name,
        "path": path.to_string_lossy(),
        "size": contents.len(),
    }))
}

/// 分页读取会话消息（F076 / F085）
#[tauri::command]
pub async fn list_session_messages(
    state: State<'_, AppState>,
    session_id: String,
    before_seq: Option<i64>,
    limit: Option<u32>,
) -> Result<PagedResult<SessionMessage>, AppError> {
    let default_limit = {
        let db = state.db.lock().await;
        db.get_setting("session_message_page_size")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(50)
    };
    let limit = limit.unwrap_or(default_limit).clamp(1, 500);

    let db = state.db.lock().await;
    let items = db
        .list_session_messages(&session_id, before_seq, limit)
        .map_err(|e| format!("读取会话消息失败: {}", e))?;
    let total = db
        .count_session_messages(&session_id)
        .map_err(|e| format!("统计会话消息失败: {}", e))?;

    Ok(PagedResult::new(items, total as u64, 1, limit))
}

/// 中断当前执行（F086）
#[tauri::command]
pub async fn interrupt_session(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
) -> Result<(), AppError> {
    let session = {
        let db = state.db.lock().await;
        db.get_agent_session(&session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("会话不存在: {}", session_id))?
    };

    let ctx = build_message_context(state.inner(), &session).await;
    session_driver::interrupt_session(
        &state.db,
        &app,
        &state.session_registry,
        &session_id,
        &ctx,
    )
    .await.map_err(AppError::legacy)
}

/// 以新模式/新权限重建会话（F081 / F083）。
///
/// `allowed_tools` 会**并入**会话既有白名单，用于"允许并重试"；
/// `resend_last = true` 时自动重发上一条用户指令。
#[tauri::command]
pub async fn set_session_mode(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
    mode: String,
    permission_mode: Option<String>,
    model: Option<String>,
    allowed_tools: Option<Vec<String>>,
    resend_last: Option<bool>,
) -> Result<AgentSession, AppError> {
    rebuild_structured_session(
        state.inner(),
        &app,
        &session_id,
        Some(mode),
        permission_mode,
        model,
        allowed_tools,
        resend_last.unwrap_or(false),
    )
    .await.map_err(AppError::legacy)
}

/// 续接既有 CLI 会话（F082）
#[tauri::command]
pub async fn resume_session(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
) -> Result<AgentSession, AppError> {
    rebuild_structured_session(
        state.inner(),
        &app,
        &session_id,
        None,
        None,
        None,
        None,
        false,
    )
    .await.map_err(AppError::legacy)
}
