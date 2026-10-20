use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex as StdMutex};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{info, warn};

/// PTY 会话信息（对外只读视图）
#[derive(Debug, Clone, serde::Serialize)]
pub struct PtySessionInfo {
    pub id: String,
    pub agent_path: String,
    pub work_dir: String,
    pub status: String,
    pub started_at: String,
    pub exit_code: Option<i32>,
}

/// PTY 事件类型
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum PtyEvent {
    #[serde(rename = "data")]
    Data { session_id: String, data: String },
    #[serde(rename = "exit")]
    Exit { session_id: String, exit_code: i32 },
    #[serde(rename = "error")]
    Error { session_id: String, error: String },
}

/// PTY 会话元数据
#[derive(Debug, Clone)]
struct PtySessionMeta {
    agent_path: String,
    work_dir: String,
    status: String,
    started_at: String,
    exit_code: Option<i32>,
}

/// 单次 PTY 会话。
///
/// 关键点：这里**真实持有**子进程相关的句柄，因此
/// `write_input` / `resize` / `kill` 都能作用到操作系统进程，
/// 而不是仅更新内存状态。
pub struct PtySession {
    meta: StdMutex<PtySessionMeta>,
    /// 已取出的 stdin 写入端（`master.take_writer()`）
    writer: StdMutex<Option<Box<dyn Write + Send>>>,
    /// PTY master，用于 `resize`
    master: StdMutex<Option<Box<dyn MasterPty + Send>>>,
    /// 进程终止器（`child.clone_killer()`），可独立于 child 调用
    killer: StdMutex<Option<Box<dyn ChildKiller + Send + Sync>>>,
    /// 子进程退出码接收端，供调用方等待会话结束
    exit_rx: tokio::sync::Mutex<Option<oneshot::Receiver<i32>>>,
    /// 关联的数据库会话 id（`agent_sessions.id`），用于从数据库会话反查并终止进程
    db_session_id: StdMutex<Option<String>>,
    /// 输出回放缓冲（截断保尾，供终端视图挂载时补齐挂载前的历史输出）
    replay: StdMutex<String>,
}

impl PtySession {
    fn info(&self, id: &str) -> PtySessionInfo {
        let meta = self.meta.lock().unwrap_or_else(|e| e.into_inner());
        PtySessionInfo {
            id: id.to_string(),
            agent_path: meta.agent_path.clone(),
            work_dir: meta.work_dir.clone(),
            status: meta.status.clone(),
            started_at: meta.started_at.clone(),
            exit_code: meta.exit_code,
        }
    }
}

/// PTY 会话管理器
pub struct PtyManager {
    sessions: Arc<RwLock<HashMap<String, Arc<PtySession>>>>,
    event_sender: mpsc::Sender<PtyEvent>,
}

impl PtyManager {
    /// 创建新的 PTY 管理器
    pub fn new(event_sender: mpsc::Sender<PtyEvent>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        }
    }

    /// 创建新的 PTY 会话并启动 Agent
    pub async fn spawn_agent(
        &self,
        agent_path: &str,
        work_dir: &str,
        env_vars: Vec<(String, String)>,
    ) -> Result<String, String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        info!(
            "Creating PTY session {} for agent: {} (work_dir={})",
            session_id, agent_path, work_dir
        );

        // 1. 创建 PTY
        let pty_system = native_pty_system();
        let pty_size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = pty_system
            .openpty(pty_size)
            .map_err(|e| format!("创建 PTY 失败: {}", e))?;

        // 2. 构建命令
        let mut cmd = CommandBuilder::new(agent_path);
        cmd.cwd(work_dir);
        for (key, value) in &env_vars {
            cmd.env(key, value);
        }

        // 3. 启动子进程
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("启动 Agent 进程失败: {}", e))?;

        // 4. 取出真实句柄：killer / reader / writer
        let killer = child.clone_killer();
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("克隆 PTY reader 失败: {}", e))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("获取 PTY writer 失败: {}", e))?;

        // 5. 退出码通道
        let (exit_tx, exit_rx) = oneshot::channel::<i32>();

        let session = Arc::new(PtySession {
            meta: StdMutex::new(PtySessionMeta {
                agent_path: agent_path.to_string(),
                work_dir: work_dir.to_string(),
                status: "running".to_string(),
                started_at: chrono::Utc::now().to_rfc3339(),
                exit_code: None,
            }),
            writer: StdMutex::new(Some(writer)),
            master: StdMutex::new(Some(pair.master)),
            killer: StdMutex::new(Some(killer)),
            exit_rx: tokio::sync::Mutex::new(Some(exit_rx)),
            db_session_id: StdMutex::new(None),
            replay: StdMutex::new(String::new()),
        });

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session.clone());

        // 6. 阻塞线程：读取输出 + 等待进程退出
        let event_sender = self.event_sender.clone();
        let sid = session_id.clone();
        let thread_session = session.clone();

        tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        {
                            // 输出回放缓冲（截断保尾，避免无限增长）
                            const MAX_REPLAY: usize = 128 * 1024;
                            let mut replay = thread_session
                                .replay
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            replay.push_str(&text);
                            if replay.len() > MAX_REPLAY {
                                let mut cut = MAX_REPLAY;
                                while cut > 0 && !replay.is_char_boundary(cut) {
                                    cut -= 1;
                                }
                                replay.drain(..cut);
                            }
                        }
                        if event_sender
                            .blocking_send(PtyEvent::Data {
                                session_id: sid.clone(),
                                data: text,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock
                            && e.kind() != std::io::ErrorKind::Interrupted
                        {
                            warn!("PTY read error for session {}: {}", sid, e);
                            break;
                        }
                    }
                }
            }

            // 等待子进程真正退出，拿到真实退出码
            let exit_code: i32 = match child.wait() {
                Ok(status) => status.exit_code() as i32,
                Err(e) => {
                    warn!("等待 PTY 子进程退出失败: {}", e);
                    -1
                }
            };

            // 若为主动终止，保留 terminated 状态，不覆盖为 completed/failed
            let terminated = {
                let mut meta = thread_session.meta.lock().unwrap_or_else(|e| e.into_inner());
                if meta.status == "terminated" {
                    true
                } else {
                    meta.status = if exit_code == 0 {
                        "completed".to_string()
                    } else {
                        "failed".to_string()
                    };
                    meta.exit_code = Some(exit_code);
                    false
                }
            };

            if !terminated {
                let _ = event_sender.blocking_send(PtyEvent::Exit {
                    session_id: sid.clone(),
                    exit_code,
                });
            }

            // 通知等待方（无论是否主动终止都要放行，避免任务悬挂）
            let _ = exit_tx.send(if terminated { -1 } else { exit_code });
        });

        info!("PTY session {} created successfully", session_id);
        Ok(session_id)
    }

    /// 向 Agent 的 stdin 真实写入输入
    pub async fn write_input(&self, session_id: &str, input: &str) -> Result<(), String> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .cloned()
                .ok_or_else(|| format!("Session {} not found", session_id))?
        };

        // 注意：以下均为同步临界区，不跨 await 持有锁
        let mut guard = session
            .writer
            .lock()
            .map_err(|_| "PTY 写入锁已损坏".to_string())?;
        let writer = guard
            .as_mut()
            .ok_or_else(|| "PTY 写入通道已关闭".to_string())?;

        writer
            .write_all(input.as_bytes())
            .map_err(|e| format!("写入 PTY 失败: {}", e))?;
        writer
            .flush()
            .map_err(|e| format!("刷新 PTY 缓冲区失败: {}", e))?;

        Ok(())
    }

    /// 调整终端大小（真实作用到 PTY）
    pub async fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<(), String> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .cloned()
                .ok_or_else(|| format!("Session {} not found", session_id))?
        };

        let guard = session
            .master
            .lock()
            .map_err(|_| "PTY master 锁已损坏".to_string())?;
        let master = guard
            .as_ref()
            .ok_or_else(|| "PTY master 已释放".to_string())?;

        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("调整 PTY 尺寸失败: {}", e))?;

        info!("PTY resize for session {}: {}x{}", session_id, rows, cols);
        Ok(())
    }

    /// 终止 Agent 进程（真实 kill 操作系统进程）
    pub async fn kill(&self, session_id: &str) -> Result<(), String> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .cloned()
                .ok_or_else(|| format!("Session {} not found", session_id))?
        };

        // 先标记状态，避免读线程把 terminated 覆盖为 failed
        {
            let mut meta = session.meta.lock().unwrap_or_else(|e| e.into_inner());
            meta.status = "terminated".to_string();
            meta.exit_code = Some(-1);
        }

        {
            let mut guard = session
                .killer
                .lock()
                .map_err(|_| "PTY killer 锁已损坏".to_string())?;
            if let Some(killer) = guard.as_mut() {
                killer
                    .kill()
                    .map_err(|e| format!("终止 PTY 进程失败: {}", e))?;
            }
        }

        // 关闭写入端，促使子进程尽快退出
        if let Ok(mut w) = session.writer.lock() {
            *w = None;
        }

        info!("PTY session {} terminated", session_id);
        Ok(())
    }

    /// 绑定数据库会话 id（`agent_sessions.id`）到 PTY 会话，
    /// 以便从数据库会话反查并终止真实进程。
    pub async fn bind_db_session(&self, pty_session_id: &str, db_session_id: &str) {
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(pty_session_id).cloned()
        };

        if let Some(session) = session {
            if let Ok(mut guard) = session.db_session_id.lock() {
                *guard = Some(db_session_id.to_string());
            }
        }
    }

    /// 依据数据库会话 id 查找对应的 PTY 会话 id
    pub async fn find_pty_by_db_session(&self, db_session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        for (pty_id, session) in sessions.iter() {
            let matched = session
                .db_session_id
                .lock()
                .map(|guard| guard.as_deref() == Some(db_session_id))
                .unwrap_or(false);
            if matched {
                return Some(pty_id.clone());
            }
        }
        None
    }

    /// 等待会话结束，返回退出码（会话不存在或已被其它调用方等待时返回 None）
    pub async fn wait_exit(&self, session_id: &str) -> Option<i32> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id).cloned()
        }?;

        let rx = {
            let mut guard = session.exit_rx.lock().await;
            guard.take()
        }?;

        rx.await.ok()
    }

    /// 获取所有会话信息
    pub async fn get_sessions(&self) -> Vec<PtySessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .map(|(id, session)| session.info(id))
            .collect()
    }

    /// 附着：按数据库会话 id 解析真实 PTY id，并回放既有输出。
    ///
    /// 终端视图挂载时调用：PTY id 与数据库会话 id 不同源，前端须先换取真实
    /// PTY id 才能收发 `pty:event` / 调用 `write_input|resize|kill`；
    /// 第二个返回值为挂载前的输出缓冲。进程不存在或已退出时返回 Err，
    /// 由前端展示重启入口（`pty_reconnect`）。
    pub async fn attach_by_db_session(
        &self,
        db_session_id: &str,
    ) -> Result<(String, String), String> {
        let pty_id = self
            .find_pty_by_db_session(db_session_id)
            .await
            .ok_or_else(|| "Agent 进程未运行".to_string())?;

        let (running, replay) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&pty_id)
                .cloned()
                .ok_or_else(|| "Agent 进程未运行".to_string())?;
            let running = session
                .meta
                .lock()
                .map(|m| m.status == "running")
                .unwrap_or(false);
            let replay = session
                .replay
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default();
            (running, replay)
        };

        if !running {
            return Err("Agent 进程未运行".to_string());
        }
        Ok((pty_id, replay))
    }

    /// 返回当前仍处于 `running` 的 PTY 会话所绑定的数据库会话 id 集合。
    ///
    /// 用于"进程存活对账"：数据库中 `running` 但这些 id 不在集合内的会话，
    /// 说明其真实进程已不存在，应被纠正为终态。
    pub async fn live_db_session_ids(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|session| {
                session
                    .meta
                    .lock()
                    .map(|meta| meta.status == "running")
                    .unwrap_or(false)
            })
            .filter_map(|session| {
                session
                    .db_session_id
                    .lock()
                    .ok()
                    .and_then(|guard| guard.clone())
            })
            .collect()
    }

    /// 获取指定会话信息
    pub async fn get_session(&self, session_id: &str) -> Option<PtySessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|session| session.info(session_id))
    }
}
