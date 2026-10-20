//! 域命令：tasks（由 commands.rs 拆分生成，P2-4）

use super::*;

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
) -> Result<Task, AppError> {
    // 输入验证
    validate_task(&title, &task_type, &priority)?;
    if let Some(desc) = &description {
        if desc.chars().count() > 100_000 {
            return Err(AppError::legacy("任务描述不能超过 100000 字".to_string()));
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
    ).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn list_tasks(
    state: State<'_, AppState>,
    status: Option<String>,
    task_type: Option<String>,
) -> Result<Vec<Task>, AppError> {
    let svc = TaskService::new(&state.db, &state.cache);
    svc.list_tasks(status.as_deref(), task_type.as_deref()).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn update_task_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<(), AppError> {
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
            None => return Err(AppError::legacy(format!("任务 {} 不存在", id))),
        }
    }

    let svc = TaskService::new(&state.db, &state.cache);
    svc.update_task_status(&id, &status).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn delete_task(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let svc = TaskService::new(&state.db, &state.cache);
    svc.delete_task(&id).await.map_err(AppError::legacy)
}

// ==================== 任务管理扩展命令 ====================

#[tauri::command]
pub async fn get_task(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Task>, AppError> {
    let db = state.db.lock().await;
    db.get_task(&id)
        .map_err(|e| format!("获取任务详情失败: {}", e)).map_err(AppError::legacy)
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
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    db.update_task(&id, title.as_deref(), description.as_deref(), task_type.as_deref(), priority.as_deref(), assignee.as_deref(), due_date.as_deref(), tags.as_deref(), project_id.as_deref())
        .map_err(|e| format!("更新任务失败: {}", e)).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn list_tasks_by_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<Task>, AppError> {
    let db = state.db.lock().await;
    db.list_tasks_by_project(&project_id)
        .map_err(|e| format!("获取项目任务列表失败: {}", e)).map_err(AppError::legacy)
}

// ==================== 任务评论命令 ====================

#[tauri::command]
pub async fn create_task_comment(
    state: State<'_, AppState>,
    task_id: String,
    author: String,
    content: String,
) -> Result<crate::db::TaskComment, AppError> {
    let db = state.db.lock().await;
    db.create_task_comment(&task_id, &author, &content)
        .map_err(|e| format!("创建评论失败: {}", e)).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn list_task_comments(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Vec<crate::db::TaskComment>, AppError> {
    let db = state.db.lock().await;
    db.list_task_comments(&task_id)
        .map_err(|e| format!("获取评论列表失败: {}", e)).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn delete_task_comment(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = state.db.lock().await;
    db.delete_task_comment(&id)
        .map_err(|e| format!("删除评论失败: {}", e)).map_err(AppError::legacy)
}
