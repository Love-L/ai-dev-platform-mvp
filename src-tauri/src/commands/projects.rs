//! 域命令：projects（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== 项目管理命令 ====================

#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
    key: Option<String>,
) -> Result<Project, AppError> {
    // 输入验证
    validate_project(&name)?;

    let svc = ProjectService::new(&state.db, &state.cache);
    svc.create_project(&name, description.as_deref(), key.as_deref()).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, AppState>,
) -> Result<Vec<Project>, AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.list_projects().await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Project>, AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.get_project(&id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn update_project(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    key: Option<String>,
    status: Option<String>,
) -> Result<(), AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.update_project(&id, name.as_deref(), description.as_deref(), key.as_deref(), status.as_deref()).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn delete_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
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
            return Err(AppError::Business(format!(
                "项目下有 {} 个需求、{} 个任务、{} 个 Skill 配置和 {} 个 MCP 配置，请先清理",
                req_count, task_count, skill_count, mcp_count
            )));
        }
    }

    svc.delete_project(&id).await.map_err(AppError::legacy)
}

// ==================== 项目 Skill 配置命令 ====================

#[tauri::command]
pub async fn list_project_skills(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<crate::db::ProjectSkill>, AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.list_project_skills(&project_id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn add_project_skill(
    state: State<'_, AppState>,
    project_id: String,
    skill_id: String,
    is_default: Option<bool>,
    custom_params: Option<String>,
) -> Result<crate::db::ProjectSkill, AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.add_project_skill(&project_id, &skill_id, is_default, custom_params.as_deref()).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn remove_project_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.remove_project_skill(&id).await.map_err(AppError::legacy)
}

// ==================== 项目 MCP 配置命令 ====================

#[tauri::command]
pub async fn list_project_mcps(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<crate::db::ProjectMcp>, AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.list_project_mcps(&project_id).await.map_err(AppError::legacy)
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
) -> Result<crate::db::ProjectMcp, AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.add_project_mcp(&project_id, &mcp_name, &endpoint, description.as_deref(), is_default, auth_config.as_deref(), custom_params.as_deref()).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn remove_project_mcp(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let svc = ProjectService::new(&state.db, &state.cache);
    svc.remove_project_mcp(&id).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_pipeline_stages() -> Result<Vec<serde_json::Value>, AppError> {
    use crate::db::PipelineStage;
    Ok(PipelineStage::all().iter().map(|s| {
        serde_json::json!({
            "value": s.as_str(),
            "label": s.display_name(),
        })
    }).collect())
}
