use tracing::info;

/// Git 服务 - 处理 Git 操作的业务逻辑
pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        Self
    }

    /// 校验 work_dir 是否在允许的工作目录范围内
    fn validate_work_dir(work_dir: &str) -> Result<(), String> {
        let path = std::path::Path::new(work_dir);
        if !path.exists() {
            return Err(format!("工作目录不存在: {}", work_dir));
        }
        if !path.is_dir() {
            return Err(format!("路径不是目录: {}", work_dir));
        }
        // 检查是否是 git 仓库
        if !path.join(".git").exists() {
            return Err(format!("不是 Git 仓库: {}", work_dir));
        }
        Ok(())
    }

    /// 统一构造 git 子进程：绑定工作目录并抑制控制台窗口。
    ///
    /// GUI 应用（windows_subsystem="windows"）派生的控制台子进程会弹出
    /// 黑色控制台窗口，必须显式抑制（与 agent_adapter/agent 同策略）。
    fn git_command(work_dir: &str) -> std::process::Command {
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(work_dir);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd
    }

    /// 获取当前分支名
    fn get_current_branch(work_dir: &str) -> String {
        Self::git_command(work_dir)
            .arg("branch")
            .arg("--show-current")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// 获取 Git 工作目录状态
    pub fn get_status(&self, work_dir: &str) -> Result<serde_json::Value, String> {
        info!("获取 Git 状态: work_dir={}", work_dir);
        Self::validate_work_dir(work_dir)?;

        let output = Self::git_command(work_dir)
            .arg("status")
            .arg("--porcelain")
            .output()
            .map_err(|e| format!("执行 git status 失败: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

        let files: Vec<serde_json::Value> = lines.iter().map(|line| {
            let status = line.get(..2).unwrap_or("  ").trim();
            let path = line.get(3..).unwrap_or("");
            serde_json::json!({
                "status": status,
                "path": path,
            })
        }).collect();

        Ok(serde_json::json!({
            "branch": Self::get_current_branch(work_dir),
            "files": files,
            "total_changes": files.len(),
        }))
    }

    /// 获取 Git Diff
    pub fn get_diff(&self, work_dir: &str, file_path: Option<&str>) -> Result<String, String> {
        info!("获取 Git Diff: work_dir={}", work_dir);
        Self::validate_work_dir(work_dir)?;

        let mut cmd = Self::git_command(work_dir);
        cmd.arg("diff");

        if let Some(path) = file_path {
            cmd.arg("--").arg(path);
        }

        let output = cmd
            .current_dir(work_dir)
            .output()
            .map_err(|e| format!("执行 git diff 失败: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 创建 Git 提交
    ///
    /// `include_untracked = true` 时使用 `git add -A`，可提交 Agent 新增的
    /// 未跟踪文件；否则沿用 `git add -u`，仅暂存已跟踪文件的修改，
    /// 避免误提交密钥等敏感文件。
    pub fn create_commit(
        &self,
        work_dir: &str,
        message: &str,
        include_untracked: bool,
    ) -> Result<serde_json::Value, String> {
        info!(
            "创建 Git 提交: work_dir={}, message={}, include_untracked={}",
            work_dir, message, include_untracked
        );
        Self::validate_work_dir(work_dir)?;

        let add_flag = if include_untracked { "-A" } else { "-u" };
        Self::git_command(work_dir)
            .arg("add")
            .arg(add_flag)
            .output()
            .map_err(|e| format!("执行 git add 失败: {}", e))?;

        let output = Self::git_command(work_dir)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .output()
            .map_err(|e| format!("执行 git commit 失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(format!("Git commit 失败: {}", stderr));
        }

        let hash_output = Self::git_command(work_dir)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .map_err(|e| format!("获取 commit hash 失败: {}", e))?;

        let commit_hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();

        let diff_output = Self::git_command(work_dir)
            .arg("diff")
            .arg("--stat")
            .arg("HEAD~1")
            .output()
            .ok();

        let diff_stat = diff_output
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let files_changed = diff_stat.lines().count().saturating_sub(1) as i32;

        info!("Git 提交成功: hash={}", commit_hash);

        Ok(serde_json::json!({
            "commit_hash": commit_hash,
            "short_hash": &commit_hash[..7.min(commit_hash.len())],
            "message": message,
            "files_changed": files_changed,
            "diff_stat": diff_stat,
        }))
    }

    /// 获取 Git 日志
    pub fn log(&self, work_dir: &str, limit: Option<i32>) -> Result<Vec<serde_json::Value>, String> {
        Self::validate_work_dir(work_dir)?;

        let count = limit.unwrap_or(50);
        let output = Self::git_command(work_dir)
            .arg("log")
            .arg(format!("-{}", count))
            .arg("--format=%H|%h|%s|%an|%ai")
            .output()
            .map_err(|e| format!("执行 git log 失败: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let commits: Vec<serde_json::Value> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(5, '|').collect();
                if parts.len() < 5 {
                    return None;
                }
                Some(serde_json::json!({
                    "hash": parts[0],
                    "short_hash": parts[1],
                    "message": parts[2],
                    "author": parts[3],
                    "date": parts[4],
                }))
            })
            .collect();

        Ok(commits)
    }
}
