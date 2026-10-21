import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  MessageSquare,
  Play,
  Send,
  Trash2,
  Clock,
  User,
  Tag,
  Pencil,
  FileText,
  ExternalLink,
  GitCommit,
  Bot,
} from "lucide-react";
import type { Task, TaskComment, Requirement } from "../lib/types";
import {
  getTask,
  listTaskComments,
  createTaskComment,
  deleteTaskComment,
  createSessionFromTask,
  updateTask,
  updateTaskStatus,
  getRequirement,
  getTaskTransitions,
  listGitCommits,
  getAgentSessions,
  type GitCommitRecord,
} from "../lib/tauri";
import { TaskTypeBadge, PriorityBadge, StatusBadge } from "../components/Badges";
import { showToast } from "../components/Toast";
import TaskForm from "../components/TaskForm";

/** 任务状态文案 */
const TASK_STATUS_LABEL: Record<string, string> = {
  pending: "待处理",
  in_progress: "进行中",
  blocked: "阻塞",
  completed: "已完成",
};

/** 需求状态文案（来源需求展示用） */
const REQ_STATUS_LABEL: Record<string, string> = {
  draft: "草稿",
  reviewing: "评审中",
  approved: "已批准",
  rejected: "已拒绝",
  frozen: "已冻结",
  in_development: "开发中",
  testing: "测试中",
  completed: "已完成",
};

/**
 * 状态机兜底表：仅在后端 `get_task_transitions` 拉取失败（纯浏览器预览等）时使用。
 * 正常运行以后端下发为唯一真源（与 validation::task_transitions 同源）。
 */
const TASK_TRANSITIONS_FALLBACK: Record<string, string[]> = {
  pending: ["in_progress", "blocked"],
  in_progress: ["completed", "blocked", "pending"],
  blocked: ["pending", "in_progress"],
  completed: ["pending"],
};

function nextTaskStatuses(current: string, table: Record<string, string[]>): string[] {
  return table[current] ?? [];
}

export default function TaskDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [task, setTask] = useState<Task | null>(null);
  const [comments, setComments] = useState<TaskComment[]>([]);
  const [loading, setLoading] = useState(true);
  const [newComment, setNewComment] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [startingSession, setStartingSession] = useState(false);
  const [showEdit, setShowEdit] = useState(false);
  const [changingStatus, setChangingStatus] = useState(false);
  const [sourceReq, setSourceReq] = useState<Requirement | null>(null);
  // P1-T5：状态机以后端下发为唯一真源
  const [transitions, setTransitions] = useState<Record<string, string[]>>(TASK_TRANSITIONS_FALLBACK);
  // P1-T8：追溯视图（本任务的关联提交与会话）
  const [linkedCommits, setLinkedCommits] = useState<GitCommitRecord[]>([]);
  const [linkedSessions, setLinkedSessions] = useState<
    Array<{ id: string; title?: string | null; status: string; started_at?: string | null }>
  >([]);

  useEffect(() => {
    if (id) {
      loadTask(id);
      loadComments(id);
      loadTrace(id);
    }
  }, [id]);

  // 状态机以后端为唯一真源（拉取失败退回兜底表）
  useEffect(() => {
    getTaskTransitions()
      .then((table) => {
        if (Object.keys(table).length > 0) setTransitions(table);
      })
      .catch(() => {});
  }, []);

  /** 加载追溯数据：关联提交 + 关联会话 */
  async function loadTrace(taskId: string) {
    try {
      const [commits, sessions] = await Promise.all([
        listGitCommits(taskId, undefined, 20).catch(() => []),
        getAgentSessions().catch(() => []),
      ]);
      setLinkedCommits(commits);
      setLinkedSessions(sessions.filter((s) => s.task_id === taskId || s.related_task_id === taskId));
    } catch {
      // 追溯数据加载失败不阻断页面
    }
  }

  // 加载来源需求内容（任务详情展示 + 跳转）
  const requirementId = task?.requirement_id || task?.related_requirement_id;
  useEffect(() => {
    if (!requirementId) {
      setSourceReq(null);
      return;
    }
    getRequirement(requirementId)
      .then(setSourceReq)
      .catch(() => setSourceReq(null));
  }, [requirementId]);

  async function loadTask(taskId: string) {
    try {
      const data = await getTask(taskId);
      setTask(data);
    } catch (error) {
      console.error("加载任务失败:", error);
    } finally {
      setLoading(false);
    }
  }

  async function loadComments(taskId: string) {
    try {
      const data = await listTaskComments(taskId);
      setComments(data);
    } catch (error) {
      console.error("加载评论失败:", error);
    }
  }

  async function handleAddComment() {
    if (!id || !newComment.trim() || submitting) return;
    setSubmitting(true);
    try {
      await createTaskComment(id, "user", newComment);
      setNewComment("");
      await loadComments(id);
    } catch (error) {
      console.error("添加评论失败:", error);
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDeleteComment(commentId: string) {
    if (!confirm("确定要删除这条评论吗？")) return;
    try {
      await deleteTaskComment(commentId);
      if (id) await loadComments(id);
    } catch (error) {
      console.error("删除评论失败:", error);
    }
  }

  /** 编辑保存：基础字段 + 扩展字段（P0-T2 起可回改估时/优先级/阻塞原因/备注） */
  async function handleUpdate(data: Partial<Task>) {
    if (!task) return;
    try {
      await updateTask(task.id, {
        title: data.title,
        description: data.description,
        task_type: data.task_type,
        priority: data.priority,
        assignee: data.assignee,
        due_date: data.due_date,
        tags: data.tags,
        estimated_hours: data.estimated_hours,
        priority_level: data.priority_level,
        blocked_reason: data.blocked_reason,
        notes: data.notes,
      });
      setShowEdit(false);
      await loadTask(task.id);
      showToast("success", "任务已更新");
    } catch (error) {
      console.error("更新任务失败:", error);
      showToast("error", "保存失败: " + (error instanceof Error ? error.message : String(error)));
    }
  }

  /** 手动变更状态：仅提供合法流转目标，后端再次校验 */
  async function handleStatusChange(next: string) {
    if (!task || next === task.status || changingStatus) return;
    setChangingStatus(true);
    try {
      await updateTaskStatus(task.id, next);
      await loadTask(task.id);
      showToast("success", `状态已更新为「${TASK_STATUS_LABEL[next] ?? next}」`);
    } catch (error) {
      console.error("更新状态失败:", error);
      showToast("error", "状态更新失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setChangingStatus(false);
    }
  }

  /** 触发 Claude 会话：以结构化（消息）模式启动真实 Agent 进程，任务作为上下文注入 */
  async function handleStartSession() {
    if (!id) return;
    setStartingSession(true);
    try {
      const session = await createSessionFromTask(id);
      showToast("success", "Claude 会话已启动");
      navigate(`/sessions/${session.id}`);
    } catch (error) {
      console.error("启动 Claude 会话失败:", error);
      showToast("error", "启动会话失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setStartingSession(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">加载中...</div>
      </div>
    );
  }

  if (!task) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <div className="text-sm text-gray-500">任务不存在</div>
        <button onClick={() => navigate("/board")} className="text-sm text-blue-600 hover:text-blue-700">
          返回看板
        </button>
      </div>
    );
  }

  return (
    <div className="p-6 h-full flex flex-col">
      {/* 头部 */}
      <div className="flex items-center gap-3 mb-6">
        <button onClick={() => navigate("/board")} className="p-1.5 rounded-lg hover:bg-gray-100 text-gray-500">
          <ArrowLeft className="w-4 h-4" />
        </button>
        <div className="flex-1">
          <h1 className="text-xl font-semibold text-gray-900">{task.title}</h1>
          <div className="flex items-center gap-2 mt-1">
            <TaskTypeBadge type={task.task_type} />
            <PriorityBadge priority={task.priority} />
            <StatusBadge status={task.status} />
          </div>
        </div>
        <button onClick={() => setShowEdit(true)} className="btn-secondary text-sm" title="编辑任务内容（标题/描述等）">
          <Pencil className="w-4 h-4" />
          编辑
        </button>
        <button
          onClick={handleStartSession}
          disabled={startingSession}
          className="btn-primary text-sm"
          title="以消息模式启动 Claude 会话，任务标题/描述/所属需求将作为上下文注入"
        >
          {startingSession ? <Clock className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
          {startingSession ? "启动中..." : "启动 Claude 会话"}
        </button>
      </div>

      <div className="flex-1 flex gap-6 overflow-hidden">
        {/* 左侧：任务信息 */}
        <div className="flex-1 overflow-auto">
          {/* 描述（可编辑；旧任务 description 为空时给出引导） */}
          <div className="mb-6">
            <div className="flex items-center justify-between mb-2">
              <h3 className="text-sm font-medium text-gray-700">描述</h3>
              <button
                onClick={() => setShowEdit(true)}
                className="text-xs text-blue-600 hover:text-blue-700 flex items-center gap-1"
              >
                <Pencil className="w-3 h-3" /> 编辑内容
              </button>
            </div>
            <div className="text-sm text-gray-600 whitespace-pre-wrap bg-gray-50 rounded-lg p-4 min-h-[56px]">
              {task.description || <span className="text-gray-400">暂无描述，点击「编辑内容」补充任务详情…</span>}
            </div>
          </div>

          {/* 来源需求（展示需求内容并可跳转） */}
          {requirementId && (
            <div className="mb-6">
              <h3 className="text-sm font-medium text-gray-700 mb-2 flex items-center gap-2">
                <FileText className="w-4 h-4" /> 来源需求
              </h3>
              <div className="bg-gray-50 rounded-lg p-4">
                {sourceReq ? (
                  <>
                    <div className="flex items-start justify-between gap-2">
                      <span className="text-sm font-medium text-gray-900">{sourceReq.title}</span>
                      <button
                        onClick={() => navigate(`/requirements/${sourceReq.id}`)}
                        className="text-xs text-blue-600 hover:text-blue-700 flex items-center gap-1 shrink-0"
                      >
                        查看需求 <ExternalLink className="w-3 h-3" />
                      </button>
                    </div>
                    <p className="text-xs text-gray-500 mt-1">
                      优先级 {sourceReq.priority} · 状态 {REQ_STATUS_LABEL[sourceReq.status] ?? sourceReq.status}
                    </p>
                    {(sourceReq.description || sourceReq.prd_content) && (
                      <div className="mt-2 text-sm text-gray-600 whitespace-pre-wrap max-h-64 overflow-y-auto">
                        {sourceReq.description || sourceReq.prd_content}
                      </div>
                    )}
                  </>
                ) : (
                  <p className="text-xs text-gray-400">需求加载中或不可用（ID: {requirementId}）</p>
                )}
              </div>
            </div>
          )}

          {/* 追溯视图（P1-T8）：本任务关联的会话与提交 */}
          {(linkedSessions.length > 0 || linkedCommits.length > 0) && (
            <div>
              <h3 className="text-sm font-medium text-gray-700 mb-3 flex items-center gap-2">
                <ExternalLink className="w-4 h-4" />
                执行追溯
              </h3>
              <div className="space-y-3">
                {linkedSessions.length > 0 && (
                  <div>
                    <p className="text-xs text-gray-500 mb-1.5 flex items-center gap-1">
                      <Bot className="w-3.5 h-3.5" /> 关联会话（{linkedSessions.length}）
                    </p>
                    <div className="space-y-1">
                      {linkedSessions.map((s) => (
                        <button
                          key={s.id}
                          onClick={() => navigate(`/sessions/${s.id}`)}
                          className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-gray-50 text-xs flex items-center gap-2"
                        >
                          <StatusBadge status={s.status} />
                          <span className="truncate flex-1 text-gray-700">{s.title || `会话 ${s.id.slice(0, 8)}`}</span>
                          {s.started_at && (
                            <span className="text-gray-400 flex-shrink-0">
                              {new Date(s.started_at).toLocaleString("zh-CN")}
                            </span>
                          )}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
                {linkedCommits.length > 0 && (
                  <div>
                    <p className="text-xs text-gray-500 mb-1.5 flex items-center gap-1">
                      <GitCommit className="w-3.5 h-3.5" /> 关联提交（{linkedCommits.length}）
                    </p>
                    <div className="space-y-1">
                      {linkedCommits.map((c) => (
                        <button
                          key={c.id}
                          onClick={() => navigate("/commits")}
                          className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-gray-50 text-xs flex items-center gap-2"
                          title="前往提交页查看详情"
                        >
                          <span className="font-mono text-[11px] px-1.5 py-0.5 rounded bg-gray-100 text-gray-600 flex-shrink-0">
                            {c.short_hash}
                          </span>
                          <span className="truncate flex-1 text-gray-700">{c.message || "(无提交信息)"}</span>
                          <span className="text-gray-400 flex-shrink-0">
                            {c.files_changed} 文件 · {new Date(c.created_at).toLocaleDateString("zh-CN")}
                          </span>
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* 评论区 */}
          <div>
            <h3 className="text-sm font-medium text-gray-700 mb-3 flex items-center gap-2">
              <MessageSquare className="w-4 h-4" />
              评论 ({comments.length})
            </h3>

            {/* 评论列表 */}
            <div className="space-y-3 mb-4">
              {comments.map((comment) => (
                <div key={comment.id} className="bg-gray-50 rounded-lg p-3 group">
                  <div className="flex items-center justify-between mb-1">
                    <div className="flex items-center gap-2">
                      <User className="w-3.5 h-3.5 text-gray-400" />
                      <span className="text-xs font-medium text-gray-700">{comment.author}</span>
                      <span className="text-xs text-gray-400">{new Date(comment.created_at).toLocaleString()}</span>
                    </div>
                    <button
                      onClick={() => handleDeleteComment(comment.id)}
                      className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-gray-200 text-gray-400 hover:text-red-500 transition-opacity"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                  <p className="text-sm text-gray-600 whitespace-pre-wrap">{comment.content}</p>
                </div>
              ))}

              {comments.length === 0 && <div className="text-xs text-gray-400 text-center py-4">暂无评论</div>}
            </div>

            {/* 添加评论 */}
            <div className="flex gap-2">
              <input
                type="text"
                value={newComment}
                onChange={(e) => setNewComment(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleAddComment()}
                placeholder="输入评论..."
                className="flex-1 px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <button
                onClick={handleAddComment}
                disabled={!newComment.trim() || submitting}
                className="px-3 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Send className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>

        {/* 右侧：元信息 */}
        <div className="w-64 flex-shrink-0">
          <div className="bg-gray-50 rounded-lg p-4 space-y-4">
            <div>
              <label className="text-xs font-medium text-gray-500">状态</label>
              <div className="mt-1">
                <StatusBadge status={task.status} />
              </div>
              <select
                value={task.status}
                onChange={(e) => handleStatusChange(e.target.value)}
                disabled={changingStatus}
                className="select mt-1.5 text-xs"
                title="手动变更状态（仅列出当前状态的合法流转）"
              >
                <option value={task.status}>{TASK_STATUS_LABEL[task.status] ?? task.status}（当前）</option>
                {nextTaskStatuses(task.status, transitions).map((s) => (
                  <option key={s} value={s}>
                    {TASK_STATUS_LABEL[s] ?? s}
                  </option>
                ))}
              </select>
              {task.status === "blocked" && task.blocked_reason && (
                <p className="text-[11px] text-red-500 mt-1">原因：{task.blocked_reason}</p>
              )}
            </div>

            <div>
              <label className="text-xs font-medium text-gray-500">类型</label>
              <div className="mt-1">
                <TaskTypeBadge type={task.task_type} />
              </div>
            </div>

            <div>
              <label className="text-xs font-medium text-gray-500">优先级</label>
              <div className="mt-1">
                <PriorityBadge priority={task.priority} />
              </div>
            </div>

            {task.assignee && (
              <div>
                <label className="text-xs font-medium text-gray-500">负责人</label>
                <div className="mt-1 text-sm text-gray-700 flex items-center gap-1">
                  <User className="w-3.5 h-3.5 text-gray-400" />
                  {task.assignee}
                </div>
              </div>
            )}

            {task.due_date && (
              <div>
                <label className="text-xs font-medium text-gray-500">截止日期</label>
                <div className="mt-1 text-sm text-gray-700 flex items-center gap-1">
                  <Clock className="w-3.5 h-3.5 text-gray-400" />
                  {task.due_date}
                </div>
              </div>
            )}

            {task.tags && (
              <div>
                <label className="text-xs font-medium text-gray-500">标签</label>
                <div className="mt-1 text-sm text-gray-700 flex items-center gap-1">
                  <Tag className="w-3.5 h-3.5 text-gray-400" />
                  {task.tags}
                </div>
              </div>
            )}

            <div>
              <label className="text-xs font-medium text-gray-500">创建时间</label>
              <div className="mt-1 text-sm text-gray-700">{new Date(task.created_at).toLocaleString()}</div>
            </div>

            <div>
              <label className="text-xs font-medium text-gray-500">更新时间</label>
              <div className="mt-1 text-sm text-gray-700">{new Date(task.updated_at).toLocaleString()}</div>
            </div>
          </div>
        </div>
      </div>

      {/* 编辑任务弹窗 */}
      {showEdit && task && <TaskForm task={task} onSubmit={handleUpdate} onCancel={() => setShowEdit(false)} />}
    </div>
  );
}
