use rusqlite::{Connection, Result, params};
use super::models::{TokenUsage, TokenBudget, TokenUsageSummary, DailyUsage, ModelUsage};
use chrono::Datelike;

/// `token_usage` 的显式列清单。
///
/// **不要改回 `SELECT *`**：旧库经 `ALTER TABLE ADD COLUMN` 追加
/// `cache_creation_tokens` / `cache_read_tokens` / `thinking_tokens` /
/// `source` / `recorded_at` 后，这些列位于表尾，与逻辑顺序不一致，
/// 使用 `SELECT *` + 固定下标会导致取值错位（见差距分析 B-01）。
const TOKEN_USAGE_COLUMNS: &str = "id, session_id, task_id, agent_type, model, \
     input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, \
     thinking_tokens, total_cost, source, recorded_at";

/// `token_budgets` 的显式列清单（同样避免 `SELECT *` + 下标）
const TOKEN_BUDGET_COLUMNS: &str =
    "id, scope, monthly_limit, alert_threshold, current_usage, reset_date, created_at";

/// 模型价格表（美元 / 每百万 tokens）：(输入价, 输出价)
///
/// 用于在缺少真实计费信息时给出**粗略估算**，避免 `total_cost` 恒为 0。
/// 价格取自公开定价，仅作成本参考，不作为账单依据。
fn model_price_per_million(model: &str) -> (f64, f64) {
    let m = model.to_lowercase();
    if m.contains("opus") {
        (15.0, 75.0)
    } else if m.contains("haiku") {
        (0.25, 1.25)
    } else if m.contains("sonnet") {
        (3.0, 15.0)
    } else {
        // 未知模型按 Sonnet 档位估算
        (3.0, 15.0)
    }
}

/// 估算一次调用的成本（美元）。
///
/// 口径：输入按输入价，输出与思考按输出价，缓存写入按输入价 1.25 倍、
/// 缓存读取按输入价 0.1 倍（与主流实现一致）。
pub fn estimate_cost(
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
    thinking_tokens: i64,
) -> f64 {
    let (pin, pout) = model_price_per_million(model);
    let per = 1_000_000.0;

    let input_cost = (input_tokens as f64 / per) * pin;
    let output_cost = ((output_tokens + thinking_tokens) as f64 / per) * pout;
    let cache_write_cost = (cache_creation_tokens as f64 / per) * pin * 1.25;
    let cache_read_cost = (cache_read_tokens as f64 / per) * pin * 0.1;

    // 保留 6 位小数，避免浮点噪声
    ((input_cost + output_cost + cache_write_cost + cache_read_cost) * 1_000_000.0).round() / 1_000_000.0
}

/// 记录 Token 使用（完整字段版本）。
///
/// 同时按当月累计更新 `token_budgets.current_usage`，跨月自动重置。
#[allow(clippy::too_many_arguments)]
pub fn record_token_usage_ext(
    conn: &Connection,
    session_id: Option<&str>,
    task_id: Option<&str>,
    agent_type: Option<&str>,
    model: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
    thinking_tokens: i64,
    total_cost: f64,
    source: &str,
) -> Result<TokenUsage> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO token_usage
            (id, session_id, task_id, agent_type, model, input_tokens, output_tokens,
             cache_creation_tokens, cache_read_tokens, thinking_tokens,
             total_cost, source, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id,
            session_id,
            task_id,
            agent_type,
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            thinking_tokens,
            total_cost,
            source,
            now
        ],
    )?;

    // 预算用量累计（跨月自动重置）
    let total_tokens = input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens + thinking_tokens;
    if let Err(e) = add_budget_usage(conn, total_tokens) {
        tracing::warn!("更新 Token 预算用量失败: {}", e);
    }

    Ok(TokenUsage {
        id,
        session_id: session_id.map(|s| s.to_string()),
        task_id: task_id.map(|s| s.to_string()),
        agent_type: agent_type.map(|s| s.to_string()),
        model: model.map(|s| s.to_string()),
        input_tokens,
        output_tokens,
        cache_creation_tokens,
        cache_read_tokens,
        thinking_tokens,
        total_cost,
        source: source.to_string(),
        recorded_at: now,
    })
}

/// 记录 Token 使用（兼容旧签名，仅输入/输出）
pub fn record_token_usage(
    conn: &Connection,
    session_id: Option<&str>,
    task_id: Option<&str>,
    agent_type: Option<&str>,
    model: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
    total_cost: f64,
) -> Result<TokenUsage> {
    record_token_usage_ext(
        conn,
        session_id,
        task_id,
        agent_type,
        model,
        input_tokens,
        output_tokens,
        0,
        0,
        0,
        total_cost,
        "api",
    )
}

/// 按当月累计 Token 预算用量；若 `reset_date` 不是当前月则先重置。
pub fn add_budget_usage(conn: &Connection, tokens: i64) -> Result<()> {
    let today = chrono::Utc::now().date_naive();
    let month_start = format!("{}-{:02}-01", today.year(), today.month());

    conn.execute(
        "UPDATE token_budgets
         SET current_usage = CASE
                WHEN reset_date IS NULL OR reset_date <> ?1 THEN ?2
                ELSE COALESCE(current_usage, 0) + ?2
             END,
             reset_date = ?1
         WHERE scope = 'global'",
        params![month_start, tokens],
    )?;

    Ok(())
}

pub fn get_token_usage(conn: &Connection, task_id: Option<&str>) -> Result<Vec<TokenUsage>> {
    let mut sql = format!("SELECT {} FROM token_usage", TOKEN_USAGE_COLUMNS);
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(id) = task_id {
        sql.push_str(" WHERE task_id = ?");
        params.push(Box::new(id.to_string()));
    }

    sql.push_str(" ORDER BY recorded_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(TokenUsage {
            id: row.get(0)?,
            session_id: row.get(1)?,
            task_id: row.get(2)?,
            agent_type: row.get(3)?,
            model: row.get(4)?,
            input_tokens: row.get(5)?,
            output_tokens: row.get(6)?,
            cache_creation_tokens: row.get(7)?,
            cache_read_tokens: row.get(8)?,
            thinking_tokens: row.get(9)?,
            total_cost: row.get(10)?,
            source: row.get(11)?,
            recorded_at: row.get(12)?,
        })
    })?;

    rows.collect()
}

pub fn get_token_usage_summary(conn: &Connection) -> Result<TokenUsageSummary> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(SUM(input_tokens + output_tokens), 0),
                COALESCE(SUM(total_cost), 0),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(cache_creation_tokens + cache_read_tokens), 0),
                COALESCE(SUM(thinking_tokens), 0),
                COUNT(*)
         FROM token_usage"
    )?;

    let row = stmt.query_row([], |row| {
        Ok(TokenUsageSummary {
            total_tokens: row.get(0)?,
            total_cost: row.get(1)?,
            input_tokens: row.get(2)?,
            output_tokens: row.get(3)?,
            cache_tokens: row.get(4)?,
            thinking_tokens: row.get(5)?,
            record_count: row.get(6)?,
        })
    })?;

    Ok(row)
}

pub fn get_token_usage_trend(conn: &Connection, days: i32) -> Result<Vec<DailyUsage>> {
    let mut stmt = conn.prepare(
        "SELECT DATE(recorded_at) as date,
                COALESCE(SUM(input_tokens + output_tokens), 0),
                COALESCE(SUM(total_cost), 0),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0)
         FROM token_usage
         WHERE recorded_at >= datetime('now', ? || ' days')
         GROUP BY DATE(recorded_at)
         ORDER BY date ASC"
    )?;

    let days_str = format!("-{}", days);
    let rows = stmt.query_map(params![days_str], |row| {
        Ok(DailyUsage {
            date: row.get(0)?,
            total_tokens: row.get(1)?,
            total_cost: row.get(2)?,
            input_tokens: row.get(3)?,
            output_tokens: row.get(4)?,
        })
    })?;

    rows.collect()
}

pub fn get_token_usage_by_model(conn: &Connection) -> Result<Vec<ModelUsage>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(model, 'unknown'),
                COALESCE(SUM(input_tokens + output_tokens), 0),
                COALESCE(SUM(total_cost), 0),
                COUNT(*)
         FROM token_usage
         GROUP BY model
         ORDER BY SUM(input_tokens + output_tokens) DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ModelUsage {
            model: row.get(0)?,
            total_tokens: row.get(1)?,
            total_cost: row.get(2)?,
            record_count: row.get(3)?,
        })
    })?;

    rows.collect()
}

pub fn get_token_budget(conn: &Connection) -> Result<Option<TokenBudget>> {
    let sql = format!(
        "SELECT {} FROM token_budgets WHERE scope = 'global' LIMIT 1",
        TOKEN_BUDGET_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map([], |row| {
        Ok(TokenBudget {
            id: row.get(0)?,
            scope: row.get(1)?,
            monthly_limit: row.get(2)?,
            alert_threshold: row.get(3)?,
            current_usage: row.get(4)?,
            reset_date: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn set_token_budget(conn: &Connection, monthly_limit: i64, alert_threshold: f64) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let today = chrono::Utc::now().date_naive();
    let reset_date = format!("{}-{:02}-01", today.year(), today.month());

    conn.execute(
        "INSERT OR REPLACE INTO token_budgets (id, scope, monthly_limit, alert_threshold, current_usage, reset_date, created_at)
         SELECT id, scope, ?, ?, current_usage, ?, ? FROM token_budgets WHERE scope = 'global'",
        params![monthly_limit, alert_threshold, reset_date, now],
    )?;

    // 如果没有记录则插入
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM token_budgets WHERE scope = 'global'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO token_budgets (id, scope, monthly_limit, alert_threshold, reset_date) VALUES (?, 'global', ?, ?, ?)",
            params![id, monthly_limit, alert_threshold, reset_date],
        )?;
    }

    Ok(())
}

/// 预算状态：用于前端/状态栏判断是否需要告警
pub fn get_budget_alert(conn: &Connection) -> Result<Option<serde_json::Value>> {
    let Some(budget) = get_token_budget(conn)? else {
        return Ok(None);
    };

    let usage_ratio = if budget.monthly_limit > 0 {
        budget.current_usage as f64 / budget.monthly_limit as f64
    } else {
        0.0
    };

    Ok(Some(serde_json::json!({
        "id": budget.id,
        "scope": budget.scope,
        "monthly_limit": budget.monthly_limit,
        "alert_threshold": budget.alert_threshold,
        "current_usage": budget.current_usage,
        "reset_date": budget.reset_date,
        "created_at": budget.created_at,
        // 以下为派生字段，供前端直接展示告警状态
        "usage_ratio": (usage_ratio * 10000.0).round() / 10000.0,
        "exceeded": usage_ratio >= budget.alert_threshold,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_cost_sonnet_is_per_million() {
        // 100 万输入 + 100 万输出，Sonnet 档位应为 3 + 15 = 18 美元
        let cost = estimate_cost("claude-sonnet-4", 1_000_000, 1_000_000, 0, 0, 0);
        assert!((cost - 18.0).abs() < 1e-6, "actual={}", cost);
    }

    #[test]
    fn estimate_cost_unknown_model_falls_back_to_sonnet() {
        let a = estimate_cost("unknown-model", 1000, 1000, 0, 0, 0);
        let b = estimate_cost("claude-sonnet-4", 1000, 1000, 0, 0, 0);
        assert!((a - b).abs() < 1e-9);
    }

    #[test]
    fn estimate_cost_includes_cache_and_thinking() {
        let base = estimate_cost("claude-sonnet-4", 0, 0, 0, 0, 0);
        assert_eq!(base, 0.0);

        let with_cache = estimate_cost("claude-sonnet-4", 0, 0, 1_000_000, 1_000_000, 0);
        // 缓存写入 1.25 * 3 = 3.75，缓存读取 0.1 * 3 = 0.3
        assert!((with_cache - 4.05).abs() < 1e-6, "actual={}", with_cache);

        let with_thinking = estimate_cost("claude-sonnet-4", 0, 0, 0, 0, 1_000_000);
        assert!((with_thinking - 15.0).abs() < 1e-6, "actual={}", with_thinking);
    }

    /// 回归：列顺序被追加到表尾时，get_token_usage 仍应正确取值
    #[test]
    fn get_token_usage_reads_by_explicit_columns() {
        let conn = Connection::open_in_memory().unwrap();

        // 模拟"旧库"：先建基础列，再 ALTER 追加缓存列（追加到表尾）
        conn.execute_batch(
            "CREATE TABLE token_usage (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                task_id TEXT,
                agent_type TEXT,
                model TEXT,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0,
                recorded_at DATETIME
            );
            ALTER TABLE token_usage ADD COLUMN cache_creation_tokens INTEGER DEFAULT 0;
            ALTER TABLE token_usage ADD COLUMN cache_read_tokens INTEGER DEFAULT 0;
            ALTER TABLE token_usage ADD COLUMN thinking_tokens INTEGER DEFAULT 0;
            ALTER TABLE token_usage ADD COLUMN source TEXT DEFAULT 'api';",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO token_usage
                (id, session_id, task_id, agent_type, model, input_tokens, output_tokens,
                 total_cost, recorded_at, cache_creation_tokens, cache_read_tokens, thinking_tokens, source)
             VALUES ('t1', NULL, 'task-1', 'claude-api', 'claude-sonnet-4',
                     100, 200, 1.5, '2026-01-01T00:00:00Z', 11, 22, 33, 'api')",
            [],
        )
        .unwrap();

        let rows = get_token_usage(&conn, None).unwrap();
        assert_eq!(rows.len(), 1);

        let r = &rows[0];
        assert_eq!(r.input_tokens, 100);
        assert_eq!(r.output_tokens, 200);
        assert_eq!(r.cache_creation_tokens, 11);
        assert_eq!(r.cache_read_tokens, 22);
        assert_eq!(r.thinking_tokens, 33);
        assert_eq!(r.total_cost, 1.5);
        assert_eq!(r.source, "api");
    }

    /// 回归：跨月时预算用量应重置
    #[test]
    fn budget_usage_resets_on_new_month() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE token_budgets (
                id TEXT PRIMARY KEY,
                scope TEXT NOT NULL DEFAULT 'global',
                monthly_limit INTEGER NOT NULL DEFAULT 2000000,
                alert_threshold REAL NOT NULL DEFAULT 0.8,
                current_usage INTEGER DEFAULT 0,
                reset_date TEXT,
                created_at DATETIME
            );
            INSERT INTO token_budgets (id, scope, monthly_limit, alert_threshold, current_usage, reset_date, created_at)
            VALUES ('b1', 'global', 1000, 0.8, 900, '2020-01-01', '2020-01-01T00:00:00Z');",
        )
        .unwrap();

        add_budget_usage(&conn, 100).unwrap();

        let budget = get_token_budget(&conn).unwrap().unwrap();
        assert_eq!(budget.current_usage, 100, "跨月应重置为本次用量");

        // 同月再累加
        add_budget_usage(&conn, 50).unwrap();
        let budget = get_token_budget(&conn).unwrap().unwrap();
        assert_eq!(budget.current_usage, 150);

        let alert = get_budget_alert(&conn).unwrap().unwrap();
        assert_eq!(alert["exceeded"], false);

        add_budget_usage(&conn, 900).unwrap();
        let alert = get_budget_alert(&conn).unwrap().unwrap();
        assert_eq!(alert["exceeded"], true);
    }
}
