//! 数据库层真实链路测试。
//!
//! 与 `tests/integration.rs`（依赖 Victauri E2E 桥、默认跳过）不同，
//! 这里直接使用临时目录中的真实 SQLite 数据库，覆盖：
//! 审计写入、数据导入导出、Token 成本与预算累计、会话对账与结束回写、
//! 提交记录落库、MCP 日志统计、数据库备份轮转。
//!
//! 这些用例**默认执行**（`cargo test`），无需启动应用。

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 测试用临时数据库（不触碰应用数据目录）
struct TempDb {
    pub db: Database,
    pub dir: std::path::PathBuf,
}

impl TempDb {
    fn new(tag: &str) -> Self {
        let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "ace-test-{}-{}-{}",
            tag,
            std::process::id(),
            seq
        ));
        std::fs::create_dir_all(&dir).expect("创建临时目录失败");
        let path = dir.join("database.db");
        let db = Database::new(&path).expect("创建测试数据库失败");
        Self { db, dir }
    }

    fn db_path(&self) -> std::path::PathBuf {
        self.dir.join("database.db")
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 创建一个属于默认项目的需求，返回 id
fn seed_requirement(db: &Database, title: &str) -> String {
    db.create_requirement("default-project", title, Some("测试描述"), "P1")
        .expect("创建需求失败")
        .id
}

/// 注册一个 Agent，返回 id
fn seed_agent(db: &Database, suffix: &str) -> String {
    let agent = Agent {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("test-agent-{}", suffix),
        display_name: format!("Test Agent {}", suffix),
        binary_path: format!("/tmp/test-agent-{}", suffix),
        version: Some("1.0.0".to_string()),
        status: "available".to_string(),
        capabilities: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    // 使用 upsert 返回的"实际生效 id"，与生产代码保持一致
    db.upsert_agent(&agent).expect("保存 Agent 失败")
}

// ==================== 审计链路 ====================

#[test]
fn audit_history_is_written_and_queryable() {
    let t = TempDb::new("audit");
    let req_id = seed_requirement(&t.db, "审计用例");

    t.db.insert_requirement_history(&req_id, "create", None, Some("审计用例"))
        .unwrap();
    t.db.insert_requirement_history(&req_id, "status_change", Some("draft"), Some("reviewing"))
        .unwrap();

    let logs = t.db.list_audit_logs(Some(100), None).unwrap();
    assert_eq!(logs.len(), 2, "应写入 2 条审计记录");

    let actions: Vec<&str> = logs
        .iter()
        .filter_map(|v| v.get("action").and_then(|a| a.as_str()))
        .collect();
    assert!(actions.contains(&"create"));
    assert!(actions.contains(&"status_change"));

    // entity_type 过滤：仅支持 requirement
    assert_eq!(t.db.list_audit_logs(Some(100), Some("requirement")).unwrap().len(), 2);
    assert_eq!(t.db.list_audit_logs(Some(100), Some("task")).unwrap().len(), 0);
}

#[test]
fn audit_keeps_title_snapshot_after_requirement_deleted() {
    let t = TempDb::new("audit-del");
    let req_id = seed_requirement(&t.db, "将被删除的需求");

    // 删除前写入快照（与 RequirementService 的实现一致）
    t.db.insert_requirement_history(&req_id, "delete", Some("将被删除的需求"), Some("draft"))
        .unwrap();
    t.db.delete_requirement(&req_id).unwrap();

    let logs = t.db.list_audit_logs(Some(10), None).unwrap();
    assert_eq!(logs.len(), 1);
    // LEFT JOIN 取不到标题时应回退为 old_value 快照
    assert_eq!(
        logs[0].get("entity_title").and_then(|v| v.as_str()),
        Some("将被删除的需求")
    );
}

#[test]
fn create_task_persists_description() {
    let t = TempDb::new("task-desc");
    let req_id = seed_requirement(&t.db, "描述代入需求");

    // 需求代入：description 随创建写入，关联关系保留
    let task = t
        .db
        .create_task(
            "带描述的任务",
            Some(&req_id),
            "development",
            "P1",
            Some("【来源需求】描述代入需求\n需求正文与 PRD 内容"),
            None,
        )
        .unwrap();

    let loaded = t.db.get_task(&task.id).unwrap().expect("任务应存在");
    assert_eq!(
        loaded.description.as_deref(),
        Some("【来源需求】描述代入需求\n需求正文与 PRD 内容")
    );
    assert_eq!(loaded.requirement_id.as_deref(), Some(req_id.as_str()));
    // P0-T1：经需求创建的任务自动归属项目
    let req_project = t
        .db
        .get_requirement(&req_id)
        .unwrap()
        .expect("需求应存在")
        .project_id;
    if !req_project.is_empty() {
        assert_eq!(loaded.project_id.as_deref(), Some(req_project.as_str()));
    }

    // 手动创建：description 可为空
    let plain = t
        .db
        .create_task("无描述任务", None, "testing", "P2", None, None)
        .unwrap();
    let plain_loaded = t.db.get_task(&plain.id).unwrap().expect("任务应存在");
    assert!(plain_loaded.description.is_none());
}

// ==================== 数据导入导出 ====================

#[test]
fn export_snapshot_contains_real_data() {
    let t = TempDb::new("export");
    let req_id = seed_requirement(&t.db, "导出用例");
    t.db.create_task("导出用任务", Some(&req_id), "development", "P1", None, None)
        .unwrap();

    let snapshot = t.db.export_snapshot().unwrap();

    assert_eq!(snapshot["format"], "ace-export");

    let requirements = snapshot["tables"]["requirements"].as_array().unwrap();
    assert_eq!(requirements.len(), 1, "导出必须包含真实需求数据");
    assert_eq!(requirements[0]["title"], "导出用例");

    let tasks = snapshot["tables"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);

    // counts 与 tables 一致
    assert_eq!(snapshot["counts"]["requirements"], 1);
}

#[test]
fn import_snapshot_restores_data_into_fresh_database() {
    let src = TempDb::new("import-src");
    let req_id = seed_requirement(&src.db, "待迁移需求");
    src.db
        .create_task("待迁移任务", Some(&req_id), "testing", "P2", None, None)
        .unwrap();
    src.db
        .insert_requirement_history(&req_id, "create", None, Some("待迁移需求"))
        .unwrap();

    let snapshot = src.db.export_snapshot().unwrap();

    let dst = TempDb::new("import-dst");
    let result = dst.db.import_snapshot(&snapshot, false).unwrap();

    // 快照包含全部表（含默认项目、内置 Skill、默认预算），
    // 这里只断言业务表被正确写入
    assert_eq!(result["tables"]["requirements"], 1);
    assert_eq!(result["tables"]["tasks"], 1);
    assert_eq!(result["tables"]["requirement_history"], 1);
    assert!(
        result["total"].as_i64().unwrap_or(0) >= 3,
        "total 应至少包含 3 行业务数据，实际 {:?}",
        result["total"]
    );

    let requirements = dst.db.list_requirements(None).unwrap();
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].title, "待迁移需求");

    let tasks = dst.db.list_tasks(None, None).unwrap();
    assert_eq!(tasks.len(), 1);

    assert_eq!(dst.db.list_audit_logs(Some(10), None).unwrap().len(), 1);
}

#[test]
fn import_snapshot_replace_mode_clears_target_tables() {
    let src = TempDb::new("replace-src");
    seed_requirement(&src.db, "唯一需求");
    let snapshot = src.db.export_snapshot().unwrap();

    let dst = TempDb::new("replace-dst");
    seed_requirement(&dst.db, "旧需求 A");
    seed_requirement(&dst.db, "旧需求 B");
    assert_eq!(dst.db.list_requirements(None).unwrap().len(), 2);

    dst.db.import_snapshot(&snapshot, true).unwrap();

    let requirements = dst.db.list_requirements(None).unwrap();
    assert_eq!(requirements.len(), 1, "完全恢复模式应先清空再写入");
    assert_eq!(requirements[0].title, "唯一需求");
}

// ==================== Token 成本与预算 ====================

#[test]
fn recording_tokens_updates_cost_and_budget_usage() {
    let t = TempDb::new("token");

    t.db
        .record_token_usage_ext(
            None,
            None,
            Some("claude-api"),
            Some("claude-sonnet-4"),
            500_000,
            500_000,
            0,
            0,
            0,
            9.0,
            "api",
        )
        .unwrap();

    let rows = t.db.get_token_usage(None).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].input_tokens, 500_000);
    assert_eq!(rows[0].total_cost, 9.0);
    assert_eq!(rows[0].source, "api");

    // 预算用量应被自动累计
    let budget = t.db.get_token_budget().unwrap().unwrap();
    assert_eq!(budget.current_usage, 1_000_000);

    // 成本估算：50 万输入 + 50 万输出（Sonnet 档位）= 1.5 + 7.5 = 9.0
    let estimated = t.db.estimate_cost("claude-sonnet-4", 500_000, 500_000, 0, 0, 0);
    assert!((estimated - 9.0).abs() < 1e-6, "actual={}", estimated);

    let alert = t.db.get_budget_alert().unwrap().unwrap();
    assert_eq!(alert["current_usage"], 1_000_000);
}

// ==================== 会话生命周期 ====================

#[test]
fn finish_agent_session_writes_status_duration_and_exit_code() {
    let t = TempDb::new("session-finish");
    let agent_id = seed_agent(&t.db, "finish");
    let session = t.db.create_agent_session(&agent_id, None).unwrap();
    assert_eq!(session.status, "running");

    t.db.finish_agent_session(&session.id, "completed", Some(0))
        .unwrap();

    let sessions = t.db.get_agent_sessions(None, true).unwrap();
    let s = sessions.iter().find(|s| s.id == session.id).unwrap();
    assert_eq!(s.status, "completed");
    assert_eq!(s.exit_code, Some(0));
    assert!(s.ended_at.is_some(), "ended_at 应被写入");
    assert!(s.duration_seconds.is_some(), "duration_seconds 应被写入");
}

#[test]
fn finish_agent_session_does_not_override_terminal_state() {
    let t = TempDb::new("session-terminal");
    let agent_id = seed_agent(&t.db, "terminal");
    let session = t.db.create_agent_session(&agent_id, None).unwrap();

    // 用户主动停止：置为 completed
    t.db.finish_agent_session(&session.id, "completed", Some(0))
        .unwrap();
    // 后台等待任务随后回写 failed —— 不应覆盖终态
    t.db.finish_agent_session(&session.id, "failed", Some(-1))
        .unwrap();

    let sessions = t.db.get_agent_sessions(None, true).unwrap();
    let s = sessions.iter().find(|s| s.id == session.id).unwrap();
    assert_eq!(s.status, "completed", "终态不应被后续回写覆盖");
    assert_eq!(s.exit_code, Some(0));
}

#[test]
fn reconcile_marks_stale_running_sessions_as_failed() {
    let t = TempDb::new("session-reconcile");
    let agent_id = seed_agent(&t.db, "reconcile");

    let running_a = t.db.create_agent_session(&agent_id, None).unwrap();
    let running_b = t.db.create_agent_session(&agent_id, None).unwrap();
    let done = t.db.create_agent_session(&agent_id, None).unwrap();
    t.db.finish_agent_session(&done.id, "completed", Some(0))
        .unwrap();

    let affected = t.db.reconcile_running_sessions().unwrap();
    assert_eq!(affected, 2, "应只修正残留的 running 会话");

    let sessions = t.db.get_agent_sessions(None, true).unwrap();
    for id in [&running_a.id, &running_b.id] {
        let s = sessions.iter().find(|s| &s.id == id).unwrap();
        assert_eq!(s.status, "failed");
        assert_eq!(s.exit_code, Some(-1));
        assert!(s.ended_at.is_some());
    }

    // 已完成会话不受影响
    let d = sessions.iter().find(|s| s.id == done.id).unwrap();
    assert_eq!(d.status, "completed");

    // 再次对账应无副作用
    assert_eq!(t.db.reconcile_running_sessions().unwrap(), 0);
}

// ==================== Git 提交记录 ====================

#[test]
fn git_commit_is_persisted_and_queryable_by_task() {
    let t = TempDb::new("git");
    let req_id = seed_requirement(&t.db, "提交用例");
    let task = t
        .db
        .create_task("提交任务", Some(&req_id), "development", "P1", None, None)
        .unwrap();

    t.db
        .insert_git_commit(
            Some(&task.id),
            None,
            "abcdef1234567890",
            Some("main"),
            Some("feat: 提交落库"),
            3,
        )
        .unwrap();

    let all = t.db.list_git_commits(None, None, None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0]["short_hash"], "abcdef1");
    assert_eq!(all[0]["task_id"], task.id);

    let by_task = t.db.list_git_commits(Some(&task.id), None, None).unwrap();
    assert_eq!(by_task.len(), 1);

    let other = t.db.list_git_commits(Some("not-exist"), None, None).unwrap();
    assert_eq!(other.len(), 0);
}

// ==================== MCP 执行日志与统计 ====================

#[test]
fn mcp_execution_log_feeds_usage_stats() {
    let t = TempDb::new("mcp");
    let agent_id = seed_agent(&t.db, "mcp");
    let session = t.db.create_agent_session(&agent_id, None).unwrap();

    t.db.insert_mcp_execution_log(
        &session.id,
        "context7",
        Some("https://mcp.example.com/sse"),
        Some("session_start"),
        "connected",
        None,
        Some(0),
    )
    .unwrap();

    let logs = t.db.get_mcp_execution_logs(&session.id).unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].mcp_name, "context7");

    let stats = t.db.get_mcp_usage_stats().unwrap();
    assert_eq!(stats.len(), 1, "使用统计应能读到刚写入的日志");
    assert_eq!(stats[0]["mcp_name"], "context7");
    assert_eq!(stats[0]["call_count"], 1);
    assert_eq!(stats[0]["success_count"], 1);
}

// ==================== 敏感设置加密 ====================

#[test]
fn sensitive_setting_is_encrypted_at_rest_and_decrypted_on_read() {
    let t = TempDb::new("secret");

    t.db.set_setting("claude_api_key", "sk-ant-secret-value")
        .unwrap();

    // 库中应为密文，不出现明文
    let raw = settings::get_setting(&t.db.conn, "claude_api_key")
        .unwrap()
        .unwrap();
    assert!(
        crate::crypto::is_encrypted_secret(&raw),
        "库中应存储密文，实际: {}",
        raw
    );
    assert!(!raw.contains("sk-ant-secret-value"));

    // 通过 API 读取应自动解密
    assert_eq!(
        t.db.get_setting("claude_api_key").unwrap().unwrap(),
        "sk-ant-secret-value"
    );

    // 覆盖写入后仍可正常读取
    t.db.set_setting("claude_api_key", "sk-ant-second").unwrap();
    assert_eq!(
        t.db.get_setting("claude_api_key").unwrap().unwrap(),
        "sk-ant-second"
    );

    // 非敏感项保持明文
    t.db.set_setting("session_mode", "manual").unwrap();
    let raw_mode = settings::get_setting(&t.db.conn, "session_mode")
        .unwrap()
        .unwrap();
    assert_eq!(raw_mode, "manual");
}

#[test]
fn legacy_plaintext_setting_still_readable() {
    let t = TempDb::new("secret-legacy");

    // 模拟历史数据：直接以明文写入 settings 表
    settings::set_setting(&t.db.conn, "claude_api_key", "sk-ant-legacy").unwrap();

    // 读取时明文应原样返回（向后兼容）
    assert_eq!(
        t.db.get_setting("claude_api_key").unwrap().unwrap(),
        "sk-ant-legacy"
    );
}

/// 历史"前端固定口令层"数据：读取时透明解封并以本地 keystore 重新封存。
#[test]
fn legacy_frontend_encrypted_key_is_migrated_on_read() {
    let t = TempDb::new("secret-legacy-frontend");
    let secret = *t.db.secret_key.as_ref().expect("测试库应有 keystore");

    // 模拟旧版设置页写入：先用历史口令加密，再被后端 keystore 封存
    let blob = crate::crypto::encrypt_api_key(
        "sk-ant-real-key",
        crate::crypto::LEGACY_SETTING_PASSWORD,
    )
    .unwrap();
    let sealed = crate::crypto::encrypt_secret(&blob, &secret).unwrap();
    settings::set_setting(&t.db.conn, "claude_api_key", &sealed).unwrap();

    // 读取应直接得到真实 Key（LLM 链路拿到的不再是坏 blob）
    assert_eq!(
        t.db.get_setting("claude_api_key").unwrap().unwrap(),
        "sk-ant-real-key"
    );

    // 读取后库中应已重新封存为"纯本地 keystore 密文"，历史口令层不复存在
    let raw = settings::get_setting(&t.db.conn, "claude_api_key")
        .unwrap()
        .unwrap();
    assert!(crate::crypto::is_encrypted_secret(&raw));
    let unsealed = crate::crypto::decrypt_secret(&raw, &secret).unwrap();
    assert_eq!(unsealed, "sk-ant-real-key");
}

/// running 会话计数（并发上限检查的基础）
#[test]
fn counts_only_running_sessions() {
    let t = TempDb::new("count-running");
    let agent_id = seed_agent(&t.db, "count-running");

    let running_a = t.db.create_agent_session(&agent_id, None).unwrap();
    let _running_b = t.db.create_agent_session(&agent_id, None).unwrap();
    let done = t.db.create_agent_session(&agent_id, None).unwrap();
    t.db.finish_agent_session(&done.id, "completed", Some(0)).unwrap();

    assert_eq!(t.db.count_running_sessions().unwrap(), 2);
    assert_ne!(running_a.status, "completed");
}

/// keystore 不可用时：敏感项写入必须报错，绝不落明文。
#[test]
fn sensitive_setting_write_is_refused_without_keystore() {
    let mut t = TempDb::new("secret-no-keystore");
    t.db.secret_key = None;

    let err = t.db.set_setting("claude_api_key", "sk-ant-plain");
    assert!(err.is_err(), "keystore 缺失时敏感项写入必须失败");

    // 库中不应出现明文
    let raw = settings::get_setting(&t.db.conn, "claude_api_key").unwrap();
    assert!(raw.is_none(), "拒绝写入后不应存在该设置项");

    // 读取侧：keystore 缺失时密文不可解 → 返回 None 而非密文本身
    settings::set_setting(
        &t.db.conn,
        "claude_api_key",
        "enc:v1:AAAAAAAAAAAAAAAA",
    )
    .unwrap();
    assert_eq!(t.db.get_setting("claude_api_key").unwrap(), None);
}

// ==================== 数据库备份 ====================

#[test]
fn backup_creates_file_and_prunes_old_ones() {
    let t = TempDb::new("backup");
    seed_requirement(&t.db, "备份用例");

    let db_path = t.db_path();
    assert!(db_path.exists());

    // 只保留 2 份
    let first = t.db.backup_to(&db_path, 2).unwrap();
    assert!(first.exists(), "应生成备份文件");

    // 再生成 2 份（时间戳精度为秒，稍作等待以产生不同文件名）
    for _ in 0..2 {
        std::thread::sleep(std::time::Duration::from_millis(1100));
        t.db.backup_to(&db_path, 2).unwrap();
    }

    let backups = t.db.list_backups(&t.dir);
    assert!(backups.len() <= 2, "轮转后应只保留最近 2 份，实际 {} 份", backups.len());
    assert!(!backups.is_empty());

    // 备份文件应是可用的 SQLite 库（存在且非空）
    let size = std::fs::metadata(&backups[0]).map(|m| m.len()).unwrap_or(0);
    assert!(size > 0, "备份文件不应为空（WAL 已 checkpoint）");
}

// ==================== Agent 注册 id 与会话外键 ====================

/// 回归：同一 `binary_path` 重复 upsert 必须返回**已存在记录的 id**，且不产生重复行。
///
/// 背景：`start_agent` 曾用"新造的 UUID"去建会话，而 `upsert_agent` 命中已有
/// `binary_path` 时执行的是 UPDATE（库里保留原 id），导致
/// `agent_sessions.agent_id` 外键指向不存在的行 → `FOREIGN KEY constraint failed`。
/// 该路径此前无用例覆盖（`seed_agent` 总是走 INSERT 分支），故漏检。
#[test]
fn upsert_agent_returns_effective_id_when_binary_path_exists() {
    let t = TempDb::new("agent-upsert-id");
    let existing_id = seed_agent(&t.db, "dup");

    // 模拟 start_agent：针对同一 binary_path 再次 upsert，但携带一个全新的 id
    let duplicate = Agent {
        id: uuid::Uuid::new_v4().to_string(),
        name: "claude-code".to_string(),
        display_name: "Claude Code".to_string(),
        binary_path: "/tmp/test-agent-dup".to_string(),
        version: None,
        status: "available".to_string(),
        capabilities: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let effective_id = t.db.upsert_agent(&duplicate).expect("upsert 失败");

    // 关键断言：返回的是库中已存在的 id，而不是本次新造的 id
    assert_eq!(effective_id, existing_id);
    assert_ne!(effective_id, duplicate.id);

    // 不应产生重复行
    let same_path = t
        .db
        .get_agents()
        .unwrap()
        .into_iter()
        .filter(|a| a.binary_path == duplicate.binary_path)
        .count();
    assert_eq!(same_path, 1, "同一 binary_path 应只有 1 行");

    // 用返回的 id 建会话必须成功（外键成立）
    let session = t
        .db
        .create_agent_session(&effective_id, None)
        .expect("用实际生效 id 创建会话失败");
    assert_eq!(session.agent_id, effective_id);
}

/// 未注册过的 `binary_path` 首次 upsert 走 INSERT，返回的应是新 id。
#[test]
fn upsert_agent_returns_new_id_for_new_binary_path() {
    let t = TempDb::new("agent-upsert-new");

    let agent = Agent {
        id: uuid::Uuid::new_v4().to_string(),
        name: "fresh".to_string(),
        display_name: "Fresh Agent".to_string(),
        binary_path: "/tmp/test-agent-fresh".to_string(),
        version: Some("9.9.9".to_string()),
        status: "available".to_string(),
        capabilities: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let id = t.db.upsert_agent(&agent).expect("upsert 失败");
    assert_eq!(id, agent.id);

    // 会话外键同样成立
    let session = t.db.create_agent_session(&id, None).expect("创建会话失败");
    assert_eq!(session.agent_id, id);
}


/// 会话管理：重命名 / 归档过滤 / 级联删除（P1）
#[test]
fn session_rename_archive_and_cascade_delete() {
    let t = TempDb::new("session-manage");
    let agent_id = seed_agent(&t.db, "manage");
    let session = t.db.create_agent_session(&agent_id, None).unwrap();

    // 重命名
    t.db.rename_session(&session.id, "登录页开发 · 10-21 14:30")
        .unwrap();
    let s = t.db.get_agent_session(&session.id).unwrap().unwrap();
    assert_eq!(s.title.as_deref(), Some("登录页开发 · 10-21 14:30"));

    // 归档后默认列表不可见；include_archived = true 时可见
    t.db.set_session_archived(&session.id, true).unwrap();
    assert!(t
        .db
        .get_agent_sessions(None, false)
        .unwrap()
        .iter()
        .all(|x| x.id != session.id));
    assert!(t
        .db
        .get_agent_sessions(None, true)
        .unwrap()
        .iter()
        .any(|x| x.id == session.id));

    // 取消归档后重新可见
    t.db.set_session_archived(&session.id, false).unwrap();
    assert!(t
        .db
        .get_agent_sessions(None, false)
        .unwrap()
        .iter()
        .any(|x| x.id == session.id));

    // 级联删除：会话本体与其消息一并清理
    t.db.append_session_message(
        &session.id,
        &crate::db::session_messages::MessageContext::default(),
        crate::db::session_messages::BlockSpec::new("user", "text", Some("hi".to_string())),
    )
    .unwrap();
    assert_eq!(t.db.count_session_messages(&session.id).unwrap(), 1);

    t.db.delete_session_cascade(&session.id).unwrap();
    assert!(t.db.get_agent_session(&session.id).unwrap().is_none());
    assert_eq!(t.db.count_session_messages(&session.id).unwrap(), 0);
}
