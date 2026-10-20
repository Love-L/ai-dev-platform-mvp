import { Outlet, NavLink } from "react-router-dom";
import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import {
  LayoutDashboard,
  FileText,
  Columns,
  Bot,
  Settings,
  Coins,
  BarChart3,
  Terminal,
  GitCommit,
  Folder,
  Zap,
  History,
  Download,
  Activity,
} from "lucide-react";
import { useThemeStore } from "../stores/themeStore";
import { getMonitorData } from "../lib/tauri";

interface MonitorData {
  cpu_usage: number;
  memory_used_mb: number;
  memory_usage_percent: number;
  active_sessions: number;
  uptime_seconds: number;
}

const navSections = [
  {
    label: "工作区",
    items: [
      { to: "/", icon: LayoutDashboard, label: "工作台" },
      { to: "/projects", icon: Folder, label: "项目管理" },
      { to: "/requirements", icon: FileText, label: "需求管理" },
      { to: "/board", icon: Columns, label: "任务看板" },
      { to: "/agents", icon: Bot, label: "Agent" },
      { to: "/sessions", icon: Terminal, label: "会话管理" },
      { to: "/skills", icon: Zap, label: "Skill 管理" },
      { to: "/commits", icon: GitCommit, label: "代码提交" },
    ],
  },
  {
    label: "数据",
    items: [
      { to: "/statistics", icon: BarChart3, label: "统计报表" },
      { to: "/cost", icon: Coins, label: "Token 消耗" },
      { to: "/audit-log", icon: History, label: "审计日志" },
      { to: "/import-export", icon: Download, label: "导入导出" },
      { to: "/system-monitor", icon: Activity, label: "系统监控" },
    ],
  },
];

export default function Layout() {
  const { theme: _theme } = useThemeStore();
  const [monitorData, setMonitorData] = useState<MonitorData | null>(null);

  useEffect(() => {
    const fetchMonitorData = async () => {
      try {
        const data = await getMonitorData();
        setMonitorData(data);
      } catch (error) {
        console.error("获取监控数据失败:", error);
      }
    };

    fetchMonitorData();
    const interval = setInterval(fetchMonitorData, 5000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex h-screen overflow-hidden" style={{ background: "var(--color-bg)" }}>
      {/* 侧边栏 */}
      <aside
        className="w-[220px] flex flex-col flex-shrink-0 border-r"
        style={{
          background: "var(--color-surface)",
          borderColor: "var(--color-border)",
        }}
      >
        {/* Logo区域 */}
        <div
          className="h-14 flex items-center gap-2.5 px-4 border-b"
          style={{ borderColor: "var(--color-border-subtle)" }}
        >
          <div
            className="w-8 h-8 rounded-lg flex items-center justify-center"
            style={{ background: "var(--color-primary)" }}
          >
            <Bot className="w-4 h-4 text-white" strokeWidth={2} />
          </div>
          <div>
            <h1 className="text-[13px] font-semibold leading-tight" style={{ color: "var(--color-text)" }}>
              ACE
            </h1>
            <p className="text-[10px] leading-tight" style={{ color: "var(--color-text-tertiary)" }}>
              v2.3.1
            </p>
          </div>
        </div>

        {/* 导航菜单 */}
        <nav className="flex-1 overflow-y-auto py-3 px-2.5 space-y-5">
          {navSections.map((section) => (
            <div key={section.label}>
              <p
                className="px-2.5 mb-1 text-[10px] font-semibold uppercase tracking-wider"
                style={{ color: "var(--color-text-tertiary)" }}
              >
                {section.label}
              </p>
              <div className="space-y-0.5">
                {section.items.map((item) => (
                  <NavLink
                    key={item.to}
                    to={item.to}
                    end={item.to === "/"}
                    className={({ isActive }) =>
                      `group flex items-center gap-2.5 px-2.5 py-[7px] rounded-lg text-[13px] font-medium transition-all duration-150 ${
                        isActive ? "text-blue-600" : ""
                      }`
                    }
                    style={({ isActive }) => ({
                      background: isActive ? "var(--color-primary-subtle)" : "transparent",
                      color: isActive ? "var(--color-primary)" : "var(--color-text-secondary)",
                    })}
                  >
                    {({ isActive }) => (
                      <>
                        <item.icon
                          className="w-4 h-4 flex-shrink-0"
                          style={{
                            color: isActive ? "var(--color-primary)" : "var(--color-text-tertiary)",
                          }}
                          strokeWidth={1.75}
                        />
                        <span>{item.label}</span>
                      </>
                    )}
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </nav>

        {/* 底部设置入口 */}
        <div className="p-2.5 border-t" style={{ borderColor: "var(--color-border-subtle)" }}>
          <NavLink
            to="/settings"
            className={() =>
              `group flex items-center gap-2.5 px-2.5 py-[7px] rounded-lg text-[13px] font-medium transition-all duration-150`
            }
            style={({ isActive }) => ({
              background: isActive ? "var(--color-bg-subtle)" : "transparent",
              color: isActive ? "var(--color-text)" : "var(--color-text-secondary)",
            })}
          >
            <Settings className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} strokeWidth={1.75} />
            <span>设置</span>
          </NavLink>
        </div>
      </aside>

      {/* 主内容区 */}
      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        <main className="flex-1 overflow-auto">
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.15 }}>
            <Outlet />
          </motion.div>
        </main>

        {/* 状态栏 */}
        <div
          className="h-7 border-t flex items-center justify-between px-4 text-[11px]"
          style={{
            background: "var(--color-surface)",
            borderColor: "var(--color-border-subtle)",
            color: "var(--color-text-tertiary)",
          }}
        >
          <div className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
            <span>就绪</span>
          </div>
          <div className="flex items-center gap-3.5">
            {monitorData && (
              <>
                <span>CPU {monitorData.cpu_usage.toFixed(1)}%</span>
                <span className="flex items-center gap-1">
                  <Activity className="w-3 h-3" />
                  {monitorData.memory_used_mb} MB
                </span>
                <span className="flex items-center gap-1">
                  <Terminal className="w-3 h-3" />
                  {monitorData.active_sessions}
                </span>
              </>
            )}
            <span className="font-mono text-[10px]">v2.3.1</span>
          </div>
        </div>
      </div>
    </div>
  );
}
