use rusqlite::{Connection, Result, params};
use rusqlite::OptionalExtension;
use super::models::{Project, ProjectSkill, ProjectMcp, StageSkillConfig};

pub fn create_project(conn: &Connection, name: &str, description: Option<&str>, key: Option<&str>) -> Result<Project> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO projects (id, name, description, key, status, created_at, updated_at) VALUES (?, ?, ?, ?, 'active', ?, ?)",
        params![id, name, description, key, now, now],
    )?;

    Ok(Project {
        id,
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        key: key.map(|s| s.to_string()),
        status: Some("active".to_string()),
        owner_id: None,
        start_date: None,
        end_date: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, key, status, owner_id, start_date, end_date, created_at, updated_at FROM projects ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            key: row.get(3)?,
            status: row.get(4)?,
            owner_id: row.get(5)?,
            start_date: row.get(6)?,
            end_date: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    rows.collect()
}

pub fn get_project(conn: &Connection, id: &str) -> Result<Option<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, key, status, owner_id, start_date, end_date, created_at, updated_at FROM projects WHERE id = ?"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            key: row.get(3)?,
            status: row.get(4)?,
            owner_id: row.get(5)?,
            start_date: row.get(6)?,
            end_date: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn update_project(conn: &Connection, id: &str, name: Option<&str>, description: Option<&str>, key: Option<&str>, status: Option<&str>) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(n) = name { updates.push("name = ?"); params.push(Box::new(n.to_string())); }
    if let Some(d) = description { updates.push("description = ?"); params.push(Box::new(d.to_string())); }
    if let Some(k) = key { updates.push("key = ?"); params.push(Box::new(k.to_string())); }
    if let Some(s) = status { updates.push("status = ?"); params.push(Box::new(s.to_string())); }

    if updates.is_empty() { return Ok(()); }

    updates.push("updated_at = ?");
    params.push(Box::new(now.clone()));
    params.push(Box::new(id.to_string()));

    let sql = format!("UPDATE projects SET {} WHERE id = ?", updates.join(", "));
    conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;

    Ok(())
}

pub fn delete_project(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM projects WHERE id = ?", params![id])?;
    Ok(())
}

// ==================== 项目 Skill 配置 ====================

pub fn list_project_skills(conn: &Connection, project_id: &str) -> Result<Vec<ProjectSkill>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM project_skills WHERE project_id = ? ORDER BY is_default DESC, created_at"
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(ProjectSkill {
            id: row.get(0)?,
            project_id: row.get(1)?,
            skill_id: row.get(2)?,
            is_default: row.get::<_, i32>(3)? != 0,
            custom_params: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

/// 为项目绑定 Skill。
///
/// 幂等：同一 `(project_id, skill_id)` 已存在时直接返回已有记录并更新默认标记，
/// 避免因唯一约束（`uq_project_skills`）报错，也避免重复绑定。
pub fn add_project_skill(conn: &Connection, project_id: &str, skill_id: &str, is_default: bool, custom_params: Option<&str>) -> Result<ProjectSkill> {
    if let Some(existing) = find_project_skill(conn, project_id, skill_id)? {
        conn.execute(
            "UPDATE project_skills SET is_default = ?, custom_params = COALESCE(?, custom_params) WHERE id = ?",
            params![is_default as i32, custom_params, existing.id],
        )?;

        // 同一项目内保持"仅一个默认"
        if is_default {
            conn.execute(
                "UPDATE project_skills SET is_default = 0 WHERE project_id = ? AND id <> ?",
                params![project_id, existing.id],
            )?;
        }

        let merged_params = custom_params
            .map(|s| s.to_string())
            .or_else(|| existing.custom_params.clone());

        return Ok(ProjectSkill {
            is_default,
            custom_params: merged_params,
            ..existing
        });
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO project_skills (id, project_id, skill_id, is_default, custom_params, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        params![id, project_id, skill_id, is_default as i32, custom_params, now],
    )?;

    if is_default {
        conn.execute(
            "UPDATE project_skills SET is_default = 0 WHERE project_id = ? AND id <> ?",
            params![project_id, id],
        )?;
    }

    Ok(ProjectSkill {
        id,
        project_id: project_id.to_string(),
        skill_id: skill_id.to_string(),
        is_default,
        custom_params: custom_params.map(|s| s.to_string()),
        created_at: now,
    })
}

/// 按 (project_id, skill_id) 查找绑定记录
fn find_project_skill(conn: &Connection, project_id: &str, skill_id: &str) -> Result<Option<ProjectSkill>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, skill_id, is_default, custom_params, created_at
         FROM project_skills WHERE project_id = ? AND skill_id = ? LIMIT 1",
    )?;

    let mut rows = stmt.query_map(params![project_id, skill_id], |row| {
        Ok(ProjectSkill {
            id: row.get(0)?,
            project_id: row.get(1)?,
            skill_id: row.get(2)?,
            is_default: row.get::<_, i32>(3)? != 0,
            custom_params: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn remove_project_skill(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM project_skills WHERE id = ?", params![id])?;
    Ok(())
}

// ==================== 项目 MCP 配置 ====================

pub fn list_project_mcps(conn: &Connection, project_id: &str) -> Result<Vec<ProjectMcp>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM project_mcps WHERE project_id = ? ORDER BY is_default DESC, created_at"
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(ProjectMcp {
            id: row.get(0)?,
            project_id: row.get(1)?,
            mcp_name: row.get(2)?,
            endpoint: row.get(3)?,
            description: row.get(4)?,
            is_default: row.get::<_, i32>(5)? != 0,
            auth_config: row.get(6)?,
            custom_params: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;
    rows.collect()
}

/// 为项目添加 MCP 端点。
///
/// 若 `is_default` 为 true，会先把该项目其它 MCP 的默认标记清除，
/// 以保持"每个项目至多一个默认 MCP"（与 `uq_project_mcps_default` 一致）。
pub fn add_project_mcp(conn: &Connection, project_id: &str, mcp_name: &str, endpoint: &str, description: Option<&str>, is_default: bool, auth_config: Option<&str>, custom_params: Option<&str>) -> Result<ProjectMcp> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    if is_default {
        conn.execute(
            "UPDATE project_mcps SET is_default = 0 WHERE project_id = ?",
            params![project_id],
        )?;
    }

    conn.execute(
        "INSERT INTO project_mcps (id, project_id, mcp_name, endpoint, description, is_default, auth_config, custom_params, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![id, project_id, mcp_name, endpoint, description, is_default as i32, auth_config, custom_params, now],
    )?;

    Ok(ProjectMcp {
        id,
        project_id: project_id.to_string(),
        mcp_name: mcp_name.to_string(),
        endpoint: endpoint.to_string(),
        description: description.map(|s| s.to_string()),
        is_default,
        auth_config: auth_config.map(|s| s.to_string()),
        custom_params: custom_params.map(|s| s.to_string()),
        created_at: now,
    })
}

pub fn remove_project_mcp(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM project_mcps WHERE id = ?", params![id])?;
    Ok(())
}

// ==================== 环节 Skill 配置 ====================

pub fn list_stage_skill_configs(conn: &Connection, project_id: &str) -> Result<Vec<StageSkillConfig>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM stage_skill_configs WHERE project_id = ? ORDER BY stage"
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(StageSkillConfig {
            id: row.get(0)?,
            project_id: row.get(1)?,
            stage: row.get(2)?,
            skill_id: row.get(3)?,
            is_default: row.get::<_, i32>(4)? != 0,
            custom_params: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn get_stage_skill_config(conn: &Connection, project_id: &str, stage: &str) -> Result<Option<StageSkillConfig>> {
    conn.query_row(
        "SELECT * FROM stage_skill_configs WHERE project_id = ? AND stage = ?",
        params![project_id, stage],
        |row| Ok(StageSkillConfig {
            id: row.get(0)?,
            project_id: row.get(1)?,
            stage: row.get(2)?,
            skill_id: row.get(3)?,
            is_default: row.get::<_, i32>(4)? != 0,
            custom_params: row.get(5)?,
            created_at: row.get(6)?,
        }),
    ).optional()
}

pub fn upsert_stage_skill_config(conn: &Connection, project_id: &str, stage: &str, skill_id: &str, is_default: bool, custom_params: Option<&str>) -> Result<StageSkillConfig> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO stage_skill_configs (id, project_id, stage, skill_id, is_default, custom_params, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(project_id, stage) DO UPDATE SET
            skill_id = excluded.skill_id,
            is_default = excluded.is_default,
            custom_params = excluded.custom_params",
        params![id, project_id, stage, skill_id, is_default as i32, custom_params, now],
    )?;

    Ok(StageSkillConfig {
        id,
        project_id: project_id.to_string(),
        stage: stage.to_string(),
        skill_id: skill_id.to_string(),
        is_default,
        custom_params: custom_params.map(|s| s.to_string()),
        created_at: now,
    })
}

pub fn remove_stage_skill_config(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM stage_skill_configs WHERE id = ?", params![id])?;
    Ok(())
}

pub fn get_stage_skill_for_requirement(conn: &Connection, project_id: &str, stage: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT skill_id FROM stage_skill_configs WHERE project_id = ? AND stage = ? AND is_default = 1",
        params![project_id, stage],
        |row| row.get(0),
    ).optional()
}
