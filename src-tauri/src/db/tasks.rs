use rusqlite::{Connection, Result, params};
use super::models::{Task, TaskComment};

pub fn create_task(
    conn: &Connection,
    title: &str,
    requirement_id: Option<&str>,
    task_type: &str,
    priority: &str,
    description: Option<&str>,
    project_id: Option<&str>,
) -> Result<Task> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // 未显式传 project_id 时经需求自动归属项目（任务必须可计入项目统计）
    let effective_project_id = match project_id {
        Some(pid) => Some(pid.to_string()),
        None => requirement_id.and_then(|rid| {
            conn.query_row(
                "SELECT project_id FROM requirements WHERE id = ?1",
                params![rid],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None)
        }),
    };

    conn.execute(
        "INSERT INTO tasks (id, requirement_id, title, description, task_type, status, priority, project_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?)",
        params![id, requirement_id, title, description, task_type, priority, effective_project_id, now, now],
    )?;

    Ok(Task {
        id,
        requirement_id: requirement_id.map(|s| s.to_string()),
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
        task_type: task_type.to_string(),
        status: "pending".to_string(),
        priority: priority.to_string(),
        agent_type: None,
        token_usage: 0,
        assignee: None,
        due_date: None,
        tags: None,
        estimated_tokens: None,
        estimated_hours: None,
        actual_hours: None,
        blocked_reason: None,
        related_requirement_id: None,
        related_project_id: None,
        priority_level: None,
        notes: None,
        attachments: None,
        created_at: now.clone(),
        updated_at: now,
        project_id: effective_project_id,
        skill_config: None,
        mcp_config: None,
    })
}

pub fn list_tasks(conn: &Connection, status_filter: Option<&str>, type_filter: Option<&str>) -> Result<Vec<Task>> {
    let sql_base = "SELECT id, requirement_id, title, description, task_type, status, priority, agent_type, token_usage, assignee, due_date, tags, estimated_tokens, estimated_hours, actual_hours, blocked_reason, related_requirement_id, related_project_id, priority_level, notes, attachments, created_at, updated_at, project_id, skill_config, mcp_config FROM tasks";
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(status) = status_filter {
        conditions.push("status = ?");
        params.push(Box::new(status.to_string()));
    }

    if let Some(task_type) = type_filter {
        conditions.push("task_type = ?");
        params.push(Box::new(task_type.to_string()));
    }

    let sql = if conditions.is_empty() {
        format!("{} ORDER BY created_at DESC", sql_base)
    } else {
        format!("{} WHERE {} ORDER BY created_at DESC", sql_base, conditions.join(" AND "))
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(Task {
            id: row.get(0)?,
            requirement_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            task_type: row.get(4)?,
            status: row.get(5)?,
            priority: row.get(6)?,
            agent_type: row.get(7)?,
            token_usage: row.get(8)?,
            assignee: row.get(9)?,
            due_date: row.get(10)?,
            tags: row.get(11)?,
            estimated_tokens: row.get(12)?,
            estimated_hours: row.get(13)?,
            actual_hours: row.get(14)?,
            blocked_reason: row.get(15)?,
            related_requirement_id: row.get(16)?,
            related_project_id: row.get(17)?,
            priority_level: row.get(18)?,
            notes: row.get(19)?,
            attachments: row.get(20)?,
            created_at: row.get(21)?,
            updated_at: row.get(22)?,
            project_id: row.get(23)?,
            skill_config: row.get(24)?,
            mcp_config: row.get(25)?,
        })
    })?;

    rows.collect()
}

/// 各项目任务计数 `(project_id, total, completed)`（工作台/项目列表聚合）。
///
/// 同时认 `tasks.project_id` 直属与经 requirement 归属（历史数据可能只有后者）。
pub fn count_tasks_by_project(conn: &Connection) -> Result<Vec<(String, i64, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(t.project_id, r.project_id) AS pid,
                COUNT(t.id),
                COALESCE(SUM(CASE WHEN t.status = 'completed' THEN 1 ELSE 0 END), 0)
         FROM tasks t
         LEFT JOIN requirements r ON t.requirement_id = r.id
         WHERE COALESCE(t.project_id, r.project_id) IS NOT NULL
         GROUP BY pid",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
    })?;

    rows.collect()
}

pub fn update_task_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?",
        params![status, now, id],
    )?;

    Ok(())
}

pub fn delete_task(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?", params![id])?;
    Ok(())
}

pub fn get_task(conn: &Connection, id: &str) -> Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, requirement_id, title, description, task_type, status, priority, agent_type, token_usage, assignee, due_date, tags, estimated_tokens, estimated_hours, actual_hours, blocked_reason, related_requirement_id, related_project_id, priority_level, notes, attachments, created_at, updated_at, project_id, skill_config, mcp_config FROM tasks WHERE id = ?"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Task {
            id: row.get(0)?,
            requirement_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            task_type: row.get(4)?,
            status: row.get(5)?,
            priority: row.get(6)?,
            agent_type: row.get(7)?,
            token_usage: row.get(8)?,
            assignee: row.get(9)?,
            due_date: row.get(10)?,
            tags: row.get(11)?,
            estimated_tokens: row.get(12)?,
            estimated_hours: row.get(13)?,
            actual_hours: row.get(14)?,
            blocked_reason: row.get(15)?,
            related_requirement_id: row.get(16)?,
            related_project_id: row.get(17)?,
            priority_level: row.get(18)?,
            notes: row.get(19)?,
            attachments: row.get(20)?,
            created_at: row.get(21)?,
            updated_at: row.get(22)?,
            project_id: row.get(23)?,
            skill_config: row.get(24)?,
            mcp_config: row.get(25)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn update_task(conn: &Connection, id: &str, title: Option<&str>, description: Option<&str>, task_type: Option<&str>, priority: Option<&str>, assignee: Option<&str>, due_date: Option<&str>, tags: Option<&str>, project_id: Option<&str>) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(t) = title { updates.push("title = ?"); params.push(Box::new(t.to_string())); }
    if let Some(d) = description { updates.push("description = ?"); params.push(Box::new(d.to_string())); }
    if let Some(tt) = task_type { updates.push("task_type = ?"); params.push(Box::new(tt.to_string())); }
    if let Some(p) = priority { updates.push("priority = ?"); params.push(Box::new(p.to_string())); }
    if let Some(a) = assignee { updates.push("assignee = ?"); params.push(Box::new(a.to_string())); }
    if let Some(dd) = due_date { updates.push("due_date = ?"); params.push(Box::new(dd.to_string())); }
    if let Some(t) = tags { updates.push("tags = ?"); params.push(Box::new(t.to_string())); }
    if let Some(pid) = project_id { updates.push("project_id = ?"); params.push(Box::new(pid.to_string())); }

    if updates.is_empty() { return Ok(()); }

    updates.push("updated_at = ?");
    params.push(Box::new(now));
    params.push(Box::new(id.to_string()));

    let sql = format!("UPDATE tasks SET {} WHERE id = ?", updates.join(", "));
    conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;

    Ok(())
}

pub fn list_tasks_by_project(conn: &Connection, project_id: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, requirement_id, title, description, task_type, status, priority, agent_type, token_usage, assignee, due_date, tags, estimated_tokens, estimated_hours, actual_hours, blocked_reason, related_requirement_id, related_project_id, priority_level, notes, attachments, created_at, updated_at, project_id, skill_config, mcp_config FROM tasks WHERE project_id = ? ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(Task {
            id: row.get(0)?,
            requirement_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            task_type: row.get(4)?,
            status: row.get(5)?,
            priority: row.get(6)?,
            agent_type: row.get(7)?,
            token_usage: row.get(8)?,
            assignee: row.get(9)?,
            due_date: row.get(10)?,
            tags: row.get(11)?,
            estimated_tokens: row.get(12)?,
            estimated_hours: row.get(13)?,
            actual_hours: row.get(14)?,
            blocked_reason: row.get(15)?,
            related_requirement_id: row.get(16)?,
            related_project_id: row.get(17)?,
            priority_level: row.get(18)?,
            notes: row.get(19)?,
            attachments: row.get(20)?,
            created_at: row.get(21)?,
            updated_at: row.get(22)?,
            project_id: row.get(23)?,
            skill_config: row.get(24)?,
            mcp_config: row.get(25)?,
        })
    })?;
    rows.collect()
}

pub fn list_tasks_by_requirement(conn: &Connection, requirement_id: &str) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, requirement_id, title, description, task_type, status, priority, agent_type, token_usage, assignee, due_date, tags, estimated_tokens, estimated_hours, actual_hours, blocked_reason, related_requirement_id, related_project_id, priority_level, notes, attachments, created_at, updated_at, project_id, skill_config, mcp_config FROM tasks WHERE requirement_id = ? ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map(params![requirement_id], |row| {
        Ok(Task {
            id: row.get(0)?,
            requirement_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            task_type: row.get(4)?,
            status: row.get(5)?,
            priority: row.get(6)?,
            agent_type: row.get(7)?,
            token_usage: row.get(8)?,
            assignee: row.get(9)?,
            due_date: row.get(10)?,
            tags: row.get(11)?,
            estimated_tokens: row.get(12)?,
            estimated_hours: row.get(13)?,
            actual_hours: row.get(14)?,
            blocked_reason: row.get(15)?,
            related_requirement_id: row.get(16)?,
            related_project_id: row.get(17)?,
            priority_level: row.get(18)?,
            notes: row.get(19)?,
            attachments: row.get(20)?,
            created_at: row.get(21)?,
            updated_at: row.get(22)?,
            project_id: row.get(23)?,
            skill_config: row.get(24)?,
            mcp_config: row.get(25)?,
        })
    })?;
    rows.collect()
}

// ==================== 任务评论操作 ====================

pub fn create_task_comment(conn: &Connection, task_id: &str, author: &str, content: &str) -> Result<TaskComment> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO task_comments (id, task_id, author, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        params![id, task_id, author, content, now, now],
    )?;

    Ok(TaskComment {
        id,
        task_id: task_id.to_string(),
        author: author.to_string(),
        content: content.to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn list_task_comments(conn: &Connection, task_id: &str) -> Result<Vec<TaskComment>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, author, content, created_at, updated_at FROM task_comments WHERE task_id = ? ORDER BY created_at ASC"
    )?;
    let rows = stmt.query_map(params![task_id], |row| {
        Ok(TaskComment {
            id: row.get(0)?,
            task_id: row.get(1)?,
            author: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn delete_task_comment(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM task_comments WHERE id = ?", params![id])?;
    Ok(())
}

pub fn update_task_fields(
    conn: &Connection,
    id: &str,
    assignee: Option<&str>,
    due_date: Option<&str>,
    estimated_hours: Option<f64>,
    actual_hours: Option<f64>,
    priority_level: Option<&str>,
    blocked_reason: Option<&str>,
    related_requirement_id: Option<&str>,
    related_project_id: Option<&str>,
    tags: Option<&str>,
    notes: Option<&str>,
    attachments: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE tasks
         SET assignee = COALESCE(?, assignee),
             due_date = COALESCE(?, due_date),
             estimated_hours = COALESCE(?, estimated_hours),
             actual_hours = COALESCE(?, actual_hours),
             priority_level = COALESCE(?, priority_level),
             blocked_reason = COALESCE(?, blocked_reason),
             related_requirement_id = COALESCE(?, related_requirement_id),
             related_project_id = COALESCE(?, related_project_id),
             tags = COALESCE(?, tags),
             notes = COALESCE(?, notes),
             attachments = COALESCE(?, attachments),
             updated_at = ?
         WHERE id = ?",
        params![assignee, due_date, estimated_hours, actual_hours, priority_level, blocked_reason, related_requirement_id, related_project_id, tags, notes, attachments, now, id],
    )?;

    Ok(())
}
