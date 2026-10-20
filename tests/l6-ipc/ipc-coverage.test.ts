import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import * as tauri from '../../src/lib/tauri';

const mockInvoke = vi.mocked(invoke);

/**
 * L6 IPC Coverage - 覆盖全部 Tauri IPC 命令
 *
 * 与 `src-tauri/src/main.rs` 的 generate_handler! 注册表对齐，共 83 条命令:
 * - 需求管理: 8 个        - 任务管理: 8 个
 * - 任务评论: 3 个        - Agent 管理: 7 个
 * - Token 追踪: 7 个      - PRD Skill: 6 个
 * - 项目管理: 5 个        - 项目 Skill/MCP: 6 个
 * - 流水线环节: 5 个      - 执行日志与统计: 4 个
 * - PTY 终端: 6 个        - Git: 5 个
 * - 数据导入导出: 2 个    - 缓存: 2 个
 * - 审计: 1 个            - 系统监控: 2 个
 * - 统计/设置/加密: 5 个
 *
 * 用例编号 L6-001 ~ L6-089，覆盖率 100%（新增 L6-089 save_session_attachment）。
 * 说明：本层断言的是"前端封装 → 命令名与参数"的契约（mock IPC）；
 * 后端注册完整性与真实链路由 `src-tauri/src/db/integration_tests.rs`（默认执行）
 * 与 `src-tauri/tests/integration.rs`（Victauri 门控）分别保证。
 */

describe('L6 IPC Coverage: 全部 IPC 命令覆盖', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // 需求管理 (6个)
  describe('需求管理 IPC', () => {
    it('L6-001: create_requirement', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.createRequirement('t');
      expect(mockInvoke).toHaveBeenCalledWith('create_requirement', expect.any(Object));
    });
    it('L6-002: list_requirements', async () => {
      mockInvoke.mockResolvedValue({ items: [] });
      await tauri.listRequirements();
      expect(mockInvoke).toHaveBeenCalledWith('list_requirements', expect.any(Object));
    });
    it('L6-003: get_requirement', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.getRequirement('1');
      expect(mockInvoke).toHaveBeenCalledWith('get_requirement', { id: '1' });
    });
    it('L6-004: update_requirement', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.updateRequirement('1', { title: 'new' });
      expect(mockInvoke).toHaveBeenCalledWith('update_requirement', expect.objectContaining({ id: '1' }));
    });
    it('L6-005: delete_requirement', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.deleteRequirement('1');
      expect(mockInvoke).toHaveBeenCalledWith('delete_requirement', { id: '1' });
    });
    it('L6-006: generate_prd', async () => {
      mockInvoke.mockResolvedValue('prd content');
      await tauri.generatePrd('1', 'skill-1');
      expect(mockInvoke).toHaveBeenCalledWith('generate_prd', expect.any(Object));
    });
  });

  // 任务管理 (8个)
  describe('任务管理 IPC', () => {
    it('L6-007: create_task', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.createTask('t', 'req-1', 'development', 'P1', undefined, undefined, undefined, undefined, undefined, undefined, '备注', '手填描述');
      expect(mockInvoke).toHaveBeenCalledWith(
        'create_task',
        expect.objectContaining({ title: 't', requirementId: 'req-1', notes: '备注', description: '手填描述' })
      );
    });
    it('L6-008: list_tasks', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listTasks();
      expect(mockInvoke).toHaveBeenCalledWith('list_tasks', expect.any(Object));
    });
    it('L6-009: get_task', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.getTask('1');
      expect(mockInvoke).toHaveBeenCalledWith('get_task', { id: '1' });
    });
    it('L6-010: update_task_status', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.updateTaskStatus('1', 'done');
      expect(mockInvoke).toHaveBeenCalledWith('update_task_status', expect.any(Object));
    });
    it('L6-011: update_task', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.updateTask('1', { title: 'new' });
      expect(mockInvoke).toHaveBeenCalledWith('update_task', expect.objectContaining({ id: '1' }));
    });
    it('L6-012: delete_task', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.deleteTask('1');
      expect(mockInvoke).toHaveBeenCalledWith('delete_task', { id: '1' });
    });
    it('L6-013: list_tasks_by_project', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listTasksByProject('proj-1');
      expect(mockInvoke).toHaveBeenCalledWith('list_tasks_by_project', expect.any(Object));
    });
    it('L6-014: list_tasks_by_requirement', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listTasksByRequirement('req-1');
      expect(mockInvoke).toHaveBeenCalledWith('list_tasks_by_requirement', expect.any(Object));
    });
  });

  // 任务评论 (3个)
  describe('任务评论 IPC', () => {
    it('L6-015: create_task_comment', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.createTaskComment('task-1', 'author', 'content');
      expect(mockInvoke).toHaveBeenCalledWith('create_task_comment', expect.any(Object));
    });
    it('L6-016: list_task_comments', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listTaskComments('task-1');
      expect(mockInvoke).toHaveBeenCalledWith('list_task_comments', { taskId: 'task-1' });
    });
    it('L6-017: delete_task_comment', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.deleteTaskComment('1');
      expect(mockInvoke).toHaveBeenCalledWith('delete_task_comment', { id: '1' });
    });
  });

  // Agent管理 (5个)
  describe('Agent管理 IPC', () => {
    it('L6-018: discover_agents', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.discoverAgents();
      expect(mockInvoke).toHaveBeenCalledWith('discover_agents');
    });
    it('L6-019: get_agents', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getAgents();
      expect(mockInvoke).toHaveBeenCalledWith('get_agents');
    });
    it('L6-020: start_agent', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.startAgent('/path/to/agent');
      expect(mockInvoke).toHaveBeenCalledWith('start_agent', expect.any(Object));
    });
    it('L6-021: create_session_from_task', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.createSessionFromTask('task-1');
      expect(mockInvoke).toHaveBeenCalledWith('create_session_from_task', expect.any(Object));
    });
    it('L6-022: stop_agent', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.stopAgent('session-1');
      expect(mockInvoke).toHaveBeenCalledWith('stop_agent', { sessionId: 'session-1' });
    });
  });

  // Token追踪 (6个)
  describe('Token追踪 IPC', () => {
    it('L6-023: get_token_usage_summary', async () => {
      mockInvoke.mockResolvedValue({ total_tokens: 0 });
      await tauri.getTokenUsageSummary();
      expect(mockInvoke).toHaveBeenCalledWith('get_token_usage_summary');
    });
    it('L6-024: get_token_usage_trend', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getTokenUsageTrend(7);
      expect(mockInvoke).toHaveBeenCalledWith('get_token_usage_trend', { days: 7 });
    });
    it('L6-025: get_token_usage_by_model', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getTokenUsageByModel();
      expect(mockInvoke).toHaveBeenCalledWith('get_token_usage_by_model');
    });
    it('L6-026: get_token_budget', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.getTokenBudget();
      expect(mockInvoke).toHaveBeenCalledWith('get_token_budget');
    });
    it('L6-027: set_token_budget', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.setTokenBudget(1000, 80);
      expect(mockInvoke).toHaveBeenCalledWith('set_token_budget', expect.any(Object));
    });
    it('L6-028: sync_local_usage', async () => {
      mockInvoke.mockResolvedValue(0);
      await tauri.syncLocalUsage();
      expect(mockInvoke).toHaveBeenCalledWith('sync_local_usage');
    });
  });

  // PRD Skill (6个)
  describe('PRD Skill IPC', () => {
    it('L6-029: list_prd_skills', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listPrdSkills();
      expect(mockInvoke).toHaveBeenCalledWith('list_prd_skills');
    });
    it('L6-030: get_prd_skill', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.getPrdSkill('1');
      expect(mockInvoke).toHaveBeenCalledWith('get_prd_skill', { id: '1' });
    });
    it('L6-031: create_prd_skill', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.createPrdSkill('name', 'type', 'desc', 'template');
      expect(mockInvoke).toHaveBeenCalledWith('create_prd_skill', expect.any(Object));
    });
    it('L6-032: update_prd_skill', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.updatePrdSkill('1', 'name', 'desc', 'template');
      expect(mockInvoke).toHaveBeenCalledWith('update_prd_skill', expect.any(Object));
    });
    it('L6-033: delete_prd_skill', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.deletePrdSkill('1');
      expect(mockInvoke).toHaveBeenCalledWith('delete_prd_skill', { id: '1' });
    });
    it('L6-034: create_custom_skill_from_builtin', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.createCustomSkillFromBuiltin('parent-1', 'name', 'template');
      expect(mockInvoke).toHaveBeenCalledWith('create_custom_skill_from_builtin', expect.any(Object));
    });
  });

  // 统计 (1个)
  describe('统计 IPC', () => {
    it('L6-035: get_statistics', async () => {
      mockInvoke.mockResolvedValue({});
      await tauri.getStatistics();
      expect(mockInvoke).toHaveBeenCalledWith('get_statistics');
    });
  });

  // 设置 (2个)
  describe('设置 IPC', () => {
    it('L6-036: get_setting', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.getSetting('key');
      expect(mockInvoke).toHaveBeenCalledWith('get_setting', { key: 'key' });
    });
    it('L6-037: set_setting', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.setSetting('key', 'value');
      expect(mockInvoke).toHaveBeenCalledWith('set_setting', expect.any(Object));
    });
  });

  // 加密 (2个)
  describe('加密 IPC', () => {
    it('L6-038: encrypt_api_key', async () => {
      mockInvoke.mockResolvedValue('encrypted');
      await tauri.encryptApiKey('key', 'pass');
      expect(mockInvoke).toHaveBeenCalledWith('encrypt_api_key', expect.any(Object));
    });
    it('L6-039: decrypt_api_key', async () => {
      mockInvoke.mockResolvedValue('key');
      await tauri.decryptApiKey('encrypted', 'pass');
      expect(mockInvoke).toHaveBeenCalledWith('decrypt_api_key', expect.any(Object));
    });
  });

  // Git (3个)
  describe('Git IPC', () => {
    it('L6-040: git_log', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.gitLog('/path');
      expect(mockInvoke).toHaveBeenCalledWith('git_log', expect.any(Object));
    });
    it('L6-041: git_get_status', async () => {
      mockInvoke.mockResolvedValue({ branch: 'main', files: [], total_changes: 0 });
      await tauri.gitGetStatus('/path');
      expect(mockInvoke).toHaveBeenCalledWith('git_get_status', expect.any(Object));
    });
    it('L6-042: git_get_diff', async () => {
      mockInvoke.mockResolvedValue('');
      await tauri.gitGetDiff('/path');
      expect(mockInvoke).toHaveBeenCalledWith('git_get_diff', expect.any(Object));
    });
  });

  // 项目管理 (5个)
  describe('项目管理 IPC', () => {
    it('L6-043: create_project', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.createProject('name');
      expect(mockInvoke).toHaveBeenCalledWith('create_project', expect.any(Object));
    });
    it('L6-044: list_projects', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listProjects();
      expect(mockInvoke).toHaveBeenCalledWith('list_projects');
    });
    it('L6-045: get_project', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.getProject('1');
      expect(mockInvoke).toHaveBeenCalledWith('get_project', { id: '1' });
    });
    it('L6-046: update_project', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.updateProject('1', { name: 'new' });
      expect(mockInvoke).toHaveBeenCalledWith('update_project', expect.objectContaining({ id: '1' }));
    });
    it('L6-047: delete_project', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.deleteProject('1');
      expect(mockInvoke).toHaveBeenCalledWith('delete_project', { id: '1' });
    });
  });

  // 项目 Skill/MCP (6个)
  describe('项目配置 IPC', () => {
    it('L6-048: list_project_skills', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listProjectSkills('proj-1');
      expect(mockInvoke).toHaveBeenCalledWith('list_project_skills', expect.any(Object));
    });
    it('L6-049: add_project_skill', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.addProjectSkill('proj-1', 'skill-1');
      expect(mockInvoke).toHaveBeenCalledWith('add_project_skill', expect.any(Object));
    });
    it('L6-050: remove_project_skill', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.removeProjectSkill('1');
      expect(mockInvoke).toHaveBeenCalledWith('remove_project_skill', { id: '1' });
    });
    it('L6-051: list_project_mcps', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listProjectMcps('proj-1');
      expect(mockInvoke).toHaveBeenCalledWith('list_project_mcps', expect.any(Object));
    });
    it('L6-052: add_project_mcp', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.addProjectMcp('proj-1', 'name', 'endpoint');
      expect(mockInvoke).toHaveBeenCalledWith('add_project_mcp', expect.any(Object));
    });
    it('L6-053: remove_project_mcp', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.removeProjectMcp('1');
      expect(mockInvoke).toHaveBeenCalledWith('remove_project_mcp', { id: '1' });
    });
  });

  // 执行日志 (2个)
  describe('执行日志 IPC', () => {
    it('L6-054: get_skill_execution_logs', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getSkillExecutionLogs();
      expect(mockInvoke).toHaveBeenCalledWith('get_skill_execution_logs', expect.any(Object));
    });
    it('L6-055: get_mcp_execution_logs', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getMcpExecutionLogs('session-1');
      expect(mockInvoke).toHaveBeenCalledWith('get_mcp_execution_logs', expect.any(Object));
    });
  });

  // 需求补充 (1个)
  describe('需求补充 IPC', () => {
    it('L6-056: enrich_requirement', async () => {
      mockInvoke.mockResolvedValue({ module: '认证模块' });
      await tauri.enrichRequirement('req-1');
      expect(mockInvoke).toHaveBeenCalledWith('enrich_requirement', { requirementId: 'req-1' });
    });
  });

  // Agent 会话 (1个)
  describe('Agent 会话 IPC', () => {
    it('L6-057: get_agent_sessions', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getAgentSessions();
      expect(mockInvoke).toHaveBeenCalledWith('get_agent_sessions', expect.any(Object));
    });
  });

  // Token 明细 (1个)
  describe('Token 明细 IPC', () => {
    it('L6-058: get_token_usage', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getTokenUsage('task-1');
      expect(mockInvoke).toHaveBeenCalledWith('get_token_usage', { taskId: 'task-1' });
    });
  });

  // 流水线环节配置 (5个)
  describe('流水线环节配置 IPC', () => {
    it('L6-059: get_pipeline_stages', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getPipelineStages();
      expect(mockInvoke).toHaveBeenCalledWith('get_pipeline_stages');
    });
    it('L6-060: list_stage_skill_configs', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listStageSkillConfigs('proj-1');
      expect(mockInvoke).toHaveBeenCalledWith('list_stage_skill_configs', { projectId: 'proj-1' });
    });
    it('L6-061: get_stage_skill_config', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.getStageSkillConfig('proj-1', 'prd_generation');
      expect(mockInvoke).toHaveBeenCalledWith('get_stage_skill_config', {
        projectId: 'proj-1',
        stage: 'prd_generation',
      });
    });
    it('L6-062: upsert_stage_skill_config', async () => {
      mockInvoke.mockResolvedValue({ id: '1' });
      await tauri.upsertStageSkillConfig('proj-1', 'prd_generation', 'skill-1', true);
      expect(mockInvoke).toHaveBeenCalledWith('upsert_stage_skill_config', expect.any(Object));
    });
    it('L6-063: remove_stage_skill_config', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.removeStageSkillConfig('cfg-1');
      expect(mockInvoke).toHaveBeenCalledWith('remove_stage_skill_config', { id: 'cfg-1' });
    });
  });

  // 使用统计 (2个)
  describe('使用统计 IPC', () => {
    it('L6-064: get_skill_usage_stats', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getSkillUsageStats();
      expect(mockInvoke).toHaveBeenCalledWith('get_skill_usage_stats');
    });
    it('L6-065: get_mcp_usage_stats', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getMcpUsageStats();
      expect(mockInvoke).toHaveBeenCalledWith('get_mcp_usage_stats');
    });
  });

  // PTY 终端 (6个)
  describe('PTY 终端 IPC', () => {
    it('L6-066: pty_spawn', async () => {
      mockInvoke.mockResolvedValue('pty-1');
      await tauri.ptySpawn('/usr/bin/claude', '/tmp');
      expect(mockInvoke).toHaveBeenCalledWith('pty_spawn', expect.any(Object));
    });
    it('L6-067: pty_write', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.ptyWrite('pty-1', 'ls\r');
      expect(mockInvoke).toHaveBeenCalledWith('pty_write', { sessionId: 'pty-1', input: 'ls\r' });
    });
    it('L6-068: pty_resize', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.ptyResize('pty-1', 30, 100);
      expect(mockInvoke).toHaveBeenCalledWith('pty_resize', {
        sessionId: 'pty-1',
        rows: 30,
        cols: 100,
      });
    });
    it('L6-069: pty_kill', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.ptyKill('pty-1');
      expect(mockInvoke).toHaveBeenCalledWith('pty_kill', { sessionId: 'pty-1' });
    });
    it('L6-070: pty_get_sessions', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.ptyGetSessions();
      expect(mockInvoke).toHaveBeenCalledWith('pty_get_sessions');
    });
    it('L6-071: pty_get_session', async () => {
      mockInvoke.mockResolvedValue(null);
      await tauri.ptyGetSession('pty-1');
      expect(mockInvoke).toHaveBeenCalledWith('pty_get_session', { sessionId: 'pty-1' });
    });
  });

  // Git 提交（含落库）(2个)
  describe('Git 提交 IPC', () => {
    it('L6-072: git_create_commit', async () => {
      mockInvoke.mockResolvedValue({ commit_hash: 'abc1234' });
      await tauri.gitCreateCommit('/repo', 'feat: 优化登录', 'task-1', 'session-1');
      expect(mockInvoke).toHaveBeenCalledWith('git_create_commit', expect.any(Object));
    });
    it('L6-073: list_git_commits', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.listGitCommits('task-1');
      expect(mockInvoke).toHaveBeenCalledWith('list_git_commits', expect.any(Object));
    });
  });

  // 缓存 (2个)
  describe('缓存 IPC', () => {
    it('L6-074: get_cache_stats', async () => {
      mockInvoke.mockResolvedValue({ hits: 0, misses: 0, hit_rate: '0.00%', size: 0, enabled: true });
      await tauri.getCacheStats();
      expect(mockInvoke).toHaveBeenCalledWith('get_cache_stats');
    });
    it('L6-075: clear_cache', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.clearCache();
      expect(mockInvoke).toHaveBeenCalledWith('clear_cache');
    });
  });

  // 审计日志 (1个)
  describe('审计日志 IPC', () => {
    it('L6-076: get_audit_logs', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.getAuditLogs(200, 'requirement');
      expect(mockInvoke).toHaveBeenCalledWith('get_audit_logs', {
        limit: 200,
        entityType: 'requirement',
      });
    });
  });

  // 系统监控 (2个)
  describe('系统监控 IPC', () => {
    it('L6-077: get_monitor_data', async () => {
      mockInvoke.mockResolvedValue({ cpu_usage: 0, memory_used_mb: 0, memory_usage_percent: 0, uptime_seconds: 0, active_sessions: 0 });
      await tauri.getMonitorData();
      expect(mockInvoke).toHaveBeenCalledWith('get_monitor_data');
    });
    it('L6-078: check_alerts', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.checkAlerts();
      expect(mockInvoke).toHaveBeenCalledWith('check_alerts');
    });
  });

  // 数据导入导出 (2个)
  describe('数据导入导出 IPC', () => {
    it('L6-079: export_data', async () => {
      mockInvoke.mockResolvedValue({ format: 'ace-export', version: '2.0', tables: {} });
      await tauri.exportData();
      expect(mockInvoke).toHaveBeenCalledWith('export_data');
    });
    it('L6-080: import_data', async () => {
      mockInvoke.mockResolvedValue({ dry_run: true });
      await tauri.importData('{"format":"ace-export"}', false, true);
      expect(mockInvoke).toHaveBeenCalledWith('import_data', expect.any(Object));
    });
  });

  // 模块 16：会话交互与消息 (6个)
  describe('会话交互与消息 IPC', () => {
    it('L6-083: start_structured_session', async () => {
      mockInvoke.mockResolvedValue({ id: 'sess-1', mode: 'structured' });
      await tauri.startStructuredSession({ agentPath: '/usr/bin/claude', taskId: 'task-1' });
      expect(mockInvoke).toHaveBeenCalledWith('start_structured_session', {
        agentPath: '/usr/bin/claude',
        taskId: 'task-1',
      });
    });
    it('L6-084: send_session_message', async () => {
      mockInvoke.mockResolvedValue({ id: 'msg-1', seq: 1 });
      await tauri.sendSessionMessage('sess-1', '列出当前目录');
      expect(mockInvoke).toHaveBeenCalledWith('send_session_message', {
        sessionId: 'sess-1',
        content: '列出当前目录',
      });
    });
    it('L6-085: list_session_messages', async () => {
      mockInvoke.mockResolvedValue({ items: [], total: 0, page: 1, page_size: 50, total_pages: 0 });
      await tauri.listSessionMessages('sess-1', 10, 50);
      expect(mockInvoke).toHaveBeenCalledWith('list_session_messages', {
        sessionId: 'sess-1',
        beforeSeq: 10,
        limit: 50,
      });
    });
    it('L6-086: interrupt_session', async () => {
      mockInvoke.mockResolvedValue(undefined);
      await tauri.interruptSession('sess-1');
      expect(mockInvoke).toHaveBeenCalledWith('interrupt_session', { sessionId: 'sess-1' });
    });
    it('L6-087: set_session_mode（允许并重试）', async () => {
      mockInvoke.mockResolvedValue({ id: 'sess-1' });
      await tauri.setSessionMode({
        sessionId: 'sess-1',
        mode: 'structured',
        allowedTools: ['Write'],
        resendLast: true,
      });
      expect(mockInvoke).toHaveBeenCalledWith('set_session_mode', {
        sessionId: 'sess-1',
        mode: 'structured',
        allowedTools: ['Write'],
        resendLast: true,
      });
    });
    it('L6-088: resume_session', async () => {
      mockInvoke.mockResolvedValue({ id: 'sess-1' });
      await tauri.resumeSession('sess-1');
      expect(mockInvoke).toHaveBeenCalledWith('resume_session', { sessionId: 'sess-1' });
    });
    it('L6-089: save_session_attachment', async () => {
      mockInvoke.mockResolvedValue({ name: 'shot.png', path: 'C:\\attachments\\shot.png', size: 3 });
      await tauri.saveSessionAttachment('sess-1', 'shot.png', [1, 2, 3]);
      expect(mockInvoke).toHaveBeenCalledWith('save_session_attachment', {
        sessionId: 'sess-1',
        fileName: 'shot.png',
        contents: [1, 2, 3],
      });
    });
  });

  // 第四轮新增命令 (2个)
  describe('Agent 状态治理 / 流式 PRD IPC', () => {
    it('L6-081: verify_agents', async () => {
      mockInvoke.mockResolvedValue([]);
      await tauri.verifyAgents();
      expect(mockInvoke).toHaveBeenCalledWith('verify_agents');
    });
    it('L6-082: generate_prd_streaming', async () => {
      mockInvoke.mockResolvedValue('# PRD');
      await tauri.generatePrdStreaming('req-1', 'skill-feature');
      expect(mockInvoke).toHaveBeenCalledWith('generate_prd_streaming', {
        requirementId: 'req-1',
        skillId: 'skill-feature',
      });
    });
  });
});
