use crate::cache::CacheKeys;
use crate::db::Database;
use crate::cache::CacheWrapper;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug};

/// 设置服务 - 处理系统设置相关的业务逻辑
pub struct SettingService<'a> {
    db: &'a Arc<Mutex<Database>>,
    cache: &'a CacheWrapper,
}

impl<'a> SettingService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 获取设置（带缓存）
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let cache_key = CacheKeys::setting(key);

        if let Some(cached) = self.cache.manager.get::<Option<String>>(&cache_key).await {
            debug!("从缓存获取设置: key={}", key);
            return Ok(cached);
        }

        debug!("从数据库查询设置: key={}", key);
        let db = self.db.lock().await;
        let result = db.get_setting(key)
            .map_err(|e| {
                tracing::error!("获取设置失败: {}", e);
                format!("获取设置失败: {}", e)
            })?;

        self.cache.manager.set(&cache_key, &result, Some(300)).await;
        Ok(result)
    }

    /// 保存设置（带缓存失效）
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        debug!("保存设置: key={}", key);

        {
            let db = self.db.lock().await;
            db.set_setting(key, value)
                .map_err(|e| {
                    tracing::error!("保存设置失败: {}", e);
                    format!("保存设置失败: {}", e)
                })?;
        }

        self.cache.manager.delete(&CacheKeys::setting(key)).await;
        info!("设置保存成功: key={}", key);
        Ok(())
    }
}
