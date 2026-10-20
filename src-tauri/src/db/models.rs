use serde::{Deserialize, Serialize};

// ==================== 数据模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: String,
    pub status: String,
    pub prd_content: Option<String>,
    #[serde(default)]
    pub requirement_type: Option<String>,
    #[serde(default)]
    pub target_version: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    // 新增字段（Phase 1）
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_detail: Option<String>,
    #[serde(default)]
    pub product_owner: Option<String>,
    #[serde(default)]
    pub developer: Option<String>,
    #[serde(default)]
    pub tester: Option<String>,
    #[serde(default)]
    pub business_priority: Option<String>,
    #[serde(default)]
    pub estimated_effort: Option<f64>,
    #[serde(default)]
    pub actual_effort: Option<f64>,
    #[serde(default)]
    pub attachments: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub requirement_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub task_type: String,
    pub status: String,
    pub priority: String,
    pub agent_type: Option<String>,
    pub token_usage: i64,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub estimated_tokens: Option<i64>,
    // 新增字段（Phase 1）
    #[serde(default)]
    pub estimated_hours: Option<f64>,
    #[serde(default)]
    pub actual_hours: Option<f64>,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    #[serde(default)]
    pub related_requirement_id: Option<String>,
    #[serde(default)]
    pub related_project_id: Option<String>,
    #[serde(default)]
    pub priority_level: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub attachments: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub skill_config: Option<String>,
    #[serde(default)]
    pub mcp_config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub binary_path: String,
    pub version: Option<String>,
    pub status: String,
    pub capabilities: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub agent_id: String,
    pub task_id: Option<String>,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub total_tokens: Option<i64>,
    #[serde(default)]
    pub total_cost: Option<f64>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub related_task_id: Option<String>,
    #[serde(default)]
    pub related_requirement_id: Option<String>,
    #[serde(default)]
    pub skills: Option<String>,
    #[serde(default)]
    pub mcps: Option<String>,
    #[serde(default)]
    pub skill_execution_summary: Option<String>,
    #[serde(default)]
    pub mcp_execution_summary: Option<String>,
    // ==== 模块 16：会话交互与消息（F076–F086） ====
    /// CLI 侧会话 ID（由 `--session-id` 指定或从 `system.init` 回填），用于续接
    #[serde(default)]
    pub cli_session_id: Option<String>,
    /// 会话工作目录（长期缺失字段；结构化模式下作为子进程 cwd 与 `--add-dir`）
    #[serde(default)]
    pub work_dir: Option<String>,
    /// 启动会话使用的二进制路径（便于重启后按同参数重建）
    #[serde(default)]
    pub agent_path: Option<String>,
    /// 会话模式：`structured`（消息模式）/ `terminal`（终端模式）
    #[serde(default)]
    pub mode: Option<String>,
    /// 上次使用的视图：`messages` / `terminal`
    #[serde(default)]
    pub view: Option<String>,
    /// 权限预设（映射 `--permission-mode`）
    #[serde(default)]
    pub permission_mode: Option<String>,
    /// 会话级模型（映射 `--model`）
    #[serde(default)]
    pub model: Option<String>,
    /// 会话级工具白名单（JSON 数组，映射 `--allowedTools`）
    #[serde(default)]
    pub permission_allowlist: Option<String>,
    /// 最后一次收到 Agent 事件的时间（用于"无响应"检测）
    #[serde(default)]
    pub last_event_at: Option<String>,
    /// Agent 显示名（查询时 JOIN `agents` 表回填，非本表字段）
    #[serde(default)]
    pub agent_name: Option<String>,
    /// 任务标题（查询时 JOIN `tasks` 表回填，非本表字段）
    #[serde(default)]
    pub task_title: Option<String>,
}

/// 会话消息块（F076）。
///
/// 一个"块"是展示与存储的最小单元：正文、思考、工具调用、工具结果、单轮结果、
/// 上下文卡片、权限被拒、错误、系统提示。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: String,
    pub session_id: String,
    /// 会话内单调递增序号，回放顺序的唯一依据
    pub seq: i64,
    /// `user` / `assistant` / `system`
    pub role: String,
    /// `text` / `thinking` / `tool_use` / `tool_result` / `result` / `context` /
    /// `permission_denied` / `error` / `system`
    pub block_type: String,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    /// 子代理消息归属（配合 `--forward-subagent-text`）
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
    /// CLI 事件 id，用于幂等去重
    #[serde(default)]
    pub cli_message_id: Option<String>,
    /// 原始事件（按 `session_keep_raw_json` 决定是否保留）
    #[serde(default)]
    pub raw_json: Option<String>,
    #[serde(default)]
    pub tokens: Option<i64>,
    /// `streaming` / `completed` / `failed` / `denied`
    pub status: String,
    #[serde(default)]
    pub related_task_id: Option<String>,
    #[serde(default)]
    pub related_requirement_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    pub created_at: String,
}

/// 消息块类型常量（避免调用方手写字符串）
pub mod block_type {
    pub const TEXT: &str = "text";
    pub const THINKING: &str = "thinking";
    pub const TOOL_USE: &str = "tool_use";
    pub const TOOL_RESULT: &str = "tool_result";
    pub const RESULT: &str = "result";
    pub const CONTEXT: &str = "context";
    pub const PERMISSION_DENIED: &str = "permission_denied";
    pub const ERROR: &str = "error";
    pub const SYSTEM: &str = "system";
}

/// 会话模式常量
pub mod session_mode {
    pub const STRUCTURED: &str = "structured";
    /// 终端模式（由 `start_agent` 的 PTY 链路使用；后端仅作常量登记，前端用于视图切换）
    #[allow(dead_code)]
    pub const TERMINAL: &str = "terminal";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub id: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_type: Option<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    #[serde(default)]
    pub cache_creation_tokens: i64,
    #[serde(default)]
    pub cache_read_tokens: i64,
    #[serde(default)]
    pub thinking_tokens: i64,
    pub total_cost: f64,
    #[serde(default)]
    pub source: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub id: String,
    pub scope: String,
    pub monthly_limit: i64,
    pub alert_threshold: f64,
    pub current_usage: i64,
    pub reset_date: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrdSkill {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub skill_type: String,
    pub template: String,
    pub is_builtin: bool,
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub is_deleted: bool,
    pub created_at: String,
}

// ==================== 项目 Skill/MCP 配置 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSkill {
    pub id: String,
    pub project_id: String,
    pub skill_id: String,
    pub is_default: bool,
    pub custom_params: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMcp {
    pub id: String,
    pub project_id: String,
    pub mcp_name: String,
    pub endpoint: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub auth_config: Option<String>,
    pub custom_params: Option<String>,
    pub created_at: String,
}

// ==================== 环节 Skill 配置 ====================

/// 流水线环节枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    /// 需求分析
    RequirementAnalysis,
    /// PRD 生成
    PrdGeneration,
    /// 任务拆分
    TaskPlanning,
    /// 代码生成
    CodeGeneration,
    /// 测试
    Testing,
    /// 评审
    Review,
}

impl PipelineStage {
    /// 所有环节（按顺序）
    pub fn all() -> Vec<PipelineStage> {
        vec![
            PipelineStage::RequirementAnalysis,
            PipelineStage::PrdGeneration,
            PipelineStage::TaskPlanning,
            PipelineStage::CodeGeneration,
            PipelineStage::Testing,
            PipelineStage::Review,
        ]
    }

    /// 转为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelineStage::RequirementAnalysis => "requirement_analysis",
            PipelineStage::PrdGeneration => "prd_generation",
            PipelineStage::TaskPlanning => "task_planning",
            PipelineStage::CodeGeneration => "code_generation",
            PipelineStage::Testing => "testing",
            PipelineStage::Review => "review",
        }
    }

    /// 从字符串解析（保留为枚举的反向解析接口）
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<PipelineStage> {
        match s {
            "requirement_analysis" => Some(PipelineStage::RequirementAnalysis),
            "prd_generation" => Some(PipelineStage::PrdGeneration),
            "task_planning" => Some(PipelineStage::TaskPlanning),
            "code_generation" => Some(PipelineStage::CodeGeneration),
            "testing" => Some(PipelineStage::Testing),
            "review" => Some(PipelineStage::Review),
            _ => None,
        }
    }

    /// 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            PipelineStage::RequirementAnalysis => "需求分析",
            PipelineStage::PrdGeneration => "PRD 生成",
            PipelineStage::TaskPlanning => "任务拆分",
            PipelineStage::CodeGeneration => "代码生成",
            PipelineStage::Testing => "测试",
            PipelineStage::Review => "评审",
        }
    }
}

/// 环节 Skill 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSkillConfig {
    pub id: String,
    pub project_id: String,
    pub stage: String,
    pub skill_id: String,
    pub is_default: bool,
    pub custom_params: Option<String>,
    pub created_at: String,
}

// ==================== Skill/MCP 执行日志 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionLog {
    pub id: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub requirement_id: Option<String>,
    pub skill_id: String,
    pub skill_name: String,
    pub skill_type: Option<String>,
    pub execution_phase: Option<String>,
    pub input_params: Option<String>,
    pub output_summary: Option<String>,
    pub token_usage: i64,
    pub status: String,
    pub executed_at: String,
    /// 执行耗时（毫秒）；老数据/无法计时的场景为 None
    #[serde(default)]
    pub duration_ms: Option<i64>,
    /// 失败原因摘要
    #[serde(default)]
    pub error_message: Option<String>,
}

// ==================== Skill 自进化：评分 / 建议 / 版本快照 ====================

/// Skill 评分记录（总分 100 = 结构 60 + 效果 40）。
///
/// `effect_score` 为 None 表示仅完成结构分（无执行产出或 LLM 未配置）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillScore {
    pub id: String,
    pub skill_id: String,
    pub version: i32,
    pub structure_score: i32,
    #[serde(default)]
    pub effect_score: Option<i32>,
    pub total_score: i32,
    /// 各维度得分与证据（JSON 数组字符串）
    pub dimensions_json: String,
    #[serde(default)]
    pub judge_reason: Option<String>,
    pub judged_at: String,
}

/// Skill 优化建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSuggestion {
    pub id: String,
    pub skill_id: String,
    pub rule_type: String,
    pub severity: String,
    pub message: String,
    #[serde(default)]
    pub dimension: Option<String>,
    #[serde(default)]
    pub proposed_template: Option<String>,
    pub status: String,
    #[serde(default)]
    pub applied_version: Option<i32>,
    pub created_at: String,
}

/// Skill 模板版本快照（支持版本历史浏览与一键回退）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub id: String,
    pub skill_id: String,
    pub version: i32,
    pub template: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpExecutionLog {
    pub id: String,
    pub session_id: String,
    pub mcp_name: String,
    pub endpoint: Option<String>,
    pub action: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub duration_ms: Option<i64>,
    pub executed_at: String,
}

// ==================== 任务评论 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComment {
    pub id: String,
    pub task_id: String,
    pub author: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

// ==================== 统计数据模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub total_tokens: i64,
    pub total_cost: f64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_tokens: i64,
    pub thinking_tokens: i64,
    pub record_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub record_count: i64,
}
