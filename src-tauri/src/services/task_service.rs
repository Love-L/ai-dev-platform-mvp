use crate::cache::CacheKeys;
use crate::db::{Database, Task};
use crate::cache::CacheWrapper;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug};

/// 任务服务 - 处理任务相关的业务逻辑
pub struct TaskService<'a> {
    db: &'a Arc<Mutex<Database>>,
    cache: &'a CacheWrapper,
}

impl<'a> TaskService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 创建任务（project_id 缺省时经 requirement 自动归属项目）
    pub async fn create_task(
        &self,
        title: &str,
        requirement_id: Option<&str>,
        task_type: &str,
        priority: &str,
        assignee: Option<&str>,
        due_date: Option<&str>,
        estimated_hours: Option<f64>,
        priority_level: Option<&str>,
        blocked_reason: Option<&str>,
        tags: Option<&str>,
        notes: Option<&str>,
        description: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Task, String> {
        debug!("创建任务: title={}, type={}", title, task_type);

        let result = {
            let db = self.db.lock().await;
            db.create_task(title, requirement_id, task_type, priority, description, project_id)
                .map_err(|e| {
                    tracing::error!("创建任务失败: {}", e);
                    format!("创建任务失败: {}", e)
                })?
        };

        // 更新新增字段
        {
            let db = self.db.lock().await;
            db.update_task_fields(
                &result.id,
                assignee,
                due_date,
                estimated_hours,
                None, // actual_hours
                priority_level,
                blocked_reason,
                None, // related_requirement_id
                None, // related_project_id
                tags,
                notes,
                None, // attachments
            ).map_err(|e| {
                tracing::error!("更新任务字段失败: {}", e);
                format!("更新任务字段失败: {}", e)
            })?;
        }

        // 使相关缓存失效
        self.cache.manager.delete_pattern("tasks:list:").await;
        self.cache.manager.delete(&CacheKeys::statistics()).await;

        info!("任务创建成功: id={}", result.id);

        // 重新获取完整的任务数据
        let db = self.db.lock().await;
        db.get_task(&result.id)
            .map_err(|e| format!("获取任务失败: {}", e))?
            .ok_or("任务不存在".to_string())
    }

    /// 获取任务列表（带缓存）
    pub async fn list_tasks(
        &self,
        status: Option<&str>,
        task_type: Option<&str>,
    ) -> Result<Vec<Task>, String> {
        let cache_key = CacheKeys::tasks_list(status, task_type);

        // 1. 尝试从缓存获取
        if let Some(cached) = self.cache.manager.get::<Vec<Task>>(&cache_key).await {
            debug!("从缓存获取任务列表: status={:?}, type={:?}", status, task_type);
            return Ok(cached);
        }

        debug!("从数据库查询任务列表: status={:?}, type={:?}", status, task_type);

        // 2. 缓存未命中，查询数据库
        let db = self.db.lock().await;
        let result = db.list_tasks(status, task_type)
            .map_err(|e| {
                tracing::error!("获取任务列表失败: {}", e);
                format!("获取任务列表失败: {}", e)
            })?;

        // 3. 写入缓存 (TTL: 30秒)
        self.cache.manager.set(&cache_key, &result, Some(30)).await;

        Ok(result)
    }

    /// 更新任务状态（带缓存失效）
    ///
    /// 说明：曾有「状态置 in_progress 时自动建会话」的自动模式分支，
    /// 但 agent_service 从未被注入（命令层恒用 `new`），属死代码，已删除。
    pub async fn update_task_status(&self, id: &str, status: &str) -> Result<(), String> {
        debug!("更新任务状态: id={}, status={}", id, status);

        {
            let db = self.db.lock().await;
            db.update_task_status(id, status)
                .map_err(|e| {
                    tracing::error!("更新任务状态失败: {}", e);
                    format!("更新任务状态失败: {}", e)
                })?;
        }

        // 使相关缓存失效
        self.cache.manager.delete_pattern("tasks:list:").await;
        self.cache.manager.delete(&CacheKeys::statistics()).await;

        info!("任务状态更新成功: id={}, status={}", id, status);
        Ok(())
    }

    /// 删除任务（带缓存失效）
    pub async fn delete_task(&self, id: &str) -> Result<(), String> {
        debug!("删除任务: id={}", id);

        {
            let db = self.db.lock().await;
            db.delete_task(id)
                .map_err(|e| {
                    tracing::error!("删除任务失败: {}", e);
                    format!("删除任务失败: {}", e)
                })?;
        }

        // 使相关缓存失效
        self.cache.manager.delete_pattern("tasks:list:").await;
        self.cache.manager.delete(&CacheKeys::statistics()).await;

        info!("任务删除成功: id={}", id);
        Ok(())
    }
}
