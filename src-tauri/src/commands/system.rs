//! 域命令：system（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== 统计命令 ====================

#[tauri::command]
pub async fn get_statistics(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    let svc = StatsService::new(&state.db, &state.cache);
    svc.get_statistics().await.map_err(AppError::legacy)
}

// ==================== 缓存命令 ====================

#[tauri::command]
pub async fn get_cache_stats(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    let stats = state.cache.manager.stats().await;

    Ok(serde_json::json!({
        "hits": stats.hits,
        "misses": stats.misses,
        "hit_rate": format!("{:.2}%", stats.hit_rate() * 100.0),
        "size": stats.l1_size,
        "enabled": state.cache.manager.is_enabled(),
    }))
}

#[tauri::command]
pub async fn clear_cache(
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.cache.manager.clear().await;
    info!("缓存已清空");
    Ok(())
}

// ==================== 审计日志 ====================

#[tauri::command]
pub async fn get_audit_logs(
    state: State<'_, AppState>,
    limit: Option<i64>,
    entity_type: Option<String>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let db = state.db.lock().await;
    db.list_audit_logs(limit, entity_type.as_deref())
        .map_err(|e| format!("获取审计日志失败: {}", e)).map_err(AppError::legacy)
}
