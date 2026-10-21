use rusqlite::{Connection, Result, params};
use super::models::{Agent, AgentSession};

/// 以 `binary_path` 为逻辑唯一键写入 Agent，并返回**实际生效的记录 id**。
///
/// 返回值是必须的：命中已有记录时执行的是 `UPDATE`，传入的 `agent.id` **不会入库**。
/// 调用方若继续拿传入的 id 去写 `agent_sessions.agent_id`（外键指向 `agents(id)`），
/// 会因目标行不存在而报 `FOREIGN KEY constraint failed`。
pub fn upsert_agent(conn: &Connection, agent: &Agent) -> Result<String> {
    // 使用 binary_path 作为唯一键，如果已存在则更新
    let existing_id: Option<String> = conn.query_row(
        "SELECT id FROM agents WHERE binary_path = ?1",
        params![agent.binary_path],
        |row| row.get(0),
    ).ok();

    match existing_id {
        Some(existing_id) => {
            // 更新现有记录：id 保持不变，返回它
            conn.execute(
                "UPDATE agents SET name = ?, display_name = ?, version = ?, status = ?, capabilities = ?
                 WHERE id = ?",
                params![
                    agent.name, agent.display_name, agent.version,
                    agent.status, agent.capabilities, existing_id
                ],
            )?;
            Ok(existing_id)
        }
        None => {
            // 插入新记录：新 id 生效
            conn.execute(
                "INSERT INTO agents (id, name, display_name, binary_path, version, status, capabilities, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    agent.id, agent.name, agent.display_name, agent.binary_path,
                    agent.version, agent.status, agent.capabilities, agent.created_at
                ],
            )?;
            Ok(agent.id.clone())
        }
    }
}

pub fn get_agents(conn: &Connection) -> Result<Vec<Agent>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, display_name, binary_path, version, status, capabilities, created_at FROM agents ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Agent {
            id: row.get(0)?,
            name: row.get(1)?,
            display_name: row.get(2)?,
            binary_path: row.get(3)?,
            version: row.get(4)?,
            status: row.get(5)?,
            capabilities: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

/// 更新指定 Agent 的状态（用于二进制有效性复核后的状态纠正）
pub fn set_agent_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE agents SET status = ?1 WHERE id = ?2",
        params![status, id],
    )?;
    Ok(())
}

/// 删除 Agent 记录（P1-A2：清理失效/误检记录）。
///
/// 有运行中会话时拒绝删除，避免孤儿进程引用。
pub fn delete_agent(conn: &Connection, id: &str) -> Result<()> {
    let running: i64 = conn.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE agent_id = ?1 AND status = 'running'",
        params![id],
        |r| r.get(0),
    )?;
    if running > 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows); // 上层翻译为友好错误
    }

    // 保留历史会话，只解除外键引用后删除 Agent
    conn.execute(
        "UPDATE agent_sessions SET agent_id = NULL WHERE agent_id = ?1",
        params![id],
    )?;
    conn.execute("DELETE FROM agents WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn create_agent_session(conn: &Connection, agent_id: &str, task_id: Option<&str>) -> Result<AgentSession> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO agent_sessions (id, agent_id, task_id, status, started_at)
         VALUES (?, ?, ?, 'running', ?)",
        params![id, agent_id, task_id, now],
    )?;

    Ok(AgentSession {
        id,
        agent_id: agent_id.to_string(),
        task_id: task_id.map(|s| s.to_string()),
        status: "running".to_string(),
        started_at: now,
        ended_at: None,
        exit_code: None,
        duration_seconds: None,
        total_tokens: None,
        total_cost: None,
        project_id: None,
        source: Some("native".to_string()),
        source_path: None,
        title: None,
        description: None,
        related_task_id: None,
        related_requirement_id: None,
        skills: None,
        mcps: None,
        skill_execution_summary: None,
        mcp_execution_summary: None,
        cli_session_id: None,
        work_dir: None,
        agent_path: None,
        mode: None,
        view: None,
        permission_mode: None,
        model: None,
        permission_allowlist: None,
        last_event_at: None,
        archived: 0,
        agent_name: None,
        task_title: None,
    })
}

/// 创建会话所需参数（模块 16：结构化会话）
#[derive(Debug, Clone, Default)]
pub struct NewAgentSession {
    pub agent_id: String,
    pub task_id: Option<String>,
    pub project_id: Option<String>,
    pub related_task_id: Option<String>,
    pub related_requirement_id: Option<String>,
    pub title: Option<String>,
    pub work_dir: Option<String>,
    pub agent_path: Option<String>,
    /// `structured` / `terminal`
    pub mode: String,
    pub permission_mode: Option<String>,
    pub model: Option<String>,
    pub cli_session_id: Option<String>,
}

/// 创建会话（支持结构化/终端模式与关联上下文），返回落库后的记录
pub fn create_agent_session_full(conn: &Connection, spec: &NewAgentSession) -> Result<AgentSession> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let mode = if spec.mode.is_empty() {
        "structured".to_string()
    } else {
        spec.mode.clone()
    };

    conn.execute(
        "INSERT INTO agent_sessions
            (id, agent_id, task_id, status, started_at, project_id, related_task_id,
             related_requirement_id, title, work_dir, agent_path, mode, view,
             permission_mode, model, cli_session_id)
         VALUES (?, ?, ?, 'running', ?, ?, ?, ?, ?, ?, ?, ?, 'messages', ?, ?, ?)",
        params![
            id,
            spec.agent_id,
            spec.task_id,
            now,
            spec.project_id,
            spec.related_task_id,
            spec.related_requirement_id,
            spec.title,
            spec.work_dir,
            spec.agent_path,
            mode,
            spec.permission_mode,
            spec.model,
            spec.cli_session_id
        ],
    )?;

    Ok(AgentSession {
        id,
        agent_id: spec.agent_id.clone(),
        task_id: spec.task_id.clone(),
        status: "running".to_string(),
        started_at: now.clone(),
        ended_at: None,
        exit_code: None,
        duration_seconds: None,
        total_tokens: None,
        total_cost: None,
        project_id: spec.project_id.clone(),
        source: Some("native".to_string()),
        source_path: None,
        title: spec.title.clone(),
        description: None,
        related_task_id: spec.related_task_id.clone(),
        related_requirement_id: spec.related_requirement_id.clone(),
        skills: None,
        mcps: None,
        skill_execution_summary: None,
        mcp_execution_summary: None,
        cli_session_id: spec.cli_session_id.clone(),
        work_dir: spec.work_dir.clone(),
        agent_path: spec.agent_path.clone(),
        mode: Some(mode),
        view: Some("messages".to_string()),
        permission_mode: spec.permission_mode.clone(),
        model: spec.model.clone(),
        permission_allowlist: None,
        last_event_at: Some(now),
        archived: 0,
        agent_name: None,
        task_title: None,
    })
}

/// 读取单个会话
pub fn get_agent_session(conn: &Connection, id: &str) -> Result<Option<AgentSession>> {
    let sql = format!(
        "SELECT {SESSION_COLUMNS} {SESSION_JOIN_COLUMNS} FROM agent_sessions {SESSION_JOINS} \
         WHERE agent_sessions.id = ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![id], map_session_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// 回填 CLI 会话 id（用于 `--resume` 续接）
pub fn set_session_cli_id(conn: &Connection, id: &str, cli_session_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions SET cli_session_id = ?1 WHERE id = ?2",
        params![cli_session_id, id],
    )?;
    Ok(())
}

/// 关联/取消关联会话与开发任务。
///
/// 同步维护 `task_id`（JOIN tasks 回填 task_title 用）与 `related_task_id`；
/// `related_requirement_id` 随任务的需求联动更新（解除关联时一并清空）。
pub fn link_session_task(
    conn: &Connection,
    session_id: &str,
    task_id: Option<&str>,
    requirement_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions SET
            task_id = ?1,
            related_task_id = ?1,
            related_requirement_id = ?2
         WHERE id = ?3",
        params![task_id, requirement_id, session_id],
    )?;
    Ok(())
}

/// 写入会话已加载的 Skill 列表（JSON 数组字符串）
pub fn set_session_skills(conn: &Connection, session_id: &str, skills_json: &str) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions SET skills = ?1 WHERE id = ?2",
        params![skills_json, session_id],
    )?;
    Ok(())
}

/// 更新会话的运行期元信息（模式 / 权限预设 / 模型 / 视图）
pub fn update_session_meta(
    conn: &Connection,
    id: &str,
    mode: Option<&str>,
    permission_mode: Option<&str>,
    model: Option<&str>,
    view: Option<&str>,
    work_dir: Option<&str>,
    cli_session_id: Option<&str>,
    allowed_tools: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions SET
            mode = COALESCE(?1, mode),
            permission_mode = COALESCE(?2, permission_mode),
            model = COALESCE(?3, model),
            view = COALESCE(?4, view),
            work_dir = COALESCE(?5, work_dir),
            cli_session_id = COALESCE(?6, cli_session_id),
            permission_allowlist = COALESCE(?7, permission_allowlist)
         WHERE id = ?8",
        params![
            mode,
            permission_mode,
            model,
            view,
            work_dir,
            cli_session_id,
            allowed_tools,
            id
        ],
    )?;
    Ok(())
}

/// 累加会话用量（Token / 成本），用于会话列表与成本视图
pub fn add_session_usage(conn: &Connection, id: &str, tokens: i64, cost: f64) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions
         SET total_tokens = COALESCE(total_tokens, 0) + ?1,
             total_cost = COALESCE(total_cost, 0) + ?2
         WHERE id = ?3",
        params![tokens, cost, id],
    )?;
    Ok(())
}

/// 复位会话为运行态（用于以新模式重建会话）
pub fn restart_session(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions
         SET status = 'running',
             started_at = ?1,
             ended_at = NULL,
             exit_code = NULL
         WHERE id = ?2",
        params![chrono::Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

/// 刷新"最后一次事件时间"（无响应检测）
pub fn touch_session(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions SET last_event_at = ?1 WHERE id = ?2",
        params![chrono::Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

/// 会话结束时回写：状态、退出码、结束时间与执行时长。
///
/// 时长优先使用 `started_at` 与当前时间的差值计算，避免调用方传错。
pub fn finish_agent_session(
    conn: &Connection,
    id: &str,
    status: &str,
    exit_code: Option<i32>,
) -> Result<()> {
    let now = chrono::Utc::now();

    // 读取开始时间用于计算时长；解析失败时保留原值（NULL）
    let duration_seconds: Option<i64> = conn
        .query_row(
            "SELECT started_at FROM agent_sessions WHERE id = ?",
            params![id],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|started| {
            chrono::DateTime::parse_from_rfc3339(&started)
                .ok()
                .map(|start| (now - start.with_timezone(&chrono::Utc)).num_seconds())
        });

    // 仅当会话仍处于 running 时才回写，避免覆盖用户主动终止后的终态
    conn.execute(
        "UPDATE agent_sessions
         SET status = ?1,
             ended_at = ?2,
             exit_code = ?3,
             duration_seconds = COALESCE(?4, duration_seconds)
         WHERE id = ?5 AND status = 'running'",
        params![status, now.to_rfc3339(), exit_code, duration_seconds, id],
    )?;

    Ok(())
}

/// 会话终态 → 任务回写（P1-T3 + P1-T6）。
///
/// 会话 `completed` 且关联了任务（`related_task_id` 优先）时：
/// - 任务仍处于 `pending`/`in_progress` → 推进到 `completed`
/// - 把会话时长（小时）累加进 `tasks.actual_hours`
///
/// 无论会话终态如何，都会把该任务在 `token_usage` 表的用量汇总
/// 回写到 `tasks.token_usage`（P1-T6：消除僵尸字段）。
///
/// 会话 `failed` 不动任务状态（失败不代表工作全无产出，交由用户判断）。
/// 返回是否发生了任务状态推进。
pub fn sync_task_from_session(conn: &Connection, session_id: &str) -> Result<bool> {
    let row = conn.query_row(
        "SELECT status, related_task_id, task_id, duration_seconds
         FROM agent_sessions WHERE id = ?1",
        params![session_id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
            ))
        },
    );

    let (session_status, related_task_id, task_id, duration_seconds) = match row {
        Ok(v) => v,
        // 会话不存在：静默跳过
        Err(_) => return Ok(false),
    };

    let task_id = match related_task_id.or(task_id) {
        Some(tid) if !tid.is_empty() => tid,
        _ => return Ok(false),
    };

    // 回填实际工时（会话时长折算小时，累加）
    if let Some(secs) = duration_seconds {
        if secs > 0 {
            let hours = secs as f64 / 3600.0;
            conn.execute(
                "UPDATE tasks
                 SET actual_hours = COALESCE(actual_hours, 0) + ?1,
                     updated_at = ?2
                 WHERE id = ?3",
                params![hours, chrono::Utc::now().to_rfc3339(), task_id],
            )?;
        }
    }

    // P1-T6：汇总该任务的 Token 用量回写（input+output+cache+thinking）
    // 无论会话终态如何都执行（失败的会话同样消耗了 Token）
    let total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens + thinking_tokens), 0)
             FROM token_usage WHERE task_id = ?1",
            params![task_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if total > 0 {
        conn.execute(
            "UPDATE tasks SET token_usage = ?1, updated_at = ?2 WHERE id = ?3",
            params![total, chrono::Utc::now().to_rfc3339(), task_id],
        )?;
    }

    if session_status != "completed" {
        return Ok(false);
    }

    // 仅推进 pending/in_progress → completed；blocked/completed 不动
    let now = chrono::Utc::now().to_rfc3339();
    let changed = conn.execute(
        "UPDATE tasks
         SET status = 'completed', updated_at = ?1
         WHERE id = ?2 AND status IN ('pending', 'in_progress')",
        params![now, task_id],
    )?;

    Ok(changed > 0)
}

/// 应用启动时的会话对账。
///
/// 进程随应用退出而消失，因此数据库里处于 `running` 的会话必定已无真实进程；
/// 统一标记为 `failed` 并补齐结束时间与时长，避免"僵尸会话"。
/// 返回被修正的记录数。
pub fn reconcile_running_sessions(conn: &Connection) -> Result<usize> {
    let now = chrono::Utc::now().to_rfc3339();

    let affected = conn.execute(
        "UPDATE agent_sessions
         SET status = 'failed',
             exit_code = -1,
             ended_at = ?1,
             duration_seconds = COALESCE(
                 duration_seconds,
                 CAST(strftime('%s', ?1) AS INTEGER) - CAST(strftime('%s', started_at) AS INTEGER)
             )
         WHERE status = 'running'",
        params![now],
    )?;

    Ok(affected)
}

/// `agent_sessions` 查询列清单（新增列统一追加在末尾，保持既有下标稳定）
const SESSION_COLUMNS: &str = "agent_sessions.id, agent_sessions.agent_id, agent_sessions.task_id, \
     agent_sessions.status, agent_sessions.started_at, agent_sessions.ended_at, \
     agent_sessions.exit_code, agent_sessions.duration_seconds, agent_sessions.total_tokens, \
     agent_sessions.total_cost, agent_sessions.project_id, agent_sessions.source, \
     agent_sessions.source_path, agent_sessions.title, agent_sessions.description, \
     agent_sessions.related_task_id, agent_sessions.related_requirement_id, \
     agent_sessions.skills, agent_sessions.mcps, agent_sessions.skill_execution_summary, \
     agent_sessions.mcp_execution_summary, agent_sessions.cli_session_id, \
     agent_sessions.work_dir, agent_sessions.agent_path, agent_sessions.mode, \
     agent_sessions.view, agent_sessions.permission_mode, agent_sessions.model, \
     agent_sessions.permission_allowlist, agent_sessions.last_event_at, \
     agent_sessions.archived";

/// 回填列（Agent 显示名 / 任务标题）：非 agent_sessions 本表字段，查询时 JOIN 获取
const SESSION_JOIN_COLUMNS: &str = ", agents.display_name, tasks.title";

/// 会话查询的关联表
const SESSION_JOINS: &str = "LEFT JOIN agents ON agents.id = agent_sessions.agent_id \
     LEFT JOIN tasks ON tasks.id = agent_sessions.task_id";

fn map_session_row(row: &rusqlite::Row<'_>) -> Result<AgentSession> {
    Ok(AgentSession {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        task_id: row.get(2)?,
        status: row.get(3)?,
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
        exit_code: row.get(6)?,
        duration_seconds: row.get(7)?,
        total_tokens: row.get(8)?,
        total_cost: row.get(9)?,
        project_id: row.get(10)?,
        source: row.get(11)?,
        source_path: row.get(12)?,
        title: row.get(13)?,
        description: row.get(14)?,
        related_task_id: row.get(15)?,
        related_requirement_id: row.get(16)?,
        skills: row.get(17)?,
        mcps: row.get(18)?,
        skill_execution_summary: row.get(19)?,
        mcp_execution_summary: row.get(20)?,
        cli_session_id: row.get(21)?,
        work_dir: row.get(22)?,
        agent_path: row.get(23)?,
        mode: row.get(24)?,
        view: row.get(25)?,
        permission_mode: row.get(26)?,
        model: row.get(27)?,
        permission_allowlist: row.get(28)?,
        last_event_at: row.get(29)?,
        archived: row.get(30)?,
        agent_name: row.get(31)?,
        task_title: row.get(32)?,
    })
}

/// 统计当前 `running` 状态的会话数（并发上限检查用）
pub fn count_running_sessions(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM agent_sessions WHERE status = 'running'", [], |r| r.get(0))
}

pub fn get_agent_sessions(
    conn: &Connection,
    agent_id: Option<&str>,
    include_archived: bool,
) -> Result<Vec<AgentSession>> {
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut sql = format!(
        "SELECT {SESSION_COLUMNS} {SESSION_JOIN_COLUMNS} FROM agent_sessions {SESSION_JOINS}"
    );

    let mut wheres: Vec<String> = Vec::new();
    if let Some(id) = agent_id {
        wheres.push("agent_sessions.agent_id = ?".to_string());
        params.push(Box::new(id.to_string()));
    }
    if !include_archived {
        wheres.push("(agent_sessions.archived IS NULL OR agent_sessions.archived = 0)".to_string());
    }
    if !wheres.is_empty() {
        sql.push_str(&format!(" WHERE {}", wheres.join(" AND ")));
    }

    sql.push_str(" ORDER BY agent_sessions.started_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), map_session_row)?;

    rows.collect()
}

/// 重命名会话
pub fn rename_session(conn: &Connection, id: &str, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions SET title = ?1 WHERE id = ?2",
        params![title, id],
    )?;
    Ok(())
}

/// 归档 / 取消归档会话（归档后默认列表不可见，数据保留）
pub fn set_session_archived(conn: &Connection, id: &str, archived: bool) -> Result<()> {
    conn.execute(
        "UPDATE agent_sessions SET archived = ?1 WHERE id = ?2",
        params![if archived { 1 } else { 0 }, id],
    )?;
    Ok(())
}

/// 级联删除会话：消息、Skill/MCP 执行日志、Token 记录、最后删除会话本体。
///
/// 运行中的进程应由命令层先终止；本函数只负责数据清理。
pub fn delete_session_cascade(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM session_messages WHERE session_id = ?1", params![id])?;
    conn.execute(
        "DELETE FROM skill_execution_logs WHERE session_id = ?1",
        params![id],
    )?;
    conn.execute(
        "DELETE FROM mcp_execution_logs WHERE session_id = ?1",
        params![id],
    )?;
    conn.execute("DELETE FROM token_usage WHERE session_id = ?1", params![id])?;
    conn.execute("DELETE FROM agent_sessions WHERE id = ?1", params![id])?;
    Ok(())
}
