use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

/// 创建不显示控制台窗口的Command（Windows专用）
#[cfg(target_os = "windows")]
fn create_command_no_window(path: &PathBuf) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = Command::new(path);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// 创建Command（非Windows系统）
#[cfg(not(target_os = "windows"))]
fn create_command_no_window(path: &PathBuf) -> Command {
    Command::new(path)
}

/// Agent定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub display_name: String,
    pub binary_names: Vec<String>,
    pub min_version: String,
    pub npm_package: Option<String>,
}

/// 检测到的Agent信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAgent {
    pub name: String,
    pub display_name: String,
    pub version: Option<String>,
    pub path: String,
    pub capabilities: AgentCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub mcp_support: bool,
    pub worktree_support: bool,
    pub interactive: bool,
    /// Agent 品种（多 Agent 适配 MA-01；与 agent_adapter::Flavor 对应）
    #[serde(default)]
    pub flavor: String,
    /// 是否支持结构化事件流（L3 消息模式会话）
    #[serde(default)]
    pub structured_events: bool,
    /// 是否支持一次性调用（L2，PRD 生成/智能完善）
    #[serde(default)]
    pub one_shot: bool,
}

/// 需要检测的Agent列表
pub fn get_agent_definitions() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            name: "claude-code".to_string(),
            display_name: "Claude Code".to_string(),
            binary_names: vec!["claude".to_string()],
            min_version: "1.0.0".to_string(),
            npm_package: Some("@anthropic-ai/claude-code".to_string()),
        },
        AgentDefinition {
            name: "codex".to_string(),
            display_name: "Codex CLI".to_string(),
            binary_names: vec!["codex".to_string()],
            min_version: "0.1.0".to_string(),
            npm_package: Some("@openai/codex".to_string()),
        },
        AgentDefinition {
            name: "gemini".to_string(),
            display_name: "Gemini CLI".to_string(),
            binary_names: vec!["gemini".to_string()],
            min_version: "1.0.0".to_string(),
            npm_package: Some("@google/gemini-cli".to_string()),
        },
        AgentDefinition {
            name: "qwen".to_string(),
            display_name: "Qwen Code".to_string(),
            binary_names: vec!["qwen".to_string()],
            min_version: "1.0.0".to_string(),
            npm_package: Some("@qwen-code/qwen-code".to_string()),
        },
        AgentDefinition {
            name: "zcode".to_string(),
            display_name: "ZCode CLI".to_string(),
            binary_names: vec!["zcode".to_string()],
            min_version: "1.0.0".to_string(),
            npm_package: Some("@zcode/cli".to_string()),
        },
        AgentDefinition {
            name: "opencode".to_string(),
            display_name: "OpenCode".to_string(),
            binary_names: vec!["opencode".to_string()],
            min_version: "1.0.0".to_string(),
            npm_package: None,
        },
        AgentDefinition {
            name: "aider".to_string(),
            display_name: "Aider".to_string(),
            binary_names: vec!["aider".to_string(), "aider-chat".to_string()],
            min_version: "1.0.0".to_string(),
            npm_package: None,
        },
    ]
}

/// 发现系统上所有可用的CLI Agent
pub async fn discover_agents() -> Vec<DetectedAgent> {
    debug!("开始发现系统Agent...");

    let definitions = get_agent_definitions();
    let mut agents = Vec::new();

    for def in definitions {
        if let Some(agent) = detect_agent(&def).await {
            agents.push(agent);
        }
    }

    info!("发现 {} 个Agent", agents.len());
    agents
}

/// 检测单个Agent
async fn detect_agent(def: &AgentDefinition) -> Option<DetectedAgent> {
    debug!("检测Agent: {}", def.name);

    // 1. 在PATH中查找
    for binary in &def.binary_names {
        if let Some(path) = find_in_path(binary) {
            debug!("找到Agent二进制: {:?}", path);

            // 使用 spawn_blocking 执行阻塞的进程操作
            let path_clone = path.clone();
            let version = tokio::task::spawn_blocking(move || get_version(&path_clone))
                .await
                .ok()
                .flatten();

            let path_clone = path.clone();
            let capabilities = tokio::task::spawn_blocking(move || probe_capabilities(&path_clone))
                .await
                .unwrap_or(AgentCapabilities {
                    mcp_support: false,
                    worktree_support: false,
                    interactive: true,
                    flavor: "generic".to_string(),
                    structured_events: false,
                    one_shot: false,
                });

            info!("Agent检测成功: name={}, version={:?}", def.name, version);

            return Some(DetectedAgent {
                name: def.name.clone(),
                display_name: def.display_name.clone(),
                version,
                path: path.to_string_lossy().to_string(),
                capabilities,
            });
        }
    }

    debug!("Agent未找到: {}", def.name);
    None
}

/// 在PATH中查找可执行文件
fn find_in_path(binary_name: &str) -> Option<PathBuf> {
    // 先在 PATH 中查找
    if let Ok(path) = which::which(binary_name) {
        return Some(path);
    }

    // Windows: 扫描常见安装路径
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = find_in_windows_paths(binary_name) {
            return Some(path);
        }
    }

    // macOS / Linux: 扫描常见路径
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(path) = find_in_unix_paths(binary_name) {
            return Some(path);
        }
    }

    None
}

/// Windows 常见安装路径扫描
#[cfg(target_os = "windows")]
fn find_in_windows_paths(binary_name: &str) -> Option<PathBuf> {
    let exe_name = format!("{}.exe", binary_name);
    let candidates = vec![
        // npm global (AppData)
        directories::BaseDirs::new().map(|b| {
            b.home_dir()
                .join("AppData")
                .join("Roaming")
                .join("npm")
                .join(&exe_name)
        }),
        // npm global (Program Files)
        std::env::var("PROGRAMFILES").ok().map(|p| {
            std::path::PathBuf::from(p)
                .join("nodejs")
                .join(&exe_name)
        }),
        // nvm-windows
        std::env::var("NVM_SYMLINK").ok().map(|p| {
            std::path::PathBuf::from(p).join(&exe_name)
        }),
        // nvm-windows (AppData)
        directories::BaseDirs::new().map(|b| {
            b.home_dir()
                .join("AppData")
                .join("Roaming")
                .join("nvm")
                .join(&exe_name)
        }),
        // scoop
        directories::BaseDirs::new().map(|b| {
            b.home_dir()
                .join("scoop")
                .join("shims")
                .join(&exe_name)
        }),
        // fnm
        directories::BaseDirs::new().map(|b| {
            b.home_dir()
                .join(".fnm")
                .join("aliases")
                .join("default")
                .join("Installation")
                .join("node.exe") // fnm manages node, not claude directly
        }),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 在常见目录中搜索
    let mut search_dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(b) = directories::BaseDirs::new() {
        search_dirs.push(b.home_dir().join("AppData").join("Roaming").join("npm"));
        search_dirs.push(b.home_dir().join(".local").join("bin"));
    }

    for dir in search_dirs {
        let target = dir.join(&exe_name);
        if target.exists() {
            return Some(target);
        }
        // 也检查不带 .exe 的（shim 脚本）
        let shim = dir.join(binary_name);
        if shim.exists() {
            return Some(shim);
        }
    }

    None
}

/// Unix 常见安装路径扫描
#[cfg(not(target_os = "windows"))]
fn find_in_unix_paths(binary_name: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let candidates = vec![
        // npm global
        std::path::PathBuf::from(&home)
            .join(".npm-global")
            .join("bin")
            .join(binary_name),
        // Homebrew
        std::path::PathBuf::from("/opt/homebrew/bin").join(binary_name),
        std::path::PathBuf::from("/usr/local/bin").join(binary_name),
        // nvm
        std::path::PathBuf::from(&home)
            .join(".nvm")
            .join("versions")
            .join("node"),
        // fnm
        std::path::PathBuf::from(&home)
            .join(".local")
            .join("share")
            .join("fnm")
            .join("node-versions"),
        // Local bin
        std::path::PathBuf::from(&home)
            .join(".local")
            .join("bin")
            .join(binary_name),
        // Cargo bin
        std::path::PathBuf::from(&home)
            .join(".cargo")
            .join("bin")
            .join(binary_name),
    ];

    for candidate in candidates {
        if candidate.exists() && !candidate.is_dir() {
            return Some(candidate);
        }
    }

    None
}

/// 获取Agent版本
fn get_version(path: &PathBuf) -> Option<String> {
    let output = create_command_no_window(path)
        .arg("--version")
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_version(&stdout)
    } else {
        None
    }
}

/// 真实探测二进制可用性（P1-A2）：执行 `--version` 并解析版本。
///
/// 与 `Path::exists` 的区别：能发现"文件在但已损坏/不可执行"的情况。
/// 超时保护 5 秒（子线程阻塞执行 + 主线程限时等待），避免坏二进制挂死 verify。
pub fn probe_binary(binary_path: &str) -> Option<String> {
    use std::path::PathBuf;
    use std::process::Command;

    let path = PathBuf::from(binary_path);
    if !path.exists() {
        return None;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut cmd = Command::new(&path);
        cmd.arg("--version");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = tx.send(cmd.output());
    });

    match rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(output)) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_version(&stdout)
        }
        // 超时或失败：探测不到版本（不单独据此判不可用，交由上层综合判断）
        _ => None,
    }
}

/// 解析版本号
fn parse_version(output: &str) -> Option<String> {
    use regex::Regex;

    // 尝试多种版本号格式
    let patterns = [
        // v2.1.266 / 2.1.266 / v2.1.266-beta
        r"v?(\d+\.\d+\.\d+[-\w]*)",
        // version 2.1.266
        r"[Vv]ersion[:\s]+v?(\d+\.\d+\.\d+[-\w]*)",
        // Claude Code v2.1.266
        r"(\d+\.\d+\.\d+[-\w]*)",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(output) {
                if let Some(version) = caps.get(1) {
                    let v = version.as_str().trim_start_matches('v');
                    return Some(v.to_string());
                }
            }
        }
    }

    // 降级：返回第一行非空内容
    output.lines()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 探测Agent能力
fn probe_capabilities(path: &PathBuf) -> AgentCapabilities {
    let output = create_command_no_window(path)
        .arg("--help")
        .output()
        .ok();

    let help_text = output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // flavor 与 L2/L3 能力以适配器注册表为单一事实来源（多 Agent 适配 MA-01）
    let path_str = path.to_string_lossy().to_string();
    let adapter = crate::agent_adapter::adapter_for(&path_str);
    let caps = adapter.capabilities();

    AgentCapabilities {
        mcp_support: help_text.contains("--mcp") || help_text.contains("mcp"),
        worktree_support: help_text.contains("--worktree") || help_text.contains("worktree"),
        interactive: true,
        flavor: adapter.flavor().as_str().to_string(),
        structured_events: caps.structured,
        one_shot: caps.one_shot,
    }
}

/// 检查Agent是否可用
pub async fn check_agent_available(agent_path: &str) -> bool {
    debug!("检查Agent可用性: {}", agent_path);

    let path = PathBuf::from(agent_path);
    let result = tokio::task::spawn_blocking(move || {
        create_command_no_window(&path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false);

    debug!("Agent可用性检查结果: {}", result);
    result
}
