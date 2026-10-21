//! Agent CLI 适配层（模块 16 / F077–F083）。
//!
//! 目标：屏蔽各 Agent CLI 的参数与事件差异，向上只暴露：
//! - [`SessionLaunchConfig`] → 启动参数（[`build_command`]）
//! - 用户消息编码（[`encode_user_message`]）
//! - 事件归一化（[`decode_line`] → [`AgentEvent`]）
//!
//! 协议依据：2026-09-21 对 claude 2.1.266 的**真实调用实测**（见设计文档 §19）。
//! 关键实测结论：
//! - `--input-format stream-json` 的用户消息形如
//!   `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"..."}]}}`
//! - `--include-partial-messages` 产出 `stream_event.content_block_delta`，
//!   `delta.type` 为 `thinking_delta` / `text_delta`
//! - 该发行版下 `--permission-prompts host` **不可达**，权限被拒时下发
//!   `system/permission_denied`（`tool_name` / `tool_use_id` / `message`）

use tokio::process::Command;

use crate::db::{block_type, session_mode};

// Windows：GUI 应用（windows_subsystem="windows"）派生的子进程会新建控制台窗口，
// 必须显式抑制——否则结构化会话/一次性调用会弹出黑色 "claude" 控制台（生产版实测问题）。
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 统一为子进程命令附加"不创建控制台窗口"标志（仅 Windows）
fn suppress_console(cmd: &mut Command) {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    #[cfg(not(windows))]
    let _ = cmd;
}

// ==================== 多 Agent 适配（MA-01 / M1） ====================

/// Agent 厂商/品种
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Claude,
    Gemini,
    Qwen,
    Codex,
    OpenCode,
    Aider,
    ZCode,
    Generic,
}

impl Flavor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Flavor::Claude => "claude",
            Flavor::Gemini => "gemini",
            Flavor::Qwen => "qwen",
            Flavor::Codex => "codex",
            Flavor::OpenCode => "opencode",
            Flavor::Aider => "aider",
            Flavor::ZCode => "zcode",
            Flavor::Generic => "generic",
        }
    }
}

/// 平台侧可用的接入能力（L2 一次性 / L3 结构化会话）
#[derive(Debug, Clone, Copy)]
pub struct Capabilities {
    /// 支持结构化事件流（消息模式会话，L3）
    pub structured: bool,
    /// 支持一次性调用（PRD 生成/智能完善，L2）
    pub one_shot: bool,
}

/// 一次性调用的启动描述（进程形态），由各适配器产出、公共执行器消费
#[derive(Debug, Clone)]
pub struct OneShotInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub work_dir: Option<String>,
    /// 经 stdin 传入的 prompt（推荐）：绕开 Windows 命令行 32KB 限制与
    /// `.ps1`/`cmd` 链路的引号转义损坏问题（多行/含引号的长 prompt 必须走 stdin）
    pub stdin_prompt: Option<String>,
}

impl OneShotInvocation {
    /// 便捷构造：prompt 作为单个命令行参数（不推荐长文本，仅供无法走 stdin 的 CLI）
    pub fn with_arg_prompt(program: &str, args: Vec<String>, work_dir: Option<&str>) -> Self {
        Self {
            program: program.to_string(),
            args,
            work_dir: work_dir.map(|s| s.to_string()),
            stdin_prompt: None,
        }
    }

    /// 便捷构造：prompt 走 stdin（推荐）
    pub fn with_stdin_prompt(
        program: &str,
        args: Vec<String>,
        work_dir: Option<&str>,
        prompt: &str,
    ) -> Self {
        Self {
            program: program.to_string(),
            args,
            work_dir: work_dir.map(|s| s.to_string()),
            stdin_prompt: Some(prompt.to_string()),
        }
    }
}

/// Agent 适配器：屏蔽各 CLI 的参数与事件差异（多 Agent 适配方案 §2）
pub trait AgentAdapter: Send + Sync {
    fn flavor(&self) -> Flavor;
    fn capabilities(&self) -> Capabilities;
    /// 结构化会话启动命令；返回 `None` 表示不支持 L3（调用方回退终端模式）
    fn build_session_command(&self, cfg: &SessionLaunchConfig) -> Option<Command>;
    /// 编码一条用户消息（写入子进程 stdin 的一行）
    fn encode_user_message(&self, text: &str) -> String;
    /// 解析一行 stdout 为归一化事件
    fn decode_line(&self, line: &str) -> Vec<AgentEvent>;
    /// 构造一次性调用（L2）；返回 `None` 表示不支持
    fn build_oneshot(
        &self,
        agent_path: &str,
        prompt: &str,
        model: Option<&str>,
        work_dir: Option<&str>,
    ) -> Option<OneShotInvocation>;
}

/// 公共参数组装：`--model` 追加
fn push_model_arg(args: &mut Vec<String>, model: Option<&str>) {
    if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
        args.push("--model".to_string());
        args.push(m.to_string());
    }
}

/// 按二进制路径识别适配器（文件名 stem 匹配，大小写不敏感）
pub fn adapter_for(agent_path: &str) -> &'static dyn AgentAdapter {
    let stem = std::path::Path::new(agent_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match stem.as_str() {
        "claude" | "claude-code" => &CLAUDE_ADAPTER,
        "gemini" => &GEMINI_ADAPTER,
        // Qwen Code 为 Gemini CLI 同源 fork，共用实现
        "qwen" => &QWEN_ADAPTER,
        "zcode" => &ZCODE_ADAPTER,
        "codex" => &CODEX_ADAPTER,
        "opencode" => &OPENCODE_ADAPTER,
        "aider" | "aider-chat" => &AIDER_ADAPTER,
        _ => &GENERIC_ADAPTER,
    }
}

// ==================== 各 Agent 适配器实现 ====================

/// Claude Code 适配（协议经 2026-09-21 实测锁定，见模块 16 设计文档 §19）
pub struct ClaudeAdapter;

impl AgentAdapter for ClaudeAdapter {
    fn flavor(&self) -> Flavor {
        Flavor::Claude
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { structured: true, one_shot: true }
    }

    fn build_session_command(&self, cfg: &SessionLaunchConfig) -> Option<Command> {
        build_command(cfg)
    }

    fn encode_user_message(&self, text: &str) -> String {
        encode_user_message(text)
    }

    fn decode_line(&self, line: &str) -> Vec<AgentEvent> {
        decode_line(line)
    }

    fn build_oneshot(
        &self,
        agent_path: &str,
        prompt: &str,
        model: Option<&str>,
        work_dir: Option<&str>,
    ) -> Option<OneShotInvocation> {
        // `claude -p` 不带位置 prompt 时从 stdin 读取——长 prompt（含需求内容、
        // 项目上下文、JSON 示例）必须走 stdin，否则 .ps1/cmd 链路的引号转义会损坏内容
        let mut args = vec!["-p".to_string(), "--output-format".to_string(), "text".to_string()];
        push_model_arg(&mut args, model);
        Some(OneShotInvocation::with_stdin_prompt(
            agent_path,
            args,
            work_dir,
            prompt,
        ))
    }
}

/// Gemini CLI / Qwen Code 适配（同源协议，一份实现覆盖两个 flavor）。
///
/// ⚠️ 结构化事件流（L3）的 decode 已按公开协议实现并附 fixture 单测，
/// 但**未经本机实测**——能力探测时 `structured` 恒为 false；
/// 本机安装 gemini/qwen 并实测通过后，把 [`GEMINI_STRUCTURED_VERIFIED`]
/// 翻转为 true 即可启用消息模式（驱动层/前端零改动）。
pub struct GeminiLikeAdapter {
    flavor: Flavor,
}

/// Gemini/Qwen 结构化协议是否已通过本机实测（见 MA-03/MA-04 验收 gate）
const GEMINI_STRUCTURED_VERIFIED: bool = false;

impl GeminiLikeAdapter {
    /// 一次性调用：`gemini -p "<prompt>"`（默认 text 输出）
    fn oneshot_args(prompt: &str, model: Option<&str>) -> Vec<String> {
        let mut args = vec!["-p".to_string(), prompt.to_string()];
        if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
            args.push("--model".to_string());
            args.push(m.to_string());
        }
        args
    }
}

impl AgentAdapter for GeminiLikeAdapter {
    fn flavor(&self) -> Flavor {
        self.flavor
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured: GEMINI_STRUCTURED_VERIFIED,
            one_shot: true,
        }
    }

    fn build_session_command(&self, _cfg: &SessionLaunchConfig) -> Option<Command> {
        // L3 待实测（MA-03/MA-04 验收 gate），当前返回 None → 平台自动回退终端模式
        None
    }

    fn encode_user_message(&self, text: &str) -> String {
        text.to_string()
    }

    /// Gemini Headless JSONL 事件（公开协议）→ 归一化事件
    fn decode_line(&self, line: &str) -> Vec<AgentEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            return vec![AgentEvent::Notice {
                level: "warn".to_string(),
                message: trimmed.chars().take(200).collect(),
            }];
        };

        match v.get("type").and_then(|t| t.as_str()) {
            Some("init") => vec![AgentEvent::Init {
                cli_session_id: v
                    .get("session_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                model: v.get("model").and_then(|s| s.as_str()).map(String::from),
                tools: Vec::new(),
            }],
            Some("message") => {
                let content = v.get("content").and_then(|c| c.as_str()).map(String::from);
                match content {
                    Some(text) => {
                        let mut draft = BlockDraft::new("assistant", block_type::TEXT);
                        draft.content = Some(text);
                        vec![AgentEvent::Block(draft)]
                    }
                    None => Vec::new(),
                }
            }
            Some("tool_use") => {
                let mut draft = BlockDraft::new("assistant", block_type::TOOL_USE);
                draft.tool_name = v
                    .get("tool_name")
                    .or_else(|| v.get("name"))
                    .and_then(|s| s.as_str())
                    .map(String::from);
                draft.tool_use_id = v
                    .get("tool_use_id")
                    .or_else(|| v.get("id"))
                    .and_then(|s| s.as_str())
                    .map(String::from);
                draft.content = v.get("args").or_else(|| v.get("input")).map(|a| a.to_string());
                vec![AgentEvent::Block(draft)]
            }
            Some("tool_result") => vec![AgentEvent::ToolResult {
                tool_use_id: v
                    .get("tool_use_id")
                    .and_then(|s| s.as_str())
                    .map(String::from),
                content: v.get("content").map(|c| c.to_string()),
                is_error: v
                    .get("is_error")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false),
                raw_json: Some(v.to_string()),
            }],
            Some("error") => vec![AgentEvent::Notice {
                level: "error".to_string(),
                message: v
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("未知错误")
                    .to_string(),
            }],
            Some("result") => vec![AgentEvent::TurnResult {
                input_tokens: v
                    .get("input_tokens")
                    .and_then(|n| n.as_i64())
                    .unwrap_or(0),
                output_tokens: v
                    .get("output_tokens")
                    .and_then(|n| n.as_i64())
                    .unwrap_or(0),
                cache_read_tokens: 0,
                cost_usd: v.get("cost_usd").and_then(|n| n.as_f64()),
                duration_ms: v.get("duration_ms").and_then(|n| n.as_i64()),
                is_error: v
                    .get("is_error")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false),
                model: v.get("model").and_then(|s| s.as_str()).map(String::from),
            }],
            // 未知事件容错忽略（Claude 实测教训）
            _ => Vec::new(),
        }
    }

    fn build_oneshot(
        &self,
        agent_path: &str,
        prompt: &str,
        model: Option<&str>,
        work_dir: Option<&str>,
    ) -> Option<OneShotInvocation> {
        // gemini/qwen 的 stdin 行为未实测，暂以命令行参数传 prompt（待实测后切 stdin）
        Some(OneShotInvocation::with_arg_prompt(
            agent_path,
            Self::oneshot_args(prompt, model),
            work_dir,
        ))
    }
}

/// ZCode CLI 适配（智谱；MA-11）。
///
/// ZCode Protocol 事件格式**待本机实测**（MA-11 验收 gate）：
/// 当前仅提供 L2 一次性调用（`-p` 约定），L3 能力关闭。
pub struct ZCodeAdapter;

impl AgentAdapter for ZCodeAdapter {
    fn flavor(&self) -> Flavor {
        Flavor::ZCode
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { structured: false, one_shot: true }
    }

    fn build_session_command(&self, _cfg: &SessionLaunchConfig) -> Option<Command> {
        None
    }

    fn encode_user_message(&self, text: &str) -> String {
        text.to_string()
    }

    fn decode_line(&self, _line: &str) -> Vec<AgentEvent> {
        Vec::new()
    }

    fn build_oneshot(
        &self,
        agent_path: &str,
        prompt: &str,
        model: Option<&str>,
        work_dir: Option<&str>,
    ) -> Option<OneShotInvocation> {
        let mut args = vec!["-p".to_string(), prompt.to_string()];
        push_model_arg(&mut args, model);
        Some(OneShotInvocation::with_arg_prompt(
            agent_path,
            args,
            work_dir,
        ))
    }
}

/// Codex CLI 适配（MA-06，M2：`exec --json` 事件映射待实测，当前仅 L2）
pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn flavor(&self) -> Flavor {
        Flavor::Codex
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { structured: false, one_shot: true }
    }

    fn build_session_command(&self, _cfg: &SessionLaunchConfig) -> Option<Command> {
        None
    }

    fn encode_user_message(&self, text: &str) -> String {
        text.to_string()
    }

    fn decode_line(&self, _line: &str) -> Vec<AgentEvent> {
        Vec::new()
    }

    fn build_oneshot(
        &self,
        agent_path: &str,
        prompt: &str,
        model: Option<&str>,
        work_dir: Option<&str>,
    ) -> Option<OneShotInvocation> {
        // P2-A3：与 Claude/Gemini 对齐，透传 `--model`（codex exec 支持 -m/--model）
        let mut args = vec!["exec".to_string()];
        if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
            args.push("--model".to_string());
            args.push(m.trim().to_string());
        }
        args.push(prompt.to_string());
        Some(OneShotInvocation::with_arg_prompt(agent_path, args, work_dir))
    }
}

/// opencode 适配（MA-07：L2 一次性 `opencode run`）
pub struct OpenCodeAdapter;

impl AgentAdapter for OpenCodeAdapter {
    fn flavor(&self) -> Flavor {
        Flavor::OpenCode
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { structured: false, one_shot: true }
    }

    fn build_session_command(&self, _cfg: &SessionLaunchConfig) -> Option<Command> {
        None
    }

    fn encode_user_message(&self, text: &str) -> String {
        text.to_string()
    }

    fn decode_line(&self, _line: &str) -> Vec<AgentEvent> {
        Vec::new()
    }

    fn build_oneshot(
        &self,
        agent_path: &str,
        prompt: &str,
        _model: Option<&str>,
        work_dir: Option<&str>,
    ) -> Option<OneShotInvocation> {
        Some(OneShotInvocation::with_arg_prompt(
            agent_path,
            vec!["run".to_string(), prompt.to_string()],
            work_dir,
        ))
    }
}

/// Aider 适配（MA-08：仅 L2，`--message` + `--yes` 自动确认）
pub struct AiderAdapter;

impl AgentAdapter for AiderAdapter {
    fn flavor(&self) -> Flavor {
        Flavor::Aider
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { structured: false, one_shot: true }
    }

    fn build_session_command(&self, _cfg: &SessionLaunchConfig) -> Option<Command> {
        None
    }

    fn encode_user_message(&self, text: &str) -> String {
        text.to_string()
    }

    fn decode_line(&self, _line: &str) -> Vec<AgentEvent> {
        Vec::new()
    }

    fn build_oneshot(
        &self,
        agent_path: &str,
        prompt: &str,
        _model: Option<&str>,
        work_dir: Option<&str>,
    ) -> Option<OneShotInvocation> {
        Some(OneShotInvocation::with_arg_prompt(
            agent_path,
            vec![
                "--message".to_string(),
                prompt.to_string(),
                "--yes".to_string(),
            ],
            work_dir,
        ))
    }
}

/// 未识别 CLI 的兜底适配：仅终端模式（L1），不提供任何程序化调用
pub struct GenericAdapter;

impl AgentAdapter for GenericAdapter {
    fn flavor(&self) -> Flavor {
        Flavor::Generic
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { structured: false, one_shot: false }
    }

    fn build_session_command(&self, _cfg: &SessionLaunchConfig) -> Option<Command> {
        None
    }

    fn encode_user_message(&self, text: &str) -> String {
        text.to_string()
    }

    fn decode_line(&self, _line: &str) -> Vec<AgentEvent> {
        Vec::new()
    }

    fn build_oneshot(
        &self,
        _agent_path: &str,
        _prompt: &str,
        _model: Option<&str>,
        _work_dir: Option<&str>,
    ) -> Option<OneShotInvocation> {
        None
    }
}

static CLAUDE_ADAPTER: ClaudeAdapter = ClaudeAdapter;
static GEMINI_ADAPTER: GeminiLikeAdapter = GeminiLikeAdapter { flavor: Flavor::Gemini };
static QWEN_ADAPTER: GeminiLikeAdapter = GeminiLikeAdapter { flavor: Flavor::Qwen };
static ZCODE_ADAPTER: ZCodeAdapter = ZCodeAdapter;
static CODEX_ADAPTER: CodexAdapter = CodexAdapter;
static OPENCODE_ADAPTER: OpenCodeAdapter = OpenCodeAdapter;
static AIDER_ADAPTER: AiderAdapter = AiderAdapter;
static GENERIC_ADAPTER: GenericAdapter = GenericAdapter;

/// 结构化会话启动配置
#[derive(Debug, Clone, Default)]
pub struct SessionLaunchConfig {
    /// Agent 二进制路径
    pub agent_path: String,
    /// 工作目录（作为子进程 cwd，并追加 `--add-dir`）
    pub work_dir: Option<String>,
    /// 指定/复用 CLI 会话 id（`--session-id`）
    pub cli_session_id: Option<String>,
    /// 续接已有 CLI 会话（`--resume`），与 `cli_session_id` 二选一
    pub resume_cli_session_id: Option<String>,
    /// 会话级模型
    pub model: Option<String>,
    /// 权限预设，默认 `acceptEdits`（实测可放行编辑类操作）
    pub permission_mode: String,
    /// 工具白名单（`--allowedTools`）
    pub allowed_tools: Vec<String>,
    /// 上下文注入（`--append-system-prompt`）
    pub append_system_prompt: Option<String>,
    /// 是否请求增量消息块（`--include-partial-messages`）
    pub include_partial_messages: bool,
}

impl SessionLaunchConfig {
    pub fn new(agent_path: &str) -> Self {
        Self {
            agent_path: agent_path.to_string(),
            permission_mode: "acceptEdits".to_string(),
            include_partial_messages: true,
            ..Default::default()
        }
    }
}

/// 归一化事件：所有 Agent 差异在适配层内消化
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// 会话初始化（`system/init`）
    Init {
        cli_session_id: String,
        model: Option<String>,
        tools: Vec<String>,
    },
    /// 正文/思考增量（`stream_event`，不落库，仅推送）
    Delta { block_type: String, delta: String },
    /// 思考 Token 估算（`system/thinking_tokens`），用于折叠态计数
    ThinkingTokens { total: i64, delta: i64 },
    /// 一个完整内容块（正文 / 思考 / 工具调用）
    Block(BlockDraft),
    /// 工具结果（`user` 事件内的 `tool_result`）
    ToolResult {
        tool_use_id: Option<String>,
        content: Option<String>,
        is_error: bool,
        raw_json: Option<String>,
    },
    /// 权限被拒（`system/permission_denied`）
    PermissionDenied {
        tool_name: String,
        tool_use_id: Option<String>,
        message: String,
    },
    /// 单轮结束（`result`）
    TurnResult {
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cost_usd: Option<f64>,
        duration_ms: Option<i64>,
        is_error: bool,
        /// 实际使用的模型（`modelUsage.canonicalModel`，用于消息级记账归因）
        model: Option<String>,
    },
    /// 平台级提示（解析告警、非 JSON 行等）
    Notice { level: String, message: String },
}

/// 待落库的内容块草稿
#[derive(Debug, Clone, Default)]
pub struct BlockDraft {
    pub role: String,
    pub block_type: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub parent_tool_use_id: Option<String>,
    pub cli_message_id: Option<String>,
    pub raw_json: Option<String>,
    pub tokens: Option<i64>,
    pub status: String,
}

impl BlockDraft {
    fn new(role: &str, block_type_str: &str) -> Self {
        Self {
            role: role.to_string(),
            block_type: block_type_str.to_string(),
            status: "completed".to_string(),
            ..Default::default()
        }
    }
}

// ==================== 启动参数 ====================

/// 组装**结构化会话**的启动命令。
///
/// 返回 `None` 表示该 Agent 不支持结构化模式（调用方应回退终端模式）。
pub fn build_command(cfg: &SessionLaunchConfig) -> Option<Command> {
    if cfg.agent_path.is_empty() {
        return None;
    }

    let (program, pre_args) = shell_wrap(&cfg.agent_path);
    let mut cmd = Command::new(program);
    cmd.args(pre_args);

    cmd.arg("-p")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose");

    if cfg.include_partial_messages {
        cmd.arg("--include-partial-messages");
    }

    // 续接优先：`--resume` 会继续既有 CLI 会话；否则用 `--session-id` 绑定新会话
    if let Some(resume_id) = cfg.resume_cli_session_id.as_deref().filter(|s| !s.is_empty()) {
        cmd.arg("--resume").arg(resume_id);
    } else if let Some(sid) = cfg.cli_session_id.as_deref().filter(|s| !s.is_empty()) {
        cmd.arg("--session-id").arg(sid);
    }

    if let Some(model) = cfg.model.as_deref().filter(|s| !s.is_empty()) {
        cmd.arg("--model").arg(model);
    }

    let permission_mode = if cfg.permission_mode.is_empty() {
        "acceptEdits"
    } else {
        cfg.permission_mode.as_str()
    };
    cmd.arg("--permission-mode").arg(permission_mode);

    if !cfg.allowed_tools.is_empty() {
        cmd.arg("--allowedTools").arg(cfg.allowed_tools.join(","));
    }

    if let Some(dir) = cfg.work_dir.as_deref().filter(|s| !s.is_empty()) {
        cmd.current_dir(dir);
        cmd.arg("--add-dir").arg(dir);
    }

    if let Some(prompt) = cfg
        .append_system_prompt
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        cmd.arg("--append-system-prompt").arg(prompt);
    }

    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    suppress_console(&mut cmd);

    Some(cmd)
}

/// Windows 上的 npm 全局命令多为 `.ps1` / `.cmd` 包装脚本，无法直接 spawn，
/// 需要交给对应的解释器执行。
fn shell_wrap(path: &str) -> (String, Vec<String>) {
    let lower = path.to_lowercase();
    if lower.ends_with(".ps1") {
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                path.to_string(),
            ],
        )
    } else if lower.ends_with(".cmd") || lower.ends_with(".bat") {
        ("cmd".to_string(), vec!["/C".to_string(), path.to_string()])
    } else {
        (path.to_string(), Vec::new())
    }
}

/// 执行一次性调用（多 Agent 公共执行器，L2 通道）。
///
/// 各适配器只负责产出 [`OneShotInvocation`]（参数差异），进程执行、
/// `.ps1`/`.cmd` 包装处理与错误截取统一在此完成。
/// 不消耗 API Key（使用各 CLI 本机登录态）。
pub async fn execute_oneshot(inv: OneShotInvocation) -> Result<String, String> {
    use tokio::io::AsyncWriteExt;

    let (program, pre_args) = shell_wrap(&inv.program);
    let mut cmd = Command::new(program);
    cmd.args(pre_args).args(&inv.args);

    if let Some(dir) = inv.work_dir.filter(|d| !d.trim().is_empty()) {
        cmd.current_dir(dir);
    }

    let use_stdin = inv.stdin_prompt.is_some();
    if use_stdin {
        cmd.stdin(std::process::Stdio::piped());
    } else {
        cmd.stdin(std::process::Stdio::null());
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    suppress_console(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("调用 Agent CLI 失败: {}", e))?;

    // prompt 经 stdin 传入后立即关闭写端（CLI 收到 EOF 开始处理）
    if let Some(prompt) = inv.stdin_prompt {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(|e| format!("写入 Agent CLI 输入失败: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("写入 Agent CLI 输入失败: {}", e))?;
            drop(stdin);
        }
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("调用 Agent CLI 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr
            .lines().rfind(|l| !l.trim().is_empty())
            .unwrap_or("未知错误")
            .chars()
            .take(300)
            .collect();
        return Err(format!("Agent CLI 退出码 {:?}: {}", output.status.code(), tail));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err("Agent CLI 返回了空结果".to_string());
    }
    Ok(text)
}

/// 编码一条用户消息（写入子进程 stdin 的一行）
pub fn encode_user_message(text: &str) -> String {
    let payload = serde_json::json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{ "type": "text", "text": text }]
        }
    });
    payload.to_string()
}

/// 会话模式辅助：判断是否为结构化模式
pub fn is_structured_mode(mode: Option<&str>) -> bool {
    matches!(mode, Some(m) if m == session_mode::STRUCTURED)
}

// ==================== 事件归一化 ====================

/// 解析子进程 stdout 的一行，产出 0..n 个归一化事件
pub fn decode_line(line: &str) -> Vec<AgentEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return vec![AgentEvent::Notice {
            level: "warn".to_string(),
            message: trimmed.chars().take(200).collect(),
        }];
    };

    match value.get("type").and_then(|v| v.as_str()) {
        Some("system") => decode_system(&value),
        Some("assistant") => decode_assistant(&value),
        Some("user") => decode_user(&value),
        Some("stream_event") => decode_stream_event(&value),
        Some("result") => decode_result(&value),
        _ => Vec::new(),
    }
}

fn decode_system(value: &serde_json::Value) -> Vec<AgentEvent> {
    match value.get("subtype").and_then(|v| v.as_str()) {
        Some("init") => {
            let tools = value
                .get("tools")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            vec![AgentEvent::Init {
                cli_session_id: value
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                model: value
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tools,
            }]
        }
        Some("thinking_tokens") => {
            let total = value
                .get("estimated_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let delta = value
                .get("estimated_tokens_delta")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            vec![AgentEvent::ThinkingTokens { total, delta }]
        }
        Some("permission_denied") => vec![AgentEvent::PermissionDenied {
            tool_name: value
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            tool_use_id: value
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            message: value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("该操作因权限策略被拒绝")
                .to_string(),
        }],
        // hook_* / status 等系统事件对会话展示无价值，忽略（避免噪声块）
        _ => Vec::new(),
    }
}

fn decode_assistant(value: &serde_json::Value) -> Vec<AgentEvent> {
    let message = match value.get("message") {
        Some(m) => m,
        None => return Vec::new(),
    };

    let cli_message_id = message
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let parent_tool_use_id = value
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut events = Vec::new();
    for block in message
        .get("content")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
    {
        match block.get("type").and_then(|v| v.as_str()) {
            Some("text") => {
                let mut draft = BlockDraft::new("assistant", block_type::TEXT);
                draft.content = block
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                draft.cli_message_id = cli_message_id.clone();
                draft.parent_tool_use_id = parent_tool_use_id.clone();
                events.push(AgentEvent::Block(draft));
            }
            Some("thinking") => {
                let mut draft = BlockDraft::new("assistant", block_type::THINKING);
                draft.content = block
                    .get("thinking")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                draft.cli_message_id = cli_message_id.clone();
                draft.parent_tool_use_id = parent_tool_use_id.clone();
                events.push(AgentEvent::Block(draft));
            }
            Some("tool_use") => {
                let mut draft = BlockDraft::new("assistant", block_type::TOOL_USE);
                draft.tool_name = block
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                draft.tool_use_id = block
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                draft.content = block.get("input").map(|v| v.to_string());
                draft.cli_message_id = cli_message_id.clone();
                draft.parent_tool_use_id = parent_tool_use_id.clone();
                events.push(AgentEvent::Block(draft));
            }
            _ => {}
        }
    }

    events
}

fn decode_user(value: &serde_json::Value) -> Vec<AgentEvent> {
    let Some(message) = value.get("message") else {
        return Vec::new();
    };

    let mut events = Vec::new();
    for block in message
        .get("content")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
    {
        if block.get("type").and_then(|v| v.as_str()) != Some("tool_result") {
            continue;
        }

        // `content` 可能是字符串，也可能是内容块数组
        let content = match block.get("content") {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(other) => Some(other.to_string()),
            None => None,
        };

        events.push(AgentEvent::ToolResult {
            tool_use_id: block
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            content,
            is_error: block
                .get("is_error")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            raw_json: value
                .get("tool_use_result")
                .map(|v| v.to_string())
                .or_else(|| Some(value.to_string())),
        });
    }

    events
}

fn decode_stream_event(value: &serde_json::Value) -> Vec<AgentEvent> {
    let Some(event) = value.get("event") else {
        return Vec::new();
    };
    if event.get("type").and_then(|v| v.as_str()) != Some("content_block_delta") {
        return Vec::new();
    }

    let delta = match event.get("delta") {
        Some(d) => d,
        None => return Vec::new(),
    };

    match delta.get("type").and_then(|v| v.as_str()) {
        Some("thinking_delta") => delta
            .get("thinking")
            .and_then(|v| v.as_str())
            .map(|t| {
                vec![AgentEvent::Delta {
                    block_type: block_type::THINKING.to_string(),
                    delta: t.to_string(),
                }]
            })
            .unwrap_or_default(),
        Some("text_delta") => delta
            .get("text")
            .and_then(|v| v.as_str())
            .map(|t| {
                vec![AgentEvent::Delta {
                    block_type: block_type::TEXT.to_string(),
                    delta: t.to_string(),
                }]
            })
            .unwrap_or_default(),
        // input_json_delta / signature_delta：由完整块兜底，无需增量渲染
        _ => Vec::new(),
    }
}

fn decode_result(value: &serde_json::Value) -> Vec<AgentEvent> {
    let usage = value.get("usage");

    vec![AgentEvent::TurnResult {
        input_tokens: usage
            .and_then(|u| u.get("input_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        output_tokens: usage
            .and_then(|u| u.get("output_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        cache_read_tokens: usage
            .and_then(|u| u.get("cache_read_input_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        cost_usd: value.get("total_cost_usd").and_then(|v| v.as_f64()),
        duration_ms: value.get("duration_api_ms").and_then(|v| v.as_i64()),
        is_error: value
            .get("is_error")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        // 实测（§19.2.2）：`modelUsage.canonicalModel` 为本轮实际模型；
        // 旧发行版无此字段时回退顶层 `model`
        model: value
            .get("modelUsage")
            .and_then(|u| u.get("canonicalModel"))
            .and_then(|v| v.as_str())
            .or_else(|| value.get("model").and_then(|v| v.as_str()))
            .map(|s| s.to_string()),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_init_captures_session_id_and_tools() {
        let line = r#"{"type":"system","subtype":"init","cwd":"E:\\proj","session_id":"sess-1","tools":["Bash","Edit"],"model":"claude-x"}"#;
        match &decode_line(line)[0] {
            AgentEvent::Init {
                cli_session_id,
                tools,
                model,
            } => {
                assert_eq!(cli_session_id, "sess-1");
                assert_eq!(tools.len(), 2);
                assert_eq!(model.as_deref(), Some("claude-x"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn decode_thinking_and_text_blocks() {
        // 实测：thinking 与 text 是同一 message.id 的两条独立 assistant 事件
        let thinking = r#"{"type":"assistant","message":{"id":"m1","content":[{"type":"thinking","thinking":"推理中","signature":""}]}}"#;
        let text = r#"{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"ok"}]}}"#;

        match &decode_line(thinking)[0] {
            AgentEvent::Block(d) => {
                assert_eq!(d.block_type, "thinking");
                assert_eq!(d.content.as_deref(), Some("推理中"));
            }
            other => panic!("unexpected {other:?}"),
        }
        match &decode_line(text)[0] {
            AgentEvent::Block(d) => {
                assert_eq!(d.block_type, "text");
                assert_eq!(d.content.as_deref(), Some("ok"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn decode_tool_use_and_result() {
        let tool_use = r#"{"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"call_1","name":"Bash","input":{"command":"echo hi"}}]}}"#;
        match &decode_line(tool_use)[0] {
            AgentEvent::Block(d) => {
                assert_eq!(d.block_type, "tool_use");
                assert_eq!(d.tool_name.as_deref(), Some("Bash"));
                assert_eq!(d.tool_use_id.as_deref(), Some("call_1"));
                assert!(d.content.as_deref().unwrap().contains("echo hi"));
            }
            other => panic!("unexpected {other:?}"),
        }

        let tool_result = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"call_1","type":"tool_result","content":"hi","is_error":false}]},"tool_use_result":{"stdout":"hi"}}"#;
        match &decode_line(tool_result)[0] {
            AgentEvent::ToolResult {
                tool_use_id,
                content,
                is_error,
                ..
            } => {
                assert_eq!(tool_use_id.as_deref(), Some("call_1"));
                assert_eq!(content.as_deref(), Some("hi"));
                assert!(!is_error);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn decode_permission_denied() {
        let line = r#"{"type":"system","subtype":"permission_denied","tool_name":"Write","tool_use_id":"call_9","message":"Claude requested permissions to write to X, but you haven't granted it yet."}"#;
        match &decode_line(line)[0] {
            AgentEvent::PermissionDenied {
                tool_name,
                tool_use_id,
                message,
            } => {
                assert_eq!(tool_name, "Write");
                assert_eq!(tool_use_id.as_deref(), Some("call_9"));
                assert!(message.contains("haven't granted"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn decode_partial_message_deltas() {
        let thinking_delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"片段"},"index":0}}"#;
        match &decode_line(thinking_delta)[0] {
            AgentEvent::Delta { block_type, delta } => {
                assert_eq!(block_type, "thinking");
                assert_eq!(delta, "片段");
            }
            other => panic!("unexpected {other:?}"),
        }

        let text_delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"正文"},"index":1}}"#;
        match &decode_line(text_delta)[0] {
            AgentEvent::Delta { block_type, delta } => {
                assert_eq!(block_type, "text");
                assert_eq!(delta, "正文");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn decode_result_provides_cost_and_cache_tokens() {
        let line = r#"{"type":"result","subtype":"success","duration_api_ms":4745,"total_cost_usd":0.14,"model":"claude-sonnet-4","modelUsage":{"claude-sonnet-4":{"costUSD":0.14,"canonicalModel":"claude-sonnet-4"}},"usage":{"input_tokens":28133,"cache_creation_input_tokens":0,"cache_read_input_tokens":16384,"output_tokens":15}}"#;
        match &decode_line(line)[0] {
            AgentEvent::TurnResult {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cost_usd,
                duration_ms,
                is_error,
                model,
            } => {
                assert_eq!(*input_tokens, 28133);
                assert_eq!(*output_tokens, 15);
                assert_eq!(*cache_read_tokens, 16384);
                assert_eq!(*cost_usd, Some(0.14));
                assert_eq!(*duration_ms, Some(4745));
                assert!(!is_error);
                assert_eq!(model.as_deref(), Some("claude-sonnet-4"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn encode_user_message_matches_measured_schema() {
        let encoded = encode_user_message("列出当前目录");
        let parsed: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(parsed["type"], "user");
        assert_eq!(parsed["message"]["role"], "user");
        assert_eq!(parsed["message"]["content"][0]["type"], "text");
        assert_eq!(parsed["message"]["content"][0]["text"], "列出当前目录");
    }

    #[test]
    fn non_json_line_becomes_notice_instead_of_panicking() {
        match &decode_line("Warning: no stdin data received")[0] {
            AgentEvent::Notice { level, .. } => assert_eq!(level, "warn"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn hook_and_status_events_are_ignored() {
        assert!(decode_line(r#"{"type":"system","subtype":"hook_started","hook_name":"x"}"#).is_empty());
        assert!(decode_line(r#"{"type":"system","subtype":"status","status":"ok"}"#).is_empty());
    }

    // ==================== 多 Agent 适配（MA-01 / M1） ====================

    #[test]
    fn adapter_for_maps_by_binary_stem() {
        assert!(matches!(
            adapter_for("C:\\bin\\claude.exe").flavor(),
            Flavor::Claude
        ));
        assert!(matches!(
            adapter_for("/usr/bin/gemini").flavor(),
            Flavor::Gemini
        ));
        assert!(matches!(adapter_for("/usr/bin/qwen").flavor(), Flavor::Qwen));
        assert!(matches!(
            adapter_for("C:\\bin\\zcode.cmd").flavor(),
            Flavor::ZCode
        ));
        assert!(matches!(adapter_for("/usr/bin/codex").flavor(), Flavor::Codex));
        assert!(matches!(
            adapter_for("/usr/bin/opencode").flavor(),
            Flavor::OpenCode
        ));
        assert!(matches!(
            adapter_for("/usr/bin/aider-chat").flavor(),
            Flavor::Aider
        ));
        // 未识别 CLI → Generic 兜底（仅终端）
        assert!(matches!(
            adapter_for("/usr/bin/whatever").flavor(),
            Flavor::Generic
        ));
    }

    #[test]
    fn claude_capabilities_structured_and_one_shot() {
        let caps = adapter_for("claude").capabilities();
        assert!(caps.structured);
        assert!(caps.one_shot);
    }

    /// Gemini/Qwen：L3 decode 已按公开协议实现，但实测 gate 未过 → structured=false
    #[test]
    fn gemini_capabilities_l2_only_until_verified() {
        let caps = adapter_for("gemini").capabilities();
        assert!(!caps.structured, "实测 gate 未通过前不得启用 L3");
        assert!(caps.one_shot);
    }

    #[test]
    fn gemini_decode_init_and_message() {
        let adapter = adapter_for("gemini");
        match &adapter.decode_line(r#"{"type":"init","session_id":"g1","model":"gemini-2"}"#)[0] {
            AgentEvent::Init { cli_session_id, model, .. } => {
                assert_eq!(cli_session_id, "g1");
                assert_eq!(model.as_deref(), Some("gemini-2"));
            }
            other => panic!("unexpected {other:?}"),
        }

        match &adapter.decode_line(r#"{"type":"message","role":"assistant","content":"你好"}"#)[0] {
            AgentEvent::Block(d) => {
                assert_eq!(d.block_type, "text");
                assert_eq!(d.content.as_deref(), Some("你好"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn gemini_decode_tool_lifecycle_and_result() {
        let adapter = adapter_for("qwen");

        match &adapter
            .decode_line(r#"{"type":"tool_use","tool_name":"write_file","tool_use_id":"t1","args":{"path":"a.txt"}}"#)
            [0]
        {
            AgentEvent::Block(d) => {
                assert_eq!(d.block_type, "tool_use");
                assert_eq!(d.tool_name.as_deref(), Some("write_file"));
                assert_eq!(d.tool_use_id.as_deref(), Some("t1"));
            }
            other => panic!("unexpected {other:?}"),
        }

        match &adapter
            .decode_line(r#"{"type":"tool_result","tool_use_id":"t1","content":"ok","is_error":false}"#)
            [0]
        {
            AgentEvent::ToolResult { tool_use_id, is_error, .. } => {
                assert_eq!(tool_use_id.as_deref(), Some("t1"));
                assert!(!is_error);
            }
            other => panic!("unexpected {other:?}"),
        }

        match &adapter.decode_line(r#"{"type":"error","message":"boom"}"#)[0] {
            AgentEvent::Notice { level, message } => {
                assert_eq!(level, "error");
                assert_eq!(message, "boom");
            }
            other => panic!("unexpected {other:?}"),
        }

        match &adapter
            .decode_line(r#"{"type":"result","input_tokens":100,"output_tokens":20,"model":"qwen-max"}"#)
            [0]
        {
            AgentEvent::TurnResult { input_tokens, output_tokens, model, .. } => {
                assert_eq!(*input_tokens, 100);
                assert_eq!(*output_tokens, 20);
                assert_eq!(model.as_deref(), Some("qwen-max"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn gemini_unknown_events_are_tolerated() {
        let adapter = adapter_for("gemini");
        assert!(adapter.decode_line(r#"{"type":"stats","x":1}"#).is_empty());
        assert!(adapter.decode_line("not json").len() == 1); // Notice
        assert!(adapter.decode_line("").is_empty());
    }

    #[test]
    fn oneshot_invocations_per_flavor() {
        // Claude：-p + --output-format text
        let inv = adapter_for("claude")
            .build_oneshot("claude", "hi", Some("m1"), Some("D:\\w"))
            .unwrap();
        assert_eq!(inv.args[0], "-p");
        assert!(inv.args.contains(&"--output-format".to_string()));

        // Gemini/Qwen：-p + --model
        let inv = adapter_for("gemini")
            .build_oneshot("gemini", "hi", Some("m2"), None)
            .unwrap();
        assert!(inv.args.contains(&"--model".to_string()));

        // Codex：exec
        let inv = adapter_for("codex").build_oneshot("codex", "hi", None, None).unwrap();
        assert_eq!(inv.args[0], "exec");

        // Aider：--message + --yes
        let inv = adapter_for("aider").build_oneshot("aider", "hi", None, None).unwrap();
        assert_eq!(inv.args[0], "--message");
        assert!(inv.args.contains(&"--yes".to_string()));

        // Generic：不支持一次性调用
        assert!(adapter_for("whatever").build_oneshot("whatever", "hi", None, None).is_none());
    }
}
