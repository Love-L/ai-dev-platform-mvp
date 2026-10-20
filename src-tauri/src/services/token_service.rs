use crate::cache::CacheKeys;
use crate::db::{Database, TokenUsage, TokenUsageSummary, DailyUsage, ModelUsage};
use crate::cache::CacheWrapper;
use crate::usage_reader::LocalUsageReader;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug, warn};

/// Token 服务 - 处理 Token 追踪相关的业务逻辑
pub struct TokenService<'a> {
    db: &'a Arc<Mutex<Database>>,
    cache: &'a CacheWrapper,
}

impl<'a> TokenService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 获取 Token 使用记录
    pub async fn get_token_usage(&self, task_id: Option<&str>) -> Result<Vec<TokenUsage>, String> {
        debug!("获取Token使用记录: task_id={:?}", task_id);
        let db = self.db.lock().await;
        db.get_token_usage(task_id)
            .map_err(|e| {
                tracing::error!("获取Token使用记录失败: {}", e);
                format!("获取Token使用记录失败: {}", e)
            })
    }

    /// 获取 Token 使用摘要（带缓存）
    pub async fn get_token_usage_summary(&self) -> Result<TokenUsageSummary, String> {
        let cache_key = CacheKeys::token_summary();

        if let Some(cached) = self.cache.manager.get::<TokenUsageSummary>(&cache_key).await {
            debug!("从缓存获取Token统计摘要");
            return Ok(cached);
        }

        debug!("从数据库查询Token统计摘要");
        let db = self.db.lock().await;
        let result = db.get_token_usage_summary()
            .map_err(|e| {
                tracing::error!("获取Token统计失败: {}", e);
                format!("获取Token统计失败: {}", e)
            })?;

        self.cache.manager.set(&cache_key, &result, Some(60)).await;
        Ok(result)
    }

    /// 获取 Token 使用趋势（带缓存）
    pub async fn get_token_usage_trend(&self, days: i32) -> Result<Vec<DailyUsage>, String> {
        let cache_key = CacheKeys::token_trend(days);

        if let Some(cached) = self.cache.manager.get::<Vec<DailyUsage>>(&cache_key).await {
            debug!("从缓存获取Token趋势: days={}", days);
            return Ok(cached);
        }

        debug!("从数据库查询Token趋势: days={}", days);
        let db = self.db.lock().await;
        let result = db.get_token_usage_trend(days)
            .map_err(|e| {
                tracing::error!("获取Token趋势失败: {}", e);
                format!("获取Token趋势失败: {}", e)
            })?;

        self.cache.manager.set(&cache_key, &result, Some(60)).await;
        Ok(result)
    }

    /// 获取按模型统计的 Token 使用（带缓存）
    pub async fn get_token_usage_by_model(&self) -> Result<Vec<ModelUsage>, String> {
        let cache_key = CacheKeys::token_by_model();

        if let Some(cached) = self.cache.manager.get::<Vec<ModelUsage>>(&cache_key).await {
            debug!("从缓存获取模型Token统计");
            return Ok(cached);
        }

        debug!("从数据库查询模型Token统计");
        let db = self.db.lock().await;
        let result = db.get_token_usage_by_model()
            .map_err(|e| {
                tracing::error!("获取模型统计失败: {}", e);
                format!("获取模型统计失败: {}", e)
            })?;

        self.cache.manager.set(&cache_key, &result, Some(60)).await;
        Ok(result)
    }

    /// 获取 Token 预算（带缓存）
    /// 获取 Token 预算（含派生字段 `usage_ratio`、`exceeded`，供前端直接展示告警）
    pub async fn get_token_budget(&self) -> Result<Option<serde_json::Value>, String> {
        let cache_key = "token_budget:global".to_string();

        if let Some(cached) = self
            .cache
            .manager
            .get::<Option<serde_json::Value>>(&cache_key)
            .await
        {
            debug!("从缓存获取Token预算");
            return Ok(cached);
        }

        debug!("从数据库查询Token预算");
        let db = self.db.lock().await;
        let result = db.get_budget_alert()
            .map_err(|e| {
                tracing::error!("获取预算失败: {}", e);
                format!("获取预算失败: {}", e)
            })?;

        self.cache.manager.set(&cache_key, &result, Some(300)).await;
        Ok(result)
    }

    /// 设置 Token 预算（带缓存失效）
    pub async fn set_token_budget(&self, monthly_limit: i64, alert_threshold: f64) -> Result<(), String> {
        info!("设置Token预算: monthly_limit={}, alert_threshold={}", monthly_limit, alert_threshold);

        {
            let db = self.db.lock().await;
            db.set_token_budget(monthly_limit, alert_threshold)
                .map_err(|e| {
                    tracing::error!("设置预算失败: {}", e);
                    format!("设置预算失败: {}", e)
                })?;
        }

        self.cache.manager.delete("token_budget:global").await;
        info!("Token预算设置成功");
        Ok(())
    }

    /// 同步本地 Token 使用记录
    pub async fn sync_local_usage(&self) -> Result<i64, String> {
        info!("开始同步本地Token使用记录...");

        let mut reader = LocalUsageReader::new();

        if !reader.exists() {
            warn!("Claude目录不存在，跳过同步");
            return Ok(0);
        }

        let records = tokio::task::spawn_blocking(move || reader.scan_all_projects())
            .await
            .map_err(|e| {
                tracing::error!("扫描本地使用记录失败: {}", e);
                format!("扫描本地使用记录失败: {}", e)
            })?;

        let count = records.len() as i64;
        info!("扫描到 {} 条Token使用记录", count);

        if count > 0 {
            let db = self.db.lock().await;
            for record in records {
                let input_tokens = record.input_tokens as i64;
                let output_tokens = record.output_tokens as i64;
                let cost = db.estimate_cost(&record.model, input_tokens, output_tokens, 0, 0, 0);

                if let Err(e) = db.record_token_usage_ext(
                    None,
                    None,
                    Some("claude-code"),
                    Some(&record.model),
                    input_tokens,
                    output_tokens,
                    0,
                    0,
                    0,
                    cost,
                    "local",
                ) {
                    warn!("记录Token使用失败: {}", e);
                }
            }
        }

        info!("Token使用记录同步完成: count={}", count);
        Ok(count)
    }
}
