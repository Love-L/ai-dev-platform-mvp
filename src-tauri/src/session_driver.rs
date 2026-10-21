//! 结构化会话驱动（模块 16）。
//!
//! 职责：
//! 1. 以 `stream-json` 双向模式启动 Agent CLI 子进程；
//! 2. 逐行解析 stdout，经适配层归一化后**落库**（完整块）并**推送事件**（含增量）；
//! 3. 提供指令下发与中断能力；
//! 4. 进程退出时收尾会话状态与残留 `streaming` 块。
//!
//! 增量（`session:delta`）不落库，仅用于前端实时渲染；完整块落库后以
//! `session:message` 下发，前端以落库内容覆盖增量缓冲（幂等）。

use std::collections::HashMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{oneshot, Mutex};
use tracing::{error, info, warn};

use crate::agent_adapter::{self, AgentEvent, SessionLaunchConfig};
use crate::db::{block_type, SessionMessage};
use crate::db::session_messages::{BlockSpec, MessageContext};
use crate::db::Database;

/// 事件名常量（与前端 `listen` 保持一致）
pub mod events {
    pub const MESSAGE: &str = "session:message";
    pub const DELTA: &str = "session:delta";
    pub const TOOL: &str = "session:tool";
    pub const RESULT: &str = "session:result";
    pub const ERROR: &str = "session:error";
    pub const NOTICE: &str = "session:notice";
    /// 会话生命周期状态变化（`{ session_id, status }`），供列表/详情自动刷新
    pub const STATUS: &str = "session:status";
}

/// 推送会话状态变化事件（前端订阅后自动刷新，无需手动轮询）
pub fn emit_session_status(app: &AppHandle, session_id: &str, status: &str) {
    emit(
        app,
        events::STATUS,
        serde_json::json!({ "session_id": session_id, "status": status }),
    );
}

/// 活动会话句柄
pub struct SessionHandle {
    /// 子进程标准输入（用于下发用户消息）
    pub stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    /// 中断信号（发送后由读取任务 kill 子进程）
    pub shutdown: Option<oneshot::Sender<()>>,
    /// 启动使用的二进制路径（供"允许并重试"重建会话）
    pub agent_path: String,
}

/// 活动结构化会话注册表
pub type SessionRegistry = Arc<Mutex<HashMap<String, SessionHandle>>>;

/// 创建空注册表
pub fn new_registry() -> SessionRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

fn emit(app: &AppHandle, event: &str, payload: serde_json::Value) {
    if let Err(e) = app.emit(event, payload) {
        warn!("会话事件推送失败: event={}, err={}", event, e);
    }
}

/// 启动结构化会话。
///
/// 调用前应已创建 `agent_sessions` 记录（`mode = structured`）。
pub async fn spawn_structured_session(
    db: Arc<Mutex<Database>>,
    app: AppHandle,
    registry: SessionRegistry,
    session_id: String,
    ctx: MessageContext,
    cfg: SessionLaunchConfig,
) -> Result<(), String> {
    let mut cmd = agent_adapter::adapter_for(&cfg.agent_path)
        .build_session_command(&cfg)
        .ok_or_else(|| "该 Agent 不支持结构化会话（缺少可执行路径或未适配 L3）".to_string())?;

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动结构化会话进程失败: {}", e))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "无法获取子进程 stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法获取子进程 stdout".to_string())?;
    let stderr = child.stderr.take();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    registry.lock().await.insert(
        session_id.clone(),
        SessionHandle {
            stdin: Arc::new(Mutex::new(stdin)),
            shutdown: Some(shutdown_tx),
            agent_path: cfg.agent_path.clone(),
        },
    );

    info!(
        "结构化会话已启动: session={}, agent={}, permission_mode={}",
        session_id, cfg.agent_path, cfg.permission_mode
    );
    emit_session_status(&app, &session_id, "running");

    // 会话的 Agent 适配器（多 Agent 适配）：驱动层只依赖 trait 与归一化事件
    let adapter = agent_adapter::adapter_for(&cfg.agent_path);

    // stderr 仅用于排障，单独消费避免管道写满阻塞子进程
    if let Some(stderr) = stderr {
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    warn!("[agent stderr] {}", line);
                }
            }
        });
    }

    let task_db = db.clone();
    let task_app = app.clone();
    let task_registry = registry.clone();

    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut shutdown = Box::pin(shutdown_rx);
        let mut exit_status: Option<i32> = None;

        loop {
            tokio::select! {
                biased;

                _ = &mut shutdown => {
                    info!("收到中断信号，终止会话进程: session={}", session_id);
                    let _ = child.start_kill();
                    break;
                }

                line = lines.next_line() => {
                    match line {
                        Ok(Some(raw)) => {
                            handle_line(&task_db, &task_app, adapter, &session_id, &ctx, &raw)
                                .await;
                        }
                        Ok(None) => break,
                        Err(e) => {
                            error!("读取会话输出失败: session={}, err={}", session_id, e);
                            let detail = format!("读取会话输出失败: {}", e);
                            // 错误也作为消息块落库，保证回放时可见（F086）
                            {
                                let db = task_db.lock().await;
                                if let Ok(msg) = db.append_session_message(
                                    &session_id,
                                    &ctx,
                                    BlockSpec::new(
                                        "system",
                                        block_type::ERROR,
                                        Some(detail.clone()),
                                    ),
                                ) {
                                    emit(
                                        &task_app,
                                        events::MESSAGE,
                                        serde_json::json!({ "session_id": session_id, "message": msg }),
                                    );
                                }
                            }
                            emit(&task_app, events::ERROR, serde_json::json!({
                                "session_id": session_id,
                                "error": detail,
                            }));
                            break;
                        }
                    }
                }
            }
        }

        match child.wait().await {
            Ok(status) => {
                exit_status = status.code();
                info!(
                    "会话进程已退出: session={}, code={:?}",
                    session_id, exit_status
                );
            }
            Err(e) => warn!("等待会话进程退出失败: session={}, err={}", session_id, e),
        }

        // 收尾：残留 streaming 块标记失败 + 会话终态回写
        let code = exit_status.unwrap_or(-1);
        {
            let db = task_db.lock().await;
            if let Err(e) = db.fail_streaming_session_messages(&session_id) {
                warn!("收尾 streaming 消息失败: session={}, err={}", session_id, e);
            }
            let status = if code == 0 { "completed" } else { "failed" };
            if let Err(e) = db.finish_agent_session(&session_id, status, Some(code)) {
                warn!("会话终态回写失败: session={}, err={}", session_id, e);
            }
            // P1-T3：会话终态回写关联任务（推状态 + 回填 actual_hours）
            if status == "completed" {
                match db.sync_task_from_session(&session_id) {
                    Ok(true) => info!("会话 {} 完成，已推进关联任务状态", session_id),
                    Ok(false) => {}
                    Err(e) => warn!("会话→任务回写失败: session={}, err={}", session_id, e),
                }
            }
        }

        task_registry.lock().await.remove(&session_id);
        emit_session_status(&task_app, &session_id, if code == 0 { "completed" } else { "failed" });
        emit(
            &task_app,
            events::NOTICE,
            serde_json::json!({
                "session_id": session_id,
                "level": if code == 0 { "info" } else { "warn" },
                "kind": "exit",
                "message": format!("会话进程已退出（退出码 {}）", code),
            }),
        );
    });

    Ok(())
}

/// 处理一行输出：归一化 → 落库 / 推送
async fn handle_line(
    db: &Arc<Mutex<Database>>,
    app: &AppHandle,
    adapter: &(dyn agent_adapter::AgentAdapter + 'static),
    session_id: &str,
    ctx: &MessageContext,
    raw: &str,
) {
    if raw.trim().is_empty() {
        return;
    }

    for event in adapter.decode_line(raw) {
        match event {
            AgentEvent::Init {
                cli_session_id,
                model,
                tools,
            } => {
                let db = db.lock().await;
                if !cli_session_id.is_empty() {
                    if let Err(e) = db.set_session_cli_id(session_id, &cli_session_id) {
                        warn!("回填 CLI 会话 id 失败: {}", e);
                    }
                }
                let content = format!(
                    "会话已就绪：模型 {}，可用工具 {} 个",
                    model.as_deref().unwrap_or("默认"),
                    tools.len()
                );
                match db.append_session_message(
                    session_id,
                    ctx,
                    BlockSpec::new("system", block_type::SYSTEM, Some(content)),
                ) {
                    Ok(msg) => emit(
                        app,
                        events::MESSAGE,
                        serde_json::json!({ "session_id": session_id, "message": msg }),
                    ),
                    Err(e) => warn!("写入系统消息失败: {}", e),
                }
            }

            AgentEvent::Delta { block_type, delta } => {
                emit(
                    app,
                    events::DELTA,
                    serde_json::json!({
                        "session_id": session_id,
                        "block_type": block_type,
                        "delta": delta,
                    }),
                );
            }

            AgentEvent::ThinkingTokens { total, delta } => {
                emit(
                    app,
                    events::DELTA,
                    serde_json::json!({
                        "session_id": session_id,
                        "block_type": "thinking_tokens",
                        "delta": delta.to_string(),
                        "total": total,
                    }),
                );
            }

            AgentEvent::Block(draft) => {
                let db = db.lock().await;
                let mut spec = BlockSpec::new(&draft.role, &draft.block_type, draft.content);
                spec.tool_name = draft.tool_name.clone();
                spec.tool_use_id = draft.tool_use_id.clone();
                spec.parent_tool_use_id = draft.parent_tool_use_id.clone();
                spec.cli_message_id = draft.cli_message_id.clone();
                spec.raw_json = draft.raw_json.clone();
                spec.tokens = draft.tokens;
                if !draft.status.is_empty() {
                    spec.status = draft.status.clone();
                }

                match db.append_session_message(session_id, ctx, spec) {
                    Ok(msg) => {
                        if draft.block_type == block_type::TOOL_USE {
                            emit(
                                app,
                                events::TOOL,
                                serde_json::json!({
                                    "session_id": session_id,
                                    "phase": "start",
                                    "tool_use_id": draft.tool_use_id,
                                    "tool_name": draft.tool_name,
                                }),
                            );
                        }
                        emit(
                            app,
                            events::MESSAGE,
                            serde_json::json!({ "session_id": session_id, "message": msg }),
                        );
                    }
                    Err(e) => warn!("写入消息块失败: {}", e),
                }
            }

            AgentEvent::ToolResult {
                tool_use_id,
                content,
                is_error,
                raw_json,
            } => {
                let db = db.lock().await;
                let mut spec = BlockSpec::new("user", block_type::TOOL_RESULT, content.clone());
                spec.tool_use_id = tool_use_id.clone();
                spec.raw_json = raw_json;
                spec.status = if is_error {
                    "failed".to_string()
                } else {
                    "completed".to_string()
                };

                match db.append_session_message(session_id, ctx, spec) {
                    Ok(msg) => {
                        emit(
                            app,
                            events::TOOL,
                            serde_json::json!({
                                "session_id": session_id,
                                "phase": "result",
                                "tool_use_id": tool_use_id,
                                "content": content,
                                "is_error": is_error,
                            }),
                        );
                        emit(
                            app,
                            events::MESSAGE,
                            serde_json::json!({ "session_id": session_id, "message": msg }),
                        );
                    }
                    Err(e) => warn!("写入工具结果失败: {}", e),
                }
            }

            AgentEvent::PermissionDenied {
                tool_name,
                tool_use_id,
                message,
            } => {
                let db = db.lock().await;
                let mut spec =
                    BlockSpec::new("system", block_type::PERMISSION_DENIED, Some(message.clone()));
                spec.tool_name = Some(tool_name.clone());
                spec.tool_use_id = tool_use_id.clone();
                spec.status = "denied".to_string();

                if let Ok(msg) = db.append_session_message(session_id, ctx, spec) {
                    emit(
                        app,
                        events::MESSAGE,
                        serde_json::json!({ "session_id": session_id, "message": msg }),
                    );
                }
                emit(
                    app,
                    events::NOTICE,
                    serde_json::json!({
                        "session_id": session_id,
                        "level": "warn",
                        "kind": "permission_denied",
                        "tool_name": tool_name,
                        "message": message,
                    }),
                );
            }

            AgentEvent::TurnResult {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cost_usd,
                duration_ms,
                is_error,
                model,
            } => {
                let total_tokens = input_tokens + output_tokens;
                let summary = serde_json::json!({
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                    "cache_read_input_tokens": cache_read_tokens,
                    "cost_usd": cost_usd,
                    "duration_ms": duration_ms,
                })
                .to_string();

                let db = db.lock().await;
                let mut spec = BlockSpec::new("system", block_type::RESULT, Some(summary));
                spec.tokens = Some(total_tokens);
                spec.status = if is_error {
                    "failed".to_string()
                } else {
                    "completed".to_string()
                };

                if let Ok(msg) = db.append_session_message(session_id, ctx, spec) {
                    emit(
                        app,
                        events::MESSAGE,
                        serde_json::json!({ "session_id": session_id, "message": msg }),
                    );
                }
                if let Err(e) = db.add_session_usage(session_id, total_tokens, cost_usd.unwrap_or(0.0))
                {
                    warn!("累加会话用量失败: {}", e);
                }

                // 消息级记账（F048）：写入 token_usage（按会话/任务归因）并累计预算
                if let Err(e) = db.record_session_token_usage(
                    session_id,
                    ctx.task_id.as_deref(),
                    "claude-code",
                    model.as_deref(),
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cost_usd.unwrap_or(0.0),
                ) {
                    warn!("记录会话 Token 用量失败: {}", e);
                }

                emit(
                    app,
                    events::RESULT,
                    serde_json::json!({
                        "session_id": session_id,
                        "input_tokens": input_tokens,
                        "output_tokens": output_tokens,
                        "cache_read_input_tokens": cache_read_tokens,
                        "cost_usd": cost_usd,
                        "duration_ms": duration_ms,
                        "is_error": is_error,
                    }),
                );
            }

            AgentEvent::Notice { level, message } => {
                emit(
                    app,
                    events::NOTICE,
                    serde_json::json!({
                        "session_id": session_id,
                        "level": level,
                        "kind": "parse",
                        "message": message,
                    }),
                );
            }
        }
    }

    // 记录活跃时间，供"无响应"检测
    let db = db.lock().await;
    if let Err(e) = db.touch_agent_session(session_id) {
        warn!("刷新会话活跃时间失败: {}", e);
    }
}

/// 下发一条用户指令：落库 → 推送 → 写入子进程 stdin
pub async fn send_user_message(
    db: &Arc<Mutex<Database>>,
    app: &AppHandle,
    registry: &SessionRegistry,
    session_id: &str,
    ctx: &MessageContext,
    content: &str,
) -> Result<SessionMessage, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("指令内容不能为空".to_string());
    }

    let (stdin, agent_path) = {
        let handles = registry.lock().await;
        match handles.get(session_id) {
            Some(handle) => (handle.stdin.clone(), handle.agent_path.clone()),
            None => return Err("会话进程未在运行，请重建或续接会话".to_string()),
        }
    };

    let message = {
        let db = db.lock().await;
        db.append_session_message(
            session_id,
            ctx,
            BlockSpec::new("user", block_type::TEXT, Some(trimmed.to_string())),
        )
        .map_err(|e| format!("写入用户消息失败: {}", e))?
    };

    emit(
        app,
        events::MESSAGE,
        serde_json::json!({ "session_id": session_id, "message": message }),
    );

    // 按会话的 Agent 品种编码用户消息（多 Agent 适配）
    let line = agent_adapter::adapter_for(&agent_path).encode_user_message(trimmed);
    let mut guard = stdin.lock().await;
    guard
        .write_all(line.as_bytes())
        .await
        .map_err(|e| format!("写入会话进程失败: {}", e))?;
    guard
        .write_all(b"\n")
        .await
        .map_err(|e| format!("写入会话进程失败: {}", e))?;
    guard
        .flush()
        .await
        .map_err(|e| format!("刷新会话进程输入失败: {}", e))?;

    Ok(message)
}

/// 中断会话：记录一条系统消息并终止子进程
pub async fn interrupt_session(
    db: &Arc<Mutex<Database>>,
    app: &AppHandle,
    registry: &SessionRegistry,
    session_id: &str,
    ctx: &MessageContext,
) -> Result<(), String> {
    {
        let db = db.lock().await;
        if let Ok(msg) = db.append_session_message(
            session_id,
            ctx,
            BlockSpec::new(
                "system",
                block_type::SYSTEM,
                Some("用户中断了本次执行".to_string()),
            ),
        ) {
            emit(
                app,
                events::MESSAGE,
                serde_json::json!({ "session_id": session_id, "message": msg }),
            );
        }
    }

    let mut handles = registry.lock().await;
    match handles.get_mut(session_id) {
        Some(handle) => {
            if let Some(tx) = handle.shutdown.take() {
                let _ = tx.send(());
            }
            Ok(())
        }
        None => Err("会话进程未在运行".to_string()),
    }
}

/// 会话是否处于活动状态
pub async fn is_active(registry: &SessionRegistry, session_id: &str) -> bool {
    registry.lock().await.contains_key(session_id)
}

/// 活动会话 id 列表（供启动对账复用）
pub async fn active_session_ids(registry: &SessionRegistry) -> Vec<String> {
    registry.lock().await.keys().cloned().collect()
}
