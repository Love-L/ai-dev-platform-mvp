import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, Terminal, Zap, Plug, Link2, Coins, MessageSquare, RefreshCw } from "lucide-react";
import {
  getAgentSessions,
  getSkillExecutionLogs,
  getMcpExecutionLogs,
  setSessionMode,
  resumeSession,
} from "../lib/tauri";
import type { AgentSession, SkillExecutionLog, McpExecutionLog } from "../lib/types";
import SessionChat from "../components/SessionChat";
import SessionInput from "../components/SessionInput";
import AgentTerminal from "../components/AgentTerminal";
import { showToast } from "../components/Toast";
import { useSessionMessagesStore } from "../stores/sessionMessagesStore";

export default function SessionDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [session, setSession] = useState<AgentSession | null>(null);
  const [skillLogs, setSkillLogs] = useState<SkillExecutionLog[]>([]);
  const [mcpLogs, setMcpLogs] = useState<McpExecutionLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [view, setView] = useState<"messages" | "terminal">("messages");
  const [busy, setBusy] = useState(false);

  const { send: sendMessage, interrupt, sending } = useSessionMessagesStore();

  useEffect(() => {
    if (id) loadData(id);
  }, [id]);

  async function loadData(sessionId: string) {
    try {
      const sessions = await getAgentSessions();
      const found = sessions.find((s) => s.id === sessionId);
      if (found) {
        setSession(found);
        setView(found.view === "terminal" ? "terminal" : "messages");
        // 加载 Skill/MCP 日志
        const [sLogs, mLogs] = await Promise.all([
          getSkillExecutionLogs(undefined, sessionId),
          getMcpExecutionLogs(sessionId),
        ]);
        setSkillLogs(sLogs);
        setMcpLogs(mLogs);
      }
    } catch (error) {
      console.error("加载会话详情失败:", error);
    } finally {
      setLoading(false);
    }
  }

  /** 会话处理结果后刷新会话本体（状态/用量会变化） */
  async function refreshSession() {
    if (!id) return;
    try {
      const sessions = await getAgentSessions();
      const found = sessions.find((s) => s.id === id);
      if (found) setSession(found);
    } catch (error) {
      console.error("刷新会话失败:", error);
    }
  }

  /**
   * 被拒卡片的重建流程：会话进程运行中时后端拒绝重建（"会话正在运行"），
   * 用户直接点击会得到无效反馈。这里先中断进程，再对"正在运行"错误短轮询重试，
   * 直到 registry 释放后完成白名单合并 + 重建 + 重发。
   */
  async function rebuildSession(params: Parameters<typeof setSessionMode>[0]) {
    if (session?.status === "running") {
      await interrupt();
    }
    let lastError: unknown;
    for (let attempt = 0; attempt < 12; attempt++) {
      try {
        await setSessionMode(params);
        return;
      } catch (error) {
        lastError = error;
        if (!String(error).includes("正在运行")) throw error;
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
    }
    throw lastError;
  }

  /** 被拒卡片：允许该工具并重试（沉淀进会话白名单，本会话始终生效） */
  async function handleAllowAndRetry(toolName?: string) {
    if (!id) return;
    setBusy(true);
    try {
      await rebuildSession({
        sessionId: id,
        mode: "structured",
        allowedTools: toolName ? [toolName] : undefined,
        resendLast: true,
      });
      showToast("success", toolName ? `已允许 ${toolName}（本会话生效）并重试` : "已允许并重试");
      await refreshSession();
    } catch (error) {
      showToast("error", `重试失败: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  /** 被拒卡片：改用更宽松的权限预设 */
  async function handleUseAcceptEdits() {
    if (!id) return;
    setBusy(true);
    try {
      await rebuildSession({
        sessionId: id,
        mode: "structured",
        permissionMode: "acceptEdits",
        resendLast: true,
      });
      showToast("success", "已切换权限预设为 acceptEdits 并重试");
      await refreshSession();
    } catch (error) {
      showToast("error", `切换失败: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  /** 续接既有 CLI 会话 */
  async function handleResume() {
    if (!id) return;
    setBusy(true);
    try {
      await resumeSession(id);
      showToast("success", "已续接会话");
      await refreshSession();
    } catch (error) {
      showToast("error", `续接失败: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  // 解析 skills/mcps JSON
  const sessionSkills: { skill_id: string; skill_name?: string }[] = session?.skills ? JSON.parse(session.skills) : [];
  const sessionMcps: { mcp_name: string; endpoint?: string; status?: string }[] = session?.mcps
    ? JSON.parse(session.mcps)
    : [];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">加载中...</div>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">会话不存在</div>
      </div>
    );
  }

  // 会话模式：显式字段优先；历史会话（无 mode）有 CLI 会话 id 视为结构化，否则终端
  const sessionMode = session.mode ?? (session.cli_session_id ? "structured" : "terminal");
  const isStructured = sessionMode === "structured";
  const isRunning = session.status === "running";

  return (
    <div className="p-6 space-y-5">
      {/* 顶部导航 */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => navigate("/sessions")}
          className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
        </button>
        <div className="flex-1">
          <h1 className="text-xl font-semibold text-gray-900">{session.title || `会话 ${session.id.slice(0, 8)}`}</h1>
          <p className="text-sm text-gray-500 mt-0.5">
            状态:
            <span
              className={`ml-1 inline-flex items-center px-1.5 py-0.5 rounded-full text-[10px] font-medium ${
                session.status === "running"
                  ? "bg-blue-50 text-blue-700"
                  : session.status === "completed"
                    ? "bg-emerald-50 text-emerald-700"
                    : "bg-red-50 text-red-700"
              }`}
            >
              {session.status === "running" ? "运行中" : session.status === "completed" ? "已完成" : "失败"}
            </span>
            {session.source && (
              <span className="ml-2 text-[10px] bg-gray-100 text-gray-500 px-1.5 py-0.5 rounded">
                {session.source === "native" ? "原生" : session.source === "imported" ? "导入" : "手动"}
              </span>
            )}
          </p>
        </div>
      </div>

      <div className="grid gap-5" style={{ gridTemplateColumns: "minmax(0, 1fr) 300px" }}>
        {/* 左侧：对话/终端主区域 */}
        <div className="min-w-0 space-y-5">
          {/* 会话内容：消息视图（主）/ 终端视图（兜底），高度自适应视口 */}
          <div
            className="panel flex flex-col overflow-hidden"
            style={{ height: "calc(100vh - 230px)", minHeight: 420 }}
          >
            <div className="panel-header flex items-center justify-between gap-3">
              <div
                className="flex items-center gap-1 p-0.5 rounded-lg"
                style={{ background: "var(--color-bg-subtle)" }}
              >
                <button
                  type="button"
                  onClick={() => setView("messages")}
                  className="px-3 py-1 text-xs rounded-md flex items-center gap-1.5"
                  style={
                    view === "messages"
                      ? { background: "var(--color-bg)", color: "var(--color-primary)" }
                      : { color: "var(--color-text-tertiary)" }
                  }
                >
                  <MessageSquare className="w-3.5 h-3.5" /> 消息
                </button>
                <button
                  type="button"
                  onClick={() => setView("terminal")}
                  className="px-3 py-1 text-xs rounded-md flex items-center gap-1.5"
                  style={
                    view === "terminal"
                      ? { background: "var(--color-bg)", color: "var(--color-primary)" }
                      : { color: "var(--color-text-tertiary)" }
                  }
                >
                  <Terminal className="w-3.5 h-3.5" /> 终端
                </button>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
                  {isStructured ? "结构化会话" : "终端会话"}
                  {session.permission_mode ? ` · ${session.permission_mode}` : ""}
                </span>
                <button
                  type="button"
                  onClick={handleResume}
                  disabled={busy || isRunning}
                  className="btn-secondary text-xs px-2.5 py-1 flex items-center gap-1.5"
                  title="使用 CLI 会话 id 续接上下文"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${busy ? "animate-spin" : ""}`} />
                  续接会话
                </button>
              </div>
            </div>

            <div className="flex-1 min-h-0">
              {view === "messages" ? (
                isStructured ? (
                  <SessionChat
                    sessionId={session.id}
                    running={isRunning}
                    onAllowAndRetry={handleAllowAndRetry}
                    onUseAcceptEdits={handleUseAcceptEdits}
                    deniedBusy={busy}
                  />
                ) : (
                  <div className="h-full flex items-center justify-center px-6">
                    <p className="text-sm text-center" style={{ color: "var(--color-text-tertiary)" }}>
                      终端会话暂不支持消息视图。
                      <br />
                      请切换到「终端」查看输出；消息模式的会话请在 Agent 页以消息模式启动。
                    </p>
                  </div>
                )
              ) : isStructured ? (
                <div className="h-full flex items-center justify-center px-6">
                  <p className="text-sm text-center" style={{ color: "var(--color-text-tertiary)" }}>
                    结构化会话以消息协议驱动，终端视图<b>不提供交互输入</b>（仅用于排障，原始事件流视图将在 P3 提供）。
                    <br />
                    如需在原生终端中逐次授权确认，请在 Agent 页以「终端模式」新建会话。
                  </p>
                </div>
              ) : (
                <AgentTerminal
                  sessionId={session.id}
                  agentName={session.title || "Agent"}
                  onStatusChange={() => refreshSession()}
                />
              )}
            </div>

            {view === "messages" && isStructured && (
              <SessionInput
                sessionId={session.id}
                running={isRunning}
                sending={sending}
                onSend={(content) => {
                  sendMessage(content).catch((err) => showToast("error", `发送失败: ${err}`));
                }}
                onInterrupt={() => {
                  interrupt().then(() => refreshSession());
                }}
              />
            )}
          </div>
        </div>

        {/* 右侧：信息面板（Skill/MCP 执行记录置顶） */}
        <div className="min-w-0 space-y-5">
          {/* Skill 执行记录 */}
          <div className="panel">
            <div className="panel-header">
              <h2 className="panel-title flex items-center gap-2">
                <Zap className="w-4 h-4" /> Skill 执行记录
              </h2>
            </div>
            {skillLogs.length === 0 ? (
              <p className="text-sm text-gray-400 py-4 text-center">暂无执行记录</p>
            ) : (
              <div className="space-y-2 max-h-56 overflow-y-auto">
                {skillLogs.map((log) => (
                  <div key={log.id} className="flex items-center justify-between p-2 bg-gray-50 rounded-lg">
                    <div className="flex items-center gap-2">
                      <Zap className="w-3.5 h-3.5 text-purple-600" />
                      <span className="text-sm font-medium text-gray-900">{log.skill_name}</span>
                      <span className="text-[10px] bg-gray-200 text-gray-600 px-1.5 py-0.5 rounded">
                        {log.execution_phase}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 text-xs text-gray-500">
                      <span>{log.token_usage.toLocaleString()} tokens</span>
                      <span className={log.status === "success" ? "text-emerald-600" : "text-red-600"}>
                        {log.status === "success" ? "✓" : "✗"}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* MCP 调用记录 */}
          <div className="panel">
            <div className="panel-header">
              <h2 className="panel-title flex items-center gap-2">
                <Plug className="w-4 h-4" /> MCP 调用记录
              </h2>
            </div>
            {mcpLogs.length === 0 ? (
              <p className="text-sm text-gray-400 py-4 text-center">暂无调用记录</p>
            ) : (
              <div className="space-y-2 max-h-56 overflow-y-auto">
                {mcpLogs.map((log) => (
                  <div key={log.id} className="flex items-center justify-between p-2 bg-gray-50 rounded-lg">
                    <div className="flex items-center gap-2">
                      <Plug className="w-3.5 h-3.5 text-green-600" />
                      <span className="text-sm font-medium text-gray-900">{log.mcp_name}</span>
                      {log.action && (
                        <span className="text-[10px] bg-gray-200 text-gray-600 px-1.5 py-0.5 rounded">
                          {log.action}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-xs text-gray-500">
                      {log.duration_ms && <span>{log.duration_ms}ms</span>}
                      <span
                        className={
                          log.status === "connected" || log.status === "call_success"
                            ? "text-emerald-600"
                            : "text-red-600"
                        }
                      >
                        {log.status === "connected" || log.status === "call_success" ? "✓" : "✗"}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* 关联任务 */}
          {(session.related_task_id || session.task_id) && (
            <div className="panel">
              <h2 className="panel-title mb-3 flex items-center gap-1.5">
                <Link2 className="w-4 h-4" /> 关联任务
              </h2>
              <div className="text-sm text-gray-700">
                <p className="font-mono text-xs text-gray-400">{session.related_task_id || session.task_id}</p>
              </div>
            </div>
          )}

          {/* 执行人 */}
          <div className="panel">
            <h2 className="panel-title mb-3">执行人</h2>
            <dl className="space-y-2">
              <div className="flex justify-between text-sm">
                <dt className="text-gray-500">Agent</dt>
                <dd className="text-gray-700">claude-code</dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt className="text-gray-500">来源</dt>
                <dd className="text-gray-700">
                  {session.source === "native" ? "平台创建" : session.source === "imported" ? "文件导入" : "手动导入"}
                </dd>
              </div>
            </dl>
          </div>

          {/* 使用的 Skill */}
          <div className="panel">
            <h2 className="panel-title mb-3 flex items-center gap-1.5">
              <Zap className="w-4 h-4" /> 使用的 Skill
            </h2>
            {sessionSkills.length === 0 ? (
              <p className="text-xs text-gray-400">未配置 Skill</p>
            ) : (
              <div className="space-y-1.5">
                {sessionSkills.map((s, i) => (
                  <div key={i} className="flex items-center gap-2 text-sm">
                    <Zap className="w-3 h-3 text-purple-500" />
                    <span className="text-gray-700">{s.skill_name || s.skill_id}</span>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* 连接的 MCP */}
          <div className="panel">
            <h2 className="panel-title mb-3 flex items-center gap-1.5">
              <Plug className="w-4 h-4" /> 连接的 MCP
            </h2>
            {sessionMcps.length === 0 ? (
              <p className="text-xs text-gray-400">未配置 MCP</p>
            ) : (
              <div className="space-y-1.5">
                {sessionMcps.map((m, i) => (
                  <div key={i} className="flex items-center justify-between text-sm">
                    <div className="flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full ${
                          m.status === "connected" ? "bg-emerald-500" : "bg-gray-300"
                        }`}
                      />
                      <span className="text-gray-700">{m.mcp_name}</span>
                    </div>
                    {m.endpoint && (
                      <span className="text-[10px] font-mono text-gray-400 truncate max-w-[100px]">{m.endpoint}</span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Token 消耗 */}
          <div className="panel">
            <h2 className="panel-title mb-3 flex items-center gap-1.5">
              <Coins className="w-4 h-4" /> Token 消耗
            </h2>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between">
                <dt className="text-gray-500">总计</dt>
                <dd className="text-gray-700 font-medium">{(session.total_tokens || 0).toLocaleString()}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-gray-500">费用</dt>
                <dd className="text-gray-700">${(session.total_cost || 0).toFixed(4)}</dd>
              </div>
              {session.duration_seconds && (
                <div className="flex justify-between">
                  <dt className="text-gray-500">耗时</dt>
                  <dd className="text-gray-700">
                    {Math.floor(session.duration_seconds / 60)}分{session.duration_seconds % 60}秒
                  </dd>
                </div>
              )}
            </dl>
          </div>

          {/* 元信息 */}
          <div className="panel">
            <h2 className="panel-title mb-3">元信息</h2>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between">
                <dt className="text-gray-500">会话ID</dt>
                <dd className="text-xs font-mono text-gray-400 truncate max-w-[120px]">{session.id}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-gray-500">开始时间</dt>
                <dd className="text-gray-700">{new Date(session.started_at).toLocaleString("zh-CN")}</dd>
              </div>
              {session.ended_at && (
                <div className="flex justify-between">
                  <dt className="text-gray-500">结束时间</dt>
                  <dd className="text-gray-700">{new Date(session.ended_at).toLocaleString("zh-CN")}</dd>
                </div>
              )}
            </dl>
          </div>
        </div>
      </div>
    </div>
  );
}
