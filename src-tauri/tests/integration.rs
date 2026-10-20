//! Integration tests — auto-generated from discovered #[tauri::command] functions.
//!
//! Run with: VICTAURI_E2E=1 cargo test --test integration

use victauri_test::VictauriClient;

fn skip_unless_e2e() -> bool {
    if !victauri_test::is_e2e() {
        eprintln!("Skipping: set VICTAURI_E2E=1 with your Tauri dev server running");
        return true;
    }
    false
}

// ── Health check ─────────────────────────────────────────────────────────

#[tokio::test]
async fn full_stack_health_check() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await
        .expect("Failed to connect — is your Tauri dev server running?");

    let report = client.verify()
        .ipc_healthy()
        .no_console_errors()
        .run()
        .await
        .unwrap();

    report.assert_all_passed();
}

// ── Command tests ────────────────────────────────────────────────────────
// Generated from discovered #[tauri::command] functions.
// Adapt each test to match your command's expected arguments and behavior.

#[tokio::test]
async fn command_add_project_mcp() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("add_project_mcp", None).await;
    assert!(
        result.is_ok(),
        "add_project_mcp should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_add_project_skill() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("add_project_skill", None).await;
    assert!(
        result.is_ok(),
        "add_project_skill should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_check_alerts() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("check_alerts", None).await;
    assert!(
        result.is_ok(),
        "check_alerts should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_clear_cache() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("clear_cache", None).await;
    assert!(
        result.is_ok(),
        "clear_cache should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_create_custom_skill_from_builtin() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("create_custom_skill_from_builtin", None).await;
    assert!(
        result.is_ok(),
        "create_custom_skill_from_builtin should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_create_prd_skill() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("create_prd_skill", None).await;
    assert!(
        result.is_ok(),
        "create_prd_skill should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_create_project() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("create_project", None).await;
    assert!(
        result.is_ok(),
        "create_project should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_create_requirement() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("create_requirement", None).await;
    assert!(
        result.is_ok(),
        "create_requirement should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_create_session_from_task() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("create_session_from_task", None).await;
    assert!(
        result.is_ok(),
        "create_session_from_task should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_create_task() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("create_task", None).await;
    assert!(
        result.is_ok(),
        "create_task should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_create_task_comment() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("create_task_comment", None).await;
    assert!(
        result.is_ok(),
        "create_task_comment should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_decrypt_api_key() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("decrypt_api_key", None).await;
    assert!(
        result.is_ok(),
        "decrypt_api_key should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_delete_prd_skill() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("delete_prd_skill", None).await;
    assert!(
        result.is_ok(),
        "delete_prd_skill should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_delete_project() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("delete_project", None).await;
    assert!(
        result.is_ok(),
        "delete_project should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_delete_requirement() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("delete_requirement", None).await;
    assert!(
        result.is_ok(),
        "delete_requirement should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_delete_task() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("delete_task", None).await;
    assert!(
        result.is_ok(),
        "delete_task should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_delete_task_comment() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("delete_task_comment", None).await;
    assert!(
        result.is_ok(),
        "delete_task_comment should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_discover_agents() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("discover_agents", None).await;
    assert!(
        result.is_ok(),
        "discover_agents should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_encrypt_api_key() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("encrypt_api_key", None).await;
    assert!(
        result.is_ok(),
        "encrypt_api_key should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_generate_prd() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("generate_prd", None).await;
    assert!(
        result.is_ok(),
        "generate_prd should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_agent_sessions() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_agent_sessions", None).await;
    assert!(
        result.is_ok(),
        "get_agent_sessions should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_agents() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_agents", None).await;
    assert!(
        result.is_ok(),
        "get_agents should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_cache_stats() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_cache_stats", None).await;
    assert!(
        result.is_ok(),
        "get_cache_stats should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_mcp_execution_logs() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_mcp_execution_logs", None).await;
    assert!(
        result.is_ok(),
        "get_mcp_execution_logs should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_mcp_usage_stats() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_mcp_usage_stats", None).await;
    assert!(
        result.is_ok(),
        "get_mcp_usage_stats should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_monitor_data() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_monitor_data", None).await;
    assert!(
        result.is_ok(),
        "get_monitor_data should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_prd_skill() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_prd_skill", None).await;
    assert!(
        result.is_ok(),
        "get_prd_skill should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_project() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_project", None).await;
    assert!(
        result.is_ok(),
        "get_project should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_requirement() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_requirement", None).await;
    assert!(
        result.is_ok(),
        "get_requirement should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_setting() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_setting", None).await;
    assert!(
        result.is_ok(),
        "get_setting should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_skill_execution_logs() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_skill_execution_logs", None).await;
    assert!(
        result.is_ok(),
        "get_skill_execution_logs should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_skill_usage_stats() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_skill_usage_stats", None).await;
    assert!(
        result.is_ok(),
        "get_skill_usage_stats should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_statistics() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_statistics", None).await;
    assert!(
        result.is_ok(),
        "get_statistics should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_task() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_task", None).await;
    assert!(
        result.is_ok(),
        "get_task should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_token_budget() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_token_budget", None).await;
    assert!(
        result.is_ok(),
        "get_token_budget should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_token_usage() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_token_usage", None).await;
    assert!(
        result.is_ok(),
        "get_token_usage should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_token_usage_by_model() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_token_usage_by_model", None).await;
    assert!(
        result.is_ok(),
        "get_token_usage_by_model should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_token_usage_summary() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_token_usage_summary", None).await;
    assert!(
        result.is_ok(),
        "get_token_usage_summary should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_get_token_usage_trend() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("get_token_usage_trend", None).await;
    assert!(
        result.is_ok(),
        "get_token_usage_trend should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_git_create_commit() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("git_create_commit", None).await;
    assert!(
        result.is_ok(),
        "git_create_commit should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_git_get_diff() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("git_get_diff", None).await;
    assert!(
        result.is_ok(),
        "git_get_diff should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_git_get_status() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("git_get_status", None).await;
    assert!(
        result.is_ok(),
        "git_get_status should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_git_log() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("git_log", None).await;
    assert!(
        result.is_ok(),
        "git_log should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_prd_skills() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_prd_skills", None).await;
    assert!(
        result.is_ok(),
        "list_prd_skills should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_project_mcps() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_project_mcps", None).await;
    assert!(
        result.is_ok(),
        "list_project_mcps should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_project_skills() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_project_skills", None).await;
    assert!(
        result.is_ok(),
        "list_project_skills should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_projects() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_projects", None).await;
    assert!(
        result.is_ok(),
        "list_projects should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_requirements() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_requirements", None).await;
    assert!(
        result.is_ok(),
        "list_requirements should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_task_comments() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_task_comments", None).await;
    assert!(
        result.is_ok(),
        "list_task_comments should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_tasks() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_tasks", None).await;
    assert!(
        result.is_ok(),
        "list_tasks should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_tasks_by_project() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_tasks_by_project", None).await;
    assert!(
        result.is_ok(),
        "list_tasks_by_project should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_list_tasks_by_requirement() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("list_tasks_by_requirement", None).await;
    assert!(
        result.is_ok(),
        "list_tasks_by_requirement should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_pty_get_session() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("pty_get_session", None).await;
    assert!(
        result.is_ok(),
        "pty_get_session should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_pty_get_sessions() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("pty_get_sessions", None).await;
    assert!(
        result.is_ok(),
        "pty_get_sessions should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_pty_kill() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("pty_kill", None).await;
    assert!(
        result.is_ok(),
        "pty_kill should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_pty_resize() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("pty_resize", None).await;
    assert!(
        result.is_ok(),
        "pty_resize should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_pty_spawn() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("pty_spawn", None).await;
    assert!(
        result.is_ok(),
        "pty_spawn should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_pty_write() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("pty_write", None).await;
    assert!(
        result.is_ok(),
        "pty_write should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_remove_project_mcp() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("remove_project_mcp", None).await;
    assert!(
        result.is_ok(),
        "remove_project_mcp should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_remove_project_skill() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("remove_project_skill", None).await;
    assert!(
        result.is_ok(),
        "remove_project_skill should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_set_setting() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("set_setting", None).await;
    assert!(
        result.is_ok(),
        "set_setting should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_set_token_budget() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("set_token_budget", None).await;
    assert!(
        result.is_ok(),
        "set_token_budget should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_start_agent() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("start_agent", None).await;
    assert!(
        result.is_ok(),
        "start_agent should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_stop_agent() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("stop_agent", None).await;
    assert!(
        result.is_ok(),
        "stop_agent should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_sync_local_usage() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("sync_local_usage", None).await;
    assert!(
        result.is_ok(),
        "sync_local_usage should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_update_prd_skill() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("update_prd_skill", None).await;
    assert!(
        result.is_ok(),
        "update_prd_skill should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_update_project() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("update_project", None).await;
    assert!(
        result.is_ok(),
        "update_project should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_update_requirement() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("update_requirement", None).await;
    assert!(
        result.is_ok(),
        "update_requirement should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_update_task() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("update_task", None).await;
    assert!(
        result.is_ok(),
        "update_task should respond without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn command_update_task_status() {
    if skip_unless_e2e() { return; }
    let mut client = VictauriClient::discover().await.unwrap();

    let result = client.invoke_command("update_task_status", None).await;
    assert!(
        result.is_ok(),
        "update_task_status should respond without error: {:?}",
        result.err()
    );
}

