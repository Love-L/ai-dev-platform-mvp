use crate::cache::CacheKeys;
use crate::db::{Database, Requirement};
use crate::cache::CacheWrapper;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug};

/// 需求扩展字段（更新时可选传入，全部为 None 表示不修改）
#[derive(Debug, Default, Clone)]
pub struct RequirementExtFields<'a> {
    pub module: Option<&'a str>,
    pub source: Option<&'a str>,
    pub source_detail: Option<&'a str>,
    pub product_owner: Option<&'a str>,
    pub developer: Option<&'a str>,
    pub tester: Option<&'a str>,
    pub business_priority: Option<&'a str>,
    pub estimated_effort: Option<f64>,
    pub tags: Option<&'a str>,
    pub notes: Option<&'a str>,
}

impl<'a> RequirementExtFields<'a> {
    /// 是否包含任何需要写入的扩展字段
    pub fn has_any(&self) -> bool {
        self.module.is_some()
            || self.source.is_some()
            || self.source_detail.is_some()
            || self.product_owner.is_some()
            || self.developer.is_some()
            || self.tester.is_some()
            || self.business_priority.is_some()
            || self.estimated_effort.is_some()
            || self.tags.is_some()
            || self.notes.is_some()
    }
}

/// 需求服务 - 处理需求相关的业务逻辑
pub struct RequirementService<'a> {
    db: &'a Arc<Mutex<Database>>,
    cache: &'a CacheWrapper,
}

impl<'a> RequirementService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 创建需求
    pub async fn create_requirement(
        &self,
        title: &str,
        description: Option<&str>,
        priority: &str,
        project_id: Option<&str>,
        module: Option<&str>,
        source: Option<&str>,
        source_detail: Option<&str>,
        product_owner: Option<&str>,
        developer: Option<&str>,
        tester: Option<&str>,
        business_priority: Option<&str>,
        estimated_effort: Option<f64>,
        tags: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Requirement, String> {
        debug!("创建需求: title={}", title);
        let db = self.db.lock().await;
        let pid = project_id.unwrap_or("default-project");
        let result = db.create_requirement(pid, title, description, priority)
            .map_err(|e| {
                tracing::error!("创建需求失败: {}", e);
                format!("创建需求失败: {}", e)
            })?;

        // 更新新增字段
        db.update_requirement_fields(
            &result.id,
            module,
            source,
            source_detail,
            product_owner,
            developer,
            tester,
            business_priority,
            estimated_effort,
            None, // actual_effort
            None, // attachments
            tags,
            notes,
        ).map_err(|e| {
            tracing::error!("更新需求字段失败: {}", e);
            format!("更新需求字段失败: {}", e)
        })?;

        info!("需求创建成功: id={}", result.id);

        // 记录变更历史（审计）
        if let Err(e) = db.insert_requirement_history(&result.id, "create", None, Some(title)) {
            tracing::error!("写入需求创建历史失败: {}", e);
        }

        // 重新获取完整的需求数据
        db.get_requirement(&result.id)
            .map_err(|e| format!("获取需求失败: {}", e))?
            .ok_or("需求不存在".to_string())
    }

    /// 获取需求列表（分页）
    pub async fn list_requirements_paged(
        &self,
        params: &crate::pagination::PaginationParams,
        status: Option<&str>,
    ) -> Result<crate::pagination::PagedResult<Requirement>, String> {
        debug!(
            "查询需求列表（分页）: page={}, page_size={}, offset={}, status={:?}",
            params.page,
            params.page_size,
            params.offset(),
            status
        );

        let db = self.db.lock().await;
        let result = db.list_requirements_paged(params.page, params.limit(), status)
            .map_err(|e| {
                tracing::error!("获取需求列表失败: {}", e);
                format!("获取需求列表失败: {}", e)
            })?;

        Ok(result)
    }

    /// 获取单个需求（带缓存）
    pub async fn get_requirement(&self, id: &str) -> Result<Option<Requirement>, String> {
        let cache_key = CacheKeys::requirement(id);

        // 1. 尝试从缓存获取
        if let Some(cached) = self.cache.manager.get::<Option<Requirement>>(&cache_key).await {
            debug!("从缓存获取需求详情: id={}", id);
            return Ok(cached);
        }

        debug!("从数据库查询需求详情: id={}", id);

        // 2. 缓存未命中，查询数据库
        let db = self.db.lock().await;
        let result = db.get_requirement(id)
            .map_err(|e| {
                tracing::error!("获取需求详情失败: {}", e);
                format!("获取需求详情失败: {}", e)
            })?;

        // 3. 写入缓存 (TTL: 30秒)
        self.cache.manager.set(&cache_key, &result, Some(30)).await;

        Ok(result)
    }

    /// 更新需求（带缓存失效）
    #[allow(clippy::too_many_arguments)]
    pub async fn update_requirement(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        priority: Option<&str>,
        status: Option<&str>,
        prd_content: Option<&str>,
        ext: &RequirementExtFields<'_>,
    ) -> Result<(), String> {
        debug!("更新需求: id={}", id);

        // 1. 更新数据库，并记录变更历史（审计）
        {
            let db = self.db.lock().await;

            // 1.1 读取更新前的快照，用于生成审计差异
            let before = db.get_requirement(id).unwrap_or(None);

            db.update_requirement(id, title, description, priority, status, prd_content)
                .map_err(|e| {
                    tracing::error!("更新需求失败: {}", e);
                    format!("更新需求失败: {}", e)
                })?;

            // 1.2 扩展字段（module/source/owner/effort/tags/notes 等）
            if ext.has_any() {
                db.update_requirement_fields(
                    id,
                    ext.module,
                    ext.source,
                    ext.source_detail,
                    ext.product_owner,
                    ext.developer,
                    ext.tester,
                    ext.business_priority,
                    ext.estimated_effort,
                    None, // actual_effort
                    None, // attachments
                    ext.tags,
                    ext.notes,
                )
                .map_err(|e| {
                    tracing::error!("更新需求扩展字段失败: {}", e);
                    format!("更新需求扩展字段失败: {}", e)
                })?;
            }

            // 1.3 生成审计记录：状态变更单独成条，其它字段逐项记录
            if let Some(old) = before {
                let mut entries: Vec<(&str, String, String)> = Vec::new();

                if let Some(new_module) = ext.module {
                    let old_module = old.module.clone().unwrap_or_default();
                    if new_module != old_module {
                        entries.push((
                            "update",
                            format!("module: {}", if old_module.is_empty() { "-" } else { old_module.as_str() }),
                            format!("module: {}", new_module),
                        ));
                    }
                }
                if let Some(new_source) = ext.source {
                    let old_source = old.source.clone().unwrap_or_default();
                    if new_source != old_source {
                        entries.push(("update", format!("source: {}", old_source), format!("source: {}", new_source)));
                    }
                }
                if let Some(new_bp) = ext.business_priority {
                    let old_bp = old.business_priority.clone().unwrap_or_default();
                    if new_bp != old_bp {
                        entries.push(("update", format!("business_priority: {}", old_bp), format!("business_priority: {}", new_bp)));
                    }
                }
                if let Some(new_effort) = ext.estimated_effort {
                    let old_effort = old.estimated_effort.unwrap_or(0.0);
                    if (new_effort - old_effort).abs() > f64::EPSILON {
                        entries.push((
                            "update",
                            format!("estimated_effort: {}", old_effort),
                            format!("estimated_effort: {}", new_effort),
                        ));
                    }
                }

                if let Some(new_status) = status {
                    if new_status != old.status {
                        entries.push(("status_change", old.status.clone(), new_status.to_string()));
                    }
                }
                if let Some(new_title) = title {
                    if new_title != old.title {
                        entries.push(("update", format!("title: {}", old.title), format!("title: {}", new_title)));
                    }
                }
                if let Some(new_priority) = priority {
                    if new_priority != old.priority {
                        entries.push(("update", format!("priority: {}", old.priority), format!("priority: {}", new_priority)));
                    }
                }
                if let Some(new_desc) = description {
                    let old_desc = old.description.clone().unwrap_or_default();
                    if new_desc != old_desc {
                        entries.push(("update", "description: 已修改".to_string(), "description: 已修改".to_string()));
                    }
                }
                if let Some(new_prd) = prd_content {
                    let old_prd = old.prd_content.clone().unwrap_or_default();
                    if new_prd != old_prd {
                        entries.push(("update", "prd_content: 已修改".to_string(), "prd_content: 已修改".to_string()));
                    }
                }

                for (action, old_value, new_value) in entries {
                    if let Err(e) = db.insert_requirement_history(id, action, Some(&old_value), Some(&new_value)) {
                        tracing::error!("写入需求变更历史失败: {}", e);
                    }
                }
            }
        }

        // 2. 使相关缓存失效
        self.cache.manager.delete(&CacheKeys::requirement(id)).await;
        self.cache.manager.delete(&CacheKeys::statistics()).await;

        info!("需求更新成功: id={}", id);
        Ok(())
    }

    /// 删除需求（带缓存失效）
    pub async fn delete_requirement(&self, id: &str) -> Result<(), String> {
        debug!("删除需求: id={}", id);

        // 1. 删除数据库（先写审计快照，再删除；否则外键约束会阻止写入历史）
        {
            let db = self.db.lock().await;

            let before = db.get_requirement(id).unwrap_or(None);
            if let Some(old) = &before {
                if let Err(e) = db.insert_requirement_history(
                    id,
                    "delete",
                    Some(&old.title),
                    Some(&old.status),
                ) {
                    tracing::error!("写入需求删除历史失败: {}", e);
                }
            }

            db.delete_requirement(id)
                .map_err(|e| {
                    tracing::error!("删除需求失败: {}", e);
                    format!("删除需求失败: {}", e)
                })?;
        }

        // 2. 使相关缓存失效
        self.cache.manager.delete(&CacheKeys::requirement(id)).await;
        self.cache.manager.delete(&CacheKeys::statistics()).await;

        info!("需求删除成功: id={}", id);
        Ok(())
    }
}
