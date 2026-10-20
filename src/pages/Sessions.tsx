import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Terminal,
  RefreshCw,
  Search,
  Filter,
  Clock,
  CheckCircle,
  XCircle,
  Square,
  Eye,
  Bot,
  Link2,
} from "lucide-react";
import type { AgentSession, Task } from "../lib/types";
import { useSessionsStore } from "../stores";
import { listTasks, linkSessionTask } from "../lib/tauri";
import AgentTerminal from "../components/AgentTerminal";
import { showToast } from "../components/Toast";

export default function Sessions() {
  const navigate = useNavigate();
  const { sessions, loading, fetchSessions, stopSession } = useSessionsStore();
  const [refreshing, setRefreshing] = useState(false);
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedSession, setSelectedSession] = useState<AgentSession | null>(null);
  const [showTerminal, setShowTerminal] = useState(false);
  // 关联开发任务弹窗
  const [linkTarget, setLinkTarget] = useState<AgentSession | null>(null);
  const [taskOptions, setTaskOptions] = useState<Task[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState("");
  const [loadingTasks, setLoadingTasks] = useState(false);
  const [linking, setLinking] = useState(false);

  useEffect(() => {
    fetchSessions();
  }, []);

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
      console.error('终止会话失败:', error);
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

  // 过滤会话
  const filteredSessions = sessions.filter((session) => {
    if (filterStatus !== 'all' && session.status !== filterStatus) {
      return false;
    }
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        session.id.toLowerCase().includes(query) ||
        session.agent_name?.toLowerCase().includes(query) ||
        session.task_title?.toLowerCase().includes(query)
      );
    }
    return true;
  });

  // 统计数据
  const stats = {
    total: sessions.length,
    running: sessions.filter((s) => s.status === 'running').length,
    completed: sessions.filter((s) => s.status === 'completed').length,
    failed: sessions.filter((s) => s.status === 'failed').length,
  };

  // 获取状态图标
  function getStatusIcon(status: string) {
    switch (status) {
      case 'running':
        return <div className="w-2 h-2 rounded-full animate-pulse" style={{ background: "var(--color-success)" }} />;
      case 'completed':
        return <CheckCircle className="w-4 h-4" style={{ color: "var(--color-success)" }} />;
      case 'failed':
        return <XCircle className="w-4 h-4" style={{ color: "var(--color-danger)" }} />;
      case 'terminated':
        return <Square className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />;
      default:
        return <Clock className="w-4 h-4" style={{ color: "var(--color-warning)" }} />;
    }
  }

  // 获取状态文本
  function getStatusText(status: string) {
    switch (status) {
      case 'created':
        return '已创建';
      case 'launching':
        return '启动中';
      case 'running':
        return '运行中';
      case 'completed':
        return '已完成';
      case 'failed':
        return '失败';
      case 'terminated':
        return '已终止';
      default:
        return status;
    }
  }

  // 格式化时长
  function formatDuration(seconds?: number) {
    if (!seconds) return '-';
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s`;
  }

  // 格式化 Token
  function formatTokens(tokens?: number) {
    if (!tokens) return '0';
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
          <div className="w-4 h-4 border-2 rounded-full animate-spin" style={{ borderColor: "var(--color-border)", borderTopColor: "var(--color-primary)" }} />
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
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>会话管理</h1>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>查看和管理 Agent 执行会话</p>
        </div>
        <button
          onClick={handleRefresh}
          disabled={refreshing}
          className="btn-secondary"
          aria-label="刷新会话列表"
        >
          <RefreshCw className={`w-4 h-4 ${refreshing ? "animate-spin" : ""}`} />
          刷新
        </button>
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
            <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>{card.label}</p>
            <p className="text-lg font-semibold mt-1" style={{ color: card.color }}>{card.value}</p>
          </motion.div>
        ))}
      </div>

      {/* 筛选和搜索 */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center gap-3">
        <div className="flex items-center gap-2">
          <Filter className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
          <select
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value)}
            className="select w-auto"
          >
            <option value="all">全部状态</option>
            <option value="running">运行中</option>
            <option value="completed">已完成</option>
            <option value="failed">失败</option>
            <option value="terminated">已终止</option>
          </select>
        </div>

        <div className="flex-1 w-full sm:w-auto relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
          <input
            type="text"
            placeholder="搜索会话..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input pl-9"
          />
        </div>
      </div>

      {/* 会话列表 */}
      <div className="table-container">
        <table className="table">
          <thead>
            <tr>
              <th scope="col">会话 ID</th>
              <th scope="col">Agent</th>
              <th scope="col">任务</th>
              <th scope="col">状态</th>
              <th scope="col">时长</th>
              <th scope="col">Token</th>
              <th scope="col">费用</th>
              <th scope="col">开始时间</th>
              <th scope="col" className="text-right">操作</th>
            </tr>
          </thead>
          <tbody>
            {filteredSessions.length === 0 ? (
              <tr>
                <td colSpan={9} className="text-center py-12">
                  <Terminal className="w-10 h-10 mx-auto mb-3" style={{ color: "var(--color-text-tertiary)" }} />
                  <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>暂无会话记录</p>
                  <p className="text-xs mt-1" style={{ color: "var(--color-text-tertiary)" }}>Agent 启动后会话将显示在此处</p>
                </td>
              </tr>
            ) : (
              filteredSessions.map((session) => (
                <tr key={session.id}>
                  <td>
                    <span className="font-mono text-sm" style={{ color: "var(--color-text-secondary)" }}>
                      {session.id.slice(0, 8)}
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
                    {session.started_at
                      ? new Date(session.started_at).toLocaleString("zh-CN")
                      : "-"}
                  </td>
                  <td className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      {/* 进入会话（所有状态可用）：SessionDetail 会按会话模式自动选择消息/终端视图 */}
                      <button
                        onClick={() => navigate(`/sessions/${session.id}`)}
                        className="p-1.5 rounded-lg"
                        style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
                        onMouseEnter={(e) => { e.currentTarget.style.color = "var(--color-primary)"; e.currentTarget.style.background = "var(--color-primary-subtle)"; }}
                        onMouseLeave={(e) => { e.currentTarget.style.color = ""; e.currentTarget.style.background = ""; }}
                        aria-label="进入会话"
                        title="进入会话（消息/终端视图）"
                      >
                        <Eye className="w-4 h-4" />
                      </button>
                      {session.status === "running" && (
                        <>
                          <button
                            onClick={() => handleViewTerminal(session)}
                            className="p-1.5 rounded-lg"
                            style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
                            onMouseEnter={(e) => { e.currentTarget.style.color = "var(--color-primary)"; e.currentTarget.style.background = "var(--color-primary-subtle)"; }}
                            onMouseLeave={(e) => { e.currentTarget.style.color = ""; e.currentTarget.style.background = ""; }}
                            aria-label="查看终端"
                            title="快速打开终端"
                          >
                            <Terminal className="w-4 h-4" />
                          </button>
                          <button
                            onClick={() => handleTerminateSession(session.id)}
                            className="p-1.5 rounded-lg"
                            style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
                            onMouseEnter={(e) => { e.currentTarget.style.color = "var(--color-danger)"; e.currentTarget.style.background = "var(--color-danger-light)"; }}
                            onMouseLeave={(e) => { e.currentTarget.style.color = ""; e.currentTarget.style.background = ""; }}
                            aria-label="终止会话"
                            title="终止会话"
                          >
                            <Square className="w-4 h-4" />
                          </button>
                        </>
                      )}
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

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
              <button
                className="btn-secondary"
                onClick={() => setLinkTarget(null)}
                disabled={linking}
              >
                取消
              </button>
              <button
                className="btn-primary"
                onClick={handleLinkConfirm}
                disabled={linking || loadingTasks}
              >
                {linking ? "关联中..." : "确认"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 终端弹窗 */}
      {showTerminal && selectedSession && (
        <div className="modal-overlay" onClick={() => { setShowTerminal(false); setSelectedSession(null); }}>
          <div className="w-[90vw] h-[80vh] rounded-xl overflow-hidden shadow-2xl" style={{ background: "var(--color-bg)" }} onClick={(e) => e.stopPropagation()}>
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
