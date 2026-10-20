use serde::{Deserialize, Serialize};

/// 分页参数
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    /// 页码（从1开始）
    #[serde(default = "default_page")]
    pub page: u32,
    /// 每页数量
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

impl PaginationParams {
    /// 获取SQL LIMIT子句的偏移量（分页由 db 层按 page/page_size 统一计算）
    #[allow(dead_code)]
    pub fn offset(&self) -> u32 {
        (self.page - 1) * self.page_size
    }

    /// 获取SQL LIMIT子句的数量
    pub fn limit(&self) -> u32 {
        self.page_size.min(100) // 最大100条
    }
}

/// 分页结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 总记录数
    pub total: u64,
    /// 当前页码
    pub page: u32,
    /// 每页数量
    pub page_size: u32,
    /// 总页数
    pub total_pages: u32,
}

impl<T> PagedResult<T> {
    /// 创建分页结果
    pub fn new(items: Vec<T>, total: u64, page: u32, page_size: u32) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            ((total as f64) / (page_size as f64)).ceil() as u32
        };

        Self {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}
