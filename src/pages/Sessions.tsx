import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Archive,
  ArchiveRestore,
  Bot,
  CheckCircle,
  Clock,
  Download,
  Eye,
  Filter,
  LayoutGrid,
  Link2,
  Pencil,
  RefreshCw,
  Search,
  Square,
  Table2,
  Terminal,
  Trash2,
  XCircle,
} from "lucide-react";
import type { AgentSession, Task } from "../lib/types";
import { useSessionsStore, useProjectsStore } from "../stores";
import {
  deleteAgentSession,
  exportAgentSession,
  linkSessionTask,
  listTasks,
  renameAgentSession,
  setAgentSessionArchived,
} from "../lib/tauri";
import AgentTerminal from "../components/AgentTerminal";
import { showToast } from "../components/Toast";

/** 行内操作按钮（统一 hover 样式） */
function RowAction({
  title,
  onClick,
  danger,
  children,
}: {
  title: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className="p-1.5 rounded-lg"
      style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
      onMouseEnter={(e) => {
        e.currentTarget.style.color = danger ? "var(--color-danger)" : "var(--color-primary)";
        e.currentTarget.style.background = danger ? "var(--color-danger-light)" : "var(--color-primary-subtle)";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.color = "";
        e.currentTarget.style.background = "";
      }}
      aria-label={title}
      title={title}
    >
      {children}
    </button>
  );
}

export default function Sessions() {
  const navigate = useNavigate();
  const { sessions, loading, fetchSessions, stopSession, showArchived, setShowArchived, ensureStatusSubscription } =
    useSessionsStore();
  const [refreshing, setRefreshing] = useState(false);
  const [filterStatus, setFilterStatus] = useState<string>("all");
  const [projectFilter, setProjectFilter] = useState<string>("");
  const [searchQuery, setSearchQuery] = useState("");
  const projects = useProjectsStore((s) => s.projects);
  const fetchProjects = useProjectsStore((s) => s.fetchProjects);
  const [layout, setLayout] = useState<"table" | "cards">("table");
  const [selectedSession, setSelectedSession] = useState<AgentSession | null>(null);
  const [showTerminal, setShowTerminal] = useState(false);
  // 关联开发任务弹窗
  const [linkTarget, setLinkTarget] = useState<AgentSession | null>(null);
  const [taskOptions, setTaskOptions] = useState<Task[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState("");
  const [loadingTasks, setLoadingTasks] = useState(false);
  const [linking, setLinking] = useState(false);
  // 重命名弹窗
  const [renameTarget, setRenameTarget] = useState<AgentSession | null>(null);
  const [renameValue, setRenameValue] = useState("");
  // 删除确认弹窗
  const [deleteTarget, setDeleteTarget] = useState<AgentSession | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    void fetchSessions();
    void fetchProjects();
    // 订阅 session:status 自动刷新（P1-12）
    let release: (() => void) | undefined;
    let disposed = false;
    ensureStatusSubscription().then((fn) => {
      if (disposed) fn();
      else release = fn;
    });
    return () => {
      disposed = true;
      release?.();
    };
  }, [fetchSessions, ensureStatusSubscription]);

  async function handleRefresh() {
    setRefreshing(true);
    try {
      await fetchSessions();
    } finally {
      setRefreshing(false);
    }
  }

  async function handleTerminateSession(sessionId: string) {
    try {
      await stopSession(sessionId);
    } catch (error) {
      console.error("终止会话失败:", error);
    }
  }

  function handleViewTerminal(session: AgentSession) {
    setSelectedSession(session);
    setShowTerminal(true);
  }

  /** 打开"关联开发任务"弹窗：加载任务列表并预选当前关联 */
  async function openLinkModal(session: AgentSession) {
    setLinkTarget(session);
    setSelectedTaskId(session.related_task_id || session.task_id || "");
    setLoadingTasks(true);
    try {
      setTaskOptions(await listTasks());
    } catch (error) {
      console.error("加载任务列表失败:", error);
      showToast("error", `加载任务列表失败: ${error}`);
      setLinkTarget(null);
    } finally {
      setLoadingTasks(false);
    }
  }

  /** 确认关联（空值 = 解除关联）并刷新会话列表 */
  async function handleLinkConfirm() {
    if (!linkTarget) return;
    setLinking(true);
    try {
      await linkSessionTask(linkTarget.id, selectedTaskId || null);
      showToast("success", selectedTaskId ? "已关联开发任务" : "已解除任务关联");
      setLinkTarget(null);
      await fetchSessions();
    } catch (error) {
      console.error("关联任务失败:", error);
      showToast("error", `关联任务失败: ${error}`);
    } finally {
      setLinking(false);
    }
  }

  /** 重命名（P1-6） */
  async function handleRenameConfirm() {
    if (!renameTarget) return;
    try {
      await renameAgentSession(renameTarget.id, renameValue);
      showToast("success", "已重命名");
      setRenameTarget(null);
      await fetchSessions();
    } catch (error) {
      showToast("error", `重命名失败: ${error}`);
    }
  }

  /** 归档 / 取消归档（P1-7） */
  async function handleToggleArchive(session: AgentSession) {
    try {
      const next = session.archived !== 1;
      await setAgentSessionArchived(session.id, next);
      showToast("success", next ? "已归档" : "已取消归档");
      await fetchSessions();
    } catch (error) {
      showToast("error", `操作失败: ${error}`);
    }
  }

  /** 导出 Markdown（P1-7） */
  async function handleExport(session: AgentSession) {
    try {
      const path = await exportAgentSession(session.id);
      showToast("success", `已导出：${path}`);
    } catch (error) {
      showToast("error", `导出失败: ${error}`);
    }
  }

  /** 删除（P1-7） */
  async function handleDeleteConfirm() {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await deleteAgentSession(deleteTarget.id);
      showToast("success", "会话已删除");
      setDeleteTarget(null);
      await fetchSessions();
    } catch (error) {
      showToast("error", `删除失败: ${error}`);
    } finally {
      setDeleting(false);
    }
  }

  // 过滤会话
  const filteredSessions = sessions.filter((session) => {
    if (filterStatus !== "all" && session.status !== filterStatus) {
      return false;
    }
    if (projectFilter && session.project_id !== projectFilter) {
      return false;
    }
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        session.id.toLowerCase().includes(query) ||
        (session.title || "").toLowerCase().includes(query) ||
        session.agent_name?.toLowerCase().includes(query) ||
        session.task_title?.toLowerCase().includes(query)
      );
    }
    return true;
  });

  // 统计数据
  const stats = {
    total: sessions.length,
    running: sessions.filter((s) => s.status === "running").length,
    completed: sessions.filter((s) => s.status === "completed").length,
    failed: sessions.filter((s) => s.status === "failed").length,
  };

  /** 列表展示标题：title → 任务名 → agent 兜底（P1-6 告别 UUID） */
  function displayTitle(session: AgentSession): string {
    return (
      session.title?.trim() ||
      session.task_title?.trim() ||
      `${session.agent_name || session.agent_path || "Agent"} 会话`
    );
  }

  function getStatusIcon(status: string) {
    switch (status) {
      case "running":
        return <div className="w-2 h-2 rounded-full animate-pulse" style={{ background: "var(--color-success)" }} />;
      case "completed":
        return <CheckCircle className="w-4 h-4" style={{ color: "var(--color-success)" }} />;
      case "failed":
        return <XCircle className="w-4 h-4" style={{ color: "var(--color-danger)" }} />;
      case "terminated":
        return <Square className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />;
      default:
        return <Clock className="w-4 h-4" style={{ color: "var(--color-warning)" }} />;
    }
  }

  function getStatusText(status: string) {
    switch (status) {
      case "created":
        return "已创建";
      case "launching":
        return "启动中";
      case "running":
        return "运行中";
      case "completed":
        return "已完成";
      case "failed":
        return "失败";
      case "terminated":
        return "已终止";
      default:
        return status;
    }
  }

  function formatDuration(seconds?: number) {
    if (!seconds) return "-";
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s`;
  }

  function formatTokens(tokens?: number) {
    if (!tokens) return "0";
    if (tokens >= 1000000) {
      return `${(tokens / 1000000).toFixed(1)}M`;
    }
    if (tokens >= 1000) {
      return `${(tokens / 1000).toFixed(1)}k`;
    }
    return tokens.toString();
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-sm" style={{ color: "var(--color-text-tertiary)" }}>
          <div
            className="w-4 h-4 border-2 rounded-full animate-spin"
            style={{ borderColor: "var(--color-border)", borderTopColor: "var(--color-primary)" }}
          />
          加载中...
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5">
      {/* 页面标题 */}
      <motion.div
        initial={{ opacity: 0, y: -6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2 }}
        className="flex items-center justify-between"
      >
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            会话管理
          </h1>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
            查看和管理 Agent 执行会话
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setLayout(layout === "table" ? "cards" : "table")}
            className="btn-secondary"
            title={layout === "table" ? "切换为卡片视图" : "切换为表格视图"}
          >
            {layout === "table" ? <LayoutGrid className="w-4 h-4" /> : <Table2 className="w-4 h-4" />}
            {layout === "table" ? "卡片" : "表格"}
          </button>
          <button onClick={handleRefresh} disabled={refreshing} className="btn-secondary" aria-label="刷新会话列表">
            <RefreshCw className={`w-4 h-4 ${refreshing ? "animate-spin" : ""}`} />
            刷新
          </button>
        </div>
      </motion.div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        {[
          { label: "总会话", value: stats.total, color: "var(--color-text)" },
          { label: "运行中", value: stats.running, color: "var(--color-success)" },
          { label: "已完成", value: stats.completed, color: "var(--color-primary)" },
          { label: "失败", value: stats.failed, color: "var(--color-danger)" },
        ].map((card, i) => (
          <motion.div
            key={card.label}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.2, delay: i * 0.04 }}
            className="card p-4"
          >
            <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
              {card.label}
            </p>
            <p className="text-lg font-semibold mt-1" style={{ color: card.color }}>
              {card.value}
            </p>
          </motion.div>
        ))}
      </div>

      {/* 筛选和搜索 */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center gap-3">
        <div className="flex items-center gap-2">
          <Filter className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
          <select value={filterStatus} onChange={(e) => setFilterStatus(e.target.value)} className="select w-auto">
            <option value="all">全部状态</option>
            <option value="running">运行中</option>
            <option value="completed">已完成</option>
            <option value="failed">失败</option>
            <option value="terminated">已终止</option>
          </select>
          <label
            className="flex items-center gap-1.5 text-sm cursor-pointer select-none"
            style={{ color: "var(--color-text-secondary)" }}
            title="归档会话默认隐藏，数据与消息全部保留"
          >
            <input type="checkbox" checked={showArchived} onChange={(e) => void setShowArchived(e.target.checked)} />
            显示已归档
          </label>
          <select
            value={projectFilter}
            onChange={(e) => setProjectFilter(e.target.value)}
            className="select w-auto text-xs"
            style={{ minWidth: 120 }}
            title="按项目过滤会话"
          >
            <option value="">全部项目</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </div>

        <div className="flex-1 w-full sm:w-auto relative">
          <Search
            className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4"
            style={{ color: "var(--color-text-tertiary)" }}
          />
          <input
            type="text"
            placeholder="搜索标题 / Agent / 任务..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input pl-9"
          />
        </div>
      </div>

      {/* 会话列表：卡片墙（P2-17）/ 表格 */}
      {layout === "cards" ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
          {filteredSessions.length === 0 ? (
            <div className="col-span-full card p-10 text-center">
              <Terminal className="w-10 h-10 mx-auto mb-3" style={{ color: "var(--color-text-tertiary)" }} />
              <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                暂无会话记录
              </p>
              <p className="text-xs mt-1" style={{ color: "var(--color-text-tertiary)" }}>
                Agent 启动后会话将显示在此处
              </p>
            </div>
          ) : (
            filteredSessions.map((session, i) => (
              <motion.div
                key={session.id}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.15, delay: Math.min(i * 0.02, 0.2) }}
                className="card p-4 flex flex-col gap-2.5"
              >
                <div className="flex items-start justify-between gap-2">
                  <button
                    onClick={() => navigate(`/sessions/${session.id}`)}
                    className="text-sm font-medium text-left hover:underline line-clamp-2"
                    style={{ color: "var(--color-text)" }}
                    title={session.id}
                  >
                    {displayTitle(session)}
                  </button>
                  <span className="font-mono text-[10px] shrink-0" style={{ color: "var(--color-text-tertiary)" }}>
                    {session.id.slice(0, 8)}
                  </span>
                </div>
                <div className="flex items-center gap-2 text-xs" style={{ color: "var(--color-text-secondary)" }}>
                  {getStatusIcon(session.status)}
                  <span>{getStatusText(session.status)}</span>
                  <span className="flex items-center gap-1 ml-1">
                    <Bot className="w-3.5 h-3.5" style={{ color: "var(--color-primary)" }} />
                    {session.agent_name || "Unknown"}
                  </span>
                  {session.archived === 1 && (
                    <span
                      className="text-[10px] px-1.5 py-0.5 rounded"
                      style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-tertiary)" }}
                    >
                      已归档
                    </span>
                  )}
                </div>
                <div
                  className="flex items-center gap-3 text-[11px] pt-1"
                  style={{ color: "var(--color-text-tertiary)", borderTop: "1px solid var(--color-border-subtle)" }}
                >
                  <span>{formatDuration(session.duration_seconds)}</span>
                  <span>{formatTokens(session.total_tokens)} tokens</span>
                  <span>${session.total_cost?.toFixed(4) || "0.00"}</span>
                  <span className="ml-auto">
                    {session.started_at ? new Date(session.started_at).toLocaleDateString("zh-CN") : ""}
                  </span>
                </div>
                <div className="flex items-center justify-end gap-0.5">
                  <RowAction title="进入会话" onClick={() => navigate(`/sessions/${session.id}`)}>
                    <Eye className="w-4 h-4" />
                  </RowAction>
                  {session.status === "running" && (
                    <>
                      <RowAction title="快速打开终端" onClick={() => handleViewTerminal(session)}>
                        <Terminal className="w-4 h-4" />
                      </RowAction>
                      <RowAction title="终止会话" onClick={() => handleTerminateSession(session.id)} danger>
                        <Square className="w-4 h-4" />
                      </RowAction>
                    </>
                  )}
                  <RowAction
                    title="重命名"
                    onClick={() => {
                      setRenameTarget(session);
                      setRenameValue(session.title || "");
                    }}
                  >
                    <Pencil className="w-4 h-4" />
                  </RowAction>
                  <RowAction
                    title={session.archived === 1 ? "取消归档" : "归档"}
                    onClick={() => handleToggleArchive(session)}
                  >
                    {session.archived === 1 ? <ArchiveRestore className="w-4 h-4" /> : <Archive className="w-4 h-4" />}
                  </RowAction>
                  <RowAction title="导出为 Markdown" onClick={() => handleExport(session)}>
                    <Download className="w-4 h-4" />
                  </RowAction>
                  <RowAction title="删除会话" onClick={() => setDeleteTarget(session)} danger>
                    <Trash2 className="w-4 h-4" />
                  </RowAction>
                </div>
              </motion.div>
            ))
          )}
        </div>
      ) : (
        <div className="table-container">
          <table className="table">
            <thead>
              <tr>
                <th scope="col">会话</th>
                <th scope="col">Agent</th>
                <th scope="col">任务</th>
                <th scope="col">状态</th>
                <th scope="col">时长</th>
                <th scope="col">Token</th>
                <th scope="col">费用</th>
                <th scope="col">开始时间</th>
                <th scope="col" className="text-right">
                  操作
                </th>
              </tr>
            </thead>
            <tbody>
              {filteredSessions.length === 0 ? (
                <tr>
                  <td colSpan={9} className="text-center py-12">
                    <Terminal className="w-10 h-10 mx-auto mb-3" style={{ color: "var(--color-text-tertiary)" }} />
                    <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                      暂无会话记录
                    </p>
                    <p className="text-xs mt-1" style={{ color: "var(--color-text-tertiary)" }}>
                      Agent 启动后会话将显示在此处
                    </p>
                  </td>
                </tr>
              ) : (
                filteredSessions.map((session) => (
                  <tr key={session.id} style={session.archived === 1 ? { opacity: 0.6 } : undefined}>
                    <td>
                      {/* P1-6：标题优先，UUID 降级为悬浮提示 */}
                      <button
                        onClick={() => navigate(`/sessions/${session.id}`)}
                        className="text-sm font-medium text-left hover:underline max-w-[220px] truncate block"
                        style={{ color: "var(--color-text)" }}
                        title={`${displayTitle(session)}\n${session.id}`}
                      >
                        {displayTitle(session)}
                      </button>
                      <span className="font-mono text-[10px]" style={{ color: "var(--color-text-tertiary)" }}>
                        {session.id.slice(0, 8)}
                        {session.archived === 1 ? " · 已归档" : ""}
                      </span>
                    </td>
                    <td>
                      <div className="flex items-center gap-2">
                        <Bot className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
                        <span className="text-sm">{session.agent_name || "Unknown"}</span>
                      </div>
                    </td>
                    <td>
                      <div className="flex items-center gap-1">
                        {session.task_title && (session.related_task_id || session.task_id) ? (
                          <button
                            onClick={() => navigate(`/tasks/${session.related_task_id || session.task_id}`)}
                            className="text-sm text-left hover:underline"
                            style={{ color: "var(--color-text-secondary)" }}
                            title="查看任务详情"
                          >
                            {session.task_title}
                          </button>
                        ) : (
                          <span className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
                            {session.task_title || "-"}
                          </span>
                        )}
                        <button
                          onClick={() => openLinkModal(session)}
                          className="p-1 rounded transition-colors"
                          style={{ color: "var(--color-text-tertiary)" }}
                          title={session.task_title ? "变更关联任务" : "关联开发任务"}
                          aria-label="关联开发任务"
                        >
                          <Link2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </td>
                    <td>
                      <div className="flex items-center gap-2">
                        {getStatusIcon(session.status)}
                        <span className="text-sm">{getStatusText(session.status)}</span>
                      </div>
                    </td>
                    <td className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                      {formatDuration(session.duration_seconds)}
                    </td>
                    <td className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                      {formatTokens(session.total_tokens)}
                    </td>
                    <td className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                      ${session.total_cost?.toFixed(4) || "0.00"}
                    </td>
                    <td className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
                      {session.started_at ? new Date(session.started_at).toLocaleString("zh-CN") : "-"}
                    </td>
                    <td className="text-right">
                      <div className="flex items-center justify-end gap-1">
                        <RowAction
                          title="进入会话（消息/终端/原始流）"
                          onClick={() => navigate(`/sessions/${session.id}`)}
                        >
                          <Eye className="w-4 h-4" />
                        </RowAction>
                        {session.status === "running" && (
                          <>
                            <RowAction title="快速打开终端" onClick={() => handleViewTerminal(session)}>
                              <Terminal className="w-4 h-4" />
                            </RowAction>
                            <RowAction title="终止会话" onClick={() => handleTerminateSession(session.id)} danger>
                              <Square className="w-4 h-4" />
                            </RowAction>
                          </>
                        )}
                        <RowAction
                          title="重命名会话"
                          onClick={() => {
                            setRenameTarget(session);
                            setRenameValue(session.title || "");
                          }}
                        >
                          <Pencil className="w-4 h-4" />
                        </RowAction>
                        <RowAction
                          title={session.archived === 1 ? "取消归档" : "归档（默认列表隐藏）"}
                          onClick={() => handleToggleArchive(session)}
                        >
                          {session.archived === 1 ? (
                            <ArchiveRestore className="w-4 h-4" />
                          ) : (
                            <Archive className="w-4 h-4" />
                          )}
                        </RowAction>
                        <RowAction title="导出为 Markdown" onClick={() => handleExport(session)}>
                          <Download className="w-4 h-4" />
                        </RowAction>
                        <RowAction title="删除会话（级联清理）" onClick={() => setDeleteTarget(session)} danger>
                          <Trash2 className="w-4 h-4" />
                        </RowAction>
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      )}

      {/* 关联开发任务弹窗 */}
      {linkTarget && (
        <div
          className="modal-overlay"
          onClick={() => {
            if (!linking) setLinkTarget(null);
          }}
        >
          <div className="modal-content w-full max-w-md" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                关联开发任务
              </h2>
            </div>
            <div className="modal-body space-y-3">
              <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                会话 {linkTarget.id.slice(0, 8)} — 选择要关联的开发任务；选择「不关联任务」可解除现有关联。
              </p>
              {loadingTasks ? (
                <p className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
                  加载任务列表...
                </p>
              ) : taskOptions.length === 0 ? (
                <p className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
                  暂无任务，请先在看板创建任务
                </p>
              ) : (
                <select
                  value={selectedTaskId}
                  onChange={(e) => setSelectedTaskId(e.target.value)}
                  className="select w-full"
                >
                  <option value="">不关联任务</option>
                  {taskOptions.map((t) => (
                    <option key={t.id} value={t.id}>
                      {t.title}（{t.status}）
                    </option>
                  ))}
                </select>
              )}
            </div>
            <div className="modal-footer">
              <button className="btn-secondary" onClick={() => setLinkTarget(null)} disabled={linking}>
                取消
              </button>
              <button className="btn-primary" onClick={handleLinkConfirm} disabled={linking || loadingTasks}>
                {linking ? "关联中..." : "确认"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 重命名弹窗 */}
      {renameTarget && (
        <div className="modal-overlay" onClick={() => setRenameTarget(null)}>
          <div className="modal-content w-full max-w-md" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                重命名会话
              </h2>
            </div>
            <div className="modal-body space-y-3">
              <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                当前 ID：{renameTarget.id.slice(0, 8)}
              </p>
              <input
                autoFocus
                type="text"
                value={renameValue}
                onChange={(e) => setRenameValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleRenameConfirm();
                }}
                className="input w-full"
                placeholder="输入新的会话标题"
              />
            </div>
            <div className="modal-footer">
              <button className="btn-secondary" onClick={() => setRenameTarget(null)}>
                取消
              </button>
              <button className="btn-primary" onClick={handleRenameConfirm} disabled={!renameValue.trim()}>
                确认
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 删除确认弹窗 */}
      {deleteTarget && (
        <div className="modal-overlay" onClick={() => !deleting && setDeleteTarget(null)}>
          <div className="modal-content w-full max-w-md" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                删除会话
              </h2>
            </div>
            <div className="modal-body space-y-2">
              <p className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                将删除会话「{deleteTarget.title || deleteTarget.id.slice(0, 8)}」及其全部消息、Skill/MCP 执行记录与
                Token 记录。此操作不可恢复。
              </p>
              <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                建议先「导出为 Markdown」留存对话。
              </p>
            </div>
            <div className="modal-footer">
              <button className="btn-secondary" onClick={() => setDeleteTarget(null)} disabled={deleting}>
                取消
              </button>
              <button
                className="btn-primary"
                style={{ background: "var(--color-danger)" }}
                onClick={handleDeleteConfirm}
                disabled={deleting}
              >
                {deleting ? "删除中..." : "确认删除"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 终端弹窗 */}
      {showTerminal && selectedSession && (
        <div
          className="modal-overlay"
          onClick={() => {
            setShowTerminal(false);
            setSelectedSession(null);
          }}
        >
          <div
            className="w-[90vw] h-[80vh] rounded-xl overflow-hidden shadow-2xl"
            style={{ background: "var(--color-bg)" }}
            onClick={(e) => e.stopPropagation()}
          >
            <AgentTerminal
              sessionId={selectedSession.id}
              agentName={selectedSession.agent_name || "Agent"}
              taskTitle={selectedSession.task_title}
              onClose={() => {
                setShowTerminal(false);
                setSelectedSession(null);
              }}
              onStatusChange={() => {
                fetchSessions();
              }}
            />
          </div>
        </div>
      )}
    </div>
  );
}
