import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import {
  Archive,
  ArchiveRestore,
  ArrowLeft,
  Coins,
  Download,
  Eye,
  Link2,
  MessageSquare,
  Pencil,
  Plug,
  RefreshCw,
  Sparkles,
  Terminal,
  Trash2,
  X,
  Zap,
} from "lucide-react";
import {
  deleteAgentSession,
  exportAgentSession,
  forceRebuildSession,
  getAgentSession,
  getMcpExecutionLogs,
  getSetting,
  getSkillExecutionLogs,
  getTask,
  renameAgentSession,
  resumeSession,
  setAgentSessionArchived,
} from "../lib/tauri";
import type { AgentSession, McpExecutionLog, SkillExecutionLog, Task } from "../lib/types";
import SessionChat from "../components/SessionChat";
import SessionInput from "../components/SessionInput";
import AgentTerminal from "../components/AgentTerminal";
import { showToast } from "../components/Toast";
import { useSessionMessagesStore } from "../stores/sessionMessagesStore";

/** 容错 JSON 解析：脏数据不崩页面（P0-3） */
function safeParseJson<T>(raw: string | null | undefined, fallback: T): T {
  if (!raw) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

/** 冷启动建议开场白（可由设置项 session_opening_prompts 覆盖，JSON string[]） */
const DEFAULT_OPENING_PROMPTS = [
  "先阅读任务与所属需求，列出实现计划并确认关键取舍，再开始编码。",
  "检查当前工作目录的代码结构，说明相关模块，然后给出改造方案。",
  "实现任务并自测；完成后总结改动点、风险与建议的验证步骤。",
];

/** 操作按钮（图标 + tooltip 的统一 hover 样式） */
function IconButton({
  title,
  onClick,
  danger,
  disabled,
  children,
}: {
  title: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="p-1.5 rounded-lg"
      style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
      onMouseEnter={(e) => {
        if (disabled) return;
        e.currentTarget.style.color = danger ? "var(--color-danger)" : "var(--color-primary)";
        e.currentTarget.style.background = danger ? "var(--color-danger-light)" : "var(--color-primary-subtle)";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.color = "";
        e.currentTarget.style.background = "";
      }}
      title={title}
    >
      {children}
    </button>
  );
}

export default function SessionDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [session, setSession] = useState<AgentSession | null>(null);
  const [task, setTask] = useState<Task | null>(null);
  const [openingPrompts, setOpeningPrompts] = useState<string[]>(DEFAULT_OPENING_PROMPTS);
  const [skillLogs, setSkillLogs] = useState<SkillExecutionLog[]>([]);
  const [mcpLogs, setMcpLogs] = useState<McpExecutionLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [view, setView] = useState<"messages" | "terminal" | "raw">("messages");
  const [busy, setBusy] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);
  /** 重命名弹窗 */
  const [renameOpen, setRenameOpen] = useState(false);
  const [renameValue, setRenameValue] = useState("");
  /** 删除确认弹窗 */
  const [deleteOpen, setDeleteOpen] = useState(false);

  const { messages, send: sendMessage, interrupt, sending } = useSessionMessagesStore();

  /** 单查会话（P0-2：不再全量拉列表再 find） */
  const refreshSession = useCallback(async () => {
    if (!id) return;
    try {
      const found = await getAgentSession(id);
      if (found) setSession(found);
    } catch (error) {
      console.error("刷新会话失败:", error);
    }
  }, [id]);

  const loadData = useCallback(async (sessionId: string) => {
    try {
      const [found, sLogs, mLogs, promptsRaw] = await Promise.all([
        getAgentSession(sessionId),
        getSkillExecutionLogs(undefined, sessionId),
        getMcpExecutionLogs(sessionId),
        getSetting("session_opening_prompts").catch(() => null),
      ]);
      if (found) {
        setSession(found);
        setView(found.view === "terminal" ? "terminal" : "messages");
        // 关联任务：冷启动卡使用
        const taskId = found.related_task_id || found.task_id;
        if (taskId) {
          try {
            setTask(await getTask(taskId));
          } catch {
            setTask(null);
          }
        }
      }
      setSkillLogs(sLogs);
      setMcpLogs(mLogs);
      const prompts = safeParseJson<string[]>(promptsRaw, []);
      if (prompts.length > 0) setOpeningPrompts(prompts);
    } catch (error) {
      console.error("加载会话详情失败:", error);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (id) loadData(id);
  }, [id, loadData]);

  // 单轮 result 落库 = Token/费用变化：轻量单查刷新（P1-9 成本仪表数据源）
  useEffect(() => {
    if (!id) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;
    listen<{ session_id: string }>("session:result", (event) => {
      if (event.payload.session_id !== id) return;
      void refreshSession();
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [id, refreshSession]);

  /** 重建会话（P2-14：后端原子操作取代前端轮询 hack） */
  async function rebuildSession(params: Parameters<typeof forceRebuildSession>[0]) {
    await forceRebuildSession(params);
  }

  /** 被拒卡片：允许该工具并重试 */
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
      await useSessionMessagesStore.getState().reload();
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
      await useSessionMessagesStore.getState().reload();
    } catch (error) {
      showToast("error", `切换失败: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  /** 错误出口：发送失败后显式重建（P0-4） */
  async function handleRebuildFromError() {
    if (!id) return;
    setBusy(true);
    try {
      await rebuildSession({ sessionId: id, mode: "structured" });
      setSendError(null);
      await refreshSession();
      await useSessionMessagesStore.getState().reload();
      showToast("success", "会话已重建，可继续输入");
    } catch (error) {
      showToast("error", `重建失败: ${error}`);
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

  /** 冷启动：把任务作为首条指令发给 Agent（P1-8） */
  async function handleStartFromTask() {
    if (!id) return;
    const parts = [`请执行当前任务。`];
    if (task) {
      parts.push(`任务：${task.title}`);
      if (task.description) parts.push(`任务描述：\n${task.description}`);
    } else if (session?.work_dir) {
      parts.push(`工作目录：${session.work_dir}`);
    }
    setSendError(null);
    try {
      await sendMessage(parts.join("\n\n"));
      await refreshSession();
    } catch (e) {
      setSendError(String(e));
    }
  }

  /** 重命名（P1-6） */
  async function handleRenameConfirm() {
    if (!id) return;
    try {
      await renameAgentSession(id, renameValue);
      setRenameOpen(false);
      await refreshSession();
      showToast("success", "已重命名");
    } catch (error) {
      showToast("error", `重命名失败: ${error}`);
    }
  }

  /** 导出 Markdown（P1-7） */
  async function handleExport() {
    if (!id) return;
    setBusy(true);
    try {
      const path = await exportAgentSession(id);
      showToast("success", `已导出：${path}`);
    } catch (error) {
      showToast("error", `导出失败: ${error}`);
    } finally {
      setBusy(false);
    }
  }

  /** 归档 / 取消归档（P1-7） */
  async function handleToggleArchive() {
    if (!id || !session) return;
    try {
      const next = session.archived !== 1;
      await setAgentSessionArchived(id, next);
      await refreshSession();
      showToast("success", next ? "已归档" : "已取消归档");
    } catch (error) {
      showToast("error", `操作失败: ${error}`);
    }
  }

  /** 删除（P1-7）：级联清理，确认后返回列表 */
  async function handleDelete() {
    if (!id) return;
    setBusy(true);
    try {
      await deleteAgentSession(id);
      showToast("success", "会话已删除");
      navigate("/sessions");
    } catch (error) {
      showToast("error", `删除失败: ${error}`);
    } finally {
      setBusy(false);
      setDeleteOpen(false);
    }
  }

  const sessionSkills = useMemo(
    () => safeParseJson<{ skill_id: string; skill_name?: string }[]>(session?.skills, []),
    [session?.skills],
  );
  const sessionMcps = useMemo(
    () => safeParseJson<{ mcp_name: string; endpoint?: string; status?: string }[]>(session?.mcps, []),
    [session?.mcps],
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
          加载中...
        </div>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-3">
        <div className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
          会话不存在或已被删除
        </div>
        <button className="btn-secondary text-xs" onClick={() => navigate("/sessions")}>
          返回会话列表
        </button>
      </div>
    );
  }

  // 会话模式：显式字段优先；历史会话（无 mode）有 CLI 会话 id 视为结构化，否则终端
  const sessionMode = session.mode ?? (session.cli_session_id ? "structured" : "terminal");
  const isStructured = sessionMode === "structured";
  const isRunning = session.status === "running";
  const statusText =
    session.status === "running"
      ? "运行中"
      : session.status === "completed"
        ? "已完成"
        : session.status === "failed"
          ? "失败"
          : session.status === "launching"
            ? "启动中"
            : session.status === "terminated"
              ? "已终止"
              : "已创建";
  const statusStyle =
    session.status === "running"
      ? { background: "var(--color-primary-subtle)", color: "var(--color-primary)" }
      : session.status === "completed"
        ? { background: "var(--color-success-light)", color: "var(--color-success)" }
        : { background: "var(--color-danger-light)", color: "var(--color-danger)" };

  return (
    <div className="p-6 space-y-5">
      {/* 顶部导航 + 会话管理动作（P1-6/7） */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => navigate("/sessions")}
          className="p-1.5 rounded-lg"
          style={{ color: "var(--color-text-tertiary)" }}
          title="返回会话列表"
        >
          <ArrowLeft className="w-4 h-4" />
        </button>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h1
              className="text-xl font-semibold truncate"
              style={{ color: "var(--color-text)" }}
              title={session.title || session.id}
            >
              {session.title || `会话 ${session.id.slice(0, 8)}`}
            </h1>
            <IconButton
              title="重命名会话"
              onClick={() => {
                setRenameValue(session.title || "");
                setRenameOpen(true);
              }}
            >
              <Pencil className="w-3.5 h-3.5" />
            </IconButton>
            {session.archived === 1 && (
              <span
                className="text-[10px] px-1.5 py-0.5 rounded"
                style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-tertiary)" }}
              >
                已归档
              </span>
            )}
          </div>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
            <span
              className="ml-1 inline-flex items-center px-1.5 py-0.5 rounded-full text-[10px] font-medium"
              style={statusStyle}
            >
              {statusText}
            </span>
            {session.source && (
              <span className="ml-2 text-[10px] px-1.5 py-0.5 rounded" style={{ background: "var(--color-bg-subtle)" }}>
                {session.source === "native" ? "原生" : session.source === "imported" ? "导入" : "手动"}
              </span>
            )}
            {session.id && (
              <span className="ml-2 font-mono text-[10px]" title={session.id}>
                {session.id.slice(0, 8)}
              </span>
            )}
          </p>
        </div>
        <div className="flex items-center gap-1.5">
          <IconButton title="导出为 Markdown" onClick={handleExport} disabled={busy}>
            <Download className="w-4 h-4" />
          </IconButton>
          <IconButton
            title={session.archived === 1 ? "取消归档" : "归档（数据保留，默认列表隐藏）"}
            onClick={handleToggleArchive}
            disabled={busy}
          >
            {session.archived === 1 ? <ArchiveRestore className="w-4 h-4" /> : <Archive className="w-4 h-4" />}
          </IconButton>
          <IconButton title="删除会话（级联清理消息）" onClick={() => setDeleteOpen(true)} danger disabled={busy}>
            <Trash2 className="w-4 h-4" />
          </IconButton>
        </div>
      </div>

      <div className="grid gap-5" style={{ gridTemplateColumns: "minmax(0, 1fr) 300px" }}>
        {/* 左侧：对话/终端/原始事件流 */}
        <div className="min-w-0 space-y-5">
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
                {isStructured && (
                  <button
                    type="button"
                    onClick={() => setView("raw")}
                    className="px-3 py-1 text-xs rounded-md flex items-center gap-1.5"
                    style={
                      view === "raw"
                        ? { background: "var(--color-bg)", color: "var(--color-primary)" }
                        : { color: "var(--color-text-tertiary)" }
                    }
                    title="结构化消息的原始落库视图，排障用"
                  >
                    <Eye className="w-3.5 h-3.5" /> 原始流
                  </button>
                )}
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
                    emptyState={
                      // 冷启动引导（P1-8）：任务卡 + 一键开始 + 建议开场白
                      <div className="max-w-xl mx-auto text-left space-y-3 px-2">
                        <div
                          className="rounded-xl px-4 py-3"
                          style={{
                            border: "1px solid var(--color-primary)",
                            background: "var(--color-primary-subtle)",
                          }}
                        >
                          <p
                            className="text-xs font-medium flex items-center gap-1.5"
                            style={{ color: "var(--color-primary)" }}
                          >
                            <Sparkles className="w-3.5 h-3.5" />
                            {task ? "已关联任务" : "空会话"}
                          </p>
                          {task ? (
                            <>
                              <p className="text-sm font-medium mt-1.5" style={{ color: "var(--color-text)" }}>
                                {task.title}
                              </p>
                              {task.description && (
                                <p
                                  className="text-xs mt-1 line-clamp-3"
                                  style={{ color: "var(--color-text-secondary)" }}
                                >
                                  {task.description}
                                </p>
                              )}
                              {isRunning && (
                                <button
                                  type="button"
                                  className="btn-primary text-xs mt-2.5"
                                  onClick={handleStartFromTask}
                                  disabled={sending}
                                >
                                  按任务开始（自动发送任务描述）
                                </button>
                              )}
                            </>
                          ) : (
                            <p className="text-xs mt-1" style={{ color: "var(--color-text-secondary)" }}>
                              在下方输入指令开始与 Agent 对话；
                              {session.work_dir ? `当前工作目录：${session.work_dir}` : "未指定工作目录"}
                            </p>
                          )}
                        </div>
                        <div>
                          <p className="text-[11px] mb-1.5" style={{ color: "var(--color-text-tertiary)" }}>
                            建议开场白（点击发送，可在设置中自定义）：
                          </p>
                          <div className="flex flex-col gap-1.5">
                            {openingPrompts.map((p) => (
                              <button
                                key={p}
                                type="button"
                                disabled={!isRunning || sending}
                                onClick={() => {
                                  setSendError(null);
                                  sendMessage(p).catch((e) => setSendError(String(e)));
                                }}
                                className="text-left text-xs px-3 py-2 rounded-lg transition-colors"
                                style={{
                                  background: "var(--color-bg-subtle)",
                                  color: "var(--color-text-secondary)",
                                  border: "1px solid var(--color-border-subtle)",
                                }}
                              >
                                {p}
                              </button>
                            ))}
                          </div>
                        </div>
                      </div>
                    }
                  />
                ) : (
                  <div className="h-full flex items-center justify-center px-6">
                    <p className="text-sm text-center" style={{ color: "var(--color-text-tertiary)" }}>
                      终端会话不提供消息视图。
                      <br />
                      请切换到「终端」查看与交互。
                    </p>
                  </div>
                )
              ) : view === "raw" ? (
                <div className="h-full overflow-y-auto px-4 py-3 space-y-2">
                  {messages.length === 0 ? (
                    <p className="text-center text-xs py-8" style={{ color: "var(--color-text-tertiary)" }}>
                      暂无落库消息
                    </p>
                  ) : (
                    messages.map((m) => (
                      <div
                        key={m.id}
                        className="rounded-lg px-3 py-2"
                        style={{ border: "1px solid var(--color-border-subtle)", background: "var(--color-bg-subtle)" }}
                      >
                        <div className="flex items-center gap-2 text-[11px] mb-1">
                          <span className="font-mono" style={{ color: "var(--color-primary)" }}>
                            #{m.seq}
                          </span>
                          <span style={{ color: "var(--color-text-secondary)" }}>
                            {m.role} · {m.block_type}
                          </span>
                          {m.tool_name && <span style={{ color: "var(--color-warning, #b98900)" }}>{m.tool_name}</span>}
                          <span className="ml-auto" style={{ color: "var(--color-text-tertiary)" }}>
                            {m.status}
                          </span>
                        </div>
                        <pre
                          className="text-[11px] font-mono whitespace-pre-wrap break-words m-0"
                          style={{ color: "var(--color-text-secondary)" }}
                        >
                          {m.content || "(empty)"}
                        </pre>
                      </div>
                    ))
                  )}
                </div>
              ) : isStructured ? (
                <div className="h-full flex items-center justify-center px-6">
                  <p className="text-sm text-center" style={{ color: "var(--color-text-tertiary)" }}>
                    结构化会话的终端视图仅用于排障（原始事件见「原始流」）。
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
              <div>
                {/* 发送失败常驻错误条 + 重建出口（P0-4） */}
                {sendError && (
                  <div
                    className="mx-3 mb-1 rounded-lg px-3 py-2 flex items-center gap-2 text-xs"
                    style={{
                      background: "var(--color-danger-light, #fde8e8)",
                      border: "1px solid var(--color-danger)",
                      color: "var(--color-danger)",
                    }}
                  >
                    <span className="flex-1 break-all">发送失败：{sendError}</span>
                    <button
                      type="button"
                      className="btn-secondary text-xs"
                      onClick={handleRebuildFromError}
                      disabled={busy}
                    >
                      {busy ? "重建中…" : "重建会话"}
                    </button>
                    <button type="button" className="p-1 rounded" onClick={() => setSendError(null)} title="关闭提示">
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                )}
                <SessionInput
                  sessionId={session.id}
                  running={isRunning}
                  sending={sending}
                  onSend={(content) => {
                    setSendError(null);
                    sendMessage(content).catch((err) => setSendError(String(err)));
                  }}
                  onInterrupt={() => {
                    interrupt().then(() => refreshSession());
                  }}
                />
              </div>
            )}
          </div>
        </div>

        {/* 右侧：信息面板 */}
        <div className="min-w-0 space-y-5">
          {/* Token / 成本仪表（P1-9：result 事件驱动实时刷新） */}
          <div className="panel">
            <h2 className="panel-title mb-3 flex items-center gap-1.5">
              <Coins className="w-4 h-4" /> 成本
              {isRunning && (
                <span
                  className="ml-auto text-[10px] w-1.5 h-1.5 rounded-full animate-pulse"
                  style={{ background: "var(--color-success)" }}
                  title="运行中，单轮完成后自动刷新"
                />
              )}
            </h2>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between">
                <dt style={{ color: "var(--color-text-tertiary)" }}>Token 总计</dt>
                <dd className="font-medium" style={{ color: "var(--color-text)" }}>
                  {(session.total_tokens || 0).toLocaleString()}
                </dd>
              </div>
              <div className="flex justify-between">
                <dt style={{ color: "var(--color-text-tertiary)" }}>费用</dt>
                <dd style={{ color: "var(--color-text)" }}>${(session.total_cost || 0).toFixed(4)}</dd>
              </div>
              {session.duration_seconds ? (
                <div className="flex justify-between">
                  <dt style={{ color: "var(--color-text-tertiary)" }}>耗时</dt>
                  <dd style={{ color: "var(--color-text)" }}>
                    {Math.floor(session.duration_seconds / 60)}分{session.duration_seconds % 60}秒
                  </dd>
                </div>
              ) : null}
              {session.work_dir && (
                <div className="flex justify-between gap-2">
                  <dt className="shrink-0" style={{ color: "var(--color-text-tertiary)" }}>
                    工作目录
                  </dt>
                  <dd
                    className="font-mono text-[11px] truncate max-w-[170px]"
                    style={{ color: "var(--color-text-secondary)" }}
                    title={session.work_dir}
                  >
                    {session.work_dir}
                  </dd>
                </div>
              )}
            </dl>
          </div>

          {/* Skill 执行记录 */}
          <div className="panel">
            <div className="panel-header">
              <h2 className="panel-title flex items-center gap-2">
                <Zap className="w-4 h-4" /> Skill 执行记录
              </h2>
            </div>
            {skillLogs.length === 0 ? (
              <p className="text-sm py-4 text-center" style={{ color: "var(--color-text-tertiary)" }}>
                暂无执行记录
              </p>
            ) : (
              <div className="space-y-2 max-h-56 overflow-y-auto">
                {skillLogs.map((log) => (
                  <div
                    key={log.id}
                    className="flex items-center justify-between p-2 rounded-lg"
                    style={{ background: "var(--color-bg-subtle)" }}
                  >
                    <div className="flex items-center gap-2">
                      <Zap className="w-3.5 h-3.5" style={{ color: "var(--color-primary)" }} />
                      <span className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                        {log.skill_name}
                      </span>
                      <span
                        className="text-[10px] px-1.5 py-0.5 rounded"
                        style={{ background: "var(--color-bg)", color: "var(--color-text-tertiary)" }}
                      >
                        {log.execution_phase}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                      <span>{log.token_usage.toLocaleString()} tokens</span>
                      <span
                        style={{
                          color: log.status === "success" ? "var(--color-success)" : "var(--color-danger)",
                        }}
                      >
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
              <p className="text-sm py-4 text-center" style={{ color: "var(--color-text-tertiary)" }}>
                暂无调用记录
              </p>
            ) : (
              <div className="space-y-2 max-h-56 overflow-y-auto">
                {mcpLogs.map((log) => (
                  <div
                    key={log.id}
                    className="flex items-center justify-between p-2 rounded-lg"
                    style={{ background: "var(--color-bg-subtle)" }}
                  >
                    <div className="flex items-center gap-2">
                      <Plug className="w-3.5 h-3.5" style={{ color: "var(--color-success)" }} />
                      <span className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                        {log.mcp_name}
                      </span>
                      {log.action && (
                        <span
                          className="text-[10px] px-1.5 py-0.5 rounded"
                          style={{ background: "var(--color-bg)", color: "var(--color-text-tertiary)" }}
                        >
                          {log.action}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                      {log.duration_ms && <span>{log.duration_ms}ms</span>}
                      <span
                        style={{
                          color:
                            log.status === "connected" || log.status === "call_success"
                              ? "var(--color-success)"
                              : "var(--color-danger)",
                        }}
                      >
                        {log.status === "connected" || log.status === "call_success" ? "✓" : "✗"}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* 关联任务 / 上下文 */}
          {(session.related_task_id || session.task_id) && (
            <div className="panel">
              <h2 className="panel-title mb-3 flex items-center gap-1.5">
                <Link2 className="w-4 h-4" /> 关联任务
              </h2>
              {task ? (
                <button
                  className="text-sm text-left hover:underline"
                  style={{ color: "var(--color-text-secondary)" }}
                  onClick={() => navigate(`/tasks/${session.related_task_id || session.task_id}`)}
                >
                  {task.title}
                </button>
              ) : (
                <p className="font-mono text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                  {session.related_task_id || session.task_id}
                </p>
              )}
            </div>
          )}

          {/* 会话信息（执行人 + Skill + MCP 合并展示） */}
          <div className="panel">
            <h2 className="panel-title mb-3">会话信息</h2>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between">
                <dt style={{ color: "var(--color-text-tertiary)" }}>Agent</dt>
                <dd style={{ color: "var(--color-text)" }}>{session.agent_name || session.agent_path || "Unknown"}</dd>
              </div>
              <div className="flex justify-between">
                <dt style={{ color: "var(--color-text-tertiary)" }}>来源</dt>
                <dd style={{ color: "var(--color-text)" }}>
                  {session.source === "native" ? "平台创建" : session.source === "imported" ? "文件导入" : "手动导入"}
                </dd>
              </div>
              {session.model && (
                <div className="flex justify-between">
                  <dt style={{ color: "var(--color-text-tertiary)" }}>模型</dt>
                  <dd style={{ color: "var(--color-text)" }}>{session.model}</dd>
                </div>
              )}
            </dl>
            {sessionSkills.length > 0 && (
              <div className="mt-3 pt-3 space-y-1.5" style={{ borderTop: "1px solid var(--color-border-subtle)" }}>
                <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                  使用的 Skill
                </p>
                {sessionSkills.map((s, i) => (
                  <div key={i} className="flex items-center gap-2 text-sm">
                    <Zap className="w-3 h-3" style={{ color: "var(--color-primary)" }} />
                    <span style={{ color: "var(--color-text-secondary)" }}>{s.skill_name || s.skill_id}</span>
                  </div>
                ))}
              </div>
            )}
            {sessionMcps.length > 0 && (
              <div className="mt-3 pt-3 space-y-1.5" style={{ borderTop: "1px solid var(--color-border-subtle)" }}>
                <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                  连接的 MCP
                </p>
                {sessionMcps.map((m, i) => (
                  <div key={i} className="flex items-center justify-between text-sm">
                    <div className="flex items-center gap-2">
                      <span
                        className="w-2 h-2 rounded-full"
                        style={{
                          background: m.status === "connected" ? "var(--color-success)" : "var(--color-border)",
                        }}
                      />
                      <span style={{ color: "var(--color-text-secondary)" }}>{m.mcp_name}</span>
                    </div>
                    {m.endpoint && (
                      <span
                        className="text-[10px] font-mono truncate max-w-[100px]"
                        style={{ color: "var(--color-text-tertiary)" }}
                        title={m.endpoint}
                      >
                        {m.endpoint}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* 重命名弹窗 */}
      {renameOpen && (
        <div className="modal-overlay" onClick={() => setRenameOpen(false)}>
          <div className="modal-content w-full max-w-md" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                重命名会话
              </h2>
            </div>
            <div className="modal-body space-y-3">
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
              <button className="btn-secondary" onClick={() => setRenameOpen(false)}>
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
      {deleteOpen && (
        <div className="modal-overlay" onClick={() => !busy && setDeleteOpen(false)}>
          <div className="modal-content w-full max-w-md" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                删除会话
              </h2>
            </div>
            <div className="modal-body space-y-2">
              <p className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                将删除会话「{session.title || session.id.slice(0, 8)}」及其全部消息、Skill/MCP 执行记录与 Token
                记录。此操作不可恢复。
              </p>
              <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                建议先「导出为 Markdown」留存对话。
              </p>
            </div>
            <div className="modal-footer">
              <button className="btn-secondary" onClick={() => setDeleteOpen(false)} disabled={busy}>
                取消
              </button>
              <button
                className="btn-primary"
                style={{ background: "var(--color-danger)" }}
                onClick={handleDelete}
                disabled={busy}
              >
                {busy ? "删除中…" : "确认删除"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
