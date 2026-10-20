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
) -> Result<crate::pagination::PagedResult<Requirement>, AppError> {
    let svc = RequirementService::new(&state.db, &state.cache);
    let params = crate::pagination::PaginationParams {
        page: page.unwrap_or(1),
        page_size: page_size.unwrap_or(20),
    };
    svc.list_requirements_paged(&params, status.as_deref()).await.map_err(AppError::legacy)
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

#[tauri::command]
pub async fn list_tasks_by_requirement(
    state: State<'_, AppState>,
    requirement_id: String,
) -> Result<Vec<Task>, AppError> {
    let db = state.db.lock().await;
    db.list_tasks_by_requirement(&requirement_id)
        .map_err(|e| format!("获取需求任务列表失败: {}", e)).map_err(AppError::legacy)
}
