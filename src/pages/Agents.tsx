import { useEffect, useState } from "react";
import {
  Bot,
  RefreshCw,
  Play,
  Square,
  Terminal,
  AlertTriangle,
  ShieldCheck,
  MessageSquare,
  Plus,
  Trash2,
  X,
} from "lucide-react";
import { useNavigate } from "react-router-dom";
import type { Agent } from "../lib/types";
import { agentCapabilityLabel, parseAgentCapabilities } from "../lib/types";
import { startStructuredSession, addAgent, deleteAgent } from "../lib/tauri";
import { useAgentsStore } from "../stores/agentsStore";
import { StatusBadge } from "../components/Badges";
import { showToast } from "../components/Toast";

export default function Agents() {
  const navigate = useNavigate();
  // 统一走 store，避免页面卸载/重进后状态丢失（F021）
  const { agents, sessions, fetchAgents, discover, verify, start, stop } = useAgentsStore();
  const [busyAction, setBusyAction] = useState<"discover" | "verify" | null>(null);
  const [bootstrapped, setBootstrapped] = useState(false);
  // P1-A2：手动添加自定义路径 Agent
  const [showAddModal, setShowAddModal] = useState(false);
  const [addPath, setAddPath] = useState("");
  const [addName, setAddName] = useState("");
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    fetchAgents().finally(() => setBootstrapped(true));
  }, [fetchAgents]);

  async function handleRefresh() {
    setBusyAction("discover");
    try {
      // 检测并持久化到数据库，再从数据库重载
      await discover();
      showToast("success", "Agent检测完成");
    } catch (error) {
      console.error("刷新Agent失败:", error);
      showToast("error", `Agent检测失败: ${error}`);
    } finally {
      setBusyAction(null);
    }
  }

  /** 状态治理：进程存活对账 + 二进制有效性复核 */
  async function handleVerify() {
    setBusyAction("verify");
    try {
      await verify();
      showToast("success", "已完成进程对账与二进制有效性复核");
    } catch (error) {
      console.error("复核Agent状态失败:", error);
      showToast("error", `状态复核失败: ${error}`);
    } finally {
      setBusyAction(null);
    }
  }

  async function handleStartAgent(agent: Agent) {
    try {
      await start(agent.binary_path);
      showToast("success", `${agent.display_name} 启动成功`);
    } catch (error) {
      console.error("启动Agent失败:", error);
      showToast("error", `启动Agent失败: ${error}`);
    }
  }

  /** 以消息（结构化）模式启动：可发指令、看结构化回复与思考内容（模块 16） */
  async function handleStartStructured(agent: Agent) {
    try {
      const session = await startStructuredSession({ agentPath: agent.binary_path });
      showToast("success", `${agent.display_name} 已以消息模式启动`);
      navigate(`/sessions/${session.id}`);
    } catch (error) {
      console.error("消息模式启动失败:", error);
      showToast("error", `消息模式启动失败: ${error}`);
    }
  }

  async function handleStopAgent(sessionId: string) {
    try {
      await stop(sessionId);
      showToast("success", "Agent已停止");
    } catch (error) {
      console.error("停止Agent失败:", error);
      showToast("error", `停止Agent失败: ${error}`);
    }
  }

  /** 手动添加 Agent（后端真实执行 --version 验证） */
  async function handleAddAgent() {
    if (!addPath.trim() || adding) return;
    setAdding(true);
    try {
      const agent = await addAgent(addPath.trim(), addName.trim() || undefined);
      showToast("success", `已添加 ${agent.display_name}（v${agent.version ?? "?"}）`);
      setShowAddModal(false);
      setAddPath("");
      setAddName("");
      await fetchAgents();
    } catch (error) {
      showToast("error", `添加失败: ${error}`);
    } finally {
      setAdding(false);
    }
  }

  /** 删除 Agent 记录（运行中会话会被后端拒绝） */
  async function handleDeleteAgent(agent: Agent) {
    if (!confirm(`确定删除「${agent.display_name}」吗？\n仅删除平台内的记录，不影响本机安装。`)) return;
    try {
      await deleteAgent(agent.id);
      showToast("success", "已删除");
      await fetchAgents();
    } catch (error) {
      showToast("error", `删除失败: ${error}`);
    }
  }

  const runningSessions = sessions.filter((s) => s.status === "running");

  if (!bootstrapped) {
    return (
      <div className="flex items-center justify-center h-full">
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
    <div className="p-6 space-y-6">
      {/* 页面标题 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            Agent管理
          </h1>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
            管理本地CLI Agent
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowAddModal(true)}
            className="btn-secondary"
            title="手动指定二进制路径添加 Agent（不依赖自动检测）"
          >
            <Plus className="w-4 h-4" />
            手动添加
          </button>
          <button
            onClick={handleVerify}
            disabled={busyAction !== null}
            className="btn-secondary"
            title="真实执行 --version 复核二进制可用性，并把已无真实进程的会话纠正为失败"
          >
            <ShieldCheck className={`w-4 h-4 ${busyAction === "verify" ? "animate-pulse" : ""}`} />
            状态复核
          </button>
          <button onClick={handleRefresh} disabled={busyAction !== null} className="btn-secondary">
            <RefreshCw className={`w-4 h-4 ${busyAction === "discover" ? "animate-spin" : ""}`} />
            刷新检测
          </button>
        </div>
      </div>

      {/* 运行中的会话 */}
      {runningSessions.length > 0 && (
        <div className="panel">
          <h2 className="panel-title mb-3">运行中的Agent</h2>
          <div className="table-container">
            <table className="table">
              <thead>
                <tr>
                  <th>Agent</th>
                  <th>状态</th>
                  <th>启动时间</th>
                  <th className="text-right">操作</th>
                </tr>
              </thead>
              <tbody>
                {runningSessions.map((session) => {
                  const agent = agents.find((a) => a.id === session.agent_id);
                  return (
                    <tr key={session.id}>
                      <td>
                        <div className="flex items-center gap-2">
                          <div className="p-1.5 rounded-lg" style={{ background: "var(--color-primary-subtle)" }}>
                            <Bot className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
                          </div>
                          <span className="font-medium">{agent?.display_name || "未知Agent"}</span>
                        </div>
                      </td>
                      <td>
                        <StatusBadge status="running" />
                      </td>
                      <td style={{ color: "var(--color-text-tertiary)" }}>
                        {new Date(session.started_at).toLocaleString("zh-CN")}
                      </td>
                      <td className="text-right">
                        <div className="flex items-center justify-end gap-1">
                          <button
                            onClick={() => navigate("/sessions")}
                            className="p-1.5 rounded-lg"
                            style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.color = "var(--color-primary)";
                              e.currentTarget.style.background = "var(--color-primary-subtle)";
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.color = "";
                              e.currentTarget.style.background = "";
                            }}
                            title="查看终端"
                          >
                            <Terminal className="w-4 h-4" />
                          </button>
                          <button
                            onClick={() => handleStopAgent(session.id)}
                            className="p-1.5 rounded-lg"
                            style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.color = "var(--color-danger)";
                              e.currentTarget.style.background = "var(--color-danger-light)";
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.color = "";
                              e.currentTarget.style.background = "";
                            }}
                            title="停止"
                          >
                            <Square className="w-4 h-4" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* 可用的Agent */}
      <div>
        <h2 className="text-base font-semibold mb-3" style={{ color: "var(--color-text)" }}>
          可用的Agent
        </h2>
        {agents.length === 0 ? (
          <div className="panel">
            <div className="empty-state py-8">
              <Bot className="w-10 h-10 mb-3" style={{ color: "var(--color-text-tertiary)" }} />
              <p className="empty-state-title">未检测到已安装的Agent</p>
              <p className="empty-state-desc">请确保已安装Claude Code或Codex CLI，并添加到系统PATH</p>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {agents.map((agent) => {
              const caps = parseAgentCapabilities(agent);
              return (
                <div
                  key={agent.id}
                  className="card p-4"
                  style={{ transition: "border-color var(--transition-fast), box-shadow var(--transition-fast)" }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = "var(--color-primary)";
                    e.currentTarget.style.boxShadow = "var(--shadow-sm)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = "";
                    e.currentTarget.style.boxShadow = "";
                  }}
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-3">
                      <div
                        className="p-2 rounded-lg"
                        style={{
                          background:
                            agent.status === "available" ? "var(--color-primary-subtle)" : "var(--color-danger-light)",
                        }}
                      >
                        <Bot
                          className="w-5 h-5"
                          style={{
                            color: agent.status === "available" ? "var(--color-primary)" : "var(--color-danger)",
                          }}
                        />
                      </div>
                      <div>
                        <h3 className="text-sm font-semibold" style={{ color: "var(--color-text)" }}>
                          {agent.display_name}
                        </h3>
                        <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                          {agent.name}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {/* 仅"二进制不可用"才算失效；running 是合法状态（由会话推导），
                        不能用 !== "available" 判断，否则运行中的 Agent 会同时显示"已失效"+"运行中" */}
                      {agent.status === "unavailable" && (
                        <span
                          className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-full"
                          style={{ background: "var(--color-danger-light)", color: "var(--color-danger)" }}
                        >
                          <AlertTriangle className="w-3 h-3" />
                          已失效
                        </span>
                      )}
                      <StatusBadge status={agent.status} />
                      {/* 能力等级徽章（多 Agent 适配 MA-02）：结构化(L3) / 一次性(L2) / 仅终端(L1) */}
                      <span
                        className="text-[10px] px-1.5 py-0.5 rounded-full"
                        style={{
                          background: caps.structured_events
                            ? "var(--color-success-light, #e6f7ed)"
                            : caps.one_shot
                              ? "var(--color-primary-subtle)"
                              : "var(--color-bg-subtle)",
                          color: caps.structured_events ? "var(--color-success)" : "var(--color-text-secondary)",
                        }}
                        title={
                          caps.structured_events
                            ? "L3：支持结构化会话（发送指令、查看回复与思考），仅 Claude 已全链路实测"
                            : caps.one_shot
                              ? "L2：支持一次性调用（PRD 生成/智能拆解可用），暂不支持多轮结构化会话"
                              : "L1：仅能在外部终端使用，平台内仅做检测与启动"
                        }
                      >
                        {caps.structured_events ? "L3 " : caps.one_shot ? "L2 " : "L1 "}
                        {agentCapabilityLabel(caps)}
                      </span>
                    </div>
                  </div>

                  <div className="space-y-1.5 text-xs mb-4" style={{ color: "var(--color-text-tertiary)" }}>
                    <div className="flex justify-between">
                      <span>版本</span>
                      <span style={{ color: "var(--color-text-secondary)" }}>{agent.version || "未知"}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>路径</span>
                      <span
                        className="font-mono text-[10px] truncate max-w-[120px]"
                        style={{ color: "var(--color-text-secondary)" }}
                      >
                        {agent.binary_path}
                      </span>
                    </div>
                  </div>

                  <button
                    onClick={() => handleStartStructured(agent)}
                    disabled={agent.status !== "available" || !caps.structured_events}
                    className="w-full btn-secondary text-sm mb-1"
                    title={
                      caps.structured_events
                        ? "以消息模式启动：可发送指令并查看结构化回复与思考内容"
                        : "该 Agent 暂不支持结构化会话（L3 未实测或未适配），请使用「启动」（终端模式）"
                    }
                  >
                    <MessageSquare className="w-3.5 h-3.5" />
                    消息模式启动
                  </button>
                  <button
                    onClick={() => handleStartAgent(agent)}
                    disabled={agent.status !== "available"}
                    className="w-full btn-primary text-sm"
                  >
                    <Play className="w-3.5 h-3.5" />
                    启动
                  </button>
                  <button
                    onClick={() => handleDeleteAgent(agent)}
                    className="w-full btn-secondary text-xs mt-1"
                    style={{ color: "var(--color-danger)" }}
                    title="删除平台内记录（不影响本机安装；有运行中会话时不可删除）"
                  >
                    <Trash2 className="w-3 h-3" />
                    删除记录
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* P1-A2：手动添加 Agent */}
      {showAddModal && (
        <div className="modal-overlay" onClick={() => !adding && setShowAddModal(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 480 }}>
            <div className="modal-header">
              <h3 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                手动添加 Agent
              </h3>
              <button
                onClick={() => setShowAddModal(false)}
                className="p-1 rounded-lg"
                style={{ color: "var(--color-text-tertiary)" }}
                aria-label="关闭"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="modal-body space-y-4">
              <div>
                <label className="label">二进制路径 *</label>
                <input
                  type="text"
                  value={addPath}
                  onChange={(e) => setAddPath(e.target.value)}
                  placeholder="如 C:\tools\my-agent.exe 或 /usr/local/bin/my-agent"
                  className="input font-mono text-xs"
                  autoFocus
                />
                <p className="text-[11px] mt-1" style={{ color: "var(--color-text-tertiary)" }}>
                  添加时会真实执行 `--version` 验证可用性（5 秒超时）。
                </p>
              </div>
              <div>
                <label className="label">显示名称</label>
                <input
                  type="text"
                  value={addName}
                  onChange={(e) => setAddName(e.target.value)}
                  placeholder="留空则使用文件名"
                  className="input"
                />
              </div>
            </div>
            <div className="modal-footer">
              <button onClick={() => setShowAddModal(false)} disabled={adding} className="btn-secondary">
                取消
              </button>
              <button onClick={handleAddAgent} disabled={!addPath.trim() || adding} className="btn-primary">
                {adding ? <ShieldCheck className="w-4 h-4 animate-pulse" /> : <Plus className="w-4 h-4" />}
                验证并添加
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 最近的会话记录 */}
      {sessions.filter((s) => s.status !== "running").length > 0 && (
        <div>
          <h2 className="text-base font-semibold mb-3" style={{ color: "var(--color-text)" }}>
            最近的会话记录
          </h2>
          <div className="table-container">
            <table className="table">
              <thead>
                <tr>
                  <th>Agent</th>
                  <th>状态</th>
                  <th>启动时间</th>
                  <th>结束时间</th>
                  <th>退出码</th>
                </tr>
              </thead>
              <tbody>
                {sessions
                  .filter((s) => s.status !== "running")
                  .slice(0, 10)
                  .map((session) => {
                    const agent = agents.find((a) => a.id === session.agent_id);
                    return (
                      <tr key={session.id}>
                        <td className="font-medium">{agent?.display_name || "未知Agent"}</td>
                        <td>
                          <StatusBadge status={session.status} />
                        </td>
                        <td style={{ color: "var(--color-text-tertiary)" }}>
                          {new Date(session.started_at).toLocaleString("zh-CN")}
                        </td>
                        <td style={{ color: "var(--color-text-tertiary)" }}>
                          {session.ended_at ? new Date(session.ended_at).toLocaleString("zh-CN") : "-"}
                        </td>
                        <td>
                          {session.exit_code !== null ? (
                            <span
                              style={{
                                color: session.exit_code === 0 ? "var(--color-success)" : "var(--color-danger)",
                              }}
                            >
                              {session.exit_code}
                            </span>
                          ) : (
                            "-"
                          )}
                        </td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
