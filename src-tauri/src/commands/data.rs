//! 域命令：data（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== 数据导入导出命令 ====================

/// 导出全量数据快照（JSON）
#[tauri::command]
pub async fn export_data(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    let svc = DataService::new(&state.db, &state.cache);
    svc.export_data().await.map_err(AppError::legacy)
}

/// 导入数据快照。
///
/// 正式导入前会对当前数据库做一次文件级备份，避免误操作导致数据丢失。
#[tauri::command]
pub async fn import_data(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: String,
    replace: Option<bool>,
    dry_run: Option<bool>,
) -> Result<serde_json::Value, AppError> {
    let parsed: serde_json::Value = serde_json::from_str(&payload)
        .map_err(|e| format!("导入内容不是合法 JSON: {}", e))?;

    let replace = replace.unwrap_or(false);
    let dry_run = dry_run.unwrap_or(false);

    let svc = DataService::new(&state.db, &state.cache);

    // 正式导入前先备份当前数据库（含 WAL 检查点，保留最近 5 份）
    if !dry_run {
        if let Ok(dir) = app.path().app_data_dir() {
            let db_path = dir.join("database.db");
            if db_path.exists() {
                let db = state.db.lock().await;
                match db.backup_to(&db_path, 5) {
                    Ok(path) => info!("导入前已备份数据库: {:?}", path),
                    Err(e) => error!("导入前备份数据库失败: {}", e),
                }
            }
        }
    }

    svc.import_data(&parsed, replace, dry_run).await.map_err(AppError::legacy)
}
