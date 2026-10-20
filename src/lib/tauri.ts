import { invoke as tauriInvoke } from "@tauri-apps/api/core";

/** 后端结构化错误（`AppError` 序列化形态）附加在 Error 上的机器可读码 */
export interface AppErrorInfo extends Error {
  code?: string;
}

/**
 * 规整后端错误为带 `code` 的 Error：
 * - 结构化对象 `{ code, message }`（P1-3 起所有命令错误）→ `Error.message` + `Error.code`；
 * - 历史字符串错误 / 异常值 → 兼容包装，保持 `error.message` 可读。
 * 页面侧 `catch` 无需感知差异；需要分支处理时读 `error.code`
 * （`validation` / `business` / `not_found` / `database` / `external` / `crypto` / `legacy`）。
 */
export function normalizeInvokeError(e: unknown): AppErrorInfo {
  if (e && typeof e === "object" && "message" in e) {
    const obj = e as { code?: unknown; message?: unknown };
    const err = new Error(String(obj.message)) as AppErrorInfo;
    if (typeof obj.code === "string") {
      err.code = obj.code;
    }
    return err;
  }
  if (e instanceof Error) {
    return e;
  }
  return new Error(String(e));
}

/** 统一 IPC 出口：所有 Tauri 调用经此透传，错误在边界处结构化 */
async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    // 零参命令不传第二参数（保持历史调用形态，兼容 mock 断言）
    return args === undefined ? await tauriInvoke<T>(cmd) : await tauriInvoke<T>(cmd, args);
  } catch (e) {
    throw normalizeInvokeError(e);
  }
}
import type {
  Requirement,
  Task,
  TaskComment,
  Agent,
  AgentSession,
  SessionMessage,
  TokenBudget,
  PrdSkill,
  TokenUsageSummary,
  DailyUsage,
  ModelUsage,
  StatisticsData,
  Project,
  ProjectSkill,
  ProjectMcp,
  SkillExecutionLog,
  McpExecutionLog,
  CacheStats,
  SkillScore,
  SkillSuggestion,
  SkillVersion,
  SkillOptimizeDraft,
} from "./types";

// ==================== 需求管理 ====================

export async function createRequirement(
  title: string,
  description?: string,
  priority: string = "P1",
  projectId?: string,
  module?: string,
  source?: string,
  sourceDetail?: string,
  productOwner?: string,
  developer?: string,
  tester?: string,
  businessPriority?: string,
  estimatedEffort?: number,
  tags?: string,
  notes?: string,
): Promise<Requirement> {
  return invoke("create_requirement", {
    title,
    description,
    priority,
    projectId,
    module,
    source,
    sourceDetail,
    productOwner,
    developer,
    tester,
    businessPriority,
    estimatedEffort,
    tags,
    notes,
  });
}

export async function listRequirements(status?: string): Promise<Requirement[]> {
  const result: any = await invoke("list_requirements", { status });
  // 后端返回 PagedResult<Requirement>，提取 items 数组
  return result.items || result;
}

export async function getRequirement(id: string): Promise<Requirement | null> {
  return invoke("get_requirement", { id });
}

export async function updateRequirement(
  id: string,
  updates: {
    title?: string;
    description?: string;
    priority?: string;
    status?: string;
    /** 对应后端 prd_content（Tauri 参数为 camelCase） */
    prdContent?: string;
    module?: string;
    source?: string;
    sourceDetail?: string;
    productOwner?: string;
    developer?: string;
    tester?: string;
    businessPriority?: string;
    estimatedEffort?: number;
    tags?: string;
    notes?: string;
  },
): Promise<void> {
  return invoke("update_requirement", { id, ...updates });
}

export async function deleteRequirement(id: string): Promise<void> {
  return invoke("delete_requirement", { id });
}

export async function generatePrd(requirementId: string, skillId?: string): Promise<string> {
  return invoke("generate_prd", { requirementId, skillId });
}

export async function enrichRequirement(
  requirementId: string,
  extraContent?: string,
): Promise<Record<string, unknown>> {
  return invoke("enrich_requirement", { requirementId, extraContent });
}

/**
 * 流式生成 PRD（F049）。
 *
 * 调用前需先 `listen("prd:chunk")`（载荷 `{ requirement_id, delta }`）以接收增量文本；
 * 生成结束后后端会推送 `prd:done`（载荷 `{ requirement_id, content }`）。
 * 本函数返回值即最终完整内容。
 * `extraContent`：需求补充内容（用户生成前输入，与需求描述、项目工作区上下文一并注入）。
 */
export async function generatePrdStreaming(
  requirementId: string,
  skillId?: string,
  extraContent?: string,
): Promise<string> {
  return invoke("generate_prd_streaming", { requirementId, skillId, extraContent });
}

export interface SessionAttachment {
  name: string;
  path: string;
  size: number;
}

/**
 * 保存会话附件（图片粘贴 / 文件导入）到应用数据目录，返回保存路径。
 * 路径随后注入消息文本或终端输入行，由 Agent 读取识别。
 */
export async function saveSessionAttachment(
  sessionId: string | null,
  fileName: string,
  contents: number[],
): Promise<SessionAttachment> {
  return invoke("save_session_attachment", { sessionId, fileName, contents });
}

// ==================== 任务管理 ====================

export async function createTask(
  title: string,
  requirementId?: string,
  taskType: string = "development",
  priority: string = "P1",
  assignee?: string,
  dueDate?: string,
  estimatedHours?: number,
  priorityLevel?: string,
  blockedReason?: string,
  tags?: string,
  notes?: string,
  description?: string,
): Promise<Task> {
  return invoke("create_task", {
    title,
    requirementId,
    taskType,
    priority,
    assignee,
    dueDate,
    estimatedHours,
    priorityLevel,
    blockedReason,
    tags,
    notes,
    description,
  });
}

export async function listTasks(status?: string, taskType?: string): Promise<Task[]> {
  return invoke("list_tasks", { status, taskType });
}

export async function updateTaskStatus(id: string, status: string): Promise<void> {
  return invoke("update_task_status", { id, status });
}

export async function deleteTask(id: string): Promise<void> {
  return invoke("delete_task", { id });
}

export async function getTask(id: string): Promise<Task | null> {
  return invoke("get_task", { id });
}

export async function updateTask(
  id: string,
  updates: {
    title?: string;
    description?: string;
    task_type?: string;
    priority?: string;
    assignee?: string;
    due_date?: string;
    tags?: string;
    project_id?: string;
  },
): Promise<void> {
  // Tauri 命令参数按 camelCase 匹配；同时携带 snake_case 键以兼容历史行为
  return invoke("update_task", {
    id,
    ...updates,
    taskType: updates.task_type,
    dueDate: updates.due_date,
    projectId: updates.project_id,
  });
}

export async function listTasksByProject(projectId: string): Promise<Task[]> {
  return invoke("list_tasks_by_project", { projectId });
}

export async function listTasksByRequirement(requirementId: string): Promise<Task[]> {
  return invoke("list_tasks_by_requirement", { requirementId });
}

// ==================== 任务评论 ====================

export async function createTaskComment(taskId: string, author: string, content: string): Promise<TaskComment> {
  return invoke("create_task_comment", { taskId, author, content });
}

export async function listTaskComments(taskId: string): Promise<TaskComment[]> {
  return invoke("list_task_comments", { taskId });
}

export async function deleteTaskComment(id: string): Promise<void> {
  return invoke("delete_task_comment", { id });
}

// ==================== Agent管理 ====================

export async function discoverAgents(): Promise<Agent[]> {
  return invoke("discover_agents");
}

/**
 * Agent 状态治理：进程存活对账 + 二进制有效性复核，返回复核后的列表（F021）。
 */
export async function verifyAgents(): Promise<Agent[]> {
  return invoke("verify_agents");
}

// ==================== 会话交互与消息（模块 16） ====================

/** 会话消息分页结果 */
export interface SessionMessagePage {
  items: SessionMessage[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

/**
 * 以**结构化（消息）模式**启动会话（F077）。
 *
 * 与 `startAgent`（终端模式）并列：本命令启动 `stream-json` 双向流进程，
 * 支持指令下发、结构化回复、思考内容与工具调用展示。
 */
export async function startStructuredSession(params: {
  agentPath: string;
  taskId?: string;
  requirementId?: string;
  workDir?: string;
  permissionMode?: string;
  model?: string;
}): Promise<AgentSession> {
  return invoke("start_structured_session", params);
}

/** 下发一条用户指令（F077） */
export async function sendSessionMessage(sessionId: string, content: string): Promise<SessionMessage> {
  return invoke("send_session_message", { sessionId, content });
}

/** 分页读取会话消息（F076 / F085） */
export async function listSessionMessages(
  sessionId: string,
  beforeSeq?: number,
  limit?: number,
): Promise<SessionMessagePage> {
  return invoke("list_session_messages", { sessionId, beforeSeq, limit });
}

/** 中断当前执行（F086） */
export async function interruptSession(sessionId: string): Promise<void> {
  return invoke("interrupt_session", { sessionId });
}

/**
 * 以新模式/新权限重建会话（F081 / F083）。
 *
 * `allowedTools` 会并入会话既有白名单，用于被拒卡片的"允许并重试"；
 * `resendLast = true` 时自动重发上一条用户指令。
 */
export async function setSessionMode(params: {
  sessionId: string;
  mode: string;
  permissionMode?: string;
  model?: string;
  allowedTools?: string[];
  resendLast?: boolean;
}): Promise<AgentSession> {
  return invoke("set_session_mode", params);
}

/** 续接既有 CLI 会话（F082） */
export async function resumeSession(sessionId: string): Promise<AgentSession> {
  return invoke("resume_session", { sessionId });
}

export async function getAgents(): Promise<Agent[]> {
  return invoke("get_agents");
}

export async function startAgent(agentPath: string, taskId?: string): Promise<AgentSession> {
  return invoke("start_agent", { agentPath, taskId });
}

export async function createSessionFromTask(taskId: string): Promise<AgentSession> {
  return invoke("create_session_from_task", { taskId });
}

/**
 * 关联/取消关联会话与开发任务。
 *
 * `taskId` 为 null 时解除关联；关联成功后会话列表的「任务」列（task_title）随之刷新，
 * 且会话上下文卡片/系统提示中的任务、需求链路同步更新。
 */
export async function linkSessionTask(sessionId: string, taskId: string | null): Promise<void> {
  return invoke("link_session_task", {
    sessionId,
    session_id: sessionId,
    taskId,
    task_id: taskId,
  });
}

export async function stopAgent(sessionId: string): Promise<void> {
  return invoke("stop_agent", { sessionId });
}

export async function getAgentSessions(agentId?: string): Promise<AgentSession[]> {
  return invoke("get_agent_sessions", { agentId });
}

// ==================== Token追踪 ====================

export async function getTokenUsageSummary(): Promise<TokenUsageSummary> {
  return invoke("get_token_usage_summary");
}

export async function getTokenUsageTrend(days: number): Promise<DailyUsage[]> {
  return invoke("get_token_usage_trend", { days });
}

export async function getTokenUsageByModel(): Promise<ModelUsage[]> {
  return invoke("get_token_usage_by_model");
}

export async function getTokenBudget(): Promise<TokenBudget | null> {
  return invoke("get_token_budget");
}

export async function setTokenBudget(monthlyLimit: number, alertThreshold: number): Promise<void> {
  return invoke("set_token_budget", { monthlyLimit, alertThreshold });
}

export async function syncLocalUsage(): Promise<number> {
  return invoke("sync_local_usage");
}

// ==================== PRD Skill ====================

export async function listPrdSkills(): Promise<PrdSkill[]> {
  return invoke("list_prd_skills");
}

export async function getPrdSkill(id: string): Promise<PrdSkill | null> {
  return invoke("get_prd_skill", { id });
}

export async function createPrdSkill(
  name: string,
  skillType: string,
  description: string | null,
  template: string,
): Promise<PrdSkill> {
  return invoke("create_prd_skill", { name, skillType, description, template });
}

export async function updatePrdSkill(
  id: string,
  name: string | null,
  description: string | null,
  template: string | null,
): Promise<void> {
  return invoke("update_prd_skill", { id, name, description, template });
}

export async function deletePrdSkill(id: string): Promise<void> {
  return invoke("delete_prd_skill", { id });
}

export async function createCustomSkillFromBuiltin(
  parentId: string,
  name: string,
  template: string | null,
): Promise<PrdSkill> {
  return invoke("create_custom_skill_from_builtin", { parentId, name, template });
}

/** 导出 Skill 为可分享的 JSON 定义（模板分享） */
export async function exportSkillDefinition(id: string): Promise<string> {
  return invoke("export_skill_definition", { id });
}

/** 从 JSON 定义导入为自定义 Skill（名称冲突自动追加序号） */
export async function importSkillDefinition(json: string): Promise<PrdSkill> {
  return invoke("import_skill_definition", { json });
}

// ==================== Skill 自进化 ====================

/** L1 诊断：跑规则 + 静态体检 + 振荡检测，建议落库并返回全部建议 */
export async function analyzeSkills(): Promise<SkillSuggestion[]> {
  return invoke("analyze_skills");
}

export async function listSkillSuggestions(skillId?: string): Promise<SkillSuggestion[]> {
  return invoke("list_skill_suggestions", { skillId });
}

/** luban-skill 模板四段解析 */
export async function parseSkillTemplate(
  skillId?: string,
  template?: string,
): Promise<{
  sections: Record<string, string>;
  missing: string[];
  warnings: string[];
}> {
  return invoke("parse_skill_template", { skillId, template });
}

/** 评分（结构分静态 + 效果分 Judge）+ 棘轮判定 */
export async function judgeSkill(skillId: string): Promise<SkillScore> {
  return invoke("judge_skill", { skillId });
}

export async function getSkillScoreHistory(skillId: string): Promise<SkillScore[]> {
  return invoke("get_skill_score_history", { skillId });
}

/** L2 优化器：返回草稿（不落库，需 diff 确认后 apply） */
export async function optimizeSkill(skillId: string): Promise<SkillOptimizeDraft> {
  return invoke("optimize_skill", { skillId });
}

/** 应用优化草稿（人在回路确认入口，version+1） */
export async function applySkillSuggestion(suggestionId: string, newTemplate?: string): Promise<PrdSkill> {
  return invoke("apply_skill_suggestion", { suggestionId, newTemplate });
}

/** 应用 AI 优化草稿（无关联建议，version+1） */
export async function applySkillDraft(skillId: string, newTemplate: string): Promise<PrdSkill> {
  return invoke("apply_skill_draft", { skillId, newTemplate });
}

export async function listSkillVersions(skillId: string): Promise<SkillVersion[]> {
  return invoke("list_skill_versions", { skillId });
}

/** 棘轮回退：应用指定版本快照（写为新版本） */
export async function revertSkillVersion(skillId: string, version: number): Promise<PrdSkill> {
  return invoke("revert_skill_version", { skillId, version });
}

/** 按 Skill 查询执行日志（进化视图日志 Tab） */
export async function getSkillExecutionLogsBySkill(skillId: string, limit?: number): Promise<SkillExecutionLog[]> {
  return invoke("get_skill_execution_logs_by_skill", { skillId, limit });
}

// ==================== 统计 ====================

export async function getStatistics(): Promise<StatisticsData> {
  return invoke("get_statistics");
}

// ==================== 设置 ====================

export async function getSetting(key: string): Promise<string | null> {
  return invoke("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke("set_setting", { key, value });
}

// ==================== 加密 ====================

export async function encryptApiKey(apiKey: string, password: string): Promise<string> {
  return invoke("encrypt_api_key", { apiKey, password });
}

export async function decryptApiKey(encrypted: string, password: string): Promise<string> {
  return invoke("decrypt_api_key", { encrypted, password });
}

// ==================== Git ====================

export async function gitLog(
  workDir: string,
  limit?: number,
): Promise<
  Array<{
    hash: string;
    short_hash: string;
    message: string;
    author: string;
    date: string;
  }>
> {
  return invoke("git_log", { workDir, limit });
}

export async function gitGetStatus(
  workDir: string,
): Promise<{ branch: string; files: Array<{ status: string; path: string }>; total_changes: number }> {
  return invoke("git_get_status", { workDir });
}

export async function gitGetDiff(workDir: string, filePath?: string): Promise<string> {
  return invoke("git_get_diff", { workDir, filePath });
}

export async function gitCreateCommit(
  workDir: string,
  message: string,
  taskId?: string,
  sessionId?: string,
): Promise<any> {
  return invoke("git_create_commit", { workDir, message, taskId, sessionId });
}

// ==================== 项目管理 ====================

export async function createProject(name: string, description?: string, key?: string): Promise<Project> {
  return invoke("create_project", { name, description, key });
}

export async function listProjects(): Promise<Project[]> {
  return invoke("list_projects");
}

export async function getProject(id: string): Promise<Project | null> {
  return invoke("get_project", { id });
}

export async function updateProject(
  id: string,
  updates: {
    name?: string;
    description?: string;
    key?: string;
    status?: string;
  },
): Promise<void> {
  return invoke("update_project", { id, ...updates });
}

export async function deleteProject(id: string): Promise<void> {
  return invoke("delete_project", { id });
}

// ==================== 项目 Skill 配置 ====================

export async function listProjectSkills(projectId: string): Promise<ProjectSkill[]> {
  return invoke("list_project_skills", { projectId });
}

export async function addProjectSkill(
  projectId: string,
  skillId: string,
  isDefault?: boolean,
  customParams?: string,
): Promise<ProjectSkill> {
  return invoke("add_project_skill", {
    projectId,
    skillId,
    isDefault,
    customParams,
  });
}

export async function removeProjectSkill(id: string): Promise<void> {
  return invoke("remove_project_skill", { id });
}

// ==================== 项目 MCP 配置 ====================

export async function listProjectMcps(projectId: string): Promise<ProjectMcp[]> {
  return invoke("list_project_mcps", { projectId });
}

export async function addProjectMcp(
  projectId: string,
  mcpName: string,
  endpoint: string,
  description?: string,
  isDefault?: boolean,
  authConfig?: string,
  customParams?: string,
): Promise<ProjectMcp> {
  return invoke("add_project_mcp", {
    projectId,
    mcpName,
    endpoint,
    description,
    isDefault,
    authConfig,
    customParams,
  });
}

export async function removeProjectMcp(id: string): Promise<void> {
  return invoke("remove_project_mcp", { id });
}

// ==================== 环节 Skill 配置 ====================

export interface StageSkillConfig {
  id: string;
  project_id: string;
  stage: string;
  skill_id: string;
  is_default: boolean;
  custom_params?: string;
  created_at: string;
}

export interface PipelineStageOption {
  value: string;
  label: string;
}

export async function listStageSkillConfigs(projectId: string): Promise<StageSkillConfig[]> {
  return invoke("list_stage_skill_configs", { projectId });
}

export async function getStageSkillConfig(projectId: string, stage: string): Promise<StageSkillConfig | null> {
  return invoke("get_stage_skill_config", { projectId, stage });
}

export async function upsertStageSkillConfig(
  projectId: string,
  stage: string,
  skillId: string,
  isDefault?: boolean,
  customParams?: string,
): Promise<StageSkillConfig> {
  return invoke("upsert_stage_skill_config", {
    projectId,
    stage,
    skillId,
    isDefault,
    customParams,
  });
}

export async function removeStageSkillConfig(id: string): Promise<void> {
  return invoke("remove_stage_skill_config", { id });
}

export async function getPipelineStages(): Promise<PipelineStageOption[]> {
  return invoke("get_pipeline_stages");
}

// ==================== Skill/MCP 执行日志 ====================

export async function getSkillExecutionLogs(taskId?: string, sessionId?: string): Promise<SkillExecutionLog[]> {
  return invoke("get_skill_execution_logs", { taskId, sessionId });
}

export async function getMcpExecutionLogs(sessionId: string): Promise<McpExecutionLog[]> {
  return invoke("get_mcp_execution_logs", { sessionId });
}

// ==================== 审计日志 ====================

export interface AuditLogEntry {
  id: string;
  entity_type: string;
  entity_id: string;
  action: string;
  old_value?: string;
  new_value?: string;
  changes: string;
  created_at: string;
  entity_title?: string;
}

export async function getAuditLogs(limit?: number, entityType?: string): Promise<AuditLogEntry[]> {
  return invoke("get_audit_logs", { limit, entityType });
}

// ==================== Git 提交记录（平台落库） ====================

export interface GitCommitRecord {
  id: string;
  task_id?: string | null;
  session_id?: string | null;
  commit_hash: string;
  short_hash: string;
  branch_name?: string | null;
  message?: string | null;
  files_changed: number;
  created_at: string;
}

export async function listGitCommits(taskId?: string, sessionId?: string, limit?: number): Promise<GitCommitRecord[]> {
  return invoke("list_git_commits", { taskId, sessionId, limit });
}

// ==================== 数据导入导出 ====================

export interface ImportResult {
  dry_run: boolean;
  version?: string;
  replace?: boolean;
  total_imported?: number;
  imported?: Record<string, number>;
  after?: Record<string, number>;
  incoming?: Record<string, number>;
  current?: Record<string, number>;
  total_incoming?: number;
}

export async function exportData(): Promise<Record<string, unknown>> {
  return invoke("export_data");
}

export async function importData(payload: string, replace = false, dryRun = false): Promise<ImportResult> {
  return invoke("import_data", { payload, replace, dryRun });
}

// ==================== 缓存统计 ====================

export async function getCacheStats(): Promise<CacheStats> {
  return invoke("get_cache_stats");
}

export async function clearCache(): Promise<void> {
  return invoke("clear_cache");
}

// ==================== Skill / MCP 使用统计 ====================

export interface SkillUsageStat {
  /** 按 skill_id 聚合（重名/改名不失真） */
  skill_id: string;
  skill_name: string;
  skill_type?: string | null;
  use_count: number;
  total_tokens: number;
  success_count: number;
  fail_count: number;
  avg_duration_ms?: number | null;
  last_used_at?: string | null;
}

export interface McpUsageStat {
  mcp_name: string;
  call_count: number;
  success_count: number;
  avg_duration?: number | null;
}

export async function getSkillUsageStats(): Promise<SkillUsageStat[]> {
  return invoke("get_skill_usage_stats");
}

export async function getMcpUsageStats(): Promise<McpUsageStat[]> {
  return invoke("get_mcp_usage_stats");
}

// ==================== PTY 终端 ====================

export interface PtySessionInfo {
  id: string;
  agent_path: string;
  work_dir: string;
  status: string;
  started_at: string;
  exit_code?: number | null;
}

export async function ptySpawn(agentPath: string, workDir: string, envVars?: [string, string][]): Promise<string> {
  return invoke("pty_spawn", { agentPath, workDir, envVars });
}

export async function ptyWrite(sessionId: string, input: string): Promise<void> {
  return invoke("pty_write", { sessionId, input });
}

export async function ptyResize(sessionId: string, rows: number, cols: number): Promise<void> {
  return invoke("pty_resize", { sessionId, rows, cols });
}

export async function ptyKill(sessionId: string): Promise<void> {
  return invoke("pty_kill", { sessionId });
}

export async function ptyGetSessions(): Promise<PtySessionInfo[]> {
  return invoke("pty_get_sessions");
}

export async function ptyGetSession(sessionId: string): Promise<PtySessionInfo | null> {
  return invoke("pty_get_session", { sessionId });
}

export interface PtyAttachInfo {
  pty_id: string;
  replay: string;
}

/** 附着终端会话：按数据库会话 id 解析真实 PTY id，并回放挂载前的历史输出 */
export async function ptyAttach(dbSessionId: string): Promise<PtyAttachInfo> {
  return invoke("pty_attach", { dbSessionId });
}

/** 为终端会话重启 Agent 进程（进程已退出/应用重启后恢复可交互终端） */
export async function ptyReconnect(dbSessionId: string): Promise<string> {
  return invoke("pty_reconnect", { dbSessionId });
}

// ==================== Token 明细 ====================

export async function getTokenUsage(taskId?: string): Promise<unknown[]> {
  return invoke("get_token_usage", { taskId });
}

// ==================== 系统监控 ====================

export interface MonitorData {
  cpu_usage: number;
  memory_used_mb: number;
  memory_usage_percent: number;
  uptime_seconds: number;
  active_sessions: number;
}

export interface AlertItem {
  level: string;
  message: string;
}

export async function getMonitorData(): Promise<MonitorData> {
  return invoke("get_monitor_data");
}

export async function checkAlerts(): Promise<AlertItem[]> {
  return invoke("check_alerts");
}
