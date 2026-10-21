//! 域命令：git（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== Git 操作命令 ====================

#[tauri::command]
pub async fn git_get_status(
    work_dir: String,
) -> Result<serde_json::Value, AppError> {
    let svc = GitService::new();
    svc.get_status(&work_dir).map_err(|e| AppError::legacy(e.to_string()))
}

#[tauri::command]
pub async fn git_get_diff(
    work_dir: String,
    file_path: Option<String>,
    commit_hash: Option<String>,
) -> Result<String, AppError> {
    let svc = GitService::new();
    svc.get_diff(&work_dir, file_path.as_deref(), commit_hash.as_deref())
        .map_err(|e| AppError::legacy(e.to_string()))
}

/// 真实提交计数 `{total, today, week}`（P1-C3：取代前端 100 条窗口估算）
#[tauri::command]
pub async fn git_count_commits(
    work_dir: String,
) -> Result<serde_json::Value, AppError> {
    let svc = GitService::new();
    svc.count_commits(&work_dir).map_err(|e| AppError::legacy(e.to_string()))
}

/// 列出本地分支（含当前分支）
#[tauri::command]
pub async fn git_list_branches(
    work_dir: String,
) -> Result<serde_json::Value, AppError> {
    let svc = GitService::new();
    svc.list_branches(&work_dir).map_err(|e| AppError::legacy(e.to_string()))
}

/// 切换分支（分支名白名单校验）
#[tauri::command]
pub async fn git_switch_branch(
    work_dir: String,
    branch: String,
) -> Result<serde_json::Value, AppError> {
    let svc = GitService::new();
    svc.switch_branch(&work_dir, &branch).map_err(|e| AppError::legacy(e.to_string()))
}

/// 列出指定提交涉及的文件（name-status）
#[tauri::command]
pub async fn git_show_files(
    work_dir: String,
    commit_hash: String,
) -> Result<Vec<serde_json::Value>, AppError> {
    let svc = GitService::new();
    svc.show_files(&work_dir, &commit_hash).map_err(|e| AppError::legacy(e.to_string()))
}

#[tauri::command]
pub async fn git_create_commit(
    state: State<'_, AppState>,
    work_dir: String,
    message: String,
    task_id: Option<String>,
    session_id: Option<String>,
    include_untracked: Option<bool>,
) -> Result<serde_json::Value, AppError> {
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
            // 落库失败不回滚已完成的提交；把警告带回前端提示用户
            Err(e) => {
                error!("提交记录落库失败: {}", e);
                let mut obj = result.as_object().cloned().unwrap_or_default();
                obj.insert(
                    "db_warning".to_string(),
                    serde_json::json!(format!(
                        "Git 提交已完成，但平台追溯记录保存失败（{}），该提交不会出现在任务/会话关联视图中",
                        e
                    )),
                );
                return Ok(serde_json::Value::Object(obj));
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn git_log(
    work_dir: String,
    limit: Option<i32>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let svc = GitService::new();
    svc.log(&work_dir, limit).map_err(|e| AppError::legacy(e.to_string()))
}

/// 查询平台内记录的 Git 提交（可按任务或会话过滤）
#[tauri::command]
pub async fn list_git_commits(
    state: State<'_, AppState>,
    task_id: Option<String>,
    session_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let db = state.db.lock().await;
    db.list_git_commits(task_id.as_deref(), session_id.as_deref(), limit)
        .map_err(|e| format!("获取提交记录失败: {}", e)).map_err(AppError::legacy)
}
