use crate::db::{Database, Project, ProjectSkill, ProjectMcp};
use crate::cache::CacheWrapper;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

/// 项目服务 - 处理项目相关的业务逻辑
pub struct ProjectService<'a> {
    db: &'a Arc<Mutex<Database>>,
    /// 预留的缓存句柄（项目数据当前无缓存项，保留以便后续接入）
    #[allow(dead_code)]
    cache: &'a CacheWrapper,
}

impl<'a> ProjectService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>, cache: &'a CacheWrapper) -> Self {
        Self { db, cache }
    }

    /// 创建项目
    pub async fn create_project(
        &self,
        name: &str,
        description: Option<&str>,
        key: Option<&str>,
    ) -> Result<Project, String> {
        debug!("创建项目: name={}", name);
        let db = self.db.lock().await;
        db.create_project(name, description, key)
            .map_err(|e| {
                tracing::error!("创建项目失败: {}", e);
                format!("创建项目失败: {}", e)
            })
    }

    /// 获取项目列表
    pub async fn list_projects(&self) -> Result<Vec<Project>, String> {
        let db = self.db.lock().await;
        db.list_projects().map_err(|e| {
            tracing::error!("获取项目列表失败: {}", e);
            format!("获取项目列表失败: {}", e)
        })
    }

    /// 获取项目详情
    pub async fn get_project(&self, id: &str) -> Result<Option<Project>, String> {
        let db = self.db.lock().await;
        db.get_project(id).map_err(|e| {
            tracing::error!("获取项目详情失败: {}", e);
            format!("获取项目详情失败: {}", e)
        })
    }

    /// 更新项目
    #[allow(clippy::too_many_arguments)]
    pub async fn update_project(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        key: Option<&str>,
        status: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        owner_id: Option<&str>,
    ) -> Result<(), String> {
        debug!("更新项目: id={}", id);
        let db = self.db.lock().await;
        db.update_project(id, name, description, key, status, start_date, end_date, owner_id)
            .map_err(|e| {
                tracing::error!("更新项目失败: {}", e);
                format!("更新项目失败: {}", e)
            })?;
        Ok(())
    }

    /// 删除项目
    pub async fn delete_project(&self, id: &str) -> Result<(), String> {
        debug!("删除项目: id={}", id);
        let db = self.db.lock().await;
        db.delete_project(id).map_err(|e| {
            tracing::error!("删除项目失败: {}", e);
            format!("删除项目失败: {}", e)
        })?;
        Ok(())
    }

    /// 获取项目 Skill 列表
    pub async fn list_project_skills(&self, project_id: &str) -> Result<Vec<ProjectSkill>, String> {
        let db = self.db.lock().await;
        db.list_project_skills(project_id).map_err(|e| {
            tracing::error!("获取项目Skill失败: {}", e);
            format!("获取项目Skill失败: {}", e)
        })
    }

    /// 添加项目 Skill
    pub async fn add_project_skill(
        &self,
        project_id: &str,
        skill_id: &str,
        is_default: Option<bool>,
        custom_params: Option<&str>,
    ) -> Result<ProjectSkill, String> {
        debug!("添加项目Skill: project={}, skill={}", project_id, skill_id);
        let db = self.db.lock().await;
        db.add_project_skill(project_id, skill_id, is_default.unwrap_or(true), custom_params)
            .map_err(|e| {
                tracing::error!("添加项目Skill失败: {}", e);
                format!("添加项目Skill失败: {}", e)
            })
    }

    /// 移除项目 Skill
    pub async fn remove_project_skill(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().await;
        db.remove_project_skill(id).map_err(|e| {
            tracing::error!("移除项目Skill失败: {}", e);
            format!("移除项目Skill失败: {}", e)
        })?;
        Ok(())
    }

    /// 获取项目 MCP 列表
    pub async fn list_project_mcps(&self, project_id: &str) -> Result<Vec<ProjectMcp>, String> {
        let db = self.db.lock().await;
        db.list_project_mcps(project_id).map_err(|e| {
            tracing::error!("获取项目MCP失败: {}", e);
            format!("获取项目MCP失败: {}", e)
        })
    }

    /// 添加项目 MCP
    pub async fn add_project_mcp(
        &self,
        project_id: &str,
        mcp_name: &str,
        endpoint: &str,
        description: Option<&str>,
        is_default: Option<bool>,
        auth_config: Option<&str>,
        custom_params: Option<&str>,
    ) -> Result<ProjectMcp, String> {
        debug!("添加项目MCP: project={}, mcp={}", project_id, mcp_name);
        let db = self.db.lock().await;
        db.add_project_mcp(project_id, mcp_name, endpoint, description, is_default.unwrap_or(true), auth_config, custom_params)
            .map_err(|e| {
                tracing::error!("添加项目MCP失败: {}", e);
                format!("添加项目MCP失败: {}", e)
            })
    }

    /// 移除项目 MCP
    pub async fn remove_project_mcp(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().await;
        db.remove_project_mcp(id).map_err(|e| {
            tracing::error!("移除项目MCP失败: {}", e);
            format!("移除项目MCP失败: {}", e)
        })?;
        Ok(())
    }
}
