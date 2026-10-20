use std::fmt;

/// ACE 应用错误类型
#[derive(Debug)]
pub enum AppError {
    /// 数据库错误
    Database(String),
    /// 输入验证错误
    Validation(String),
    /// 资源未找到
    NotFound(String),
    /// 业务逻辑错误
    Business(String),
    /// 外部服务错误
    External(String),
    /// 加密/解密错误
    Crypto(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(msg) => write!(f, "[DB] {}", msg),
            AppError::Validation(msg) => write!(f, "[验证] {}", msg),
            AppError::NotFound(msg) => write!(f, "[未找到] {}", msg),
            AppError::Business(msg) => write!(f, "[业务] {}", msg),
            AppError::External(msg) => write!(f, "[外部] {}", msg),
            AppError::Crypto(msg) => write!(f, "[加密] {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}

/// 便捷 Result 类型
pub type AppResult<T> = Result<T, AppError>;
