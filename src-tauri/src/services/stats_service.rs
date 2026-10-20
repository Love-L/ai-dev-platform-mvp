use crate::cache::CacheKeys;
use crate::db::Database;
use crate::cache::CacheWrapper;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug};

/// 统计服务 - 处理统计聚合相关的业务逻辑
pub struct StatsService<'a> {
    db: &'a Arc<Mutex<Database>>,
    cache: &'a CacheWrapper,
}

impl<'a> StatsService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 获取统计数据（带缓存）
    pub async fn get_statistics(&self) -> Result<serde_json::Value, String> {
        let cache_key = CacheKeys::statistics();

        if let Some(cached) = self.cache.manager.get::<serde_json::Value>(&cache_key).await {
            debug!("从缓存获取统计数据");
            return Ok(cached);
        }

        debug!("从数据库查询统计数据");
        let db = self.db.lock().await;

        // 使用SQL聚合获取任务统计（避免加载所有记录到内存）
        let (total_tasks, completed_tasks, in_progress_tasks, pending_tasks, dev_tasks, test_tasks, bug_tasks)
            = db.get_task_stats().map_err(|e| {
            tracing::error!("获取任务统计失败: {}", e);
            format!("获取任务统计失败: {}", e)
        })?;

        // 使用SQL聚合获取需求统计
        let total_requirements = db.get_requirement_count().map_err(|e| {
            tracing::error!("获取需求统计失败: {}", e);
            format!("获取需求统计失败: {}", e)
        })?;

        // 获取 Token 统计
        let token_summary = db.get_token_usage_summary().map_err(|e| {
            tracing::error!("获取Token统计失败: {}", e);
            format!("获取Token统计失败: {}", e)
        })?;

        info!("统计数据获取成功: tasks={}, requirements={}, tokens={}",
            total_tasks, total_requirements, token_summary.total_tokens);

        let stats = serde_json::json!({
            "tasks": {
                "total": total_tasks,
                "completed": completed_tasks,
                "in_progress": in_progress_tasks,
                "pending": pending_tasks,
                "by_type": {
                    "development": dev_tasks,
                    "testing": test_tasks,
                    "bug_fix": bug_tasks,
                }
            },
            "requirements": {
                "total": total_requirements,
            },
            "tokens": {
                "total": token_summary.total_tokens,
                "cost": token_summary.total_cost,
                "input": token_summary.input_tokens,
                "output": token_summary.output_tokens,
            }
        });

        self.cache.manager.set(&cache_key, &stats, Some(300)).await;
        Ok(stats)
    }
}
