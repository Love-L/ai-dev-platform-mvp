// ==================== 数据模型 ====================

export interface Requirement {
  id: string;
  project_id: string;
  title: string;
  description?: string;
  priority: "P0" | "P1" | "P2" | "P3";
  status: "draft" | "reviewing" | "approved" | "rejected" | "frozen" | "in_development" | "testing" | "completed";
  prd_content?: string;
  requirement_type?: "feature" | "bug" | "optimization" | "tech_debt";
  target_version?: string;
  due_date?: string;
  tags?: string;
  // 新增字段（Phase 1）
  module?: string;
  source?: string;
  source_detail?: string;
  product_owner?: string;
  developer?: string;
  tester?: string;
  business_priority?: string;
  estimated_effort?: number;
  actual_effort?: number;
  attachments?: string;
  notes?: string;
  created_at: string;
  updated_at: string;
}

export interface Task {
  id: string;
  requirement_id?: string;
  title: string;
  description?: string;
  task_type: "requirement" | "development" | "testing" | "bug_fix" | "review";
  status: "pending" | "in_progress" | "completed" | "blocked";
  priority: "P0" | "P1" | "P2" | "P3";
  agent_type?: string;
  token_usage: number;
  assignee?: string;
  due_date?: string;
  tags?: string;
  estimated_tokens?: number;
  // 新增字段（Phase 1）
  estimated_hours?: number;
  actual_hours?: number;
  blocked_reason?: string;
  related_requirement_id?: string;
  related_project_id?: string;
  priority_level?: string;
  notes?: string;
  attachments?: string;
  project_id?: string;
  skill_config?: string;
  mcp_config?: string;
  created_at: string;
  updated_at: string;
}

export interface TaskComment {
  id: string;
  task_id: string;
  author: string;
  content: string;
  created_at: string;
  updated_at: string;
}

export interface Agent {
  id: string;
  name: string;
  display_name: string;
  binary_path: string;
  version?: string;
  status: "available" | "running" | "unavailable";
  capabilities?: string;
  created_at: string;
}

/** Agent 能力（`agents.capabilities` JSON 解析后结构，多 Agent 适配 MA-01/MA-02） */
export interface AgentCapabilityInfo {
  flavor?: string;
  structured_events?: boolean;
  one_shot?: boolean;
  mcp_support?: boolean;
  worktree_support?: boolean;
  interactive?: boolean;
}

/** 安全解析 Agent 能力 JSON */
export function parseAgentCapabilities(agent: { capabilities?: string }): AgentCapabilityInfo {
  try {
    const parsed = JSON.parse(agent.capabilities || "{}");
    return typeof parsed === "object" && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

/** 能力等级标签：🟢 结构化（L3）/ 🔵 一次性（L2）/ ⚪ 仅终端（L1） */
export function agentCapabilityLabel(info: AgentCapabilityInfo): string {
  if (info.structured_events) return "结构化";
  if (info.one_shot) return "一次性";
  return "仅终端";
}

export interface AgentSession {
  id: string;
  agent_id: string;
  agent_name?: string;
  task_id?: string;
  task_title?: string;
  status: "created" | "launching" | "running" | "completed" | "failed" | "terminated";
  started_at: string;
  ended_at?: string;
  exit_code?: number;
  work_dir?: string;
  project_id?: string;
  source?: "native" | "imported" | "manual_import";
  source_path?: string;
  title?: string;
  description?: string;
  related_task_id?: string;
  related_requirement_id?: string;
  skills?: string;
  mcps?: string;
  duration_seconds?: number;
  input_tokens?: number;
  output_tokens?: number;
  total_tokens?: number;
  total_cost?: number;
  /** 归档标记（1 = 已归档，默认列表隐藏） */
  archived?: number;
  // ==== 模块 16：会话交互与消息 ====
  /** CLI 侧会话 ID（用于 `--resume` 续接） */
  cli_session_id?: string;
  /** 会话模式：structured = 消息模式，terminal = 终端模式 */
  mode?: "structured" | "terminal";
  /** 上次使用的视图 */
  view?: "messages" | "terminal";
  /** 启动使用的二进制路径 */
  agent_path?: string;
  /** 权限预设（映射 --permission-mode） */
  permission_mode?: string;
  /** 会话级模型 */
  model?: string;
  /** "始终允许"工具白名单（JSON 数组字符串） */
  permission_allowlist?: string;
  /** 最后一次事件时间 */
  last_event_at?: string;
}

/** 会话消息块类型 */
export type SessionBlockType =
  "text" | "thinking" | "tool_use" | "tool_result" | "result" | "context" | "permission_denied" | "error" | "system";

/** 会话消息块（模块 16 / F076） */
export interface SessionMessage {
  id: string;
  session_id: string;
  /** 会话内单调递增序号，回放顺序的唯一依据 */
  seq: number;
  role: "user" | "assistant" | "system";
  block_type: SessionBlockType;
  content?: string;
  tool_name?: string;
  tool_use_id?: string;
  parent_tool_use_id?: string;
  cli_message_id?: string;
  raw_json?: string;
  tokens?: number;
  status: "streaming" | "completed" | "failed" | "denied";
  related_task_id?: string;
  related_requirement_id?: string;
  project_id?: string;
  created_at: string;
}

/** `session:delta` 事件载荷（增量，不落库） */
export interface SessionDelta {
  session_id: string;
  block_type: "text" | "thinking" | "thinking_tokens";
  delta: string;
  total?: number;
}

/** `session:notice` 事件载荷 */
export interface SessionNotice {
  session_id: string;
  level: "info" | "warn" | "error";
  kind: string;
  message: string;
  tool_name?: string;
}

export interface TokenBudget {
  id: string;
  scope: string;
  monthly_limit: number;
  alert_threshold: number;
  /** 当月已用量（后端自动累计，跨月重置） */
  current_usage: number;
  reset_date?: string;
  created_at: string;
  /** 后端派生的用量比例（0~1） */
  usage_ratio?: number;
  /** 后端派生的超阈值标记 */
  exceeded?: boolean;
}

export interface PrdSkill {
  id: string;
  name: string;
  description?: string;
  skill_type: string;
  template: string;
  is_builtin: boolean;
  version?: number;
  parent_id?: string;
  is_deleted?: boolean;
  created_at: string;
}

// ==================== 项目管理 ====================

export interface Project {
  id: string;
  name: string;
  description?: string;
  key?: string;
  status?: "active" | "paused" | "archived";
  owner_id?: string;
  start_date?: string;
  end_date?: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectSkill {
  id: string;
  project_id: string;
  skill_id: string;
  is_default: boolean;
  custom_params?: string;
  created_at: string;
  // 关联的 Skill 信息（从 prd_skills 表来）
  skill_name?: string;
  skill_type?: string;
  skill_description?: string;
}

export interface ProjectMcp {
  id: string;
  project_id: string;
  mcp_name: string;
  endpoint: string;
  description?: string;
  is_default: boolean;
  auth_config?: string;
  custom_params?: string;
  created_at: string;
}

// ==================== Skill/MCP 执行日志 ====================

export interface SkillExecutionLog {
  id: string;
  task_id?: string;
  session_id?: string;
  requirement_id?: string;
  skill_id: string;
  skill_name: string;
  skill_type?: string;
  execution_phase?: "prd_generation" | "task_execution" | "session_start";
  input_params?: string;
  output_summary?: string;
  token_usage: number;
  status: "success" | "failed" | "skipped";
  executed_at: string;
  /** 执行耗时（毫秒） */
  duration_ms?: number | null;
  /** 失败原因 */
  error_message?: string | null;
}

// ==================== Skill 自进化 ====================

/** 评分维度明细（结构分 6 个维度） */
export interface SkillScoreDimension {
  key: string;
  label: string;
  score: number;
  max: number;
  evidence?: string | null;
}

/** Skill 评分记录：总分 = 结构分(60) + 效果分(40)；effect_score 为 null 表示仅结构分 */
export interface SkillScore {
  id: string;
  skill_id: string;
  version: number;
  structure_score: number;
  effect_score?: number | null;
  total_score: number;
  /** SkillScoreDimension[] 的 JSON 字符串 */
  dimensions_json: string;
  judge_reason?: string | null;
  judged_at: string;
}

/** Skill 优化建议 */
export interface SkillSuggestion {
  id: string;
  skill_id: string;
  rule_type:
    | "low_success"
    | "high_token"
    | "unused"
    | "slow"
    | "struct_missing"
    | "dead_ref"
    | "stale_ref"
    | "soft_words"
    | "long_template"
    | "sentinel"
    | "ratchet_revert"
    | "oscillation"
    | "ai_optimize"
    | string;
  severity: "HIGH" | "MEDIUM" | "LOW" | string;
  message: string;
  dimension?: string | null;
  proposed_template?: string | null;
  status: "pending" | "applied" | "rejected" | "verified" | string;
  applied_version?: number | null;
  created_at: string;
}

/** Skill 模板版本快照 */
export interface SkillVersion {
  id: string;
  skill_id: string;
  version: number;
  template: string;
  note?: string | null;
  created_at: string;
}

/** L2 优化器草稿 */
export interface SkillOptimizeDraft {
  new_template: string;
  change_summary: string;
  target_dimensions: string[];
  input_tokens?: number;
  output_tokens?: number;
}

export interface McpExecutionLog {
  id: string;
  session_id: string;
  mcp_name: string;
  endpoint?: string;
  action?: string;
  status: "connected" | "disconnected" | "error" | "call_success" | "call_failed";
  error_message?: string;
  duration_ms?: number;
  executed_at: string;
}

// ==================== 统计数据模型 ====================

export interface TokenUsageSummary {
  total_tokens: number;
  total_cost: number;
  input_tokens: number;
  output_tokens: number;
  cache_tokens: number;
  thinking_tokens: number;
  record_count: number;
}

export interface DailyUsage {
  date: string;
  total_tokens: number;
  total_cost: number;
  input_tokens: number;
  output_tokens: number;
}

export interface ModelUsage {
  model: string;
  total_tokens: number;
  total_cost: number;
  record_count: number;
}

export interface StatisticsData {
  tasks: {
    total: number;
    completed: number;
    in_progress: number;
    pending: number;
    by_type: {
      development: number;
      testing: number;
      bug_fix: number;
    };
  };
  requirements: {
    total: number;
  };
  tokens: {
    total: number;
    cost: number;
    input: number;
    output: number;
    /** 近 7 天每日用量（正序，含补零日期） */
    by_day?: { date: string; tokens: number; cost: number }[];
  };
  /** 运行中的会话数（工作台态势指示） */
  sessions?: { running: number };
}

export interface CacheStats {
  hits: number;
  misses: number;
  hit_rate: string;
  size: number;
  enabled: boolean;
}

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
