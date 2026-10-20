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

// ==================== 导入 / 导出（模板分享） ====================

use serde::{Deserialize, Serialize};

/// Skill 定义交换格式（导入 / 导出 / 跨机器分享通用）。
/// 只携带模板本体，不携带血缘与统计——导入后作为独立的自定义 Skill 起步。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub schema_version: u32,
    pub name: String,
    pub skill_type: String,
    pub description: Option<String>,
    pub template: String,
    /// 导出来源：`builtin` / `custom`（仅供展示，不参与导入逻辑）
    pub exported_from: String,
    pub exported_at: String,
}

pub const SKILL_DEF_SCHEMA_VERSION: u32 = 1;

/// 导出 Skill 为可分享的 JSON 定义
#[tauri::command]
pub async fn export_skill_definition(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, AppError> {
    let svc = PrdSkillService::new(&state.db);
    let skill = svc
        .get_skill(&id)
        .await
        .map_err(AppError::legacy)?
        .ok_or_else(|| AppError::NotFound(format!("Skill {} 不存在", id)))?;

    let def = SkillDefinition {
        schema_version: SKILL_DEF_SCHEMA_VERSION,
        name: skill.name,
        skill_type: skill.skill_type,
        description: skill.description,
        template: skill.template,
        exported_from: if skill.is_builtin { "builtin" } else { "custom" }.to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
    };
    serde_json::to_string_pretty(&def).map_err(AppError::from)
}

/// 从 JSON 定义导入为自定义 Skill（名称冲突自动追加序号）
#[tauri::command]
pub async fn import_skill_definition(
    state: State<'_, AppState>,
    json: String,
) -> Result<PrdSkill, AppError> {
    let def: SkillDefinition = serde_json::from_str(&json)
        .map_err(|e| AppError::Validation(format!("Skill 定义 JSON 解析失败: {}", e)))?;
    if def.schema_version != SKILL_DEF_SCHEMA_VERSION {
        return Err(AppError::Validation(format!(
            "不支持的 Skill 定义版本 {}（当前支持 v{}）",
            def.schema_version, SKILL_DEF_SCHEMA_VERSION
        )));
    }
    if def.name.trim().is_empty() || def.template.trim().is_empty() {
        return Err(AppError::Validation(
            "Skill 定义缺少 name 或 template 字段".to_string(),
        ));
    }

    let svc = PrdSkillService::new(&state.db);
    let existing = svc.list_skills().await.map_err(AppError::legacy)?;

    let mut name = def.name.trim().to_string();
    if existing.iter().any(|s| s.name == name) {
        let mut n = 2;
        loop {
            let candidate = format!("{} ({})", def.name.trim(), n);
            if !existing.iter().any(|s| s.name == candidate) {
                name = candidate;
                break;
            }
            n += 1;
        }
    }

    svc.create_skill(&name, &def.skill_type, def.description.as_deref(), &def.template)
        .await
        .map_err(AppError::legacy)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_definition_serde_roundtrip() {
        let def = SkillDefinition {
            schema_version: SKILL_DEF_SCHEMA_VERSION,
            name: "测试模板".to_string(),
            skill_type: "feature".to_string(),
            description: Some("描述".to_string()),
            template: "## Prompt\n正文 {requirement}".to_string(),
            exported_from: "custom".to_string(),
            exported_at: "2026-10-20T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string_pretty(&def).unwrap();
        let back: SkillDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, def.name);
        assert_eq!(back.template, def.template);
        assert_eq!(back.schema_version, SKILL_DEF_SCHEMA_VERSION);
    }

    #[test]
    fn skill_definition_rejects_future_schema() {
        let json = r#"{
            "schema_version": 99, "name": "x", "skill_type": "feature",
            "description": null, "template": "t",
            "exported_from": "custom", "exported_at": ""
        }"#;
        let def: SkillDefinition = serde_json::from_str(json).unwrap();
        assert_ne!(
            def.schema_version, SKILL_DEF_SCHEMA_VERSION,
            "未来版本应在命令层被拒绝"
        );
    }
}
