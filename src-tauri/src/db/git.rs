use rusqlite::{Connection, Result, params};

/// 写入一条 Git 提交记录，实现"提交 ↔ 任务/会话"的可追溯关联。
///
/// 返回新记录的主键 id。
pub fn insert_git_commit(
    conn: &Connection,
    task_id: Option<&str>,
    session_id: Option<&str>,
    commit_hash: &str,
    branch_name: Option<&str>,
    message: Option<&str>,
    files_changed: i64,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO git_commits
            (id, task_id, session_id, commit_hash, branch_name, message, files_changed, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            task_id,
            session_id,
            commit_hash,
            branch_name,
            message,
            files_changed,
            now
        ],
    )?;

    Ok(id)
}

/// 查询提交记录：可按任务或会话过滤，按创建时间倒序。
pub fn list_git_commits(
    conn: &Connection,
    task_id: Option<&str>,
    session_id: Option<&str>,
    limit: i64,
) -> Result<Vec<serde_json::Value>> {
    let sql = match (task_id.is_some(), session_id.is_some()) {
        (true, true) => {
            "SELECT id, task_id, session_id, commit_hash, branch_name, message, files_changed, created_at
             FROM git_commits WHERE task_id = ?1 AND session_id = ?2
             ORDER BY created_at DESC LIMIT ?3"
        }
        (true, false) => {
            "SELECT id, task_id, session_id, commit_hash, branch_name, message, files_changed, created_at
             FROM git_commits WHERE task_id = ?1
             ORDER BY created_at DESC LIMIT ?3"
        }
        (false, true) => {
            "SELECT id, task_id, session_id, commit_hash, branch_name, message, files_changed, created_at
             FROM git_commits WHERE session_id = ?2
             ORDER BY created_at DESC LIMIT ?3"
        }
        (false, false) => {
            "SELECT id, task_id, session_id, commit_hash, branch_name, message, files_changed, created_at
             FROM git_commits
             ORDER BY created_at DESC LIMIT ?3"
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(
        params![task_id, session_id, limit],
        |row| {
            let hash: String = row.get(3)?;
            let short_hash: String = hash.chars().take(7).collect();
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "task_id": row.get::<_, Option<String>>(1)?,
                "session_id": row.get::<_, Option<String>>(2)?,
                "commit_hash": hash,
                "short_hash": short_hash,
                "branch_name": row.get::<_, Option<String>>(4)?,
                "message": row.get::<_, Option<String>>(5)?,
                "files_changed": row.get::<_, i64>(6)?,
                "created_at": row.get::<_, String>(7)?,
            }))
        },
    )?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// 统计某任务关联的提交数量
#[allow(dead_code)]
pub fn count_commits_by_task(conn: &Connection, task_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM git_commits WHERE task_id = ?",
        params![task_id],
        |row| row.get(0),
    )
}
