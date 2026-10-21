//! 域命令：requirements（由 commands.rs 拆分生成，P2-4）

use super::*;

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
) -> Result<Requirement, AppError> {
    // 输入验证
    validate_requirement(&title, &priority)?;

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
    ).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn list_requirements(
    state: State<'_, AppState>,
    page: Option<u32>,
    page_size: Option<u32>,
    status: Option<String>,
    project_id: Option<String>,
) -> Result<crate::pagination::PagedResult<Requirement>, AppError> {
    let svc = RequirementService::new(&state.db, &state.cache);
    let params = crate::pagination::PaginationParams {
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    };
    svc.list_requirements_paged(&params, status.as_deref(), project_id.as_deref())
        .await
        .map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_requirement(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Requirement>, AppError> {
    let svc = RequirementService::new(&state.db, &state.cache);
    svc.get_requirement(&id).await.map_err(AppError::legacy)
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
    attachments: Option<String>,
) -> Result<(), AppError> {
    // 如果要更新状态，先校验状态流转合法性
    if let Some(ref new_status) = status {
        let svc = RequirementService::new(&state.db, &state.cache);
        let current = svc.get_requirement(&id).await.map_err(AppError::legacy)
            .map_err(|e| format!("查询需求失败: {}", e))?;
        match current {
            Some(req) => {
                crate::validation::validate_requirement_status_transition(&req.status, new_status)
                    .map_err(|e| e.to_string())?;
            }
            None => return Err(AppError::legacy(format!("需求 {} 不存在", id))),
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
        attachments: attachments.as_deref(),
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
    ).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn delete_requirement(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let svc = RequirementService::new(&state.db, &state.cache);

    // 检查是否有关联任务
    {
        let db = state.db.lock().await;
        let task_count = db.get_task_count_by_requirement(&id)
            .map_err(|e| format!("查询关联任务失败: {}", e))?;

        if task_count > 0 {
            return Err(AppError::Business(format!(
                "需求下有 {} 个关联任务，请先删除或解绑",
                task_count
            )));
        }
    }

    svc.delete_requirement(&id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn generate_prd(
    state: State<'_, AppState>,
    requirement_id: String,
    skill_id: Option<String>,
    extra_content: Option<String>,
) -> Result<String, AppError> {
    let svc = PrdService::new(&state.db, &state.cache);
    svc.generate_prd(&requirement_id, skill_id.as_deref(), extra_content.as_deref())
        .await.map_err(AppError::legacy)
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
) -> Result<String, AppError> {
    let svc = PrdService::new(&state.db, &state.cache);
    svc.generate_prd_streaming(
        &requirement_id,
        skill_id.as_deref(),
        extra_content.as_deref(),
        app,
    )
    .await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn enrich_requirement(
    state: State<'_, AppState>,
    requirement_id: String,
    extra_content: Option<String>,
) -> Result<serde_json::Value, AppError> {
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

// ==================== LLM 智能拆解（P1-⑪） ====================

/// 按需求+PRD 生成候选任务清单（仅返回清单，前端勾选确认后再批量落库）。
///
/// 双通道与 PRD 生成共用 `prd_generation_mode` 设置；Token 消耗照常记账。
#[tauri::command]
pub async fn generate_task_breakdown(
    state: State<'_, AppState>,
    requirement_id: String,
) -> Result<Vec<crate::llm::BreakdownTask>, AppError> {
    let requirement = {
        let db = state.db.lock().await;
        db.get_requirement(&requirement_id)
            .map_err(|e| format!("获取需求失败: {}", e))?
            .ok_or("需求不存在")?
    };

    let prd = requirement.prd_content.clone().unwrap_or_default();
    if prd.trim().is_empty() {
        return Err(AppError::legacy("该需求还没有 PRD，请先生成 PRD 再拆解"));
    }
    let description = requirement.description.clone().unwrap_or_default();

    // 双通道分派：与 PRD 生成共用设置
    let (tasks, token_stats, agent_type, source) = {
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
            let prompt = crate::llm::build_breakdown_prompt(&requirement.title, &description, &prd);
            let agent_path = first_available_agent(&state).await.ok_or_else(|| {
                "未检测到可用的 Agent CLI，请先在 Agent 页面执行「刷新检测」".to_string()
            })?;
            let adapter = crate::agent_adapter::adapter_for(&agent_path);
            let inv = adapter
                .build_oneshot(&agent_path, &prompt, model.as_deref(), None)
                .ok_or_else(|| "该 Agent 不支持一次性调用（仅终端模式）".to_string())?;
            let text = crate::agent_adapter::execute_oneshot(inv).await?;
            let tasks = crate::llm::sanitize_breakdown_tasks(crate::llm::parse_breakdown_json(&text));
            if tasks.is_empty() {
                return Err(AppError::legacy("LLM 未返回可解析的任务清单，请重试或补充 PRD 后再拆解"));
            }
            (
                tasks,
                crate::llm::TokenStats {
                    input_tokens: 0,
                    output_tokens: 0,
                    model: model.unwrap_or_else(|| "claude-cli".to_string()),
                },
                "claude-cli",
                "agent",
            )
        } else {
            let api_key = {
                let db = state.db.lock().await;
                db.get_setting("claude_api_key")
                    .map_err(|e| format!("获取API Key失败: {}", e))?
                    .ok_or("请先配置Claude API Key，或在设置中把 PRD 生成方式切换为 Claude CLI")?
            };

            let client = crate::llm::ClaudeClient::new(api_key).with_model(model.as_deref());
            let (tasks, token_stats) = client
                .breakdown_tasks(&requirement.title, &description, &prd)
                .await
                .map_err(AppError::legacy)?;
            (tasks, token_stats, "claude-api", "api")
        }
    };

    // Token 记账（与 enrich 同口径）
    {
        let db = state.db.lock().await;
        let input_tokens = token_stats.input_tokens as i64;
        let output_tokens = token_stats.output_tokens as i64;
        let cost = db.estimate_cost(&token_stats.model, input_tokens, output_tokens, 0, 0, 0);
        if let Err(e) = db.record_token_usage_ext(
            None,
            Some(&requirement_id),
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
            error!("记录智能拆解Token使用失败: {}", e);
        }
    }

    info!("智能拆解返回清单: requirement_id={}, {} 条", requirement_id, tasks.len());
    Ok(tasks)
}

#[tauri::command]
pub async fn list_tasks_by_requirement(
    state: State<'_, AppState>,
    requirement_id: String,
) -> Result<Vec<Task>, AppError> {
    let db = state.db.lock().await;
    db.list_tasks_by_requirement(&requirement_id)
        .map_err(|e| format!("获取需求任务列表失败: {}", e)).map_err(AppError::legacy)
}

// ==================== 状态机下发（P2，取代前端复刻） ====================

/// 下发需求状态机全表 `{status: [next, ...]}`，前端流转按钮据此渲染。
#[tauri::command]
pub fn get_requirement_transitions() -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> =
        crate::validation::requirement_transitions()
            .into_iter()
            .map(|(state, next)| (state.to_string(), serde_json::json!(next)))
            .collect();
    serde_json::Value::Object(map)
}

// ==================== PRD 版本保护（P0） ====================

/// PRD 快照版本（含全文，可整版查看与恢复）
#[derive(serde::Serialize)]
pub struct PrdVersion {
    pub id: String,
    pub content: String,
    pub created_at: String,
}

/// 列出某需求的 PRD 快照（按时间倒序）。
///
/// 快照在每次 `generate_prd*` 成功落库前写入（action=`prd_snapshot`，new_value=该版全文）。
#[tauri::command]
pub async fn list_prd_versions(
    state: State<'_, AppState>,
    requirement_id: String,
) -> Result<Vec<PrdVersion>, AppError> {
    let db = state.db.lock().await;
    let rows = db
        .list_requirement_history(&requirement_id, 200)
        .map_err(|e| AppError::legacy(format!("查询PRD版本失败: {}", e)))?;
    // 元组实际序（db/requirements.rs push）：0=action 1=id 2=old_value 3=new_value 4=created_at
    Ok(rows
        .into_iter()
        .filter(|r| r.0 == "prd_snapshot")
        .map(|r| PrdVersion {
            id: r.1,
            content: r.3.unwrap_or_default(),
            created_at: r.4,
        })
        .collect())
}

/// 恢复指定 PRD 快照为当前 `prd_content`（恢复动作自身留痕 action=`prd_restore`）。
#[tauri::command]
pub async fn restore_prd_version(
    state: State<'_, AppState>,
    requirement_id: String,
    history_id: String,
) -> Result<Requirement, AppError> {
    let content = {
        let db = state.db.lock().await;
        let rows = db
            .list_requirement_history(&requirement_id, 500)
            .map_err(|e| AppError::legacy(format!("查询PRD版本失败: {}", e)))?;
        rows.iter()
            .find(|r| r.0 == "prd_snapshot" && r.1 == history_id)
            .and_then(|r| r.3.clone())
            .ok_or_else(|| AppError::legacy("未找到该 PRD 版本"))?
    };

    let db = state.db.lock().await;
    db.update_requirement(&requirement_id, None, None, None, None, Some(&content))
        .map_err(|e| AppError::legacy(format!("恢复PRD失败: {}", e)))?;
    if let Err(e) = db.insert_requirement_history(
        &requirement_id,
        "prd_restore",
        Some(&history_id),
        Some(&content),
    ) {
        tracing::error!("写入PRD恢复历史失败（不阻断）: {}", e);
    }

    db.get_requirement(&requirement_id)
        .map_err(|e| AppError::legacy(format!("查询需求失败: {}", e)))?
        .ok_or_else(|| AppError::legacy("需求不存在"))
}
