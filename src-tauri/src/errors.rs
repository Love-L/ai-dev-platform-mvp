use serde::Serialize;
use std::fmt;

/// ACE 应用错误。
///
/// 通过 Tauri IPC 暴露给前端时序列化为 `{ code, message }`：
/// - `code`：机器可读错误码（`validation` / `business` / `not_found` / `database` /
///   `external` / `crypto` / `legacy`），前端可据此分支处理；
/// - `message`：人类可读信息，直接展示。
///
/// 历史 String 错误经 `From<String>` 进入时统一归为 `legacy` 码。
/// 兼容说明：构造函数沿用旧枚举变体名（`AppError::Validation(...)` 等），
/// 存量调用点无需修改。
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    /// 机器可读错误码
    pub code: String,
    /// 人类可读信息
    pub message: String,
}

macro_rules! error_ctor {
    ($name:ident, $code:literal) => {
        #[allow(non_snake_case)]
        pub fn $name(msg: impl Into<String>) -> Self {
            Self {
                code: $code.to_string(),
                message: msg.into(),
            }
        }
    };
}

impl AppError {
    error_ctor!(Validation, "validation");
    error_ctor!(Business, "business");
    error_ctor!(NotFound, "not_found");
    error_ctor!(Database, "database");
    error_ctor!(External, "external");
    error_ctor!(Crypto, "crypto");

    /// 未分类错误（历史 String 错误的统一归口）
    pub fn legacy(msg: impl Into<String>) -> Self {
        Self {
            code: "legacy".to_string(),
            message: msg.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // legacy 保持原始字符串输出（历史 String 错误的展示语义不变）
        match self.code.as_str() {
            "validation" => write!(f, "[验证] {}", self.message),
            "business" => write!(f, "[业务] {}", self.message),
            "not_found" => write!(f, "[未找到] {}", self.message),
            "database" => write!(f, "[DB] {}", self.message),
            "external" => write!(f, "[外部] {}", self.message),
            "crypto" => write!(f, "[加密] {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for AppError {}

impl From<String> for AppError {
    fn from(msg: String) -> Self {
        AppError::legacy(msg)
    }
}

impl From<&str> for AppError {
    fn from(msg: &str) -> Self {
        AppError::legacy(msg.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::legacy(format!("JSON 序列化失败: {}", e))
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}

/// 便捷 Result 类型
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serializes_as_structured_code_and_message() {
        let e = AppError::Validation("标题不能为空".to_string());
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["code"], "validation");
        assert_eq!(json["message"], "标题不能为空");
    }

    #[test]
    fn legacy_string_conversion_preserves_message() {
        let e: AppError = "出错了".to_string().into();
        assert_eq!(e.code, "legacy");
        assert_eq!(e.to_string(), "出错了", "legacy 展示不加前缀");
    }

    #[test]
    fn display_keeps_prefix_for_typed_errors() {
        assert_eq!(AppError::Business("x".to_string()).to_string(), "[业务] x");
        assert_eq!(AppError::Database("y".to_string()).to_string(), "[DB] y");
    }
}
