//! 域命令：mcp（由 commands.rs 拆分生成，P2-4）

use super::*;

#[tauri::command]
pub async fn get_mcp_execution_logs(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<crate::db::McpExecutionLog>, AppError> {
    let db = state.db.lock().await;
    db.get_mcp_execution_logs(&session_id).map_err(|e| {
        error!("获取MCP执行日志失败: {}", e);
        AppError::legacy(format!("获取MCP执行日志失败: {}", e))
    })
}

#[tauri::command]
pub async fn get_mcp_usage_stats(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let db = state.db.lock().await;
    db.get_mcp_usage_stats().map_err(|e| {
        error!("获取MCP统计失败: {}", e);
        AppError::legacy(format!("获取MCP统计失败: {}", e))
    })
}
