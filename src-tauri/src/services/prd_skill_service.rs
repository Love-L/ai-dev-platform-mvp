use crate::db::{PrdSkill, Database};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

/// PRD Skill 服务 - 处理 PRD 技能模板的业务逻辑
pub struct PrdSkillService<'a> {
    db: &'a Arc<Mutex<Database>>,
}

impl<'a> PrdSkillService<'a> {
    pub fn new(db: &'a Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// 获取所有 PRD Skill 列表
    pub async fn list_skills(&self) -> Result<Vec<PrdSkill>, String> {
        let db = self.db.lock().await;
        db.list_prd_skills()
            .map_err(|e| {
                error!("获取Skill列表失败: {}", e);
                format!("获取Skill列表失败: {}", e)
            })
    }

    /// 获取指定 PRD Skill 详情
    pub async fn get_skill(&self, id: &str) -> Result<Option<PrdSkill>, String> {
        let db = self.db.lock().await;
        db.get_prd_skill(id)
            .map_err(|e| {
                error!("获取Skill详情失败: {}", e);
                format!("获取Skill详情失败: {}", e)
            })
    }

    /// 创建新的 PRD Skill
    pub async fn create_skill(&self, name: &str, skill_type: &str, description: Option<&str>, template: &str) -> Result<PrdSkill, String> {
        let db = self.db.lock().await;
        db.create_prd_skill(name, skill_type, description, template)
            .map_err(|e| {
                error!("创建Skill失败: {}", e);
                format!("创建Skill失败: {}", e)
            })
    }

    /// 更新 PRD Skill
    pub async fn update_skill(&self, id: &str, name: Option<&str>, description: Option<&str>, template: Option<&str>) -> Result<(), String> {
        let db = self.db.lock().await;
        db.update_prd_skill(id, name, description, template)
            .map_err(|e| {
                error!("更新Skill失败: {}", e);
                format!("更新Skill失败: {}", e)
            })
    }

    /// 删除 PRD Skill
    pub async fn delete_skill(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().await;
        db.delete_prd_skill(id)
            .map_err(|e| {
                error!("删除Skill失败: {}", e);
                format!("删除Skill失败: {}", e)
            })
    }

    /// 基于内置Skill创建自定义版本
    pub async fn create_custom_skill(&self, parent_id: &str, name: &str, template: Option<&str>) -> Result<PrdSkill, String> {
        let db = self.db.lock().await;
        db.create_custom_skill_from_builtin(parent_id, name, template)
            .map_err(|e| {
                error!("创建自定义Skill失败: {}", e);
                format!("创建自定义Skill失败: {}", e)
            })
    }
}
