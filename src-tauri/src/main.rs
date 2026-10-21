// ACE - AI Collaboration Engine
// 基于 Tauri 2.0 + React + SQLite

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// 服务层方法普遍携带多个 Option 字段（批量更新/查询入参），逐个改造为参数
// 结构体收益有限且易引入回归，crate 级放行 too_many_arguments 由 review 把关。
#![allow(clippy::too_many_arguments)]

mod db;
mod commands;
mod llm;
mod agent;
mod usage_reader;
mod cache;
mod pty;
mod services;
mod errors;
mod validation;
mod crypto;
mod tray;
mod system_monitor;
mod pagination;
mod agent_adapter;
mod session_driver;
mod fatal;

use db::Database;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tauri::{Manager, Emitter};
use tracing::info;
use cache::{CacheConfig, CacheWrapper};
use pty::{PtyManager, PtyEvent};

/// 应用状态
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub cache: CacheWrapper,
    pub pty_manager: Arc<PtyManager>,
    /// 活动结构化会话注册表（模块 16）
    pub session_registry: session_driver::SessionRegistry,
}

fn main() {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(false)
        .with_thread_ids(true)
        .init();

    info!("ACE - AI Collaboration Engine 启动中...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 初始化数据库
            let app_dir = match app.path().app_data_dir() {
                Ok(dir) => dir,
                Err(e) => fatal::show_fatal_and_exit(
                    "ACE 无法启动",
                    &format!(
                        "无法确定应用数据目录：{}\n\n请以当前用户重新登录系统后重试。",
                        e
                    ),
                ),
            };
            std::fs::create_dir_all(&app_dir).ok();

            let db_path = app_dir.join("database.db");
            info!("数据库路径: {:?}", db_path);

            let db = match Database::new(&db_path) {
                Ok(db) => {
                    info!("数据库初始化完成");
                    db
                }
                Err(e) => {
                    tracing::error!("数据库初始化失败: {}", e);
                    // 尝试备份损坏的数据库并重新创建（带时间戳，纳入备份轮转）
                    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
                    let backup_path = app_dir.join(format!("database.db.bak.{}", ts));
                    let backup_ok = std::fs::copy(&db_path, &backup_path).is_ok();
                    info!("已备份损坏数据库到: {:?}", backup_path);
                    match Database::new(&db_path) {
                        Ok(db) => {
                            info!("数据库已重建（原数据已备份）");
                            db
                        }
                        Err(e2) => fatal::show_fatal_and_exit(
                            "数据库无法恢复",
                            &format!(
                                "数据库初始化失败：{e}\n\n自动重建也失败：{e2}\n\n\
                                 损坏文件备份：{}\n\
                                 处置建议：确认磁盘可写、杀毒软件未锁定文件后重启；\n\
                                 若仍失败，可退出应用后删除 database.db（应用会自动重建空库）。",
                                if backup_ok {
                                    backup_path.display().to_string()
                                } else {
                                    "备份失败（文件可能已被占用）".to_string()
                                }
                            ),
                        ),
                    }
                }
            };

            // 启动备份：每次启动保留最近 5 份，作为数据保护基线（NFR006）
            match db.backup_to(&db_path, 5) {
                Ok(path) => info!("启动备份完成: {:?}", path),
                Err(e) => tracing::warn!("启动备份失败: {}", e),
            }

            // 会话对账：进程随应用退出而消失，残留的 running 会话统一标记为 failed
            match db.reconcile_running_sessions() {
                Ok(0) => {}
                Ok(n) => info!("会话对账完成：{} 个残留 running 会话已标记为 failed", n),
                Err(e) => tracing::warn!("会话对账失败: {}", e),
            }

            // 初始化缓存管理器
            let cache_config = CacheConfig {
                l1_max_size: 1000,
                default_ttl: 300, // 5分钟
                enabled: true,
                key_prefix: "ai_platform:".to_string(),
            };
            let cache = CacheWrapper::new(cache_config);
            info!("缓存管理器初始化完成");

            // 初始化 PTY 管理器
            let (pty_event_sender, mut pty_event_receiver) = mpsc::channel::<PtyEvent>(100);
            let pty_manager = Arc::new(PtyManager::new(pty_event_sender));
            info!("PTY 管理器初始化完成");

            // 设置 PTY 事件转发到前端
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = pty_event_receiver.recv().await {
                    let _ = app_handle.emit("pty:event", &event);
                }
            });

            // 结构化会话残留收尾：应用重启后不可能有活动的结构化会话，
            // 因此把所有仍处于 streaming 的消息块标记为 failed，避免前端永久"流式中"
            match db.fail_all_streaming_session_messages() {
                Ok(0) => {}
                Ok(n) => info!("会话消息收尾：{} 条 streaming 消息已标记为 failed", n),
                Err(e) => tracing::warn!("会话消息收尾失败: {}", e),
            }

            // 创建应用状态
            let state = AppState {
                db: Arc::new(Mutex::new(db)),
                cache,
                pty_manager,
                session_registry: session_driver::new_registry(),
            };

            app.manage(state);
            info!("应用状态初始化完成");

            // 定时备份：启动备份之外，每 6 小时再备份一次并保留最近 5 份。
            // 说明：`backup_to` 为阻塞 I/O，本地 SQLite 体量小、耗时短，
            // 且 tokio 为多线程运行时，因此直接在异步任务内调用。
            {
                let backup_db = app.state::<AppState>().db.clone();
                let backup_target = db_path.clone();
                tauri::async_runtime::spawn(async move {
                    let mut ticker =
                        tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
                    ticker.tick().await; // 跳过 interval 的首次立即触发
                    loop {
                        ticker.tick().await;
                        let db = backup_db.lock().await;
                        match db.backup_to(&backup_target, 5) {
                            Ok(path) => info!("定时备份完成: {:?}", path),
                            Err(e) => tracing::warn!("定时备份失败: {}", e),
                        }
                    }
                });
            }

            // 创建系统托盘
            if let Err(e) = tray::create_tray(app.handle()) {
                tracing::error!("创建系统托盘失败: {}", e);
            }

            // 设置窗口关闭行为：隐藏到托盘而不是退出
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // 阻止默认关闭行为
                        api.prevent_close();
                        // 隐藏窗口
                        let _ = window_clone.hide();
                    }
                });
            }

            // 设置窗口图标
            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes = include_bytes!("../icons/icon.ico");
                match tauri::image::Image::from_bytes(icon_bytes) {
                    Ok(icon) => {
                        let _ = window.set_icon(icon);
                        info!("窗口图标设置成功");
                    }
                    Err(e) => {
                        info!("窗口图标设置失败: {}", e);
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 需求管理
            commands::create_requirement,
            commands::list_requirements,
            commands::get_requirement,
            commands::update_requirement,
            commands::delete_requirement,
            commands::generate_prd,
            commands::generate_prd_streaming,
            commands::enrich_requirement,
            commands::generate_task_breakdown,
            commands::list_prd_versions,
            commands::restore_prd_version,
            commands::get_requirement_transitions,
            // 任务管理
            commands::create_task,
            commands::list_tasks,
            commands::update_task_status,
            commands::delete_task,
            commands::get_task,
            commands::update_task,
            commands::list_tasks_by_project,
            commands::list_tasks_by_requirement,
            commands::get_task_transitions,
            // 任务评论
            commands::create_task_comment,
            commands::list_task_comments,
            commands::delete_task_comment,
            // Agent管理
            commands::discover_agents,
            commands::get_agents,
            commands::verify_agents,
            commands::add_agent,
            commands::delete_agent,
            commands::start_agent,
            commands::create_session_from_task,
            commands::stop_agent,
            commands::get_agent_sessions,
            commands::get_agent_session,
            commands::link_session_task,
            // 会话交互与消息（模块 16）
            commands::start_structured_session,
            commands::send_session_message,
            commands::list_session_messages,
            commands::interrupt_session,
            commands::set_session_mode,
            commands::resume_session,
            commands::save_session_attachment,
            // 会话管理（P1：重命名 / 归档 / 删除 / 导出 + P2 原子重建）
            commands::rename_agent_session,
            commands::set_agent_session_archived,
            commands::delete_agent_session,
            commands::export_agent_session,
            commands::force_rebuild_session,
            // Token追踪
            commands::get_token_usage,
            commands::get_token_usage_summary,
            commands::get_token_usage_trend,
            commands::get_token_usage_by_model,
            commands::get_token_budget,
            commands::set_token_budget,
            commands::sync_local_usage,
            // PRD Skill
            commands::list_prd_skills,
            commands::get_prd_skill,
            commands::create_prd_skill,
            commands::update_prd_skill,
            commands::delete_prd_skill,
            commands::create_custom_skill_from_builtin,
            commands::export_skill_definition,
            commands::import_skill_definition,
            // Skill 自进化
            commands::analyze_skills,
            commands::list_skill_suggestions,
            commands::parse_skill_template,
            commands::judge_skill,
            commands::get_skill_score_history,
            commands::optimize_skill,
            commands::apply_skill_suggestion,
            commands::apply_skill_draft,
            commands::list_skill_versions,
            commands::revert_skill_version,
            commands::get_skill_execution_logs_by_skill,
            // 统计
            commands::get_statistics,
            // 设置
            commands::get_setting,
            commands::set_setting,
            // 加密
            commands::encrypt_api_key,
            commands::decrypt_api_key,
            // 缓存
            commands::get_cache_stats,
            commands::clear_cache,
            // 项目管理
            commands::create_project,
            commands::list_projects,
            commands::get_project,
            commands::update_project,
            commands::delete_project,
            commands::get_projects_overview,
            // 项目 Skill 配置
            commands::list_project_skills,
            commands::add_project_skill,
            commands::remove_project_skill,
            // 项目 MCP 配置
            commands::list_project_mcps,
            commands::add_project_mcp,
            commands::remove_project_mcp,
            // 环节 Skill 配置
            commands::list_stage_skill_configs,
            commands::get_stage_skill_config,
            commands::upsert_stage_skill_config,
            commands::remove_stage_skill_config,
            commands::get_pipeline_stages,
            // Skill/MCP 执行日志
            commands::get_skill_execution_logs,
            commands::get_mcp_execution_logs,
            commands::get_skill_usage_stats,
            commands::get_mcp_usage_stats,
            // PTY 终端
            commands::pty_spawn,
            commands::pty_write,
            commands::pty_resize,
            commands::pty_kill,
            commands::pty_get_sessions,
            commands::pty_get_session,
            commands::pty_attach,
            commands::pty_reconnect,
            // Git 操作
            commands::git_get_status,
            commands::git_get_diff,
            commands::git_create_commit,
            commands::git_log,
            commands::list_git_commits,
            commands::git_list_branches,
            commands::git_switch_branch,
            commands::git_show_files,
            commands::git_count_commits,
            // 数据导入导出
            commands::export_data,
            commands::import_data,
            // 审计日志
            commands::get_audit_logs,
            // 系统监控
            system_monitor::get_monitor_data,
            system_monitor::check_alerts,
        ])
        .plugin(victauri_plugin::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
