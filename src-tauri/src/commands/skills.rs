//! 域命令：skills（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== PRD Skill 命令 ====================

#[tauri::command]
pub async fn list_prd_skills(
    state: State<'_, AppState>,
) -> Result<Vec<PrdSkill>, AppError> {
    let svc = PrdSkillService::new(&state.db);
    svc.list_skills().await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_prd_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<PrdSkill>, AppError> {
    let svc = PrdSkillService::new(&state.db);
    svc.get_skill(&id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn create_prd_skill(
    state: State<'_, AppState>,
    name: String,
    skill_type: String,
    description: Option<String>,
    template: String,
) -> Result<PrdSkill, AppError> {
    let svc = PrdSkillService::new(&state.db);
    svc.create_skill(&name, &skill_type, description.as_deref(), &template).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn update_prd_skill(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    template: Option<String>,
) -> Result<(), AppError> {
    let svc = PrdSkillService::new(&state.db);
    svc.update_skill(&id, name.as_deref(), description.as_deref(), template.as_deref()).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn delete_prd_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let svc = PrdSkillService::new(&state.db);
    svc.delete_skill(&id).await.map_err(AppError::legacy)
}

// ==================== Skill 自进化命令 ====================

/// L1 诊断：规则 + 静态体检 + 振荡检测，建议落库后返回全部建议
#[tauri::command]
pub async fn analyze_skills(
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::SkillSuggestion>, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.analyze().await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn list_skill_suggestions(
    state: State<'_, AppState>,
    skill_id: Option<String>,
) -> Result<Vec<crate::db::SkillSuggestion>, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.list_suggestions(skill_id.as_deref()).await.map_err(AppError::legacy)
}

/// luban-skill 模板四段解析
#[tauri::command]
pub async fn parse_skill_template(
    state: State<'_, AppState>,
    skill_id: Option<String>,
    template: Option<String>,
) -> Result<serde_json::Value, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.parse_template(skill_id.as_deref(), template.as_deref()).await.map_err(AppError::legacy)
}

/// 评分（结构分静态 + 效果分 Judge）+ 棘轮判定
#[tauri::command]
pub async fn judge_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<crate::db::SkillScore, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.judge(&skill_id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_skill_score_history(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<crate::db::SkillScore>, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.score_history(&skill_id).await.map_err(AppError::legacy)
}

/// L2 优化器：返回草稿（不落库，需前端 diff 确认后 apply）
#[tauri::command]
pub async fn optimize_skill(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<serde_json::Value, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.optimize(&skill_id).await.map_err(AppError::legacy)
}

/// 应用优化草稿（人在回路确认入口，version+1）
#[tauri::command]
pub async fn apply_skill_suggestion(
    state: State<'_, AppState>,
    suggestion_id: String,
    new_template: Option<String>,
) -> Result<PrdSkill, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.apply_suggestion(&suggestion_id, new_template.as_deref()).await.map_err(AppError::legacy)
}

/// 应用 AI 优化草稿（无关联建议，version+1）
#[tauri::command]
pub async fn apply_skill_draft(
    state: State<'_, AppState>,
    skill_id: String,
    new_template: String,
) -> Result<PrdSkill, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.apply_draft(&skill_id, &new_template).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn list_skill_versions(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<crate::db::SkillVersion>, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.list_versions(&skill_id).await.map_err(AppError::legacy)
}

/// 棘轮回退：应用指定版本快照（写为新版本）
#[tauri::command]
pub async fn revert_skill_version(
    state: State<'_, AppState>,
    skill_id: String,
    version: i32,
) -> Result<PrdSkill, AppError> {
    let svc = SkillEvolutionService::new(&state.db);
    svc.revert(&skill_id, version).await.map_err(AppError::legacy)
}

/// 按 Skill 查询执行日志（进化视图日志 Tab）
#[tauri::command]
pub async fn get_skill_execution_logs_by_skill(
    state: State<'_, AppState>,
    skill_id: String,
    limit: Option<i64>,
) -> Result<Vec<crate::db::SkillExecutionLog>, AppError> {
    let db = state.db.lock().await;
    db.get_skill_execution_logs_by_skill(&skill_id, limit.unwrap_or(50))
        .map_err(|e| e.to_string()).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn create_custom_skill_from_builtin(
    state: State<'_, AppState>,
    parent_id: String,
    name: String,
    template: Option<String>,
) -> Result<PrdSkill, AppError> {
    let svc = PrdSkillService::new(&state.db);
    svc.create_custom_skill(&parent_id, &name, template.as_deref()).await.map_err(AppError::legacy)
}

// ==================== 环节 Skill 配置命令 ====================

#[tauri::command]
pub async fn list_stage_skill_configs(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<crate::db::StageSkillConfig>, AppError> {
    let db = state.db.lock().await;
    db.list_stage_skill_configs(&project_id)
        .map_err(|e| format!("获取环节 Skill 配置失败: {}", e)).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_stage_skill_config(
    state: State<'_, AppState>,
    project_id: String,
    stage: String,
) -> Result<Option<crate::db::StageSkillConfig>, AppError> {
    let db = state.db.lock().await;
    db.get_stage_skill_config(&project_id, &stage)
        .map_err(|e| format!("获取环节 Skill 配置失败: {}", e)).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn upsert_stage_skill_config(
    state: State<'_, AppState>,
    project_id: String,
    stage: String,
    skill_id: String,
    is_default: bool,
    custom_params: Option<String>,
) -> Result<crate::db::StageSkillConfig, AppError> {
    let db = state.db.lock().await;
    db.upsert_stage_skill_config(&project_id, &stage, &skill_id, is_default, custom_params.as_deref())
        .map_err(|e| format!("保存环节 Skill 配置失败: {}", e)).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn remove_stage_skill_config(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    db.remove_stage_skill_config(&id)
        .map_err(|e| format!("删除环节 Skill 配置失败: {}", e)).map_err(AppError::legacy)
}

// ==================== Skill/MCP 执行日志命令 ====================

#[tauri::command]
pub async fn get_skill_execution_logs(
    state: State<'_, AppState>,
    task_id: Option<String>,
    session_id: Option<String>,
) -> Result<Vec<crate::db::SkillExecutionLog>, AppError> {
    let db = state.db.lock().await;
    if let Some(tid) = &task_id {
        db.get_skill_execution_logs_by_task(tid).map_err(|e| {
            error!("获取Skill执行日志失败: {}", e);
            AppError::legacy(format!("获取Skill执行日志失败: {}", e))
        })
    } else if let Some(sid) = &session_id {
        db.get_skill_execution_logs_by_session(sid).map_err(|e| {
            error!("获取Skill执行日志失败: {}", e);
            AppError::legacy(format!("获取Skill执行日志失败: {}", e))
        })
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn get_skill_usage_stats(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let db = state.db.lock().await;
    db.get_skill_usage_stats().map_err(|e| {
        error!("获取Skill统计失败: {}", e);
        AppError::legacy(format!("获取Skill统计失败: {}", e))
    })
}
