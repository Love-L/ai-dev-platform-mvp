use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{error, debug, warn};

/// Claude Code 会话日志中的 usage 数据
#[derive(Debug, Deserialize, Serialize)]
struct ClaudeUsage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    output_tokens_details: Option<OutputTokenDetails>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OutputTokenDetails {
    #[serde(default)]
    thinking_tokens: u64,
}

/// Claude Code 会话日志条目
#[derive(Debug, Deserialize, Serialize)]
struct ClaudeLogEntry {
    #[serde(rename = "type")]
    entry_type: String,
    message: Option<ClaudeMessage>,
    timestamp: Option<String>,
    session_id: Option<String>,
    #[serde(rename = "cwd")]
    cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ClaudeMessage {
    id: Option<String>,
    model: Option<String>,
    role: Option<String>,
    usage: Option<ClaudeUsage>,
}

/// 从本地文件读取的用量记录
#[derive(Debug, Clone, Serialize)]
pub struct UsageRecord {
    pub session_id: String,
    pub cwd: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub thinking_tokens: u64,
    pub timestamp: String,
    pub git_branch: Option<String>,
    pub version: Option<String>,
}

/// 本地用量读取器
pub struct LocalUsageReader {
    /// Claude Code 数据根目录 (~/.claude)
    claude_dir: PathBuf,
    /// 已解析的文件偏移量 (文件路径 → 字节偏移)
    parsed_offsets: HashMap<PathBuf, u64>,
}

impl LocalUsageReader {
    /// 创建新的本地用量读取器
    pub fn new() -> Self {
        let claude_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude");

        Self {
            claude_dir,
            parsed_offsets: HashMap::new(),
        }
    }

    /// 扫描所有项目的会话日志，提取 usage 数据
    pub fn scan_all_projects(&mut self) -> Vec<UsageRecord> {
        debug!("开始扫描Claude项目目录...");

        let mut records = Vec::new();

        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            warn!("Claude项目目录不存在: {:?}", projects_dir);
            return records;
        }

        // 遍历所有项目目录
        match std::fs::read_dir(&projects_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        let project_records = self.scan_project(&entry.path());
                        records.extend(project_records);
                    }
                }
            }
            Err(e) => {
                error!("读取Claude项目目录失败: {}", e);
            }
        }

        debug!("扫描完成，找到 {} 条记录", records.len());
        records
    }

    /// 扫描单个项目的会话日志
    fn scan_project(&mut self, project_dir: &Path) -> Vec<UsageRecord> {
        debug!("扫描项目目录: {:?}", project_dir);

        let mut records = Vec::new();

        match std::fs::read_dir(project_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // 只处理 .jsonl 文件
                    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                        continue;
                    }

                    let offset = self.parsed_offsets.get(&path).copied().unwrap_or(0);
                    let new_records = self.parse_jsonl_file(&path, offset);
                    records.extend(new_records);
                }
            }
            Err(e) => {
                error!("读取项目目录失败: {:?}, error: {}", project_dir, e);
            }
        }

        debug!("项目扫描完成: {:?}, records={}", project_dir, records.len());
        records
    }

    /// 解析 JSONL 文件，提取 usage 数据
    fn parse_jsonl_file(&mut self, file_path: &Path, start_offset: u64) -> Vec<UsageRecord> {
        debug!("解析JSONL文件: {:?}, start_offset={}", file_path, start_offset);

        let mut records = Vec::new();

        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("读取JSONL文件失败: {:?}, error: {}", file_path, e);
                return records;
            }
        };

        let mut current_offset = 0u64;
        let mut line_count = 0u64;

        for line in content.lines() {
            let line_bytes = line.len() as u64 + 1; // +1 for newline
            current_offset += line_bytes;
            line_count += 1;

            // 跳过已解析的行
            if current_offset <= start_offset {
                continue;
            }

            // 解析 JSON 行
            if let Ok(entry) = serde_json::from_str::<ClaudeLogEntry>(line) {
                // 只处理 assistant 类型的消息（包含 usage）
                if entry.entry_type == "assistant" {
                    if let Some(message) = &entry.message {
                        if let Some(usage) = &message.usage {
                            let record = UsageRecord {
                                session_id: entry.session_id.clone().unwrap_or_default(),
                                cwd: entry.cwd.clone().unwrap_or_default(),
                                model: message.model.clone().unwrap_or_default(),
                                input_tokens: usage.input_tokens,
                                output_tokens: usage.output_tokens,
                                cache_creation_tokens: usage.cache_creation_input_tokens,
                                cache_read_tokens: usage.cache_read_input_tokens,
                                thinking_tokens: usage.output_tokens_details
                                    .as_ref()
                                    .map(|d| d.thinking_tokens)
                                    .unwrap_or(0),
                                timestamp: entry.timestamp.clone().unwrap_or_default(),
                                git_branch: entry.git_branch.clone(),
                                version: entry.version.clone(),
                            };
                            records.push(record);
                        }
                    }
                }
            }
        }

        debug!("JSONL文件解析完成: lines={}, records={}", line_count, records.len());

        // 更新已解析偏移量
        self.parsed_offsets.insert(file_path.to_path_buf(), current_offset);

        records
    }

    /// 检查 Claude 目录是否存在
    pub fn exists(&self) -> bool {
        self.claude_dir.exists()
    }
}
