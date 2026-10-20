use rusqlite::{Connection, Result};
use tracing::info;

/// 当前 schema 基线版本。
///
/// **迁移策略（单一事实来源）**：
/// - 运行时**唯一**的迁移入口是 `Database::new()`，依次执行
///   `init_tables()`（基线表 + 索引）→ `migrate_prd_skills()` / `migrate_requirements()`
///   → `migrate_tables()`（幂等 `ALTER TABLE ADD COLUMN` 与补建表）
///   → `migrate_unique_constraints()`（去重并建立唯一约束）→ `seed_data()`（种子数据）；
/// - 本常量仅用于在 `schema_version` 表中登记"基线已应用"，**不驱动升级顺序**；
/// - 历史上存在的 12 个 `db/migrations/*.sql` 从未被 `include_str!` 加载（属死文件，
///   内容已全部并入上述运行时路径），为避免"存在版本化迁移机制"的误解，已移除。
pub const SCHEMA_BASELINE_VERSION: i32 = 8;

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
