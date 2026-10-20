//! 会话消息块读写（模块 16 / F076）。
//!
//! 设计要点：
//! - `seq` 为会话内单调递增序号，是回放顺序的**唯一依据**；
//! - 增量（`session:delta`）**不落库**，只有完整块落库，避免高频写放大；
//! - 进程异常退出时，残留的 `streaming` 块由 `mark_streaming_as_failed` 收尾。

use rusqlite::{Connection, Result, params};
use uuid::Uuid;

use super::models::SessionMessage;

const COLUMNS: &str = "id, session_id, seq, role, block_type, content, tool_name, tool_use_id, \
     parent_tool_use_id, cli_message_id, raw_json, tokens, status, \
     related_task_id, related_requirement_id, project_id, created_at";

/// 消息块写入上下文（会话级关联信息，逐块复用）
#[derive(Debug, Clone, Default)]
pub struct MessageContext {
    pub task_id: Option<String>,
    pub requirement_id: Option<String>,
    pub project_id: Option<String>,
    /// 是否保留原始事件 JSON（体积权衡，默认关闭）
    pub keep_raw_json: bool,
}

/// 消息块写入规格
#[derive(Debug, Clone)]
pub struct BlockSpec {
    pub role: String,
    pub block_type: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub parent_tool_use_id: Option<String>,
    pub cli_message_id: Option<String>,
    pub raw_json: Option<String>,
    pub tokens: Option<i64>,
    pub status: String,
}

impl BlockSpec {
    pub fn new(role: &str, block_type: &str, content: Option<String>) -> Self {
        Self {
            role: role.to_string(),
            block_type: block_type.to_string(),
            content,
            tool_name: None,
            tool_use_id: None,
            parent_tool_use_id: None,
            cli_message_id: None,
            raw_json: None,
            tokens: None,
            status: "completed".to_string(),
        }
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> Result<SessionMessage> {
    Ok(SessionMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        seq: row.get(2)?,
        role: row.get(3)?,
        block_type: row.get(4)?,
        content: row.get(5)?,
        tool_name: row.get(6)?,
        tool_use_id: row.get(7)?,
        parent_tool_use_id: row.get(8)?,
        cli_message_id: row.get(9)?,
        raw_json: row.get(10)?,
        tokens: row.get(11)?,
        status: row.get(12)?,
        related_task_id: row.get(13)?,
        related_requirement_id: row.get(14)?,
        project_id: row.get(15)?,
        created_at: row.get(16)?,
    })
}

/// 取会话内下一个序号
pub fn next_seq(conn: &Connection, session_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM session_messages WHERE session_id = ?1",
        params![session_id],
        |row| row.get(0),
    )
}

/// 插入一条完整消息块（自动分配 `seq`），返回落库后的记录
pub fn append_block(
    conn: &Connection,
    session_id: &str,
    ctx: &MessageContext,
    spec: BlockSpec,
) -> Result<SessionMessage> {
    let msg = build_message(conn, session_id, ctx, spec)?;

    conn.execute(
        "INSERT INTO session_messages (id, session_id, seq, role, block_type, content, tool_name, \
         tool_use_id, parent_tool_use_id, cli_message_id, raw_json, tokens, status, \
         related_task_id, related_requirement_id, project_id, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            msg.id, msg.session_id, msg.seq, msg.role, msg.block_type, msg.content,
            msg.tool_name, msg.tool_use_id, msg.parent_tool_use_id, msg.cli_message_id,
            msg.raw_json, msg.tokens, msg.status, msg.related_task_id,
            msg.related_requirement_id, msg.project_id, msg.created_at
        ],
    )?;

    Ok(msg)
}

fn build_message(
    conn: &Connection,
    session_id: &str,
    ctx: &MessageContext,
    spec: BlockSpec,
) -> Result<SessionMessage> {
    Ok(SessionMessage {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        seq: next_seq(conn, session_id)?,
        role: spec.role,
        block_type: spec.block_type,
        content: spec.content,
        tool_name: spec.tool_name,
        tool_use_id: spec.tool_use_id,
        parent_tool_use_id: spec.parent_tool_use_id,
        cli_message_id: spec.cli_message_id,
        raw_json: if ctx.keep_raw_json { spec.raw_json } else { None },
        tokens: spec.tokens,
        status: if spec.status.is_empty() {
            "completed".to_string()
        } else {
            spec.status
        },
        related_task_id: ctx.task_id.clone(),
        related_requirement_id: ctx.requirement_id.clone(),
        project_id: ctx.project_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// 分页读取消息块。
///
/// `before_seq` 为空时取**最新** `limit` 条；返回结果统一按 `seq` **升序**排列，
/// 便于前端直接追加渲染与回放。
pub fn list(
    conn: &Connection,
    session_id: &str,
    before_seq: Option<i64>,
    limit: u32,
) -> Result<Vec<SessionMessage>> {
    let limit = limit.clamp(1, 500) as i64;

    let mut rows: Vec<SessionMessage> = match before_seq {
        Some(seq) => {
            let sql = format!(
                "SELECT {COLUMNS} FROM session_messages WHERE session_id = ?1 AND seq < ?2 \
                 ORDER BY seq DESC LIMIT ?3"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mapped = stmt.query_map(params![session_id, seq, limit], map_row)?;
            mapped.collect::<Result<Vec<_>>>()?
        }
        None => {
            let sql = format!(
                "SELECT {COLUMNS} FROM session_messages WHERE session_id = ?1 \
                 ORDER BY seq DESC LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mapped = stmt.query_map(params![session_id, limit], map_row)?;
            mapped.collect::<Result<Vec<_>>>()?
        }
    };

    rows.reverse();
    Ok(rows)
}

/// 会话消息总数（用于分页元信息）
pub fn count(conn: &Connection, session_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM session_messages WHERE session_id = ?1",
        params![session_id],
        |row| row.get(0),
    )
}

/// 把残留的 `streaming` 块收尾（进程异常退出 / 应用重启）
pub fn mark_streaming_as_failed(conn: &Connection, session_id: &str) -> Result<usize> {
    conn.execute(
        "UPDATE session_messages SET status = 'failed' \
         WHERE session_id = ?1 AND status = 'streaming'",
        params![session_id],
    )
}

/// 应用启动时把**所有**残留的 `streaming` 块收尾（重启后不可能仍有活动流）
pub fn mark_all_streaming_as_failed(conn: &Connection) -> Result<usize> {
    conn.execute(
        "UPDATE session_messages SET status = 'failed' WHERE status = 'streaming'",
        [],
    )
}

/// 按块类型统计（用于会话概要：思考块数、工具调用数等）
pub fn count_by_block_type(conn: &Connection, session_id: &str) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT block_type, COUNT(*) FROM session_messages WHERE session_id = ?1 \
         GROUP BY block_type ORDER BY block_type",
    )?;
    let mapped = stmt.query_map(params![session_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    mapped.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> MessageContext {
        MessageContext {
            task_id: Some("task-1".to_string()),
            requirement_id: Some("req-1".to_string()),
            project_id: Some("proj-1".to_string()),
            keep_raw_json: false,
        }
    }

    #[test]
    fn seq_is_monotonic_and_list_is_ascending() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE session_messages (
                id TEXT PRIMARY KEY, session_id TEXT NOT NULL, seq INTEGER NOT NULL,
                role TEXT NOT NULL, block_type TEXT NOT NULL, content TEXT,
                tool_name TEXT, tool_use_id TEXT, parent_tool_use_id TEXT,
                cli_message_id TEXT, raw_json TEXT, tokens INTEGER,
                status TEXT DEFAULT 'completed', related_task_id TEXT,
                related_requirement_id TEXT, project_id TEXT, created_at DATETIME
            );",
        )
        .unwrap();

        for i in 0..3 {
            append_block(
                &conn,
                "s1",
                &ctx(),
                BlockSpec::new("assistant", "text", Some(format!("块{i}"))),
            )
            .unwrap();
        }

        let items = list(&conn, "s1", None, 10).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![1, 2, 3]);
        // 关联上下文与原始事件裁剪策略生效
        assert_eq!(items[0].related_task_id.as_deref(), Some("task-1"));
        assert!(items[0].raw_json.is_none());
    }

    #[test]
    fn mark_streaming_as_failed_closes_dangling_blocks() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE session_messages (
                id TEXT PRIMARY KEY, session_id TEXT NOT NULL, seq INTEGER NOT NULL,
                role TEXT NOT NULL, block_type TEXT NOT NULL, content TEXT,
                tool_name TEXT, tool_use_id TEXT, parent_tool_use_id TEXT,
                cli_message_id TEXT, raw_json TEXT, tokens INTEGER,
                status TEXT DEFAULT 'completed', related_task_id TEXT,
                related_requirement_id TEXT, project_id TEXT, created_at DATETIME
            );",
        )
        .unwrap();

        let mut spec = BlockSpec::new("assistant", "text", Some("流式中".to_string()));
        spec.status = "streaming".to_string();
        append_block(&conn, "s1", &ctx(), spec).unwrap();

        assert_eq!(mark_streaming_as_failed(&conn, "s1").unwrap(), 1);
        let items = list(&conn, "s1", None, 10).unwrap();
        assert_eq!(items[0].status, "failed");
    }
}
