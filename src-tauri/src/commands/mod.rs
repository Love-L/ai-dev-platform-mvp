//! Tauri 命令层（薄封装）。
//!
//! 2026-10 拆分说明（P2-4）：原 commands.rs（2500+ 行 / 101 命令）按业务域拆为
//! 本目录下的子模块，公共 use 与共享 helper 保留在此，子模块经 `use super::*;` 复用。
//! 对外路径不变（`commands::xxx`），main.rs 注册与集成测试引用零改动。

use crate::AppState;
use crate::errors::AppError;
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
async fn append_context_block(state: &AppState, session: &AgentSession) -> Result<(), AppError> {
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
    .map_err(|e| format!("写入上下文卡片失败: {}", e)).map_err(AppError::legacy)
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
) -> Result<AgentSession, AppError> {
    if !crate::agent::check_agent_available(&agent_path).await {
        return Err(AppError::legacy("Agent 不可用，请先在 Agent 页面执行「刷新检测」".to_string()));
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
    .await.map_err(AppError::legacy)
    {
        let db = state.db.lock().await;
        let _ = db.finish_agent_session(&session.id, "failed", Some(-1));
        return Err(AppError::legacy(e));
    }

    info!(
        "结构化会话已创建: session_id={}, permission_mode={}",
        session.id, permission_mode
    );
    Ok(session)
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
) -> Result<AgentSession, AppError> {
    let session = {
        let db = state.db.lock().await;
        db.get_agent_session(session_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("会话不存在: {}", session_id))?
    };

    if session_driver::is_active(&state.session_registry, session_id).await {
        return Err(AppError::legacy("会话正在运行，请先中断后再重建".to_string()));
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
        return Err(AppError::legacy("当前仅支持重建为结构化（消息）模式；终端模式请在 Agent 页面启动".to_string()));
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
    .await.map_err(AppError::legacy)
    {
        let db = state.db.lock().await;
        let _ = db.finish_agent_session(session_id, "failed", Some(-1));
        return Err(AppError::legacy(e));
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
            .await.map_err(AppError::legacy)
            {
                error!("重发上一条指令失败: {}", e);
            }
        }
    }

    let db = state.db.lock().await;
    db.get_agent_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::NotFound("会话不存在".to_string()))
}

/// 终端视图附着结果：真实 PTY id + 挂载前的输出回放
#[derive(serde::Serialize)]
pub struct PtyAttach {
    pub pty_id: String,
    pub replay: String,
}

mod requirements;
pub use requirements::*;

mod tasks;
pub use tasks::*;

mod projects;
pub use projects::*;

mod skills;
pub use skills::*;

mod agents;
pub use agents::*;

mod sessions;
pub use sessions::*;

mod pty;
pub use pty::*;

mod tokens;
pub use tokens::*;

mod mcp;
pub use mcp::*;

mod git;
pub use git::*;

mod data;
pub use data::*;

mod system;
pub use system::*;

mod settings;
pub use settings::*;
