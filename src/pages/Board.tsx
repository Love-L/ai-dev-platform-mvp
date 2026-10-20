import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, Trash2, Search } from "lucide-react";
import type { Task } from "../lib/types";
import { useTasksStore } from "../stores";
import { TaskTypeBadge, PriorityBadge } from "../components/Badges";
import TaskForm from "../components/TaskForm";
import { createTask } from "../lib/tauri";
import { showToast } from "../components/Toast";

const COLUMNS = [
  { id: "pending", label: "待开发", dotColor: "bg-gray-400" },
  { id: "in_progress", label: "开发中", dotColor: "bg-blue-500" },
  { id: "blocked", label: "已阻塞", dotColor: "bg-red-500" },
  { id: "completed", label: "已完成", dotColor: "bg-emerald-500" },
];

const TASK_TYPES = [
  { id: "development", label: "开发任务" },
  { id: "testing", label: "测试任务" },
  { id: "bug_fix", label: "Bug修复" },
  { id: "review", label: "评审任务" },
];

export default function Board() {
  const navigate = useNavigate();
  const { tasks, loading, fetchTasks, updateStatus, removeTask } = useTasksStore();
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [draggedTask, setDraggedTask] = useState<Task | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [typeFilter, setTypeFilter] = useState<string>("");

  useEffect(() => {
    fetchTasks();
  }, []);

  function handleDragStart(task: Task) {
    setDraggedTask(task);
  }

  function handleDragOver(e: React.DragEvent) {
    e.preventDefault();
  }

  async function handleDrop(e: React.DragEvent, status: string) {
    e.preventDefault();
    if (!draggedTask || draggedTask.status === status) {
      setDraggedTask(null);
      return;
    }
    try {
      await updateStatus(draggedTask.id, status);
      setDraggedTask(null);
    } catch (error) {
      console.error("更新状态失败:", error);
      showToast("error", `更新状态失败: ${error}`);
    }
  }

  async function handleCreate(data: any) {
    try {
      await createTask(
        data.title,
        undefined,
        data.task_type,
        data.priority,
        data.assignee,
        data.due_date,
        data.estimated_hours,
        data.priority_level,
        data.blocked_reason,
        data.tags,
        data.notes,
        data.description,
      );
      setShowCreateModal(false);
      await fetchTasks();
    } catch (error) {
      console.error("创建任务失败:", error);
      showToast("error", `创建任务失败: ${error}`);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm("确定要删除这个任务吗？")) return;
    try {
      await removeTask(id);
    } catch (error) {
      console.error("删除任务失败:", error);
      showToast("error", `删除任务失败: ${error}`);
    }
  }

  const filteredTasks = tasks.filter((task) => {
    const matchSearch = !searchTerm || task.title.toLowerCase().includes(searchTerm.toLowerCase());
    const matchType = !typeFilter || task.task_type === typeFilter;
    return matchSearch && matchType;
  });

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-sm text-gray-400">
          <div className="w-4 h-4 border-2 border-gray-300 border-t-blue-500 rounded-full animate-spin" />
          加载中...
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 h-full flex flex-col">
      {/* 页面标题 */}
      <div className="flex items-center justify-between mb-5">
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            任务看板
          </h1>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
            共 {tasks.length} 个任务
          </p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建任务
        </button>
      </div>

      {/* 筛选栏 */}
      <div className="flex flex-col sm:flex-row gap-3 mb-5">
        <div className="flex-1 relative">
          <Search
            className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2"
            style={{ color: "var(--color-text-tertiary)" }}
          />
          <input
            type="text"
            placeholder="搜索任务..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="input pl-9"
          />
        </div>
        <select value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)} className="select w-full sm:w-32">
          <option value="">全部类型</option>
          {TASK_TYPES.map((t) => (
            <option key={t.id} value={t.id}>
              {t.label}
            </option>
          ))}
        </select>
      </div>

      {/* 看板列 */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 flex-1 min-h-0">
        {COLUMNS.map((col) => {
          const columnTasks = filteredTasks.filter((t) => t.status === col.id);
          return (
            <div
              key={col.id}
              className="flex flex-col rounded-xl"
              style={{ background: "var(--color-bg-subtle)", border: "1px solid var(--color-border-subtle)" }}
              onDragOver={handleDragOver}
              onDrop={(e) => handleDrop(e, col.id)}
            >
              {/* 列标题 */}
              <div className="px-3 py-2.5" style={{ borderBottom: "1px solid var(--color-border-subtle)" }}>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className={`w-2 h-2 rounded-full ${col.dotColor}`} />
                    <h3 className="text-xs font-semibold" style={{ color: "var(--color-text-secondary)" }}>
                      {col.label}
                    </h3>
                  </div>
                  <span
                    className="text-[10px] font-mono px-1.5 py-0.5 rounded"
                    style={{ background: "var(--color-surface)", color: "var(--color-text-tertiary)" }}
                  >
                    {columnTasks.length}
                  </span>
                </div>
              </div>

              {/* 任务卡片列表 */}
              <div className="flex-1 overflow-y-auto p-2 space-y-2">
                {columnTasks.length === 0 ? (
                  <div className="text-center py-8 text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                    暂无任务
                  </div>
                ) : (
                  columnTasks.map((task) => (
                    <div
                      key={task.id}
                      draggable
                      onDragStart={() => handleDragStart(task)}
                      onClick={() => navigate(`/tasks/${task.id}`)}
                      className="rounded-lg p-3 cursor-move group"
                      style={{
                        background: "var(--color-surface)",
                        border: "1px solid var(--color-border)",
                        transition: "border-color var(--transition-fast), box-shadow var(--transition-fast)",
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.borderColor = "var(--color-primary)";
                        e.currentTarget.style.boxShadow = "var(--shadow-sm)";
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.borderColor = "";
                        e.currentTarget.style.boxShadow = "";
                      }}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <p className="text-sm font-medium line-clamp-2 flex-1" style={{ color: "var(--color-text)" }}>
                          {task.title}
                        </p>
                        <button
                          onClick={() => handleDelete(task.id)}
                          className="p-1 rounded opacity-0 group-hover:opacity-100 flex-shrink-0"
                          style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
                          onMouseEnter={(e) => {
                            e.currentTarget.style.color = "var(--color-danger)";
                            e.currentTarget.style.background = "var(--color-danger-light)";
                          }}
                          onMouseLeave={(e) => {
                            e.currentTarget.style.color = "";
                            e.currentTarget.style.background = "";
                          }}
                          aria-label="删除任务"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                      <div className="flex items-center gap-1.5 mt-2">
                        <TaskTypeBadge type={task.task_type} />
                        <PriorityBadge priority={task.priority} />
                      </div>
                      {task.agent_type && (
                        <p className="text-xs mt-2" style={{ color: "var(--color-text-tertiary)" }}>
                          {task.agent_type}
                        </p>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* 创建弹窗 */}
      {showCreateModal && <TaskForm onSubmit={handleCreate} onCancel={() => setShowCreateModal(false)} />}
    </div>
  );
}
