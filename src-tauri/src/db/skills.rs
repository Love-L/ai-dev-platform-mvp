use rusqlite::{Connection, Result, params};
use super::models::{McpExecutionLog, PrdSkill, SkillExecutionLog, SkillScore, SkillSuggestion, SkillVersion};

pub fn list_prd_skills(conn: &Connection) -> Result<Vec<PrdSkill>> {
    let mut stmt = conn.prepare("SELECT id, name, description, skill_type, template, is_builtin, version, parent_id, is_deleted, created_at FROM prd_skills WHERE is_deleted = 0 ORDER BY is_builtin DESC, name")?;
    let rows = stmt.query_map([], |row| {
        Ok(PrdSkill {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            skill_type: row.get(3)?,
            template: row.get(4)?,
            is_builtin: row.get::<_, i32>(5)? != 0,
            version: row.get(6)?,
            parent_id: row.get(7)?,
            is_deleted: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
        })
    })?;

    rows.collect()
}

pub fn get_prd_skill(conn: &Connection, id: &str) -> Result<Option<PrdSkill>> {
    let mut stmt = conn.prepare("SELECT id, name, description, skill_type, template, is_builtin, version, parent_id, is_deleted, created_at FROM prd_skills WHERE id = ? AND is_deleted = 0")?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(PrdSkill {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            skill_type: row.get(3)?,
            template: row.get(4)?,
            is_builtin: row.get::<_, i32>(5)? != 0,
            version: row.get(6)?,
            parent_id: row.get(7)?,
            is_deleted: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn create_prd_skill(conn: &Connection, name: &str, skill_type: &str, description: Option<&str>, template: &str) -> Result<PrdSkill> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO prd_skills (id, name, description, skill_type, template, is_builtin, version, parent_id, is_deleted, created_at)
         VALUES (?, ?, ?, ?, ?, 0, 1, NULL, 0, ?)",
        params![id, name, description, skill_type, template, now],
    )?;
    let _ = insert_skill_version(conn, &id, 1, template, Some("初始版本"));

    Ok(PrdSkill {
        id,
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        skill_type: skill_type.to_string(),
        template: template.to_string(),
        is_builtin: false,
        version: 1,
        parent_id: None,
        is_deleted: false,
        created_at: now,
    })
}

pub fn create_custom_skill_from_builtin(conn: &Connection, parent_id: &str, name: &str, template: Option<&str>) -> Result<PrdSkill> {
    // 获取父Skill信息
    let parent = get_prd_skill(conn, parent_id)?
        .ok_or(rusqlite::Error::InvalidParameterName("父Skill不存在".to_string()))?;

    if !parent.is_builtin {
        return Err(rusqlite::Error::InvalidParameterName("只能基于内置Skill创建自定义版本".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let custom_name = name.to_string();
    let custom_template = template.unwrap_or(&parent.template);

    conn.execute(
        "INSERT INTO prd_skills (id, name, description, skill_type, template, is_builtin, version, parent_id, is_deleted, created_at)
         VALUES (?, ?, ?, ?, ?, 0, 1, ?, 0, ?)",
        params![id, custom_name, parent.description, parent.skill_type, custom_template, parent_id, now],
    )?;
    let _ = insert_skill_version(conn, &id, 1, custom_template, Some("基于内置版本创建"));

    Ok(PrdSkill {
        id,
        name: custom_name,
        description: parent.description,
        skill_type: parent.skill_type,
        template: custom_template.to_string(),
        is_builtin: false,
        version: 1,
        parent_id: Some(parent_id.to_string()),
        is_deleted: false,
        created_at: now,
    })
}

/// 更新 Skill。
///
/// 版本号按**操作**递增：一次调用（无论修改几个字段）只 +1，
/// 而不是按字段累加（此前改 3 个字段会导致版本 +3）。
pub fn update_prd_skill(conn: &Connection, id: &str, name: Option<&str>, description: Option<&str>, template: Option<&str>) -> Result<()> {
    // 检查是否是内置Skill
    let is_builtin: bool = conn.query_row(
        "SELECT is_builtin FROM prd_skills WHERE id = ?",
        params![id],
        |row| row.get::<_, i32>(0),
    ).map(|v| v != 0).unwrap_or(false);

    if is_builtin {
        return Err(rusqlite::Error::InvalidParameterName("内置Skill不可修改".to_string()));
    }

    // 没有任何字段需要更新时不产生新版本
    if name.is_none() && description.is_none() && template.is_none() {
        return Ok(());
    }

    conn.execute(
        "UPDATE prd_skills
         SET name = COALESCE(?1, name),
             description = COALESCE(?2, description),
             template = COALESCE(?3, template),
             version = version + 1
         WHERE id = ?4",
        params![name, description, template, id],
    )?;

    // 版本快照：记录变更后的完整状态（供版本历史/回退使用）
    let new_version: i32 = conn.query_row(
        "SELECT version FROM prd_skills WHERE id = ?1",
        params![id],
        |row| row.get(0),
    ).unwrap_or(1);
    let final_template: String = match template {
        Some(t) => t.to_string(),
        None => conn.query_row(
            "SELECT template FROM prd_skills WHERE id = ?1",
            params![id],
            |row| row.get(0),
        ).unwrap_or_default(),
    };
    let _ = insert_skill_version(conn, id, new_version, &final_template, None);

    Ok(())
}

pub fn delete_prd_skill(conn: &Connection, id: &str) -> Result<()> {
    // 检查是否是内置Skill
    let is_builtin: bool = conn.query_row(
        "SELECT is_builtin FROM prd_skills WHERE id = ?",
        params![id],
        |row| row.get::<_, i32>(0),
    ).map(|v| v != 0).unwrap_or(false);

    if is_builtin {
        return Err(rusqlite::Error::InvalidParameterName("内置Skill不可删除".to_string()));
    }

    // 使用软删除
    conn.execute("UPDATE prd_skills SET is_deleted = 1 WHERE id = ?", params![id])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn record_skill_execution_log(
    conn: &Connection,
    task_id: Option<&str>,
    session_id: Option<&str>,
    requirement_id: Option<&str>,
    skill_id: &str,
    skill_name: &str,
    skill_type: &str,
    execution_phase: &str,
    input_params: Option<&str>,
    output_summary: Option<&str>,
    token_usage: i64,
    status: &str,
    duration_ms: Option<i64>,
    error_message: Option<&str>,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO skill_execution_logs (id, task_id, session_id, requirement_id, skill_id, skill_name, skill_type, execution_phase, input_params, output_summary, token_usage, status, executed_at, duration_ms, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![id, task_id, session_id, requirement_id, skill_id, skill_name, skill_type, execution_phase, input_params, output_summary, token_usage, status, now, duration_ms, error_message],
    )?;
    Ok(())
}

/// 行映射：`SELECT *` 读取 `skill_execution_logs`（新列追加在表尾）。
fn map_skill_log(row: &rusqlite::Row) -> rusqlite::Result<SkillExecutionLog> {
    Ok(SkillExecutionLog {
        id: row.get(0)?,
        task_id: row.get(1)?,
        session_id: row.get(2)?,
        requirement_id: row.get(3)?,
        skill_id: row.get(4)?,
        skill_name: row.get(5)?,
        skill_type: row.get(6)?,
        execution_phase: row.get(7)?,
        input_params: row.get(8)?,
        output_summary: row.get(9)?,
        token_usage: row.get(10)?,
        status: row.get(11)?,
        executed_at: row.get(12)?,
        duration_ms: row.get(13)?,
        error_message: row.get(14)?,
    })
}

pub fn get_skill_execution_logs_by_task(conn: &Connection, task_id: &str) -> Result<Vec<SkillExecutionLog>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM skill_execution_logs WHERE task_id = ? ORDER BY executed_at DESC"
    )?;
    let rows = stmt.query_map(params![task_id], map_skill_log)?;
    rows.collect()
}

pub fn get_skill_execution_logs_by_session(conn: &Connection, session_id: &str) -> Result<Vec<SkillExecutionLog>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM skill_execution_logs WHERE session_id = ? ORDER BY executed_at DESC"
    )?;
    let rows = stmt.query_map(params![session_id], map_skill_log)?;
    rows.collect()
}

/// 按 Skill 查询执行日志（进化视图"执行日志 Tab"数据源）
pub fn get_skill_execution_logs_by_skill(conn: &Connection, skill_id: &str, limit: i64) -> Result<Vec<SkillExecutionLog>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM skill_execution_logs WHERE skill_id = ? ORDER BY executed_at DESC LIMIT ?"
    )?;
    let rows = stmt.query_map(params![skill_id, limit], map_skill_log)?;
    rows.collect()
}

pub fn get_mcp_execution_logs(conn: &Connection, session_id: &str) -> Result<Vec<McpExecutionLog>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM mcp_execution_logs WHERE session_id = ? ORDER BY executed_at DESC"
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        Ok(McpExecutionLog {
            id: row.get(0)?,
            session_id: row.get(1)?,
            mcp_name: row.get(2)?,
            endpoint: row.get(3)?,
            action: row.get(4)?,
            status: row.get(5)?,
            error_message: row.get(6)?,
            duration_ms: row.get(7)?,
            executed_at: row.get(8)?,
        })
    })?;
    rows.collect()
}

/// 按 `skill_id` 聚合的使用统计（成功率/均值/最近使用时间）。
///
/// 此前按 `skill_name` 聚合，重名或改名后统计会失真。
pub fn get_skill_usage_stats(conn: &Connection) -> Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT skill_id,
                MAX(skill_name) as skill_name,
                MAX(skill_type) as skill_type,
                COUNT(*) as use_count,
                SUM(token_usage) as total_tokens,
                SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as fail_count,
                AVG(duration_ms) as avg_duration_ms,
                MAX(executed_at) as last_used_at
         FROM skill_execution_logs
         GROUP BY skill_id
         ORDER BY use_count DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "skill_id": row.get::<_, String>(0)?,
            "skill_name": row.get::<_, String>(1)?,
            "skill_type": row.get::<_, Option<String>>(2)?,
            "use_count": row.get::<_, i64>(3)?,
            "total_tokens": row.get::<_, i64>(4)?,
            "success_count": row.get::<_, i64>(5)?,
            "fail_count": row.get::<_, i64>(6)?,
            "avg_duration_ms": row.get::<_, Option<f64>>(7)?,
            "last_used_at": row.get::<_, Option<String>>(8)?,
        }))
    })?;
    rows.collect()
}

/// 单个 Skill 的日志聚合（L1 规则诊断输入）。
pub fn get_skill_log_aggregate(conn: &Connection, skill_id: &str) -> Result<serde_json::Value> {
    let row = conn.query_row(
        "SELECT COUNT(*),
                SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END),
                COALESCE(AVG(token_usage), 0),
                COALESCE(AVG(duration_ms), 0),
                MAX(executed_at)
         FROM skill_execution_logs WHERE skill_id = ?",
        params![skill_id],
        |r| {
            Ok(serde_json::json!({
                "use_count": r.get::<_, i64>(0)?,
                "success_count": r.get::<_, i64>(1)?,
                "fail_count": r.get::<_, i64>(2)?,
                "avg_tokens": r.get::<_, f64>(3)?,
                "avg_duration_ms": r.get::<_, f64>(4)?,
                "last_used_at": r.get::<_, Option<String>>(5)?,
            }))
        },
    ).unwrap_or_else(|_| serde_json::json!({
        "use_count": 0, "success_count": 0, "fail_count": 0,
        "avg_tokens": 0.0, "avg_duration_ms": 0.0, "last_used_at": null,
    }));
    Ok(row)
}

// ==================== Skill 自进化：评分 / 建议 / 版本快照 ====================

pub fn insert_skill_score(
    conn: &Connection,
    skill_id: &str,
    version: i32,
    structure_score: i32,
    effect_score: Option<i32>,
    total_score: i32,
    dimensions_json: &str,
    judge_reason: Option<&str>,
) -> Result<SkillScore> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO skill_scores (id, skill_id, version, structure_score, effect_score, total_score, dimensions_json, judge_reason, judged_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, skill_id, version, structure_score, effect_score, total_score, dimensions_json, judge_reason, now],
    )?;
    Ok(SkillScore {
        id, skill_id: skill_id.to_string(), version, structure_score,
        effect_score, total_score, dimensions_json: dimensions_json.to_string(),
        judge_reason: judge_reason.map(|s| s.to_string()), judged_at: now,
    })
}

fn map_skill_score(row: &rusqlite::Row) -> rusqlite::Result<SkillScore> {
    Ok(SkillScore {
        id: row.get(0)?,
        skill_id: row.get(1)?,
        version: row.get(2)?,
        structure_score: row.get(3)?,
        effect_score: row.get(4)?,
        total_score: row.get(5)?,
        dimensions_json: row.get(6)?,
        judge_reason: row.get(7)?,
        judged_at: row.get(8)?,
    })
}

pub fn get_skill_score_history(conn: &Connection, skill_id: &str) -> Result<Vec<SkillScore>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_id, version, structure_score, effect_score, total_score, dimensions_json, judge_reason, judged_at
         FROM skill_scores WHERE skill_id = ? ORDER BY judged_at ASC"
    )?;
    let rows = stmt.query_map(params![skill_id], map_skill_score)?;
    rows.collect()
}

/// 最近一次评分（棘轮判定基线）
pub fn get_latest_skill_score(conn: &Connection, skill_id: &str) -> Result<Option<SkillScore>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_id, version, structure_score, effect_score, total_score, dimensions_json, judge_reason, judged_at
         FROM skill_scores WHERE skill_id = ? ORDER BY judged_at DESC LIMIT 1"
    )?;
    let mut rows = stmt.query_map(params![skill_id], map_skill_score)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// 写入/刷新建议：同一 `skill_id + rule_type` 仅保留一条 pending。
pub fn upsert_skill_suggestion(
    conn: &Connection,
    skill_id: &str,
    rule_type: &str,
    severity: &str,
    message: &str,
    dimension: Option<&str>,
    proposed_template: Option<&str>,
) -> Result<String> {
    // 已有未处理的同类建议：更新文案（保留 id，避免重复堆积）
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM skill_suggestions WHERE skill_id = ?1 AND rule_type = ?2 AND status = 'pending' LIMIT 1",
            params![skill_id, rule_type],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        conn.execute(
            "UPDATE skill_suggestions SET severity = ?1, message = ?2, dimension = COALESCE(?3, dimension), created_at = ?4 WHERE id = ?5",
            params![severity, message, dimension, chrono::Utc::now().to_rfc3339(), id],
        )?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO skill_suggestions (id, skill_id, rule_type, severity, message, dimension, proposed_template, status, applied_version, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', NULL, ?8)",
        params![id, skill_id, rule_type, severity, message, dimension, proposed_template, now],
    )?;
    Ok(id)
}

fn map_suggestion(row: &rusqlite::Row) -> rusqlite::Result<SkillSuggestion> {
    Ok(SkillSuggestion {
        id: row.get(0)?,
        skill_id: row.get(1)?,
        rule_type: row.get(2)?,
        severity: row.get(3)?,
        message: row.get(4)?,
        dimension: row.get(5)?,
        proposed_template: row.get(6)?,
        status: row.get(7)?,
        applied_version: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn list_skill_suggestions(conn: &Connection, skill_id: Option<&str>) -> Result<Vec<SkillSuggestion>> {
    let sql = "SELECT id, skill_id, rule_type, severity, message, dimension, proposed_template, status, applied_version, created_at
               FROM skill_suggestions
               WHERE (?1 IS NULL OR skill_id = ?1)
               ORDER BY CASE severity WHEN 'HIGH' THEN 0 WHEN 'MEDIUM' THEN 1 ELSE 2 END, created_at DESC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![skill_id], map_suggestion)?;
    rows.collect()
}

pub fn get_skill_suggestion(conn: &Connection, id: &str) -> Result<Option<SkillSuggestion>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_id, rule_type, severity, message, dimension, proposed_template, status, applied_version, created_at
         FROM skill_suggestions WHERE id = ?"
    )?;
    let mut rows = stmt.query_map(params![id], map_suggestion)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn update_suggestion_status(
    conn: &Connection,
    id: &str,
    status: &str,
    applied_version: Option<i32>,
) -> Result<()> {
    conn.execute(
        "UPDATE skill_suggestions SET status = ?1, applied_version = COALESCE(?2, applied_version) WHERE id = ?3",
        params![status, applied_version, id],
    )?;
    Ok(())
}

/// 按规则类型批量关闭 pending 建议（如回退建议在分数回升后关闭）
pub fn dismiss_pending_suggestions(conn: &Connection, skill_id: &str, rule_type: &str) -> Result<()> {
    conn.execute(
        "UPDATE skill_suggestions SET status = 'verified' WHERE skill_id = ?1 AND rule_type = ?2 AND status = 'pending'",
        params![skill_id, rule_type],
    )?;
    Ok(())
}

/// 写入版本快照（版本号变化时调用）
pub fn insert_skill_version(
    conn: &Connection,
    skill_id: &str,
    version: i32,
    template: &str,
    note: Option<&str>,
) -> Result<String> {
    // 同版本重复快照直接覆盖（幂等）
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM skill_versions WHERE skill_id = ?1 AND version = ?2",
            params![skill_id, version],
            |row| row.get(0),
        )
        .ok();

    let now = chrono::Utc::now().to_rfc3339();
    if let Some(id) = existing {
        conn.execute(
            "UPDATE skill_versions SET template = ?1, note = COALESCE(?2, note) WHERE id = ?3",
            params![template, note, id],
        )?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO skill_versions (id, skill_id, version, template, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, skill_id, version, template, note, now],
    )?;
    Ok(id)
}

fn map_skill_version(row: &rusqlite::Row) -> rusqlite::Result<SkillVersion> {
    Ok(SkillVersion {
        id: row.get(0)?,
        skill_id: row.get(1)?,
        version: row.get(2)?,
        template: row.get(3)?,
        note: row.get(4)?,
        created_at: row.get(5)?,
    })
}

pub fn list_skill_versions(conn: &Connection, skill_id: &str) -> Result<Vec<SkillVersion>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_id, version, template, note, created_at FROM skill_versions WHERE skill_id = ? ORDER BY version ASC"
    )?;
    let rows = stmt.query_map(params![skill_id], map_skill_version)?;
    rows.collect()
}

pub fn get_skill_version(conn: &Connection, skill_id: &str, version: i32) -> Result<Option<SkillVersion>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_id, version, template, note, created_at FROM skill_versions WHERE skill_id = ? AND version = ?"
    )?;
    let mut rows = stmt.query_map(params![skill_id, version], map_skill_version)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// 写入一条 MCP 执行日志。
///
/// 在会话启动时，为项目绑定（或全局默认）的每个 MCP 端点记录一次连接事件，
/// 使 `mcp_execution_logs` 具备真实数据来源。
pub fn insert_mcp_execution_log(
    conn: &Connection,
    session_id: &str,
    mcp_name: &str,
    endpoint: Option<&str>,
    action: Option<&str>,
    status: &str,
    error_message: Option<&str>,
    duration_ms: Option<i64>,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO mcp_execution_logs
            (id, session_id, mcp_name, endpoint, action, status, error_message, duration_ms, executed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            session_id,
            mcp_name,
            endpoint,
            action,
            status,
            error_message,
            duration_ms,
            now
        ],
    )?;

    Ok(id)
}

pub fn get_mcp_usage_stats(conn: &Connection) -> Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT mcp_name, COUNT(*) as call_count,
         SUM(CASE WHEN status IN ('connected', 'call_success') THEN 1 ELSE 0 END) as success_count,
         AVG(duration_ms) as avg_duration
         FROM mcp_execution_logs GROUP BY mcp_name ORDER BY call_count DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "mcp_name": row.get::<_, String>(0)?,
            "call_count": row.get::<_, i64>(1)?,
            "success_count": row.get::<_, i64>(2)?,
            "avg_duration": row.get::<_, Option<f64>>(3)?,
        }))
    })?;
    rows.collect()
}
