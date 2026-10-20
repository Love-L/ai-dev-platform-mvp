//! 域命令：tokens（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== Token追踪命令 ====================

#[tauri::command]
pub async fn get_token_usage(
    state: State<'_, AppState>,
    task_id: Option<String>,
) -> Result<Vec<TokenUsage>, AppError> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage(task_id.as_deref()).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_token_usage_summary(
    state: State<'_, AppState>,
) -> Result<TokenUsageSummary, AppError> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage_summary().await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_token_usage_trend(
    state: State<'_, AppState>,
    days: i32,
) -> Result<Vec<DailyUsage>, AppError> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage_trend(days).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_token_usage_by_model(
    state: State<'_, AppState>,
) -> Result<Vec<ModelUsage>, AppError> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_usage_by_model().await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn get_token_budget(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, AppError> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.get_token_budget().await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn set_token_budget(
    state: State<'_, AppState>,
    monthly_limit: i64,
    alert_threshold: f64,
) -> Result<(), AppError> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.set_token_budget(monthly_limit, alert_threshold).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn sync_local_usage(
    state: State<'_, AppState>,
) -> Result<i64, AppError> {
    let svc = TokenService::new(&state.db, &state.cache);
    svc.sync_local_usage().await.map_err(AppError::legacy)
}
