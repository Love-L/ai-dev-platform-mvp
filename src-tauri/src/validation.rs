use crate::errors::{AppError, AppResult};

/// 验证需求创建参数
pub fn validate_requirement(title: &str, priority: &str) -> AppResult<()> {
    if title.trim().is_empty() {
        return Err(AppError::Validation("需求标题不能为空".to_string()));
    }
    if title.len() > 200 {
        return Err(AppError::Validation("需求标题不能超过200个字符".to_string()));
    }
    if !["P0", "P1", "P2", "P3"].contains(&priority) {
        return Err(AppError::Validation(format!("无效的优先级: {}", priority)));
    }
    Ok(())
}

/// 验证任务创建参数
pub fn validate_task(title: &str, task_type: &str, priority: &str) -> AppResult<()> {
    if title.trim().is_empty() {
        return Err(AppError::Validation("任务标题不能为空".to_string()));
    }
    if title.len() > 200 {
        return Err(AppError::Validation("任务标题不能超过200个字符".to_string()));
    }
    if !["requirement", "development", "testing", "bug_fix", "review"].contains(&task_type) {
        return Err(AppError::Validation(format!("无效的任务类型: {}", task_type)));
    }
    if !["P0", "P1", "P2", "P3"].contains(&priority) {
        return Err(AppError::Validation(format!("无效的优先级: {}", priority)));
    }
    Ok(())
}

/// 验证项目创建参数
pub fn validate_project(name: &str) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("项目名称不能为空".to_string()));
    }
    if name.len() > 100 {
        return Err(AppError::Validation("项目名称不能超过100个字符".to_string()));
    }
    Ok(())
}

/// 验证会话状态转换
pub fn validate_session_status_transition(current: &str, new: &str) -> AppResult<()> {
    // no-op 转换（状态未变）合法：保存/回写不应因"未变化"被业务规则拒绝
    if current == new {
        return Ok(());
    }
    let valid_transitions = match current {
        "running" => vec!["completed", "failed"],
        "completed" => vec![],
        "failed" => vec![],
        _ => vec![],
    };

    if !valid_transitions.contains(&new) {
        return Err(AppError::Business(format!(
            "不允许从 {} 转换到 {}",
            current, new
        )));
    }
    Ok(())
}

/// 验证需求状态转换
pub fn validate_requirement_status_transition(current: &str, new: &str) -> AppResult<()> {
    // no-op 转换（状态未变）合法：智能完善/编辑保存会原样回传状态（draft→draft），
    // 不应被状态机拒绝（生产问题：保存失败 [业务] 不允许从 draft 转换到 draft）
    if current == new {
        return Ok(());
    }
    let valid_transitions = match current {
        "draft" => vec!["reviewing"],
        "reviewing" => vec!["approved", "rejected"],
        "approved" => vec!["frozen", "in_development"],
        "rejected" => vec!["draft"],
        "frozen" => vec!["draft", "in_development"],
        "in_development" => vec!["testing"],
        "testing" => vec!["completed", "in_development"],
        "completed" => vec![],
        _ => vec![],
    };

    if !valid_transitions.contains(&new) {
        return Err(AppError::Business(format!(
            "不允许从 {} 转换到 {}",
            current, new
        )));
    }
    Ok(())
}

/// 验证任务状态转换
pub fn validate_task_status_transition(current: &str, new: &str) -> AppResult<()> {
    // no-op 转换（状态未变）合法，同上
    if current == new {
        return Ok(());
    }
    let valid_transitions = match current {
        "pending" => vec!["in_progress", "blocked"],
        "in_progress" => vec!["completed", "blocked", "pending"],
        "blocked" => vec!["pending", "in_progress"],
        "completed" => vec!["pending"],
        _ => vec![],
    };

    if !valid_transitions.contains(&new) {
        return Err(AppError::Business(format!(
            "不允许从 {} 转换到 {}",
            current, new
        )));
    }
    Ok(())
}
