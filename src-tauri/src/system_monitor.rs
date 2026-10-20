use serde::{Deserialize, Serialize};
use sysinfo::{System, SystemExt, CpuExt, ProcessExt, Pid};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

/// 系统监控数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMonitorData {
    /// CPU使用率（百分比）
    pub cpu_usage: f32,
    /// 应用内存使用量（MB）- RSS
    pub memory_used_mb: u64,
    /// 应用内存使用率（百分比）
    pub memory_usage_percent: f32,
    /// 运行中的会话数
    pub active_sessions: u32,
    /// 应用启动时间（秒）
    pub uptime_seconds: u64,
}

/// 系统监控器
pub struct SystemMonitor {
    system: Arc<Mutex<System>>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }

    /// 获取系统监控数据（应用级）
    pub async fn get_data(&self) -> SystemMonitorData {
        let mut sys = self.system.lock().await;
        sys.refresh_all();

        // 获取当前进程的CPU使用率
        let cpu_usage = sys.global_cpu_info().cpu_usage();

        // 获取当前进程的内存使用量（RSS）
        let pid = match sysinfo::get_current_pid() {
            Ok(pid) => pid,
            Err(_) => Pid::from(0),
        };
        let (memory_used_mb, memory_usage_percent) = if let Some(process) = sys.process(pid) {
            let memory_bytes = process.memory(); // RSS in bytes
            let memory_mb = memory_bytes / (1024 * 1024);
            // 计算相对于系统总内存的使用率
            let total_memory = sys.total_memory();
            let usage_percent = if total_memory > 0 {
                (memory_bytes as f32 / total_memory as f32) * 100.0
            } else {
                0.0
            };
            (memory_mb, usage_percent)
        } else {
            (0, 0.0)
        };

        debug!(
            "应用监控: CPU={:.1}%, 内存={}MB ({:.1}%)",
            cpu_usage, memory_used_mb, memory_usage_percent
        );

        SystemMonitorData {
            cpu_usage,
            memory_used_mb,
            memory_usage_percent,
            active_sessions: 0, // 需要从外部传入
            uptime_seconds: sys.uptime(),
        }
    }

    /// 检查资源告警
    pub async fn check_alerts(&self) -> Vec<Alert> {
        let data = self.get_data().await;
        let mut alerts = Vec::new();

        // CPU告警
        if data.cpu_usage > 90.0 {
            alerts.push(Alert {
                level: AlertLevel::Critical,
                message: format!("CPU使用率过高: {:.1}%", data.cpu_usage),
            });
        } else if data.cpu_usage > 70.0 {
            alerts.push(Alert {
                level: AlertLevel::Warning,
                message: format!("CPU使用率较高: {:.1}%", data.cpu_usage),
            });
        }

        // 内存告警
        if data.memory_usage_percent > 90.0 {
            alerts.push(Alert {
                level: AlertLevel::Critical,
                message: format!("内存使用率过高: {:.1}%", data.memory_usage_percent),
            });
        } else if data.memory_usage_percent > 70.0 {
            alerts.push(Alert {
                level: AlertLevel::Warning,
                message: format!("内存使用率较高: {:.1}%", data.memory_usage_percent),
            });
        }

        alerts
    }
}

/// 告警级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Warning,
    Critical,
}

/// 告警信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub level: AlertLevel,
    pub message: String,
}

// 全局监控器实例
lazy_static::lazy_static! {
    static ref MONITOR: SystemMonitor = SystemMonitor::new();
}

/// Tauri命令：获取系统监控数据
#[tauri::command]
pub async fn get_monitor_data() -> Result<SystemMonitorData, String> {
    Ok(MONITOR.get_data().await)
}

/// Tauri命令：检查系统告警
#[tauri::command]
pub async fn check_alerts() -> Result<Vec<Alert>, String> {
    Ok(MONITOR.check_alerts().await)
}
