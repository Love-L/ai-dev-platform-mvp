use rusqlite::{Connection, Result, params};
use super::models::Requirement;

pub fn create_requirement(conn: &Connection, project_id: &str, title: &str, description: Option<&str>, priority: &str) -> Result<Requirement> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO requirements (id, project_id, title, description, priority, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'draft', ?, ?)",
        params![id, project_id, title, description, priority, now, now],
    )?;

    Ok(Requirement {
        id,
        project_id: project_id.to_string(),
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
        priority: priority.to_string(),
        status: "draft".to_string(),
        prd_content: None,
        requirement_type: Some("feature".to_string()),
        target_version: None,
        due_date: None,
        tags: None,
        module: None,
        source: None,
        source_detail: None,
        product_owner: None,
        developer: None,
        tester: None,
        business_priority: None,
        estimated_effort: None,
        actual_effort: None,
        attachments: None,
        notes: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn list_requirements(conn: &Connection, status_filter: Option<&str>) -> Result<Vec<Requirement>> {
    let sql_base = "SELECT id, project_id, title, description, priority, status, prd_content, requirement_type, target_version, due_date, tags, module, source, source_detail, product_owner, developer, tester, business_priority, estimated_effort, actual_effort, attachments, notes, created_at, updated_at FROM requirements";
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    let sql = if let Some(status) = status_filter {
        params.push(Box::new(status.to_string()));
        format!("{} WHERE status = ? ORDER BY created_at DESC", sql_base)
    } else {
        format!("{} ORDER BY created_at DESC", sql_base)
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(Requirement {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            priority: row.get(4)?,
            status: row.get(5)?,
            prd_content: row.get(6)?,
            requirement_type: row.get(7)?,
            target_version: row.get(8)?,
            due_date: row.get(9)?,
            tags: row.get(10)?,
            module: row.get(11)?,
            source: row.get(12)?,
            source_detail: row.get(13)?,
            product_owner: row.get(14)?,
            developer: row.get(15)?,
            tester: row.get(16)?,
            business_priority: row.get(17)?,
            estimated_effort: row.get(18)?,
            actual_effort: row.get(19)?,
            attachments: row.get(20)?,
            notes: row.get(21)?,
            created_at: row.get(22)?,
            updated_at: row.get(23)?,
        })
    })?;

    rows.collect()
}

pub fn list_requirements_paged(conn: &Connection, page: u32, limit: u32, status_filter: Option<&str>) -> Result<crate::pagination::PagedResult<Requirement>> {
    let sql_base = "SELECT id, project_id, title, description, priority, status, prd_content, requirement_type, target_version, due_date, tags, module, source, source_detail, product_owner, developer, tester, business_priority, estimated_effort, actual_effort, attachments, notes, created_at, updated_at FROM requirements";
    let count_base = "SELECT COUNT(*) FROM requirements";
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    let (where_clause, count_where) = if let Some(status) = status_filter {
        params.push(Box::new(status.to_string()));
        (" WHERE status = ?", " WHERE status = ?")
    } else {
        ("", "")
    };

    // 获取总数
    let total: u64 = if !count_where.is_empty() {
        let count_sql = format!("{}{}", count_base, count_where);
        conn.query_row(&count_sql, rusqlite::params_from_iter(params.iter()), |row| row.get(0))?
    } else {
        conn.query_row(count_base, [], |row| row.get(0))?
    };

    // 获取分页数据
    let offset = (page - 1) * limit;
    let sql = format!("{}{} ORDER BY created_at DESC LIMIT {} OFFSET {}", sql_base, where_clause, limit, offset);

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(Requirement {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            priority: row.get(4)?,
            status: row.get(5)?,
            prd_content: row.get(6)?,
            requirement_type: row.get(7)?,
            target_version: row.get(8)?,
            due_date: row.get(9)?,
            tags: row.get(10)?,
            module: row.get(11)?,
            source: row.get(12)?,
            source_detail: row.get(13)?,
            product_owner: row.get(14)?,
            developer: row.get(15)?,
            tester: row.get(16)?,
            business_priority: row.get(17)?,
            estimated_effort: row.get(18)?,
            actual_effort: row.get(19)?,
            attachments: row.get(20)?,
            notes: row.get(21)?,
            created_at: row.get(22)?,
            updated_at: row.get(23)?,
        })
    })?;

    let items: Vec<Requirement> = rows.collect::<Result<Vec<_>, _>>()?;

    Ok(crate::pagination::PagedResult::new(items, total, page, limit))
}

pub fn get_requirement(conn: &Connection, id: &str) -> Result<Option<Requirement>> {
    let mut stmt = conn.prepare("SELECT id, project_id, title, description, priority, status, prd_content, requirement_type, target_version, due_date, tags, module, source, source_detail, product_owner, developer, tester, business_priority, estimated_effort, actual_effort, attachments, notes, created_at, updated_at FROM requirements WHERE id = ?")?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Requirement {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            priority: row.get(4)?,
            status: row.get(5)?,
            prd_content: row.get(6)?,
            requirement_type: row.get(7)?,
            target_version: row.get(8)?,
            due_date: row.get(9)?,
            tags: row.get(10)?,
            module: row.get(11)?,
            source: row.get(12)?,
            source_detail: row.get(13)?,
            product_owner: row.get(14)?,
            developer: row.get(15)?,
            tester: row.get(16)?,
            business_priority: row.get(17)?,
            estimated_effort: row.get(18)?,
            actual_effort: row.get(19)?,
            attachments: row.get(20)?,
            notes: row.get(21)?,
            created_at: row.get(22)?,
            updated_at: row.get(23)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn update_requirement(conn: &Connection, id: &str, title: Option<&str>, description: Option<&str>, priority: Option<&str>, status: Option<&str>, prd_content: Option<&str>) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE requirements
         SET title = COALESCE(?, title),
             description = COALESCE(?, description),
             priority = COALESCE(?, priority),
             status = COALESCE(?, status),
             prd_content = COALESCE(?, prd_content),
             updated_at = ?
         WHERE id = ?",
        params![title, description, priority, status, prd_content, now, id],
    )?;

    Ok(())
}

pub fn delete_requirement(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM requirements WHERE id = ?", params![id])?;
    Ok(())
}

/// 记录需求变更历史（审计）。
///
/// `action` 取值：`create` / `update` / `status_change` / `delete`。
/// 注意：`requirement_history.requirement_id` 为外键，删除需求时必须**先写历史再删除**，
/// 因此删除场景需把需求标题快照到 `old_value`。
pub fn insert_requirement_history(
    conn: &Connection,
    requirement_id: &str,
    action: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO requirement_history (id, requirement_id, action, old_value, new_value, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![id, requirement_id, action, old_value, new_value, now],
    )?;

    Ok(())
}

/// 需求历史行：`(action, old_value, new_value, created_at, id)`
pub type RequirementHistoryRow = (String, String, Option<String>, Option<String>, String);

/// 查询指定需求的历史记录（按时间倒序）
pub fn list_requirement_history(
    conn: &Connection,
    requirement_id: &str,
    limit: i64,
) -> Result<Vec<RequirementHistoryRow>> {
    let mut stmt = conn.prepare(
        "SELECT action, old_value, new_value, created_at, id
         FROM requirement_history
         WHERE requirement_id = ?
         ORDER BY created_at DESC
         LIMIT ?",
    )?;

    let rows = stmt.query_map(params![requirement_id, limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut result = Vec::new();
    for row in rows {
        let (action, old_value, new_value, created_at, id) = row?;
        result.push((action, id, old_value, new_value, created_at));
    }
    Ok(result)
}

pub fn update_requirement_fields(
    conn: &Connection,
    id: &str,
    module: Option<&str>,
    source: Option<&str>,
    source_detail: Option<&str>,
    product_owner: Option<&str>,
    developer: Option<&str>,
    tester: Option<&str>,
    business_priority: Option<&str>,
    estimated_effort: Option<f64>,
    actual_effort: Option<f64>,
    attachments: Option<&str>,
    tags: Option<&str>,
    notes: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE requirements
         SET module = COALESCE(?, module),
             source = COALESCE(?, source),
             source_detail = COALESCE(?, source_detail),
             product_owner = COALESCE(?, product_owner),
             developer = COALESCE(?, developer),
             tester = COALESCE(?, tester),
             business_priority = COALESCE(?, business_priority),
             estimated_effort = COALESCE(?, estimated_effort),
             actual_effort = COALESCE(?, actual_effort),
             attachments = COALESCE(?, attachments),
             tags = COALESCE(?, tags),
             notes = COALESCE(?, notes),
             updated_at = ?
         WHERE id = ?",
        params![module, source, source_detail, product_owner, developer, tester, business_priority, estimated_effort, actual_effort, attachments, tags, notes, now, id],
    )?;

    Ok(())
}
