use crate::cache::CacheWrapper;
use crate::db::Database;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// 数据服务 - 负责全量数据的导出与导入
pub struct DataService<'a> {
    db: &'a Arc<Mutex<Database>>,
    cache: &'a CacheWrapper,
}

impl<'a> DataService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 导出全量数据快照（JSON）
    pub async fn export_data(&self) -> Result<Value, String> {
        let db = self.db.lock().await;
        let snapshot = db
            .export_snapshot()
            .map_err(|e| format!("导出数据失败: {}", e))?;

        let counts = snapshot
            .get("counts")
            .cloned()
            .unwrap_or_else(|| json!({}));
        info!("数据导出完成: {:?}", counts);

        Ok(snapshot)
    }

    /// 导入数据快照。
    ///
    /// - `replace`：true 表示先清空再写入（完全恢复），false 表示按主键合并
    /// - `dry_run`：true 时只做格式校验与影响面预估，不写入任何数据
    pub async fn import_data(
        &self,
        payload: &Value,
        replace: bool,
        dry_run: bool,
    ) -> Result<Value, String> {
        // 1. 格式校验
        let format = payload
            .get("format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "无效的导入文件：缺少 format 字段".to_string())?;
        if format != "ace-export" {
            return Err(format!("无效的导入文件：format 应为 ace-export，实际为 {}", format));
        }

        let version = payload
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let tables = payload
            .get("tables")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "无效的导入文件：缺少 tables 字段".to_string())?;

        // 2. 统计 payload 中各表行数
        let mut incoming = serde_json::Map::new();
        let mut total_incoming: i64 = 0;
        for (table, value) in tables {
            let n = value.as_array().map(|a| a.len()).unwrap_or(0) as i64;
            total_incoming += n;
            incoming.insert(table.clone(), json!(n));
        }

        if dry_run {
            let db = self.db.lock().await;
            let current = db
                .table_counts()
                .map_err(|e| format!("读取当前数据量失败: {}", e))?;
            return Ok(json!({
                "dry_run": true,
                "version": version,
                "incoming": incoming,
                "current": current,
                "total_incoming": total_incoming,
            }));
        }

        // 3. 执行导入
        let result = {
            let db = self.db.lock().await;
            db.import_snapshot(payload, replace)
                .map_err(|e| format!("导入数据失败: {}", e))?
        };

        // 4. 全量失效缓存，避免读到旧数据
        self.cache.manager.clear().await;

        let after = {
            let db = self.db.lock().await;
            db.table_counts().unwrap_or_else(|_| json!({}))
        };

        info!("数据导入完成: total={}, replace={}", total_incoming, replace);

        Ok(json!({
            "dry_run": false,
            "version": version,
            "replace": replace,
            "imported": result.get("tables").cloned().unwrap_or_else(|| json!({})),
            "total_imported": result.get("total").cloned().unwrap_or_else(|| json!(0)),
            "after": after,
        }))
    }
}
