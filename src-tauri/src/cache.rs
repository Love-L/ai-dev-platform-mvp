// 缓存模块 - 纯 LRU 内存缓存管理器

use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use tracing::{debug, info};

/// 缓存条目序列化 schema 版本。
///
/// 条目以 `(版本号, 数据)` 的元组序列化，读取时校验版本：
/// 类型发生不兼容变更后，旧条目会被识别为不兼容并自动失效，而不是静默反序列化出错。
const CACHE_SCHEMA_VERSION: u32 = 1;

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub ttl_seconds: u64,
}

impl CacheEntry {
    pub fn new(data: Vec<u8>, ttl_seconds: u64) -> Self {
        Self {
            data,
            created_at: Utc::now(),
            ttl_seconds,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() - self.created_at > Duration::seconds(self.ttl_seconds as i64)
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub l1_size: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// L1 内存缓存大小
    pub l1_max_size: usize,
    /// 默认 TTL (秒)
    pub default_ttl: u64,
    /// 是否启用缓存
    pub enabled: bool,
    /// 缓存键前缀
    pub key_prefix: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_max_size: 1000,
            default_ttl: 300, // 5分钟
            enabled: true,
            key_prefix: "ai_platform:".to_string(),
        }
    }
}

/// 缓存键生成器
pub struct CacheKeys;

impl CacheKeys {
    /// 统计数据缓存键
    pub fn statistics() -> String {
        "statistics:main".to_string()
    }

    /// Token 统计摘要缓存键
    pub fn token_summary() -> String {
        "token:summary".to_string()
    }

    /// Token 趋势缓存键
    pub fn token_trend(days: i32) -> String {
        format!("token:trend:{}", days)
    }

    /// 模型统计缓存键
    pub fn token_by_model() -> String {
        "token:by_model".to_string()
    }

    /// 单个需求缓存键
    pub fn requirement(id: &str) -> String {
        format!("requirements:{}", id)
    }

    /// 任务列表缓存键
    pub fn tasks_list(status: Option<&str>, task_type: Option<&str>) -> String {
        format!(
            "tasks:list:{}:{}",
            status.unwrap_or("all"),
            task_type.unwrap_or("all")
        )
    }

    /// 设置缓存键
    pub fn setting(key: &str) -> String {
        format!("setting:{}", key)
    }
}

/// LRU 内存缓存管理器
pub struct CacheManager {
    /// L1 内存缓存
    l1_cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    /// 缓存配置
    config: CacheConfig,
    /// 缓存统计
    stats: Arc<RwLock<CacheStats>>,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(config: CacheConfig) -> Self {
        let l1_cache = Arc::new(RwLock::new(
            LruCache::new(NonZeroUsize::new(config.l1_max_size).unwrap())
        ));

        Self {
            l1_cache,
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// 获取缓存
    pub async fn get<T: Clone + for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        if !self.config.enabled {
            return None;
        }

        let full_key = format!("{}{}", self.config.key_prefix, key);

        // 取出条目原始字节（克隆后再反序列化，避免在持有 L1 写锁时做较重的解析）
        let payload: Option<Vec<u8>> = {
            let mut cache = self.l1_cache.write().await;
            match cache.get(&full_key) {
                Some(entry) if !entry.is_expired() => Some(entry.data.clone()),
                Some(_) => {
                    cache.pop(&full_key);
                    debug!("缓存过期 (L1): {}", key);
                    None
                }
                None => None,
            }
        };

        if let Some(payload) = payload {
            match bincode::deserialize::<(u32, T)>(&payload) {
                Ok((version, data)) if version == CACHE_SCHEMA_VERSION => {
                    debug!("缓存命中 (L1): {}", key);
                    self.record_hit().await;
                    return Some(data);
                }
                Ok(_) => {
                    debug!("缓存条目 schema 版本不兼容，已失效 (L1): {}", key);
                    self.delete(key).await;
                }
                Err(_) => {
                    debug!("缓存条目反序列化失败，已失效 (L1): {}", key);
                    self.delete(key).await;
                }
            }
        }

        // 未命中
        self.record_miss().await;
        debug!("缓存未命中: {}", key);
        None
    }

    /// 设置缓存
    pub async fn set<T: Clone + Serialize>(&self, key: &str, data: &T, ttl_seconds: Option<u64>) {
        if !self.config.enabled {
            return;
        }

        let full_key = format!("{}{}", self.config.key_prefix, key);
        let ttl = ttl_seconds.unwrap_or(self.config.default_ttl);

        // 序列化数据：带 schema 版本，便于类型变更后识别不兼容条目
        let Ok(serialized) = bincode::serialize(&(CACHE_SCHEMA_VERSION, data)) else {
            return;
        };

        // 写入 L1 缓存。LRU 在容量已满且写入新键时会淘汰最久未使用的条目，
        // 此处据此记录 evictions（此前该计数恒为 0）。
        let evicted = {
            let mut cache = self.l1_cache.write().await;
            let existed = cache.contains(&full_key);
            let at_capacity = cache.len() >= cache.cap().get();
            cache.put(full_key, CacheEntry::new(serialized, ttl));
            !existed && at_capacity
        };

        if evicted {
            let mut stats = self.stats.write().await;
            stats.evictions += 1;
        }

        debug!("缓存已设置: {}, ttl={}s", key, ttl);
    }

    /// 删除缓存
    pub async fn delete(&self, key: &str) {
        let full_key = format!("{}{}", self.config.key_prefix, key);

        // 删除 L1 缓存
        {
            let mut cache = self.l1_cache.write().await;
            cache.pop(&full_key);
        }

        debug!("缓存已删除: {}", key);
    }

    /// 批量删除缓存 (使用模式匹配)
    pub async fn delete_pattern(&self, pattern: &str) {
        let prefix = format!("{}{}", self.config.key_prefix, pattern);

        // 删除 L1 缓存
        {
            let mut cache = self.l1_cache.write().await;
            let keys_to_remove: Vec<String> = cache
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(k, _)| k.clone())
                .collect();

            for key in keys_to_remove {
                cache.pop(&key);
            }
        }

        debug!("批量缓存已删除: pattern={}", pattern);
    }

    /// 清空所有缓存
    pub async fn clear(&self) {
        {
            let mut cache = self.l1_cache.write().await;
            cache.clear();
        }

        info!("所有缓存已清空");
    }

    /// 获取缓存统计
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await.clone();
        let cache = self.l1_cache.read().await;
        CacheStats {
            l1_size: cache.len(),
            ..stats
        }
    }

    /// 检查缓存是否启用
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    async fn record_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.hits += 1;
    }

    async fn record_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.misses += 1;
    }
}

/// 缓存管理器包装器 (用于 Tauri 状态)
#[derive(Clone)]
pub struct CacheWrapper {
    pub manager: Arc<CacheManager>,
}

impl CacheWrapper {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            manager: Arc::new(CacheManager::new(config)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let config = CacheConfig::default();
        let cache = CacheManager::new(config);

        // 设置缓存
        cache.set("test_key", &"test_value".to_string(), Some(60)).await;

        // 获取缓存
        let result: Option<String> = cache.get("test_key").await;
        assert_eq!(result, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let config = CacheConfig {
            default_ttl: 1,
            ..Default::default()
        };
        let cache = CacheManager::new(config);

        // 设置缓存
        cache.set("test_key", &"test_value".to_string(), None).await;

        // 等待过期
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // 获取缓存
        let result: Option<String> = cache.get("test_key").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_delete() {
        let config = CacheConfig::default();
        let cache = CacheManager::new(config);

        // 设置缓存
        cache.set("test_key", &"test_value".to_string(), Some(60)).await;

        // 删除缓存
        cache.delete("test_key").await;

        // 获取缓存
        let result: Option<String> = cache.get("test_key").await;
        assert_eq!(result, None);
    }

    #[test]
    fn test_cache_keys() {
        assert_eq!(CacheKeys::statistics(), "statistics:main");
        assert_eq!(CacheKeys::token_summary(), "token:summary");
        assert_eq!(CacheKeys::token_trend(7), "token:trend:7");
        assert_eq!(CacheKeys::requirement("req-1"), "requirements:req-1");
    }
}
