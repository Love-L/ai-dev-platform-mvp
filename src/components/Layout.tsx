import { Outlet, NavLink } from "react-router-dom";
import { motion } from "framer-motion";
import { useCallback, useEffect, useState } from "react";
import {
  LayoutDashboard,
  FileText,
  Columns,
  Bot,
  PanelLeftClose,
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
import { useI18n } from "../lib/i18n";

/** 侧边栏折叠状态持久化 key */
const SIDEBAR_COLLAPSED_KEY = "ace.sidebar.collapsed";

function loadCollapsed(): boolean {
  try {
    return localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === "1";
  } catch {
    return false;
  }
}

interface MonitorData {
  cpu_usage: number;
  memory_used_mb: number;
  memory_usage_percent: number;
  active_sessions: number;
  uptime_seconds: number;
}

/** 导航定义：labelKey 为 i18n key，label 为中文源文案（fallback） */
const navSections = [
  {
    labelKey: "nav.workspace",
    label: "工作区",
    items: [
      { to: "/", icon: LayoutDashboard, labelKey: "nav.dashboard", label: "工作台" },
      { to: "/projects", icon: Folder, labelKey: "nav.projects", label: "项目管理" },
      { to: "/requirements", icon: FileText, labelKey: "nav.requirements", label: "需求管理" },
      { to: "/board", icon: Columns, labelKey: "nav.board", label: "任务看板" },
      { to: "/agents", icon: Bot, labelKey: "nav.agents", label: "Agent" },
      { to: "/sessions", icon: Terminal, labelKey: "nav.sessions", label: "会话管理" },
      { to: "/skills", icon: Zap, labelKey: "nav.skills", label: "Skill 管理" },
      { to: "/commits", icon: GitCommit, labelKey: "nav.commits", label: "代码提交" },
    ],
  },
  {
    labelKey: "nav.data",
    label: "数据",
    items: [
      { to: "/statistics", icon: BarChart3, labelKey: "nav.statistics", label: "统计报表" },
      { to: "/cost", icon: Coins, labelKey: "nav.cost", label: "Token 消耗" },
      { to: "/audit-log", icon: History, labelKey: "nav.auditLog", label: "审计日志" },
      { to: "/import-export", icon: Download, labelKey: "nav.importExport", label: "导入导出" },
      { to: "/system-monitor", icon: Activity, labelKey: "nav.systemMonitor", label: "系统监控" },
    ],
  },
];

export default function Layout() {
  const { theme: _theme } = useThemeStore();
  const { t } = useI18n();
  const [monitorData, setMonitorData] = useState<MonitorData | null>(null);
  const [collapsed, setCollapsed] = useState<boolean>(loadCollapsed);

  const toggleCollapsed = useCallback(() => {
    setCollapsed((prev) => {
      const next = !prev;
      try {
        localStorage.setItem(SIDEBAR_COLLAPSED_KEY, next ? "1" : "0");
      } catch {
        // 忽略持久化失败
      }
      return next;
    });
  }, []);

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
        className={`flex flex-col flex-shrink-0 border-r overflow-hidden ${collapsed ? "w-[64px]" : "w-[220px]"}`}
        style={{
          background: "var(--color-surface)",
          borderColor: "var(--color-border)",
          transition: "width 200ms ease",
        }}
      >
        {/* Logo区域：展开态右侧为收起按钮；折叠态点击图标展开 */}
        <div
          className="h-14 flex items-center gap-2.5 border-b flex-shrink-0"
          style={{
            borderColor: "var(--color-border-subtle)",
            padding: collapsed ? "0 16px" : "0 16px",
          }}
        >
          <button
            onClick={toggleCollapsed}
            className="flex items-center gap-2.5 min-w-0 flex-1 h-full"
            title={collapsed ? "展开菜单" : "点击图标可收起"}
            aria-label={collapsed ? "展开侧边栏" : "收起侧边栏"}
          >
            <span
              className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
              style={{ background: "var(--color-primary)" }}
            >
              <Bot className="w-4 h-4 text-white" strokeWidth={2} />
            </span>
            {!collapsed && (
              <span className="text-left min-w-0">
                <h1 className="text-[13px] font-semibold leading-tight" style={{ color: "var(--color-text)" }}>
                  ACE
                </h1>
                <p className="text-[10px] leading-tight" style={{ color: "var(--color-text-tertiary)" }}>
                  v2.3.1
                </p>
              </span>
            )}
          </button>
          {!collapsed && (
            <button
              onClick={toggleCollapsed}
              className="p-1.5 rounded-lg flex-shrink-0"
              title="收起菜单"
              aria-label="收起侧边栏"
              style={{ color: "var(--color-text-tertiary)" }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "var(--color-bg-subtle)";
                e.currentTarget.style.color = "var(--color-text)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "";
                e.currentTarget.style.color = "";
              }}
            >
              <PanelLeftClose className="w-4 h-4" />
            </button>
          )}
        </div>

        {/* 导航菜单 */}
        <nav className="flex-1 overflow-y-auto py-3 px-2.5 space-y-5">
          {navSections.map((section) => (
            <div key={section.label}>
              {!collapsed && (
                <p
                  className="px-2.5 mb-1 text-[10px] font-semibold uppercase tracking-wider"
                  style={{ color: "var(--color-text-tertiary)" }}
                >
                  {t(section.labelKey, section.label)}
                </p>
              )}
              {collapsed && (
                <div className="mx-2 mb-2 border-t" style={{ borderColor: "var(--color-border-subtle)" }} />
              )}
              <div className="space-y-0.5">
                {section.items.map((item) => (
                  <NavLink
                    key={item.to}
                    to={item.to}
                    end={item.to === "/"}
                    title={collapsed ? t(item.labelKey, item.label) : undefined}
                    className={({ isActive }) =>
                      `group flex items-center rounded-lg text-[13px] font-medium transition-all duration-150 ${
                        collapsed ? "justify-center px-0 py-2" : "gap-2.5 px-2.5 py-[7px]"
                      } ${isActive ? "text-blue-600" : ""}`
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
                        {!collapsed && <span>{t(item.labelKey, item.label)}</span>}
                      </>
                    )}
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </nav>

        {/* 底部设置入口（语言切换已移入设置页） */}
        <div className="p-2.5 border-t flex-shrink-0" style={{ borderColor: "var(--color-border-subtle)" }}>
          <NavLink
            to="/settings"
            title={collapsed ? t("nav.settings", "设置") : undefined}
            className={() =>
              `group flex items-center rounded-lg text-[13px] font-medium transition-all duration-150 ${
                collapsed ? "justify-center px-0 py-2" : "gap-2.5 px-2.5 py-[7px]"
              }`
            }
            style={({ isActive }) => ({
              background: isActive ? "var(--color-bg-subtle)" : "transparent",
              color: isActive ? "var(--color-text)" : "var(--color-text-secondary)",
            })}
          >
            <Settings className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} strokeWidth={1.75} />
            {!collapsed && <span>{t("nav.settings", "设置")}</span>}
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
            <span>{t("status.ready", "就绪")}</span>
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
