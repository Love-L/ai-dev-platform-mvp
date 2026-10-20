use crate::pty::{PtyManager, PtySessionInfo};
use std::sync::Arc;
use tracing::{info, debug};

/// PTY 服务 - 处理终端会话管理的业务逻辑
pub struct PtyService<'a> {
    pty_manager: &'a Arc<PtyManager>,
}

impl<'a> PtyService<'a> {
    pub fn new(pty_manager: &'a Arc<PtyManager>) -> Self {
        Self { pty_manager }
    }

    /// 启动一个新的 PTY 会话
    pub async fn spawn(
        &self,
        agent_path: &str,
        work_dir: &str,
        env_vars: Vec<(String, String)>,
    ) -> Result<String, String> {
        info!("启动 PTY 会话: agent={}, work_dir={}", agent_path, work_dir);
        let session_id = self.pty_manager.spawn_agent(agent_path, work_dir, env_vars).await?;
        info!("PTY 会话启动成功: session_id={}", session_id);
        Ok(session_id)
    }

    /// 向 PTY 会话写入输入
    pub async fn write(&self, session_id: &str, input: &str) -> Result<(), String> {
        debug!("PTY 写入: session_id={}", session_id);
        self.pty_manager.write_input(session_id, input).await
    }

    /// 调整 PTY 会话终端大小
    pub async fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<(), String> {
        debug!("PTY 调整大小: session_id={}, rows={}, cols={}", session_id, rows, cols);
        self.pty_manager.resize(session_id, rows, cols).await
    }

    /// 终止 PTY 会话
    pub async fn kill(&self, session_id: &str) -> Result<(), String> {
        info!("终止 PTY 会话: session_id={}", session_id);
        self.pty_manager.kill(session_id).await
    }

    /// 获取所有 PTY 会话列表
    pub async fn get_sessions(&self) -> Vec<PtySessionInfo> {
        self.pty_manager.get_sessions().await
    }

    /// 获取指定 PTY 会话信息
    pub async fn get_session(&self, session_id: &str) -> Option<PtySessionInfo> {
        self.pty_manager.get_session(session_id).await
    }
}
