use crate::AppState;
use crate::db::{Requirement, Task, Project, Agent, AgentSession, TokenUsage, PrdSkill, TokenUsageSummary, DailyUsage, ModelUsage};
use crate::services::*;
use crate::validation::{validate_requirement, validate_task, validate_project};
use crate::agent_adapter::{self, SessionLaunchConfig};
use crate::db::session_messages::{BlockSpec, MessageContext};
use crate::db::{block_type, session_mode, SessionMessage};
use crate::pagination::PagedResult;
use crate::session_driver;
use tauri::{Manager, State};
use tracing::{info, error};

// ==================== 需求管理命令 ====================

#[tauri::command]
pub async fn create_requirement(
    state: State<'_, AppState>,
    title: String,
    description: Option<String>,
    priority: String,
    project_id: Option<String>,
    module: Option<String>,
    source: Option<String>,
    source_detail: Option<String>,
    product_owner: Option<String>,
    developer: Option<String>,
    tester: Option<String>,
    business_priority: Option<String>,
    estimated_effort: Option<f64>,
    tags: Option<String>,
    notes: Option<String>,
) -> Result<Requirement, String> {
    // 输入验证
    validate_requirement(&title, &priority).map_err(|e| e.to_string())?;

    let svc = RequirementService::new(&state.db, &state.cache);
    svc.create_requirement(
        &title,
        description.as_deref(),
        &priority,
        project_id.as_deref(),
        module.as_deref(),
        source.as_deref(),
        source_detail.as_deref(),
        product_owner.as_deref(),
        developer.as_deref(),
        tester.as_deref(),
        business_priority.as_deref(),
        estimated_effort,
        tags.as_deref(),
        notes.as_deref(),
    ).await
}

#[tauri::command]
pub async fn list_requirements(
    state: State<'_, AppState>,
    page: Option<u32>,
    page_size: Option<u32>,
    status: Option<String>,
) -> Result<crate::pagination::PagedResult<Requirement>, String> {
    let svc = RequirementService::new(&state.db, &state.cache);
    let params = crate::pagination::PaginationParams {
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    };
    svc.list_requirements_paged(&params, status.as_deref()).await
}

#[tauri::command]
pub async fn get_requirement(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Requirement>, String> {
    let svc = RequirementService::new(&state.db, &state.cache);
    svc.get_requirement(&id).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_requirement(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    description: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    prd_content: Option<String>,
    module: Option<String>,
    source: Option<String>,
    source_detail: Option<String>,
    product_owner: Option<String>,
    developer: Option<String>,
    tester: Option<String>,
    business_priority: Option<String>,
    estimated_effort: Option<f64>,
    tags: Option<String>,
    notes: Option<String>,
) -> Result<(), String> {
    // 如果要更新状态，先校验状态流转合法性
    if let Some(ref new_status) = status {
        let svc = RequirementService::new(&state.db, &state.cache);
        let current = svc.get_requirement(&id).await
            .map_err(|e| format!("查询需求失败: {}", e))?;
        match current {
            Some(req) => {
                crate::validation::validate_requirement_status_transition(&req.status, new_status)
                    .map_err(|e| e.to_string())?;
            }
            None => return Err(format!("需求 {} 不存在", id)),
        }
    }

    let ext = RequirementExtFields {
        module: module.as_deref(),
        source: source.as_deref(),
        source_detail: source_detail.as_deref(),
        product_owner: product_owner.as_deref(),
        developer: developer.as_deref(),
        tester: tester.as_deref(),
        business_priority: business_priority.as_deref(),
        estimated_effort,
        tags: tags.as_deref(),
        notes: notes.as_deref(),
    };

    let svc = RequirementService::new(&state.db, &state.cache);
    svc.update_requirement(
        &id,
        title.as_deref(),
        description.as_deref(),
        priority.as_deref(),
        status.as_deref(),
        prd_content.as_deref(),
        &ext,
    ).await
}

#[tauri::command]
pub async fn delete_requirement(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = RequirementService::new(&state.db, &state.cache);

    // 检查是否有关联任务
    {
        let db = state.db.lock().await;
        let task_count = db.get_task_count_by_requirement(&id)
            .map_err(|e| format!("查询关联任务失败: {}", e))?;

        if task_count > 0 {
            return Err(format!(
                "需求下有 {} 个关联任务，请先删除或解绑",
                task_count
            ));
        }
    }

    svc.delete_requirement(&id).await
}

#[tauri::command]
pub async fn generate_prd(
    state: State<'_, AppState>,
    requirement_id: String,
    skill_id: Option<String>,
    extra_content: Option<String>,
) -> Result<String, String> {
    let svc = PrdService::new(&state.db, &state.cache);
    svc.generate_prd(&requirement_id, skill_id.as_deref(), extra_content.as_deref())
        .await
}

/// 流式生成 PRD（F049）。
///
/// 生成过程中通过 Tauri 事件 `prd:chunk` 推送增量文本，结束时推送 `prd:done`。
/// 前端需先 `listen("prd:chunk")` 再调用本命令。
/// `extra_content`：需求补充内容（用户在生成前输入，随需求描述一并注入 prompt）。
#[tauri::command]
pub async fn generate_prd_streaming(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    requirement_id: String,
    skill_id: Option<String>,
    extra_content: Option<String>,
) -> Result<String, String> {
    let svc = PrdService::new(&state.db, &state.cache);
    svc.generate_prd_streaming(
        &requirement_id,
        skill_id.as_deref(),
        extra_content.as_deref(),
        app,
    )
    .await
}

/// 取用于一次性 LLM 调用/任务会话的 Agent 路径（MA-09 选路优先级）：
/// 默认 Agent（`default_agent_path`）→ PRD 引擎（`prd_agent_path`）→ 第一个非 `unavailable` 的注册 Agent。
/// 配置项指向的二进制必须真实存在，否则跳过继续回退。
async fn first_available_agent(state: &AppState) -> Option<String> {
    let db = state.db.lock().await;

    let configured = db
        .get_setting("default_agent_path")
        .ok()
        .flatten()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| {
            db.get_setting("prd_agent_path")
                .ok()
                .flatten()
                .filter(|p| !p.trim().is_empty())
        });

    if let Some(p) = configured {
        if std::path::Path::new(&p).exists() {
            return Some(p);
        }
    }

    db.get_agents()
        .ok()?
        .into_iter()
        .find(|a| a.status != "unavailable")
        .map(|a| a.binary_path)
}

#[tauri::command]
pub async fn enrich_requirement(
    state: State<'_, AppState>,
    requirement_id: String,
    extra_content: Option<String>,
) -> Result<serde_json::Value, String> {
    // 获取需求信息
    let requirement = {
        let db = state.db.lock().await;
        db.get_requirement(&requirement_id)
            .map_err(|e| format!("获取需求失败: {}", e))?
            .ok_or("需求不存在")?
    };

    // 组合输入：需求描述 + 用户补充内容 + 项目上下文（工作区文件）
    let description = {
        let mut desc = requirement.description.clone().unwrap_or_default();
        if let Some(extra) = extra_content.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            desc.push_str("\n\n## 补充需求内容\n");
            desc.push_str(extra);
        }
        if let Some(context) =
            crate::services::prd_service::collect_project_context(&state.db).await
        {
            desc.push_str("\n\n## 项目上下文（来自工作区文件，供推断参考）\n");
            desc.push_str(&context);
        }
        desc
    };

    // 双通道分派：与 PRD 生成共用 `prd_generation_mode` 设置
    let (enriched, token_stats, agent_type, source) = {
        let (mode, model) = {
            let db = state.db.lock().await;
            let mode = db
                .get_setting(crate::services::prd_service::PRD_MODE_SETTING)
                .ok()
                .flatten()
                .unwrap_or_default();
            let model = db.get_setting("llm_model").ok().flatten();
            (mode, model)
        };

        if mode == "agent" {
            // Claude CLI 通道：无需 API Key，直接取回文本并解析 JSON
            let prompt = crate::llm::build_enrich_prompt(&requirement.title, &description);
            let agent_path = first_available_agent(&state).await.ok_or_else(|| {
                "未检测到可用的 Agent CLI，请先在 Agent 页面执行「刷新检测」".to_string()
            })?;
            let adapter = crate::agent_adapter::adapter_for(&agent_path);
            let inv = adapter
                .build_oneshot(&agent_path, &prompt, model.as_deref(), None)
                .ok_or_else(|| "该 Agent 不支持一次性调用（仅终端模式）".to_string())?;
            let text = crate::agent_adapter::execute_oneshot(inv).await?;

            // stdout 可能混入 CLI 告警 JSON（QClaw 环境实测），逐候选挑含业务字段的对象
            let enriched = crate::llm::parse_enriched_json(&text);
            (
                enriched,
                crate::llm::TokenStats {
                    input_tokens: 0,
                    output_tokens: 0,
                    model: model.unwrap_or_else(|| "claude-cli".to_string()),
                },
                "claude-cli",
                "agent",
            )
        } else {
            // API 通道
            let (api_key, model) = {
                let db = state.db.lock().await;
                let key = db
                    .get_setting("claude_api_key")
                    .map_err(|e| format!("获取API Key失败: {}", e))?
                    .ok_or("请先配置Claude API Key，或在设置中把 PRD 生成方式切换为 Claude CLI")?;
                (key, model)
            };

            let client = crate::llm::ClaudeClient::new(api_key).with_model(model.as_deref());
            let (enriched, token_stats) = client
                .enrich_requirement(&requirement.title, &description)
                .await?;
            (enriched, token_stats, "claude-api", "api")
        }
    };

    // 记录本次调用的 Token 消耗与成本（此前该路径不记账，导致成本漏算）
    {
        let db = state.db.lock().await;
        let input_tokens = token_stats.input_tokens as i64;
        let output_tokens = token_stats.output_tokens as i64;
        let cost = db.estimate_cost(&token_stats.model, input_tokens, output_tokens, 0, 0, 0);

        if let Err(e) = db.record_token_usage_ext(
            None,
            None,
            Some(agent_type),
            Some(&token_stats.model),
            input_tokens,
            output_tokens,
            0,
            0,
            0,
            cost,
            source,
        ) {
            error!("记录需求完善的Token使用失败: {}", e);
        }
    }

    // 更新需求的扩展字段
    {
        let db = state.db.lock().await;
        db.update_requirement_fields(
            &requirement_id,
            enriched.get("module").and_then(|v| v.as_str()),
            enriched.get("source").and_then(|v| v.as_str()),
            None, // source_detail
            None, // product_owner
            None, // developer
            None, // tester
            enriched.get("business_priority").and_then(|v| v.as_str()),
            enriched.get("estimated_effort").and_then(|v| v.as_f64()),
            None, // actual_effort
            None, // attachments
            enriched.get("tags").and_then(|v| v.as_str()),
            enriched.get("acceptance_criteria").and_then(|v| v.as_str()),
        ).map_err(|e| format!("更新需求失败: {}", e))?;
    }

    info!("需求完善完成: requirement_id={}", requirement_id);
    Ok(enriched)
}

// ==================== 任务管理命令 ====================

#[tauri::command]
pub async fn create_task(
    state: State<'_, AppState>,
    title: String,
    requirement_id: Option<String>,
    task_type: String,
    priority: String,
    assignee: Option<String>,
    due_date: Option<String>,
    estimated_hours: Option<f64>,
    priority_level: Option<String>,
    blocked_reason: Option<String>,
    tags: Option<String>,
    notes: Option<String>,
    description: Option<String>,
) -> Result<Task, String> {
    // 输入验证
    validate_task(&title, &task_type, &priority).map_err(|e| e.to_string())?;
    if let Some(desc) = &description {
        if desc.chars().count() > 100_000 {
            return Err("任务描述不能超过 100000 字".to_string());
        }
    }

    let svc = TaskService::new(&state.db, &state.cache);
    svc.create_task(
        &title,
        requirement_id.as_deref(),
        &task_type,
        &priority,
        assignee.as_deref(),
        due_date.as_deref(),
        estimated_hours,
        priority_level.as_deref(),
        blocked_reason.as_deref(),
        tags.as_deref(),
        notes.as_deref(),
        description.as_deref(),
    ).await
}

#[tauri::command]
pub async fn list_tasks(
    state: State<'_, AppState>,
    status: Option<String>,
    task_type: Option<String>,
) -> Result<Vec<Task>, String> {
    let svc = TaskService::new(&state.db, &state.cache);
    svc.list_tasks(status.as_deref(), task_type.as_deref()).await
}

#[tauri::command]
pub async fn update_task_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<(), String> {
    // 校验状态流转合法性
    {
        let db = state.db.lock().await;
        let current = db.get_task(&id)
            .map_err(|e| format!("查询任务失败: {}", e))?;
        match current {
            Some(task) => {
                crate::validation::validate_task_status_transition(&task.status, &status)
                    .map_err(|e| e.to_string())?;
            }
            None => return Err(format!("任务 {} 不存在", id)),
        }
    }

    let svc = TaskService::new(&state.db, &state.cache);
    svc.update_task_status(&id, &status).await
}

#[tauri::command]
pub async fn delete_task(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = TaskService::new(&state.db, &state.cache);
    svc.delete_task(&id).await
}

// ==================== 任务管理扩展命令 ====================

#[tauri::command]
pub async fn get_task(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Task>, String> {
    let db = state.db.lock().await;
    db.get_task(&id)
        .map_err(|e| format!("获取任务详情失败: {}", e))
}

#[tauri::command]
pub async fn update_task(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    description: Option<String>,
    task_type: Option<String>,
    priority: Option<String>,
    assignee: Option<String>,
    due_date: Option<String>,
    tags: Option<String>,
    project_id: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.update_task(&id, title.as_deref(), description.as_deref(), task_type.as_deref(), priority.as_deref(), assignee.as_deref(), due_date.as_deref(), tags.as_deref(), project_id.as_deref())
        .map_err(|e| format!("更新任务失败: {}", e))
}

#[tauri::command]
pub async fn list_tasks_by_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<Task>, String> {
    let db = state.db.lock().await;
    db.list_tasks_by_project(&project_id)
        .map_err(|e| format!("获取项目任务列表失败: {}", e))
}

#[tauri::command]
pub async fn list_tasks_by_requirement(
    state: State<'_, AppState>,
    requirement_id: String,
) -> Result<Vec<Task>, String> {
    let db = state.db.lock().await;
    db.list_tasks_by_requirement(&requirement_id)
        .map_err(|e| format!("获取需求任务列表失败: {}", e))
}

// ==================== 任务评论命令 ====================

#[tauri::command]
pub async fn create_task_comment(
    state: State<'_, AppState>,
    task_id: String,
    author: String,
    content: String,
) -> Result<crate::db::TaskComment, String> {
    let db = state.db.lock().await;
    db.create_task_comment(&task_id, &author, &content)
        .map_err(|e| format!("创建评论失败: {}", e))
}

#[tauri::command]
pub async fn list_task_comments(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Vec<crate::db::TaskComment>, String> {
    let db = state.db.lock().await;
    db.list_task_comments(&task_id)
        .map_err(|e| format!("获取评论列表失败: {}", e))
}

#[tauri::command]
pub async fn delete_task_comment(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_task_comment(&id)
        .map_err(|e| format!("删除评论失败: {}", e))
}

// ==================== Agent管理命令 ====================

#[tauri::command]
pub async fn discover_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, String> {
    let svc = AgentService::new(&state.db);
    svc.discover_agents().await
}

#[tauri::command]
pub async fn get_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, String> {
    let db = state.db.lock().await;
    db.get_agents().map_err(|e| e.to_string())
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
pub async fn verify_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, String> {
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
) -> Result<AgentSession, String> {
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
            return Err(format!("启动 Agent 进程失败: {}", e));
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

/// 后台等待 PTY 进程退出并回写会话状态/时长（终端模式会话生命周期）。
///
/// `start_agent` 启动进程后与 `pty_reconnect` 重启进程后共用，
/// 保证无论进程从哪条路径启动，退出时都会回写数据库会话状态。
fn watch_terminal_session_exit(state: &AppState, pty_id: String, db_session_id: String) {
    let manager = state.pty_manager.clone();
    let db = state.db.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(code) = manager.wait_exit(&pty_id).await {
            let status = if code == 0 { "completed" } else { "failed" };
            let db = db.lock().await;
            if let Err(e) = db.finish_agent_session(&db_session_id, status, Some(code)) {
                error!("回写会话结束状态失败: {}", e);
            } else {
                info!("会话 {} 结束: status={}, exit_code={}", db_session_id, status, code);
            }
        }
    });
}

/// 会话启动时，为项目绑定（或任务关联项目）的 MCP 端点写入连接日志。
async fn record_mcp_session_start(
    state: &AppState,
    session_id: &str,
    task_id: Option<&str>,
) {
    let db = state.db.lock().await;

    // 解析项目 ID：优先取任务的 project_id / related_project_id
    let mut project_id: Option<String> = None;
    if let Some(tid) = task_id {
        if let Ok(Some(task)) = db.get_task(tid) {
            project_id = task.project_id.clone().or(task.related_project_id.clone());
        }
    }

    let Some(pid) = project_id else {
        return;
    };

    let mcps = match db.list_project_mcps(&pid) {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!("读取项目 MCP 配置失败: {}", e);
            return;
        }
    };

    for mcp in mcps {
        if let Err(e) = db.insert_mcp_execution_log(
            session_id,
            &mcp.mcp_name,
            Some(mcp.endpoint.as_str()),
            Some("session_start"),
            "connected",
            None,
            Some(0),
        ) {
            error!("写入 MCP 执行日志失败: {}", e);
        }
    }
}

/// 基于任务启动 Claude 会话（修复：此前只建数据库记录、不启动进程，
/// 且前端无调用点）。现在直接以**结构化（消息）模式**启动真实 Claude 进程，
/// 任务作为上下文注入（上下文卡片 + `--append-system-prompt`）。
#[tauri::command]
pub async fn create_session_from_task(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    task_id: String,
) -> Result<AgentSession, String> {
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
    .await
}

#[tauri::command]
pub async fn stop_agent(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    // 1. 若存在对应的 PTY 进程，先真实终止，避免进程泄漏（F030）
    if let Some(pty_id) = state.pty_manager.find_pty_by_db_session(&session_id).await {
        match state.pty_manager.kill(&pty_id).await {
            Ok(_) => info!("已终止 PTY 进程: pty={}, db_session={}", pty_id, session_id),
            Err(e) => tracing::warn!("终止 PTY 进程失败（继续更新会话状态）: {}", e),
        }
    }

    // 2. 更新数据库会话状态
    let svc = AgentService::new(&state.db);
    svc.stop_agent(&session_id).await
}

#[tauri::command]
pub async fn get_agent_sessions(
    state: State<'_, AppState>,
    agent_id: Option<String>,
) -> Result<Vec<AgentSession>, String> {
    let svc = AgentService::new(&state.db);
    svc.get_agent_sessions(agent_id.as_deref()).await
}

/// 关联/取消关联会话与开发任务（task_id 为 None 时解除关联）
#[tauri::command]
pub async fn link_session_task(
    state: State<'_, AppState>,
    session_id: String,
    task_id: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    if db
        .get_agent_session(&session_id)
        .map_err(|e| format!("查询会话失败: {}", e))?
        .is_none()
    {
        return Err(format!("会话 {} 不存在", session_id));
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
        .map_err(|e| format!("关联任务失败: {}", e))
}

// ==================== Token追踪命令 ====================

#[tauri::command]
pub async fn get_token_usage(
    state: State<'_, AppState>,
    task_id: Option<String>,
) -> Result<Vec<TokenUsage>, String> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage(task_id.as_deref()).await
}

#[tauri::command]
pub async fn get_token_usage_summary(
    state: State<'_, AppState>,
) -> Result<TokenUsageSummary, String> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage_summary().await
}

#[tauri::command]
pub async fn get_token_usage_trend(
    state: State<'_, AppState>,
    days: i32,
) -> Result<Vec<DailyUsage>, String> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage_trend(days).await
}

#[tauri::command]
pub async fn get_token_usage_by_model(
    state: State<'_, AppState>,
) -> Result<Vec<ModelUsage>, String> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage_by_model().await
}

#[tauri::command]
pub async fn get_token_budget(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_budget().await
}

#[tauri::command]
pub async fn set_token_budget(
    state: State<'_, AppState>,
    monthly_limit: i64,
    alert_threshold: f64,
) -> Result<(), String> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.set_token_budget(monthly_limit, alert_threshold).await
}

#[tauri::command]
pub async fn sync_local_usage(
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.sync_local_usage().await
}

// ==================== PRD Skill 命令 ====================

#[tauri::command]
pub async fn list_prd_skills(
    state: State<'_, AppState>,
) -> Result<Vec<PrdSkill>, String> {
    let svc = PrdSkillService::new(&state.db);
    svc.list_skills().await
}

#[tauri::command]
pub async fn get_prd_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<PrdSkill>, String> {
    let svc = PrdSkillService::new(&state.db);
    svc.get_skill(&id).await
}

#[tauri::command]
pub async fn create_prd_skill(
    state: State<'_, AppState>,
    name: String,
    skill_type: String,
    description: Option<String>,
    template: String,
) -> Result<PrdSkill, String> {
    let svc = PrdSkillService::new(&state.db);
    svc.create_skill(&name, &skill_type, description.as_deref(), &template).await
}

#[tauri::command]
pub async fn update_prd_skill(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    template: Option<String>,
) -> Result<(), String> {
    let svc = PrdSkillService::new(&state.db);
    svc.update_skill(&id, name.as_deref(), description.as_deref(), template.as_deref()).await
}

#[tauri::command]
pub async fn delete_prd_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = PrdSkillService::new(&state.db);
    svc.delete_skill(&id).await
}

// ==================== Skill 自进化命令 ====================

/// L1 诊断：规则 + 静态体检 + 振荡检测，建议落库后返回全部建议
#[tauri::command]
pub async fn analyze_skills(
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::SkillSuggestion>, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.analyze().await
}

#[tauri::command]
pub async fn list_skill_suggestions(
    state: State<'_, AppState>,
    skill_id: Option<String>,
) -> Result<Vec<crate::db::SkillSuggestion>, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.list_suggestions(skill_id.as_deref()).await
}

/// luban-skill 模板四段解析
#[tauri::command]
pub async fn parse_skill_template(
    state: State<'_, AppState>,
    skill_id: Option<String>,
    template: Option<String>,
) -> Result<serde_json::Value, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.parse_template(skill_id.as_deref(), template.as_deref()).await
}

/// 评分（结构分静态 + 效果分 Judge）+ 棘轮判定
#[tauri::command]
pub async fn judge_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<crate::db::SkillScore, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.judge(&skill_id).await
}

#[tauri::command]
pub async fn get_skill_score_history(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<crate::db::SkillScore>, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.score_history(&skill_id).await
}

/// L2 优化器：返回草稿（不落库，需前端 diff 确认后 apply）
#[tauri::command]
pub async fn optimize_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<serde_json::Value, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.optimize(&skill_id).await
}

/// 应用优化草稿（人在回路确认入口，version+1）
#[tauri::command]
pub async fn apply_skill_suggestion(
    state: State<'_, AppState>,
    suggestion_id: String,
    new_template: Option<String>,
) -> Result<PrdSkill, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.apply_suggestion(&suggestion_id, new_template.as_deref()).await
}

/// 应用 AI 优化草稿（无关联建议，version+1）
#[tauri::command]
pub async fn apply_skill_draft(
    state: State<'_, AppState>,
    skill_id: String,
    new_template: String,
) -> Result<PrdSkill, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.apply_draft(&skill_id, &new_template).await
}

#[tauri::command]
pub async fn list_skill_versions(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<crate::db::SkillVersion>, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.list_versions(&skill_id).await
}

/// 棘轮回退：应用指定版本快照（写为新版本）
#[tauri::command]
pub async fn revert_skill_version(
    state: State<'_, AppState>,
    skill_id: String,
    version: i32,
) -> Result<PrdSkill, String> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.revert(&skill_id, version).await
}

/// 按 Skill 查询执行日志（进化视图日志 Tab）
#[tauri::command]
pub async fn get_skill_execution_logs_by_skill(
    state: State<'_, AppState>,
    skill_id: String,
    limit: Option<i64>,
) -> Result<Vec<crate::db::SkillExecutionLog>, String> {
    let db = state.db.lock().await;
    db.get_skill_execution_logs_by_skill(&skill_id, limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_custom_skill_from_builtin(
    state: State<'_, AppState>,
    parent_id: String,
    name: String,
    template: Option<String>,
) -> Result<PrdSkill, String> {
    let svc = PrdSkillService::new(&state.db);
    svc.create_custom_skill(&parent_id, &name, template.as_deref()).await
}

// ==================== 统计命令 ====================

#[tauri::command]
pub async fn get_statistics(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let svc = StatsService::new(&state.db, &state.cache);
    svc.get_statistics().await
}

// ==================== 缓存命令 ====================

#[tauri::command]
pub async fn get_cache_stats(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let stats = state.cache.manager.stats().await;

    Ok(serde_json::json!({
        "hits": stats.hits,
        "misses": stats.misses,
        "hit_rate": format!("{:.2}%", stats.hit_rate() * 100.0),
        "size": stats.l1_size,
        "enabled": state.cache.manager.is_enabled(),
    }))
}

#[tauri::command]
pub async fn clear_cache(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.cache.manager.clear().await;
    info!("缓存已清空");
    Ok(())
}

// ==================== 设置命令 ====================

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    let svc = SettingService::new(&state.db, &state.cache);
    svc.get_setting(&key).await
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let svc = SettingService::new(&state.db, &state.cache);
    svc.set_setting(&key, &value).await
}

// ==================== 加密命令 ====================

#[tauri::command]
pub async fn encrypt_api_key(api_key: String, password: String) -> Result<String, String> {
    crate::crypto::encrypt_api_key(&api_key, &password).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn decrypt_api_key(encrypted: String, password: String) -> Result<String, String> {
    // 先做格式预检，给出可读错误，而不是把底层解析错误直接抛给用户
    if !crate::crypto::is_encrypted_format(&encrypted) {
        return Err("密文格式不正确：请提供由 encrypt_api_key 生成的 base64 字符串".to_string());
    }
    crate::crypto::decrypt_api_key(&encrypted, &password).map_err(|e| e.to_string())
}

// ==================== 会话交互与消息（模块 16 / F076–F083） ====================

/// 默认工具白名单设置键
const DEFAULT_PERMISSION_MODE: &str = "acceptEdits";

/// 读取布尔设置项
async fn read_bool_setting(state: &AppState, key: &str, default: bool) -> bool {
    let db = state.db.lock().await;
    match db.get_setting(key).ok().flatten().as_deref() {
        Some("true") | Some("1") | Some("yes") => true,
        Some("false") | Some("0") | Some("no") => false,
        _ => default,
    }
}

/// 读取字符串设置项（空串视为未设置）
async fn read_string_setting(state: &AppState, key: &str) -> Option<String> {
    let db = state.db.lock().await;
    db.get_setting(key)
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
}

/// 组装消息块写入上下文（关联任务/需求/项目 + 原始事件保留策略）
async fn build_message_context(state: &AppState, session: &AgentSession) -> MessageContext {
    let keep_raw_json = read_bool_setting(state, "session_keep_raw_json", false).await;
    MessageContext {
        task_id: session.related_task_id.clone(),
        requirement_id: session.related_requirement_id.clone(),
        project_id: session.project_id.clone(),
        keep_raw_json,
    }
}

/// 上下文注入文本（Q7：任务/需求绑定需在消息链路中体现）
async fn build_context_prompt(state: &AppState, ctx: &MessageContext) -> Option<String> {
    let db = state.db.lock().await;
    let mut parts: Vec<String> = Vec::new();

    if let Some(task_id) = ctx.task_id.as_deref() {
        if let Ok(Some(task)) = db.get_task(task_id) {
            parts.push(format!(
                "当前任务：{}（类型 {}，状态 {}）",
                task.title, task.task_type, task.status
            ));
        }
    }
    if let Some(req_id) = ctx.requirement_id.as_deref() {
        if let Ok(Some(req)) = db.get_requirement(req_id) {
            parts.push(format!(
                "所属需求：{}（优先级 {}，状态 {}）",
                req.title, req.priority, req.status
            ));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("[ACE 会话上下文] {}", parts.join("；")))
    }
}

/// 任务类型 → 流水线环节（用于解析环节 Skill）
fn stage_for_task(task_type: Option<&str>) -> &'static str {
    match task_type {
        Some("requirement") => "requirement_analysis",
        Some("testing") => "testing",
        Some("review") => "review",
        // development / bug_fix / 未关联任务 → 代码生成（开发）环节
        _ => "code_generation",
    }
}

/// 解析会话应加载的开发类 Skill：项目默认 Skill + 任务所属环节 Skill（按 skill_id 去重）
async fn resolve_session_dev_skills(
    state: &AppState,
    project_id: Option<&str>,
    stage: &str,
) -> Vec<crate::db::PrdSkill> {
    let Some(pid) = project_id.filter(|p| !p.is_empty()) else {
        return Vec::new();
    };
    let db = state.db.lock().await;
    let mut ids: Vec<String> = Vec::new();
    if let Ok(list) = db.list_project_skills(pid) {
        if let Some(ps) = list.into_iter().find(|p| p.is_default) {
            ids.push(ps.skill_id);
        }
    }
    if let Ok(Some(cfg)) = db.get_stage_skill_config(pid, stage) {
        ids.push(cfg.skill_id);
    }
    ids.sort();
    ids.dedup();
    ids.iter()
        .filter_map(|id| db.get_prd_skill(id).ok().flatten())
        .collect()
}

/// 把已加载 Skill 的模板拼成系统提示附加段
fn build_skill_prompt(skills: &[crate::db::PrdSkill]) -> Option<String> {
    if skills.is_empty() {
        return None;
    }
    let mut lines = vec![
        "[ACE 开发 Skill] 本会话已加载以下项目 Skill，完成任务时请遵循其规范：".to_string(),
    ];
    for s in skills {
        lines.push(format!("## {}（{}）\n{}", s.name, s.skill_type, s.template));
    }
    Some(lines.join("\n\n"))
}

/// 写入会话上下文卡片（`context` 块，前端以卡片形式展示）
async fn append_context_block(state: &AppState, session: &AgentSession) -> Result<(), String> {
    let ctx = build_message_context(state, session).await;
    let payload = serde_json::json!({
        "mode": session.mode,
        "permission_mode": session.permission_mode,
        "model": session.model,
        "work_dir": session.work_dir,
        "agent_path": session.agent_path,
        "task_id": ctx.task_id,
        "requirement_id": ctx.requirement_id,
        "project_id": ctx.project_id,
    })
    .to_string();

    let db = state.db.lock().await;
    db.append_session_message(
        &session.id,
        &ctx,
        BlockSpec::new("system", block_type::CONTEXT, Some(payload)),
    )
    .map(|_| ())
    .map_err(|e| format!("写入上下文卡片失败: {}", e))
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
) -> Result<AgentSession, String> {
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
    .await
}

/// 结构化会话启动的公共实现（`start_structured_session` 与
/// `create_session_from_task` 共用，保证任务链路与手动启动行为一致）。
#[allow(clippy::too_many_arguments)]
async fn start_structured_session_inner(
    state: &AppState,
    app: &tauri::AppHandle,
    agent_path: String,
    task_id: Option<String>,
    requirement_id: Option<String>,
    work_dir: Option<String>,
    permission_mode: Option<String>,
    model: Option<String>,
) -> Result<AgentSession, String> {
    if !crate::agent::check_agent_available(&agent_path).await {
        return Err("Agent 不可用，请先在 Agent 页面执行「刷新检测」".to_string());
    }

    let agent_id = AgentService::new(&state.db).resolve_agent_id(&agent_path).await?;

    // 关联信息：显式传入优先，其次从任务反查需求与项目
    let (related_task, related_req, project_id, task_title, task_type) = {
        let db = state.db.lock().await;
        let task = match task_id.as_deref() {
            Some(tid) => db.get_task(tid).ok().flatten(),
            None => None,
        };
        let req_id = requirement_id
            .clone()
            .or_else(|| task.as_ref().and_then(|t| t.requirement_id.clone()));
        let requirement = match req_id.as_deref() {
            Some(rid) => db.get_requirement(rid).ok().flatten(),
            None => None,
        };
        let project_id = task
            .as_ref()
            .and_then(|t| t.project_id.clone())
            .or_else(|| requirement.as_ref().map(|r| r.project_id.clone()))
            .filter(|s| !s.is_empty());
        (
            task_id.clone().or_else(|| task.as_ref().map(|t| t.id.clone())),
            req_id,
            project_id,
            task.as_ref().map(|t| t.title.clone()),
            task.as_ref().map(|t| t.task_type.clone()),
        )
    };

    let work_dir = match work_dir.filter(|s| !s.trim().is_empty()) {
        Some(dir) => Some(dir),
        None => read_string_setting(state, "project_path").await,
    };

    let permission_mode = match permission_mode.filter(|s| !s.trim().is_empty()) {
        Some(m) => m,
        None => read_string_setting(state, "session_permission_mode")
            .await
            .unwrap_or_else(|| DEFAULT_PERMISSION_MODE.to_string()),
    };

    // R2：默认工具白名单（设置键 session_allowed_tools，逗号分隔），创建会话时预授权，
    // 避免 WebSearch 等工具首次调用即被权限策略拒绝
    let default_allowed_tools: Vec<String> = read_string_setting(state, "session_allowed_tools")
        .await
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let cli_session_id = uuid::Uuid::new_v4().to_string();

    // Skill 加载所需的关联信息（会话构建会消耗这些所有权，先留副本）
    let skills_task_id = related_task.clone();
    let skills_req_id = related_req.clone();
    let skills_project_id = project_id.clone();

    let session = {
        let db = state.db.lock().await;
        db.create_agent_session_full(&crate::db::NewAgentSession {
            agent_id,
            task_id: related_task.clone(),
            project_id,
            related_task_id: related_task,
            related_requirement_id: related_req,
            title: task_title,
            work_dir,
            agent_path: Some(agent_path.clone()),
            mode: session_mode::STRUCTURED.to_string(),
            permission_mode: Some(permission_mode.clone()),
            model,
            cli_session_id: Some(cli_session_id),
        })
        .map_err(|e| format!("创建会话失败: {}", e))?
    };
    let mut session = session;

    // 会话级白名单沉淀：默认白名单写入 permission_allowlist，
    // 后续"始终允许并重试"（rebuild_structured_session）在此基础上合并
    let allowlist_json =
        serde_json::to_string(&default_allowed_tools).unwrap_or_else(|_| "[]".to_string());
    {
        let db = state.db.lock().await;
        if let Err(e) = db.update_session_meta(
            &session.id,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&allowlist_json),
        ) {
            tracing::warn!("写入会话工具白名单失败: {}", e);
        }
    }
    session.permission_allowlist = Some(allowlist_json);

    if let Err(e) = append_context_block(state, &session).await {
        error!("写入上下文卡片失败: {}", e);
    }

    // 加载开发类 Skill：项目默认 Skill + 任务所属环节 Skill
    // （写入 agent_sessions.skills 供前端展示，并记录 session_start 执行日志）
    let dev_skills = resolve_session_dev_skills(
        state,
        skills_project_id.as_deref(),
        stage_for_task(task_type.as_deref()),
    )
    .await;
    if !dev_skills.is_empty() {
        let skills_json = serde_json::Value::Array(
            dev_skills
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "skill_id": s.id,
                        "skill_name": s.name,
                        "skill_type": s.skill_type,
                    })
                })
                .collect(),
        )
        .to_string();
        let db = state.db.lock().await;
        if let Err(e) = db.set_session_skills(&session.id, &skills_json) {
            tracing::warn!("写入会话 Skill 列表失败: {}", e);
        }
        for skill in &dev_skills {
            if let Err(e) = db.record_skill_execution_log(
                skills_task_id.as_deref(),
                Some(&session.id),
                skills_req_id.as_deref(),
                &skill.id,
                &skill.name,
                &skill.skill_type,
                "session_start",
                None,
                Some("会话启动时加载开发 Skill"),
                0,
                "success",
                None,
                None,
            ) {
                tracing::warn!("记录会话启动 Skill 日志失败: {}", e);
            }
        }
    }

    let ctx = build_message_context(state, &session).await;
    let base_prompt = if read_bool_setting(state, "session_inject_context", true).await {
        build_context_prompt(state, &ctx).await
    } else {
        None
    };
    // 开发 Skill 模板注入系统提示，与任务/需求上下文提示拼接
    let append_system_prompt = match (base_prompt, build_skill_prompt(&dev_skills)) {
        (Some(base), Some(skill)) => Some(format!("{}\n\n{}", base, skill)),
        (None, Some(skill)) => Some(skill),
        (base, _) => base,
    };

    let mut cfg = SessionLaunchConfig::new(&agent_path);
    cfg.work_dir = session.work_dir.clone();
    cfg.cli_session_id = session.cli_session_id.clone();
    cfg.model = session.model.clone();
    cfg.permission_mode = permission_mode.clone();
    cfg.allowed_tools = default_allowed_tools;
    cfg.append_system_prompt = append_system_prompt;

    if let Err(e) = session_driver::spawn_structured_session(
        state.db.clone(),
        app.clone(),
        state.session_registry.clone(),
        session.id.clone(),
        ctx,
        cfg,
    )
    .await
    {
        let db = state.db.lock().await;
        let _ = db.finish_agent_session(&session.id, "failed", Some(-1));
        return Err(e);
    }

    info!(
        "结构化会话已创建: session_id={}, permission_mode={}",
        session.id, permission_mode
    );
    Ok(session)
}

/// 下发一条用户指令（F077）
#[tauri::command]
pub async fn send_session_message(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
    content: String,
) -> Result<SessionMessage, String> {
    let session = {
        let db = state.db.lock().await;
        db.get_agent_session(&session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("会话不存在: {}", session_id))?
    };

    if !session_driver::is_active(&state.session_registry, &session_id).await {
        return Err("会话进程未在运行，请重建或续接会话".to_string());
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
    .await
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
) -> Result<serde_json::Value, String> {
    if contents.is_empty() {
        return Err("附件内容为空".to_string());
    }
    const MAX_SIZE: usize = 20 * 1024 * 1024;
    if contents.len() > MAX_SIZE {
        return Err(format!("附件不能超过 {}MB", MAX_SIZE / 1024 / 1024));
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
) -> Result<PagedResult<SessionMessage>, String> {
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
) -> Result<(), String> {
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
    .await
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
) -> Result<AgentSession, String> {
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
    .await
}

/// 续接既有 CLI 会话（F082）
#[tauri::command]
pub async fn resume_session(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    session_id: String,
) -> Result<AgentSession, String> {
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
    .await
}

/// 重建结构化会话的公共实现
#[allow(clippy::too_many_arguments)]
async fn rebuild_structured_session(
    state: &AppState,
    app: &tauri::AppHandle,
    session_id: &str,
    mode: Option<String>,
    permission_mode: Option<String>,
    model: Option<String>,
    allowed_tools: Option<Vec<String>>,
    resend_last: bool,
) -> Result<AgentSession, String> {
    let session = {
        let db = state.db.lock().await;
        db.get_agent_session(session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("会话不存在: {}", session_id))?
    };

    if session_driver::is_active(&state.session_registry, session_id).await {
        return Err("会话正在运行，请先中断后再重建".to_string());
    }

    let agent_path = session
        .agent_path
        .clone()
        .ok_or("会话缺少 Agent 路径，无法重建（请重新创建会话）")?;

    let mode = mode
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| {
            session
                .mode
                .clone()
                .unwrap_or_else(|| session_mode::STRUCTURED.to_string())
        });
    if !agent_adapter::is_structured_mode(Some(&mode)) {
        return Err("当前仅支持重建为结构化（消息）模式；终端模式请在 Agent 页面启动".to_string());
    }

    let permission_mode = permission_mode.or_else(|| session.permission_mode.clone());
    let model = model.or_else(|| session.model.clone());

    // "始终允许"沉淀：并入既有白名单
    let merged_tools = {
        let mut merged: Vec<String> = session
            .permission_allowlist
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();
        if let Some(extra) = allowed_tools {
            for tool in extra {
                if !tool.trim().is_empty() && !merged.contains(&tool) {
                    merged.push(tool);
                }
            }
        }
        merged
    };

    // 复位会话状态并回写元信息
    let session = {
        let db = state.db.lock().await;
        db.restart_agent_session(session_id)
            .map_err(|e| format!("重置会话状态失败: {}", e))?;
        db.update_session_meta(
            session_id,
            Some(&mode),
            permission_mode.as_deref(),
            model.as_deref(),
            Some("messages"),
            None,
            None,
            Some(&serde_json::to_string(&merged_tools).unwrap_or_else(|_| "[]".to_string())),
        )
        .map_err(|e| format!("更新会话元信息失败: {}", e))?;
        if let Err(e) = db.fail_streaming_session_messages(session_id) {
            error!("收尾残留消息失败: {}", e);
        }
        db.get_agent_session(session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "会话不存在".to_string())?
    };

    let ctx = build_message_context(state, &session).await;
    let append_system_prompt = if read_bool_setting(state, "session_inject_context", true).await {
        build_context_prompt(state, &ctx).await
    } else {
        None
    };

    let cli_session_id = session
        .cli_session_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mut cfg = SessionLaunchConfig::new(&agent_path);
    cfg.work_dir = session.work_dir.clone();
    cfg.cli_session_id = Some(cli_session_id.clone());
    // 已有 CLI 会话时走续接，保留上下文
    cfg.resume_cli_session_id = session.cli_session_id.clone().filter(|s| !s.is_empty());
    cfg.model = session.model.clone();
    cfg.permission_mode = permission_mode
        .clone()
        .unwrap_or_else(|| DEFAULT_PERMISSION_MODE.to_string());
    cfg.allowed_tools = merged_tools;
    cfg.append_system_prompt = append_system_prompt;

    if let Err(e) = session_driver::spawn_structured_session(
        state.db.clone(),
        app.clone(),
        state.session_registry.clone(),
        session_id.to_string(),
        ctx.clone(),
        cfg,
    )
    .await
    {
        let db = state.db.lock().await;
        let _ = db.finish_agent_session(session_id, "failed", Some(-1));
        return Err(e);
    }

    if resend_last {
        let last_user_text = {
            let db = state.db.lock().await;
            db.list_session_messages(session_id, None, 200)
                .unwrap_or_default()
                .into_iter()
                .rev()
                .find(|m| m.role == "user" && m.block_type == block_type::TEXT)
                .and_then(|m| m.content)
        };
        if let Some(content) = last_user_text {
            if let Err(e) = session_driver::send_user_message(
                &state.db,
                app,
                &state.session_registry,
                session_id,
                &ctx,
                &content,
            )
            .await
            {
                error!("重发上一条指令失败: {}", e);
            }
        }
    }

    let db = state.db.lock().await;
    db.get_agent_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "会话不存在".to_string())
}

// ==================== PTY 终端命令 ====================

#[tauri::command]
pub async fn pty_spawn(
    state: State<'_, AppState>,
    agent_path: String,
    work_dir: String,
    env_vars: Option<Vec<(String, String)>>,
) -> Result<String, String> {
    let svc = PtyService::new(&state.pty_manager);
    let env = env_vars.unwrap_or_default();
    svc.spawn(&agent_path, &work_dir, env).await
}

#[tauri::command]
pub async fn pty_write(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<(), String> {
    let svc = PtyService::new(&state.pty_manager);
    svc.write(&session_id, &input).await
}

#[tauri::command]
pub async fn pty_resize(
    state: State<'_, AppState>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let svc = PtyService::new(&state.pty_manager);
    svc.resize(&session_id, rows, cols).await
}

#[tauri::command]
pub async fn pty_kill(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let svc = PtyService::new(&state.pty_manager);
    svc.kill(&session_id).await
}

#[tauri::command]
pub async fn pty_get_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::pty::PtySessionInfo>, String> {
    let svc = PtyService::new(&state.pty_manager);
    Ok(svc.get_sessions().await)
}

#[tauri::command]
pub async fn pty_get_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<crate::pty::PtySessionInfo>, String> {
    let svc = PtyService::new(&state.pty_manager);
    Ok(svc.get_session(&session_id).await)
}

/// 终端视图附着结果：真实 PTY id + 挂载前的输出回放
#[derive(serde::Serialize)]
pub struct PtyAttach {
    pub pty_id: String,
    pub replay: String,
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
) -> Result<PtyAttach, String> {
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
    db_session_id: String,
) -> Result<String, String> {
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
        return Err("结构化会话不支持终端进程重启".to_string());
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
        return Err(format!("Agent 不可用: {}", agent_path));
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

    watch_terminal_session_exit(state.inner(), pty_id.clone(), db_session_id);
    Ok(pty_id)
}

// ==================== 项目管理命令 ====================

#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    key: Option<String>,
) -> Result<Project, String> {
    // 输入验证
    validate_project(&name).map_err(|e| e.to_string())?;

    let svc = ProjectService::new(&state.db, &state.cache);
    svc.create_project(&name, description.as_deref(), key.as_deref()).await
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, AppState>,
) -> Result<Vec<Project>, String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.list_projects().await
}

#[tauri::command]
pub async fn get_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Project>, String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.get_project(&id).await
}

#[tauri::command]
pub async fn update_project(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    key: Option<String>,
    status: Option<String>,
) -> Result<(), String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.update_project(&id, name.as_deref(), description.as_deref(), key.as_deref(), status.as_deref()).await
}

#[tauri::command]
pub async fn delete_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = ProjectService::new(&state.db, &state.cache);

    // 检查是否有关联数据
    {
        let db = state.db.lock().await;
        let req_count = db.get_requirement_count_by_project(&id)
            .map_err(|e| format!("查询关联需求失败: {}", e))?;
        let task_count = db.get_task_count_by_project(&id)
            .map_err(|e| format!("查询关联任务失败: {}", e))?;
        let skill_count = db.list_project_skills(&id)
            .map(|v| v.len())
            .unwrap_or(0);
        let mcp_count = db.list_project_mcps(&id)
            .map(|v| v.len())
            .unwrap_or(0);

        if req_count > 0 || task_count > 0 || skill_count > 0 || mcp_count > 0 {
            return Err(format!(
                "项目下有 {} 个需求、{} 个任务、{} 个 Skill 配置和 {} 个 MCP 配置，请先清理",
                req_count, task_count, skill_count, mcp_count
            ));
        }
    }

    svc.delete_project(&id).await
}

// ==================== 项目 Skill 配置命令 ====================

#[tauri::command]
pub async fn list_project_skills(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<crate::db::ProjectSkill>, String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.list_project_skills(&project_id).await
}

#[tauri::command]
pub async fn add_project_skill(
    state: State<'_, AppState>,
    project_id: String,
    skill_id: String,
    is_default: Option<bool>,
    custom_params: Option<String>,
) -> Result<crate::db::ProjectSkill, String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.add_project_skill(&project_id, &skill_id, is_default, custom_params.as_deref()).await
}

#[tauri::command]
pub async fn remove_project_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.remove_project_skill(&id).await
}

// ==================== 项目 MCP 配置命令 ====================

#[tauri::command]
pub async fn list_project_mcps(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<crate::db::ProjectMcp>, String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.list_project_mcps(&project_id).await
}

#[tauri::command]
pub async fn add_project_mcp(
    state: State<'_, AppState>,
    project_id: String,
    mcp_name: String,
    endpoint: String,
    description: Option<String>,
    is_default: Option<bool>,
    auth_config: Option<String>,
    custom_params: Option<String>,
) -> Result<crate::db::ProjectMcp, String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.add_project_mcp(&project_id, &mcp_name, &endpoint, description.as_deref(), is_default, auth_config.as_deref(), custom_params.as_deref()).await
}

#[tauri::command]
pub async fn remove_project_mcp(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.remove_project_mcp(&id).await
}

// ==================== 环节 Skill 配置命令 ====================

#[tauri::command]
pub async fn list_stage_skill_configs(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<crate::db::StageSkillConfig>, String> {
    let db = state.db.lock().await;
    db.list_stage_skill_configs(&project_id)
        .map_err(|e| format!("获取环节 Skill 配置失败: {}", e))
}

#[tauri::command]
pub async fn get_stage_skill_config(
    state: State<'_, AppState>,
    project_id: String,
    stage: String,
) -> Result<Option<crate::db::StageSkillConfig>, String> {
    let db = state.db.lock().await;
    db.get_stage_skill_config(&project_id, &stage)
        .map_err(|e| format!("获取环节 Skill 配置失败: {}", e))
}

#[tauri::command]
pub async fn upsert_stage_skill_config(
    state: State<'_, AppState>,
    project_id: String,
    stage: String,
    skill_id: String,
    is_default: bool,
    custom_params: Option<String>,
) -> Result<crate::db::StageSkillConfig, String> {
    let db = state.db.lock().await;
    db.upsert_stage_skill_config(&project_id, &stage, &skill_id, is_default, custom_params.as_deref())
        .map_err(|e| format!("保存环节 Skill 配置失败: {}", e))
}

#[tauri::command]
pub async fn remove_stage_skill_config(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.remove_stage_skill_config(&id)
        .map_err(|e| format!("删除环节 Skill 配置失败: {}", e))
}

#[tauri::command]
pub async fn get_pipeline_stages() -> Result<Vec<serde_json::Value>, String> {
    use crate::db::PipelineStage;
    Ok(PipelineStage::all().iter().map(|s| {
        serde_json::json!({
            "value": s.as_str(),
            "label": s.display_name(),
        })
    }).collect())
}

// ==================== Skill/MCP 执行日志命令 ====================

#[tauri::command]
pub async fn get_skill_execution_logs(
    state: State<'_, AppState>,
    task_id: Option<String>,
    session_id: Option<String>,
) -> Result<Vec<crate::db::SkillExecutionLog>, String> {
    let db = state.db.lock().await;
    if let Some(tid) = &task_id {
        db.get_skill_execution_logs_by_task(tid).map_err(|e| {
            error!("获取Skill执行日志失败: {}", e);
            format!("获取Skill执行日志失败: {}", e)
        })
    } else if let Some(sid) = &session_id {
        db.get_skill_execution_logs_by_session(sid).map_err(|e| {
            error!("获取Skill执行日志失败: {}", e);
            format!("获取Skill执行日志失败: {}", e)
        })
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn get_mcp_execution_logs(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<crate::db::McpExecutionLog>, String> {
    let db = state.db.lock().await;
    db.get_mcp_execution_logs(&session_id).map_err(|e| {
        error!("获取MCP执行日志失败: {}", e);
        format!("获取MCP执行日志失败: {}", e)
    })
}

#[tauri::command]
pub async fn get_skill_usage_stats(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().await;
    db.get_skill_usage_stats().map_err(|e| {
        error!("获取Skill统计失败: {}", e);
        format!("获取Skill统计失败: {}", e)
    })
}

#[tauri::command]
pub async fn get_mcp_usage_stats(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().await;
    db.get_mcp_usage_stats().map_err(|e| {
        error!("获取MCP统计失败: {}", e);
        format!("获取MCP统计失败: {}", e)
    })
}

// ==================== 审计日志 ====================

#[tauri::command]
pub async fn get_audit_logs(
    state: State<'_, AppState>,
    limit: Option<i64>,
    entity_type: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().await;
    db.list_audit_logs(limit, entity_type.as_deref())
        .map_err(|e| format!("获取审计日志失败: {}", e))
}

// ==================== Git 操作命令 ====================

#[tauri::command]
pub async fn git_get_status(
    work_dir: String,
) -> Result<serde_json::Value, String> {
    let svc = GitService::new();
    svc.get_status(&work_dir)
}

#[tauri::command]
pub async fn git_get_diff(
    work_dir: String,
    file_path: Option<String>,
) -> Result<String, String> {
    let svc = GitService::new();
    svc.get_diff(&work_dir, file_path.as_deref())
}

#[tauri::command]
pub async fn git_create_commit(
    state: State<'_, AppState>,
    work_dir: String,
    message: String,
    task_id: Option<String>,
    session_id: Option<String>,
    include_untracked: Option<bool>,
) -> Result<serde_json::Value, String> {
    let svc = GitService::new();
    // 默认包含未跟踪文件，保证 Agent 新建的文件也能被提交
    let result = svc.create_commit(&work_dir, &message, include_untracked.unwrap_or(true))?;

    // 落库：建立"提交 ↔ 任务/会话"的可追溯关系
    let commit_hash = result
        .get("commit_hash")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let files_changed = result
        .get("files_changed")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let branch = svc
        .get_status(&work_dir)
        .ok()
        .and_then(|s| s.get("branch").and_then(|v| v.as_str()).map(|b| b.to_string()));

    if !commit_hash.is_empty() {
        let db = state.db.lock().await;
        match db.insert_git_commit(
            task_id.as_deref(),
            session_id.as_deref(),
            &commit_hash,
            branch.as_deref(),
            Some(&message),
            files_changed,
        ) {
            Ok(id) => info!("提交记录已落库: id={}", id),
            // 落库失败不回滚已完成的提交，仅记录错误
            Err(e) => error!("提交记录落库失败: {}", e),
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn git_log(
    work_dir: String,
    limit: Option<i32>,
) -> Result<Vec<serde_json::Value>, String> {
    let svc = GitService::new();
    svc.log(&work_dir, limit)
}

/// 查询平台内记录的 Git 提交（可按任务或会话过滤）
#[tauri::command]
pub async fn list_git_commits(
    state: State<'_, AppState>,
    task_id: Option<String>,
    session_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().await;
    db.list_git_commits(task_id.as_deref(), session_id.as_deref(), limit)
        .map_err(|e| format!("获取提交记录失败: {}", e))
}

// ==================== 数据导入导出命令 ====================

/// 导出全量数据快照（JSON）
#[tauri::command]
pub async fn export_data(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let svc = DataService::new(&state.db, &state.cache);
    svc.export_data().await
}

/// 导入数据快照。
///
/// 正式导入前会对当前数据库做一次文件级备份，避免误操作导致数据丢失。
#[tauri::command]
pub async fn import_data(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: String,
    replace: Option<bool>,
    dry_run: Option<bool>,
) -> Result<serde_json::Value, String> {
    let parsed: serde_json::Value = serde_json::from_str(&payload)
        .map_err(|e| format!("导入内容不是合法 JSON: {}", e))?;

    let replace = replace.unwrap_or(false);
    let dry_run = dry_run.unwrap_or(false);

    let svc = DataService::new(&state.db, &state.cache);

    // 正式导入前先备份当前数据库（含 WAL 检查点，保留最近 5 份）
    if !dry_run {
        if let Ok(dir) = app.path().app_data_dir() {
            let db_path = dir.join("database.db");
            if db_path.exists() {
                let db = state.db.lock().await;
                match db.backup_to(&db_path, 5) {
                    Ok(path) => info!("导入前已备份数据库: {:?}", path),
                    Err(e) => error!("导入前备份数据库失败: {}", e),
                }
            }
        }
    }

    svc.import_data(&parsed, replace, dry_run).await
}
