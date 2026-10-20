use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, Result};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// 参与导出/导入的表（同时作为导入顺序，父表在前以满足潜在外键依赖）。
pub const SYNC_TABLES: &[&str] = &[
    "projects",
    "requirements",
    "tasks",
    "task_comments",
    "agents",
    "agent_sessions",
    "prd_skills",
    "project_skills",
    "project_mcps",
    "stage_skill_configs",
    "token_usage",
    "token_budgets",
    "skill_execution_logs",
    "mcp_execution_logs",
    "git_commits",
    "requirement_history",
    "settings",
];

/// 导出快照的格式版本，导入时校验
pub const SNAPSHOT_VERSION: &str = "2.0";

/// 把一行 SQLite 值转为 JSON 值（BLOB 无法表达，降级为 null）
fn value_ref_to_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(t) => Value::String(String::from_utf8_lossy(t).to_string()),
        ValueRef::Blob(_) => Value::Null,
    }
}

/// JSON 值转 SQLite 值；数组/对象序列化为 JSON 文本，避免丢字段
fn json_to_sql_value(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                SqlValue::Real(f)
            } else {
                SqlValue::Text(n.to_string())
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

/// 读取表结构中的列名（按定义顺序）
fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut cols = Vec::new();
    for row in rows {
        cols.push(row?);
    }
    Ok(cols)
}

/// 导出单张表的全部行
fn dump_table(conn: &Connection, table: &str) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM {}", table))?;
    let col_names: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut rows = stmt.query([])?;
    let mut out: Vec<Value> = Vec::new();

    while let Some(row) = rows.next()? {
        let mut obj = Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let value = value_ref_to_json(row.get_ref(i)?);
            obj.insert(name.clone(), value);
        }
        out.push(Value::Object(obj));
    }

    Ok(out)
}

/// 导出全量数据快照（JSON）
pub fn export_snapshot(conn: &Connection) -> Result<Value> {
    let mut tables = Map::new();
    let mut counts = Map::new();

    for table in SYNC_TABLES {
        let rows = dump_table(conn, table)?;
        counts.insert((*table).to_string(), json!(rows.len()));
        tables.insert((*table).to_string(), Value::Array(rows));
    }

    let schema_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(json!({
        "format": "ace-export",
        "version": SNAPSHOT_VERSION,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "schema_version": schema_version,
        "counts": counts,
        "tables": tables,
    }))
}

/// 在事务内导入单张表
fn insert_table(conn: &Connection, table: &str, rows: &[Value], replace: bool) -> Result<i64> {
    if rows.is_empty() {
        if replace {
            conn.execute(&format!("DELETE FROM {}", table), [])?;
        }
        return Ok(0);
    }

    let schema_cols = table_columns(conn, table)?;
    if schema_cols.is_empty() {
        return Ok(0);
    }

    // 取 payload 中出现过、且表结构存在的列（容忍 schema 漂移）
    let mut used_cols: Vec<String> = Vec::new();
    for row in rows {
        if let Some(obj) = row.as_object() {
            for key in obj.keys() {
                if schema_cols.contains(key) && !used_cols.contains(key) {
                    used_cols.push(key.clone());
                }
            }
        }
    }

    if used_cols.is_empty() {
        return Ok(0);
    }

    if replace {
        conn.execute(&format!("DELETE FROM {}", table), [])?;
    }

    let placeholders = used_cols
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
        table,
        used_cols.join(", "),
        placeholders
    );

    let mut inserted: i64 = 0;
    {
        let mut stmt = conn.prepare(&sql)?;
        for row in rows {
            let Some(obj) = row.as_object() else { continue };
            let values: Vec<SqlValue> = used_cols
                .iter()
                .map(|col| obj.get(col).map(json_to_sql_value).unwrap_or(SqlValue::Null))
                .collect();
            stmt.execute(rusqlite::params_from_iter(values.iter()))?;
            inserted += 1;
        }
    }

    Ok(inserted)
}

/// 导入数据快照。
///
/// - `replace = true`：先清空目标表再写入（完全恢复）
/// - `replace = false`：以主键为准合并（INSERT OR REPLACE，不删除多余数据）
///
/// 导入过程在单个事务内完成，失败自动回滚；导入期间临时关闭外键检查，
/// 结束后恢复为 `ON`。
pub fn import_snapshot(conn: &Connection, payload: &Value, replace: bool) -> Result<Value> {
    // 外键 PRAGMA 在事务内无效，需在开启事务前设置
    conn.execute_batch("PRAGMA foreign_keys=OFF;")?;

    let outcome = (|| -> Result<Value> {
        let tx = conn.unchecked_transaction()?;
        let mut per_table = Map::new();
        let mut total: i64 = 0;

        let tables_obj = payload.get("tables").and_then(|v| v.as_object());

        for table in SYNC_TABLES {
            let rows: Vec<Value> = tables_obj
                .and_then(|o| o.get(*table))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            // payload 未包含该表时：replace 模式跳过（避免误清空），merge 模式也跳过
            if tables_obj.is_none() || tables_obj.and_then(|o| o.get(*table)).is_none() {
                continue;
            }

            let n = insert_table(&tx, table, &rows, replace)?;
            total += n;
            per_table.insert((*table).to_string(), json!(n));
        }

        tx.commit()?;

        Ok(json!({
            "mode": if replace { "replace" } else { "merge" },
            "total": total,
            "tables": Value::Object(per_table),
        }))
    })();

    // 无论成功失败都恢复外键检查
    let _ = conn.execute_batch("PRAGMA foreign_keys=ON;");

    outcome
}

/// 统计各表当前行数，用于导出前展示与导入前比对
pub fn table_counts(conn: &Connection) -> Result<Value> {
    let mut counts = Map::new();
    for table in SYNC_TABLES {
        let n: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| row.get(0))
            .unwrap_or(0);
        counts.insert((*table).to_string(), json!(n));
    }
    Ok(Value::Object(counts))
}

/// 数据库文件备份的前缀
pub const BACKUP_PREFIX: &str = "database.db.bak.";

/// 备份数据库文件，并保留最近 `keep` 份。
///
/// 关键点：
/// 1. 备份前执行 `PRAGMA wal_checkpoint(TRUNCATE)`，把 WAL 中的内容落盘，
///    保证单文件备份是完整可用的（WAL 模式下仅复制主库文件会丢数据）；
/// 2. 备份文件名带时间戳，便于按时间轮转；
/// 3. 自动清理超出 `keep` 的旧备份。
pub fn backup_database_file(conn: &Connection, db_path: &Path, keep: usize) -> Result<PathBuf> {
    // 1. WAL 检查点（失败不阻断备份）
    if let Err(e) = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
        tracing::warn!("WAL checkpoint 失败（继续备份）: {}", e);
    }

    let dir = db_path
        .parent()
        .ok_or_else(|| rusqlite::Error::InvalidPath(db_path.to_path_buf()))?;

    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let target = dir.join(format!("{}{}", BACKUP_PREFIX, ts));

    std::fs::copy(db_path, &target)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    // 2. 轮转
    if let Err(e) = prune_backups(dir, keep) {
        tracing::warn!("清理旧备份失败: {}", e);
    }

    Ok(target)
}

/// 仅保留最近 `keep` 份备份（文件名含时间戳，字典序即时间序）
fn prune_backups(dir: &Path, keep: usize) -> Result<()> {
    let mut backups: Vec<(String, PathBuf)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(BACKUP_PREFIX) {
                backups.push((name, entry.path()));
            }
        }
    }

    backups.sort_by(|a, b| a.0.cmp(&b.0));

    while backups.len() > keep {
        let (_, path) = backups.remove(0);
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!("删除旧备份失败 {:?}: {}", path, e);
        }
    }

    Ok(())
}

/// 列出当前备份文件（按时间升序）
pub fn list_backups(dir: &Path) -> Vec<PathBuf> {
    let mut backups: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(BACKUP_PREFIX) {
                backups.push(entry.path());
            }
        }
    }

    backups.sort();
    backups
}
