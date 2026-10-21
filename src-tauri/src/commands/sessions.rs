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

/// 强制重建：先中断运行中的进程，等注册表释放后以新模式/新白名单重建并重发。
///
/// 把原先前端"轮询正在运行错误 ×12"的 hack 下沉为后端原子操作。
#[tauri::command]
pub async fn force_rebuild_session(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
    mode: Option<String>,
    permission_mode: Option<String>,
    model: Option<String>,
    allowed_tools: Option<Vec<String>>,
    resend_last: Option<bool>,
) -> Result<AgentSession, AppError> {
    // 1. 运行中则中断（结构化会话走 shutdown 信号，终端会话走 PTY kill）
    if session_driver::is_active(&state.session_registry, &session_id).await {
        let ctx = {
            let db = state.db.lock().await;
            db.get_agent_session(&session_id)
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| AppError::NotFound(format!("会话不存在: {}", session_id)))?
        };
        let ctx = build_message_context(state.inner(), &ctx).await;
        session_driver::interrupt_session(
            &state.db,
            &app,
            &state.session_registry,
            &session_id,
            &ctx,
        )
        .await
        .map_err(AppError::legacy)?;
    } else if let Some(pty_id) = state.pty_manager.find_pty_by_db_session(&session_id).await {
        if let Err(e) = state.pty_manager.kill(&pty_id).await {
            tracing::warn!("重建前终止 PTY 失败（继续尝试重建）: {}", e);
        }
    }

    // 2. 短轮询等待注册表释放（进程退出是异步的，通常 <500ms）
    for _ in 0..20 {
        if !session_driver::is_active(&state.session_registry, &session_id).await {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    // 3. 重建
    rebuild_structured_session(
        state.inner(),
        &app,
        &session_id,
        mode,
        permission_mode,
        model,
        allowed_tools,
        resend_last.unwrap_or(false),
    )
    .await
    .map_err(AppError::legacy)
}

/// 重命名会话
#[tauri::command]
pub async fn rename_agent_session(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), AppError> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::Validation("会话标题不能为空".to_string()));
    }
    let db = state.db.lock().await;
    if db
        .get_agent_session(&session_id)
        .map_err(|e| AppError::Database(e.to_string()))?
        .is_none()
    {
        return Err(AppError::NotFound(format!("会话 {} 不存在", session_id)));
    }
    db.rename_session(&session_id, &title)
        .map_err(|e| AppError::Database(e.to_string()))
}

/// 归档 / 取消归档会话（归档后默认列表隐藏，数据与消息全保留）
#[tauri::command]
pub async fn set_agent_session_archived(
    state: State<'_, AppState>,
    session_id: String,
    archived: bool,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    db.set_session_archived(&session_id, archived)
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

/// 删除会话（级联清理消息 / 执行日志 / Token 记录）。
///
/// 运行中的结构化会话先中断、终端会话先杀 PTY，等待进程释放后再删数据。
#[tauri::command]
pub async fn delete_agent_session(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
) -> Result<(), AppError> {
    if session_driver::is_active(&state.session_registry, &session_id).await {
        let session = {
            let db = state.db.lock().await;
            db.get_agent_session(&session_id)
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| AppError::NotFound(format!("会话不存在: {}", session_id)))?
        };
        let ctx = build_message_context(state.inner(), &session).await;
        let _ = session_driver::interrupt_session(
            &state.db,
            &app,
            &state.session_registry,
            &session_id,
            &ctx,
        )
        .await;
        for _ in 0..20 {
            if !session_driver::is_active(&state.session_registry, &session_id).await {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    } else if let Some(pty_id) = state.pty_manager.find_pty_by_db_session(&session_id).await {
        let _ = state.pty_manager.kill(&pty_id).await;
    }

    {
        let db = state.db.lock().await;
        db.delete_session_cascade(&session_id)
            .map_err(|e| AppError::Database(format!("删除会话失败: {}", e)))?;
    }
    session_driver::emit_session_status(&app, &session_id, "deleted");
    Ok(())
}

/// 导出会话为 Markdown（对话块 + 工具调用 + 单轮统计 + 会话元信息）。
///
/// 未指定 `out_path` 时写入应用数据目录 `exports/`，返回实际写入路径。
#[tauri::command]
pub async fn export_agent_session(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
    out_path: Option<String>,
) -> Result<String, AppError> {
    let (session, messages) = {
        let db = state.db.lock().await;
        let session = db
            .get_agent_session(&session_id)
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("会话不存在: {}", session_id)))?;
        let messages = db
            .list_session_messages(&session_id, None, 10_000)
            .map_err(|e| AppError::Database(e.to_string()))?;
        (session, messages)
    };

    let mut md = String::new();
    md.push_str(&format!(
        "# 会话导出：{}\n\n",
        session.title.as_deref().unwrap_or(&session.id)
    ));
    md.push_str("| 字段 | 值 |\n| --- | --- |\n");
    md.push_str(&format!("| 会话 ID | `{}` |\n", session.id));
    md.push_str(&format!(
        "| Agent | {} |\n",
        session.agent_name.clone().or(session.agent_path.clone()).unwrap_or_else(|| "未知".to_string())
    ));
    md.push_str(&format!("| 状态 | {} |\n", session.status));
    md.push_str(&format!(
        "| 模式 | {} |\n",
        session.mode.as_deref().unwrap_or("terminal")
    ));
    if let Some(dir) = session.work_dir.as_deref() {
        md.push_str(&format!("| 工作目录 | `{}` |\n", dir));
    }
    if let Some(model) = session.model.as_deref() {
        md.push_str(&format!("| 模型 | {} |\n", model));
    }
    md.push_str(&format!("| 开始时间 | {} |\n", session.started_at));
    if let Some(end) = session.ended_at.as_deref() {
        md.push_str(&format!("| 结束时间 | {} |\n", end));
    }
    md.push_str(&format!(
        "| Token 总量 | {} |\n",
        session.total_tokens.unwrap_or(0)
    ));
    md.push_str(&format!(
        "| 费用 | ${:.4} |\n\n",
        session.total_cost.unwrap_or(0.0)
    ));

    md.push_str("## 对话记录\n\n");
    for m in &messages {
        let role = match m.role.as_str() {
            "user" => "用户",
            "assistant" => "Agent",
            _ => "系统",
        };
        let kind = m.block_type.as_str();
        let content = m.content.as_deref().unwrap_or("");
        match kind {
            "text" => {
                md.push_str(&format!("### {} · {}\n\n{}\n\n", role, m.created_at, content));
            }
            "thinking" => {
                if !content.is_empty() {
                    md.push_str(&format!(
                        "<details><summary>思考过程 · {}</summary>\n\n```\n{}\n```\n\n</details>\n\n",
                        m.created_at, content
                    ));
                }
            }
            "tool_use" => {
                md.push_str(&format!(
                    "#### 工具调用 {} · {}\n\n```json\n{}\n```\n\n",
                    m.tool_name.as_deref().unwrap_or(""),
                    m.created_at,
                    content
                ));
            }
            "tool_result" => {
                md.push_str(&format!(
                    "#### 工具结果 · {}\n\n```\n{}\n```\n\n",
                    m.created_at, content
                ));
            }
            "permission_denied" => {
                md.push_str(&format!(
                    "#### ⚠ 权限拒绝 · {}\n\n```\n{}\n```\n\n",
                    m.created_at, content
                ));
            }
            "result" => {
                md.push_str(&format!("> 单轮统计（{}）：`{}`\n\n", m.created_at, content));
            }
            "context" => {
                md.push_str(&format!("> 会话上下文（{}）：`{}`\n\n", m.created_at, content));
            }
            _ => {
                if !content.is_empty() {
                    md.push_str(&format!("- [{}] {}\n", m.created_at, content));
                }
            }
        }
    }

    let path = match out_path.filter(|s| !s.trim().is_empty()) {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let base = app
                .path()
                .app_data_dir()
                .map_err(|e| AppError::legacy(format!("获取应用数据目录失败: {}", e)))?;
            let dir = base.join("exports");
            std::fs::create_dir_all(&dir)
                .map_err(|e| AppError::legacy(format!("创建导出目录失败: {}", e)))?;
            let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let safe: String = session
                .title
                .as_deref()
                .unwrap_or(&session_id)
                .chars()
                .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_') { c } else { '_' })
                .collect();
            dir.join(format!("session_{}_{}.md", safe, ts))
        }
    };

    std::fs::write(&path, md).map_err(|e| AppError::legacy(format!("写入导出文件失败: {}", e)))?;
    info!("会话已导出: session={}, path={:?}", session_id, path);
    Ok(path.to_string_lossy().to_string())
}
