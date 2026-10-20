use rusqlite::{Connection, Result};
use tracing::info;

/// 当前 schema 基线版本。
///
/// **迁移策略（单一事实来源）**：
/// - 运行时**唯一**的迁移入口是 `Database::new()`，依次执行
///   `init_tables()`（v8 基线表 + 索引）→ `migrate_prd_skills()` / `migrate_requirements()`
///   → `migrate_tables()`（幂等 `ALTER TABLE ADD COLUMN` 与补建表）
///   → `migrate_unique_constraints()`（去重并建立唯一约束）→ `seed_data()`（种子数据）
///   → `ensure_baseline()`（登记基线版本）→ `apply_pending_migrations()`（v9+ 顺序升级）；
/// - **v9 起的 schema 变更一律追加到 `MIGRATIONS`，禁止再改 `init_tables()`**：
///   `init_tables` 永远保持 v8 快照，新老库统一经 `MIGRATIONS` 升级，升级路径可回放；
/// - 每条迁移必须幂等（`IF NOT EXISTS` / 带条件更新），且只依赖 v8 已存在的表；
///   单条迁移在独立事务内执行，失败即回滚且不登记版本；
/// - 历史上存在的 12 个 `db/migrations/*.sql` 从未被 `include_str!` 加载（属死文件，
///   内容已全部并入上述运行时路径），为避免"存在版本化迁移机制"的误解，已移除。
pub const SCHEMA_BASELINE_VERSION: i32 = 8;

/// v9+ 版本化迁移清单：`(version, name, sql)`，**版本号必须严格递增**。
///
/// v9：高频查询缺索引的集中补齐（Skill 执行日志/评分/建议/版本按 skill_id 查询、
/// 任务按需求查询、评论按任务查询、Token 趋势按 recorded_at 分组）——
/// 数据量增长后这些查询会从全表扫描受益。
pub const MIGRATIONS: &[(i32, &str, &str)] = &[(9, "perf_indexes", "
CREATE INDEX IF NOT EXISTS idx_tasks_requirement ON tasks(requirement_id);
CREATE INDEX IF NOT EXISTS idx_task_comments_task ON task_comments(task_id);
CREATE INDEX IF NOT EXISTS idx_skill_exec_logs_skill ON skill_execution_logs(skill_id, executed_at);
CREATE INDEX IF NOT EXISTS idx_skill_scores_skill ON skill_scores(skill_id, judged_at);
CREATE INDEX IF NOT EXISTS idx_skill_suggestions_skill ON skill_suggestions(skill_id, status);
CREATE INDEX IF NOT EXISTS idx_skill_versions_skill ON skill_versions(skill_id, version);
CREATE INDEX IF NOT EXISTS idx_token_usage_recorded ON token_usage(recorded_at);
")];

/// 确保 `schema_version` 表存在
fn ensure_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )
}

/// 获取当前数据库版本（无记录时为 0）
pub fn get_current_version(conn: &Connection) -> Result<i32> {
    ensure_table(conn)?;
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get::<_, i32>(0),
    )
}

/// 记录版本号（仅在该版本尚未登记时插入）
pub fn set_version(conn: &Connection, version: i32) -> Result<()> {
    ensure_table(conn)?;

    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_version WHERE version = ?1",
            [version],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !exists {
        conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [version])?;
    }

    Ok(())
}

/// 登记 schema 基线版本并返回当前记录版本。
///
/// 由 `Database::new()` 在结构迁移完成后调用，是**唯一**的版本登记入口
/// （此前散落在 `db/mod.rs` 中的硬编码 `set_version(conn, 8)` 已收敛到这里）。
pub fn ensure_baseline(conn: &Connection) -> Result<i32> {
    set_version(conn, SCHEMA_BASELINE_VERSION)?;
    let current = get_current_version(conn)?;
    info!(
        "schema 基线版本: {}（当前记录版本 {}）",
        SCHEMA_BASELINE_VERSION, current
    );
    Ok(current)
}

/// 按版本号升序应用待执行迁移，返回本次实际应用条数。
///
/// - 只应用 `version > 当前记录版本` 的条目（已登记的跳过，天然幂等）；
/// - 每条迁移在**独立事务**内执行并登记版本：失败 → 回滚且不登记，
///   下次启动会重试；`Database::new` 将上抛错误并由启动路径给出可见提示。
pub fn apply_pending_migrations(conn: &Connection) -> Result<usize> {
    apply_migrations(conn, MIGRATIONS)
}

/// `apply_pending_migrations` 的可测试实现（迁移清单参数化）。
fn apply_migrations(conn: &Connection, migrations: &[(i32, &str, &str)]) -> Result<usize> {
    ensure_table(conn)?;
    let current = get_current_version(conn)?;

    // 清单须严格递增，防止乱序登记导致跳过迁移
    let mut last = current;
    for (version, _, _) in migrations {
        if *version <= current {
            continue;
        }
        if *version <= last {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "迁移清单乱序：version {} 不大于前序版本 {}",
                version, last
            )));
        }
        last = *version;
    }

    let mut applied = 0usize;
    for (version, name, sql) in migrations {
        if *version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        set_version(&tx, *version)?;
        tx.commit()?;
        info!("已应用 schema 迁移 v{}: {}", version, name);
        applied += 1;
    }

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        Connection::open_in_memory().expect("内存库创建失败")
    }

    #[test]
    fn migrations_apply_once_in_order_and_are_idempotent() {
        let conn = mem_conn();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT);").unwrap();

        let list: &[(i32, &str, &str)] = &[
            (9, "add_col", "ALTER TABLE t ADD COLUMN extra TEXT;"),
            (10, "seed", "INSERT INTO t (v) VALUES ('x');"),
        ];

        assert_eq!(apply_migrations(&conn, list).unwrap(), 2);
        assert_eq!(get_current_version(&conn).unwrap(), 10);

        // 第二次执行：全部跳过（幂等）
        assert_eq!(apply_migrations(&conn, list).unwrap(), 0);
        assert_eq!(get_current_version(&conn).unwrap(), 10);
        // 重复 INSERT 不会再发生
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn failed_migration_rolls_back_and_is_not_recorded() {
        let conn = mem_conn();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY);").unwrap();

        let list: &[(i32, &str, &str)] = &[(9, "broken", "INSERT INTO missing_table VALUES (1);")];
        assert!(apply_migrations(&conn, list).is_err());
        assert_eq!(
            get_current_version(&conn).unwrap(),
            0,
            "失败的迁移不得登记版本"
        );

        // 修复后重试成功
        let fixed: &[(i32, &str, &str)] = &[(9, "fixed", "INSERT INTO t VALUES (1);")];
        assert_eq!(apply_migrations(&conn, fixed).unwrap(), 1);
        assert_eq!(get_current_version(&conn).unwrap(), 9);
    }

    #[test]
    fn out_of_order_migration_list_is_rejected() {
        let conn = mem_conn();
        let list: &[(i32, &str, &str)] = &[
            (10, "later", "SELECT 1;"),
            (9, "earlier", "SELECT 1;"),
        ];
        assert!(apply_migrations(&conn, list).is_err());
    }

    #[test]
    fn production_migration_list_is_strictly_increasing() {
        // 真实清单静态校验：防止后续 PR 提交乱序版本
        let mut last = SCHEMA_BASELINE_VERSION;
        for (version, _, _) in MIGRATIONS {
            assert!(*version > last, "MIGRATIONS 版本号必须严格递增");
            last = *version;
        }
    }

    #[test]
    fn production_migrations_apply_cleanly_on_baseline_schema() {
        // 在内存库上跑真实清单（依赖表由调用方保证已存在，此处补最小占位表）
        let conn = mem_conn();
        conn.execute_batch(
            "CREATE TABLE tasks (requirement_id TEXT);
             CREATE TABLE task_comments (task_id TEXT);
             CREATE TABLE skill_execution_logs (skill_id TEXT, executed_at TEXT);
             CREATE TABLE skill_scores (skill_id TEXT, judged_at TEXT);
             CREATE TABLE skill_suggestions (skill_id TEXT, status TEXT);
             CREATE TABLE skill_versions (skill_id TEXT, version INTEGER);
             CREATE TABLE token_usage (recorded_at TEXT);",
        )
        .unwrap();
        assert_eq!(apply_pending_migrations(&conn).unwrap(), MIGRATIONS.len());
        // 重复执行幂等
        assert_eq!(apply_pending_migrations(&conn).unwrap(), 0);
    }
}
