//! 域命令：settings（由 commands.rs 拆分生成，P2-4）

use super::*;

// ==================== 设置命令 ====================

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, AppError> {
    let svc = SettingService::new(&state.db, &state.cache);
    svc.get_setting(&key).await.map_err(AppError::legacy)
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    let svc = SettingService::new(&state.db, &state.cache);
    svc.set_setting(&key, &value).await.map_err(AppError::legacy)
}

// ==================== 加密命令 ====================

#[tauri::command]
pub async fn encrypt_api_key(api_key: String, password: String) -> Result<String, AppError> {
    crate::crypto::encrypt_api_key(&api_key, &password).map_err(|e| e.to_string()).map_err(AppError::legacy)
}

#[tauri::command]
pub async fn decrypt_api_key(encrypted: String, password: String) -> Result<String, AppError> {
    // 先做格式预检，给出可读错误，而不是把底层解析错误直接抛给用户
    if !crate::crypto::is_encrypted_format(&encrypted) {
        return Err(AppError::legacy("密文格式不正确：请提供由 encrypt_api_key 生成的 base64 字符串".to_string()));
    }
    crate::crypto::decrypt_api_key(&encrypted, &password).map_err(|e| e.to_string()).map_err(AppError::legacy)
}
