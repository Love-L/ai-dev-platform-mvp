import { useState } from "react";
import { X, Save, Loader2 } from "lucide-react";
import type { Task } from "../lib/types";

interface TaskFormProps {
  task?: Task;
  onSubmit: (data: Partial<Task>) => Promise<void>;
  onCancel: () => void;
}

export default function TaskForm({ task, onSubmit, onCancel }: TaskFormProps) {
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formData, setFormData] = useState({
    title: task?.title || "",
    description: task?.description || "",
    task_type: task?.task_type || "development",
    priority: task?.priority || "P1",
    assignee: task?.assignee || "",
    due_date: task?.due_date || "",
    estimated_hours: task?.estimated_hours || "",
    priority_level: task?.priority_level || "",
    blocked_reason: task?.blocked_reason || "",
    tags: task?.tags || "",
    notes: task?.notes || "",
  });

  function validate(): boolean {
    const errs: Record<string, string> = {};
    const title = formData.title.trim();
    if (!title) {
      errs.title = "标题不能为空";
    } else if (title.length > 200) {
      errs.title = `标题不能超过 200 字（当前 ${title.length}）`;
    }
    if (formData.estimated_hours) {
      const hours = Number(formData.estimated_hours);
      if (isNaN(hours) || hours < 0) {
        errs.estimated_hours = "预估工时必须为非负数";
      } else if (hours > 10000) {
        errs.estimated_hours = "预估工时不能超过 10000 小时";
      }
    }
    if (formData.due_date) {
      const due = new Date(formData.due_date);
      const today = new Date();
      today.setHours(0, 0, 0, 0);
      if (due < today) {
        errs.due_date = "截止日期不能早于今天";
      }
    }
    setErrors(errs);
    return Object.keys(errs).length === 0;
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    setLoading(true);
    try {
      await onSubmit({
        ...formData,
        estimated_hours: formData.estimated_hours ? Number(formData.estimated_hours) : undefined,
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content w-full max-w-lg max-h-[90vh]">
        {/* Header */}
        <div className="modal-header">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold" style={{ color: "var(--color-text)" }}>
              {task ? "编辑任务" : "新建任务"}
            </h2>
            <button
              onClick={onCancel}
              className="p-1 rounded-lg transition-colors"
              style={{ color: "var(--color-text-tertiary)" }}
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="modal-body space-y-4 overflow-y-auto max-h-[calc(90vh-140px)]">
          {/* 标题 */}
          <div>
            <label className="label">
              标题 <span style={{ color: "var(--color-danger)" }}>*</span>
            </label>
            <input
              type="text"
              value={formData.title}
              onChange={(e) => setFormData({ ...formData, title: e.target.value })}
              className="input"
              style={errors.title ? { borderColor: "var(--color-danger)" } : undefined}
              placeholder="输入任务标题"
              required
              maxLength={200}
            />
            {errors.title && (
              <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>
                {errors.title}
              </p>
            )}
          </div>

          {/* 描述 */}
          <div>
            <label className="label">描述</label>
            <textarea
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              className="input resize-none"
              rows={3}
              placeholder="输入任务描述"
            />
          </div>

          {/* 类型和优先级 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="label">类型</label>
              <select
                value={formData.task_type}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    task_type: e.target.value as "testing" | "requirement" | "development" | "bug_fix" | "review",
                  })
                }
                className="select"
              >
                <option value="requirement">需求</option>
                <option value="development">开发</option>
                <option value="testing">测试</option>
                <option value="bug_fix">缺陷修复</option>
                <option value="review">评审</option>
              </select>
            </div>
            <div>
              <label className="label">优先级</label>
              <select
                value={formData.priority}
                onChange={(e) => setFormData({ ...formData, priority: e.target.value as "P0" | "P1" | "P2" | "P3" })}
                className="select"
              >
                <option value="P0">P0 - 紧急</option>
                <option value="P1">P1 - 高</option>
                <option value="P2">P2 - 中</option>
                <option value="P3">P3 - 低</option>
              </select>
            </div>
          </div>

          {/* 执行人和截止日期 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="label">执行人</label>
              <input
                type="text"
                value={formData.assignee}
                onChange={(e) => setFormData({ ...formData, assignee: e.target.value })}
                className="input"
                placeholder="执行人"
              />
            </div>
            <div>
              <label className="label">截止日期</label>
              <input
                type="date"
                value={formData.due_date}
                onChange={(e) => setFormData({ ...formData, due_date: e.target.value })}
                className="input"
                style={errors.due_date ? { borderColor: "var(--color-danger)" } : undefined}
              />
              {errors.due_date && (
                <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>
                  {errors.due_date}
                </p>
              )}
            </div>
          </div>

          {/* 工时和优先级 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="label">预估工时（小时）</label>
              <input
                type="number"
                value={formData.estimated_hours}
                onChange={(e) => setFormData({ ...formData, estimated_hours: e.target.value })}
                className="input"
                style={errors.estimated_hours ? { borderColor: "var(--color-danger)" } : undefined}
                placeholder="0"
                min="0"
                step="0.5"
              />
              {errors.estimated_hours && (
                <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>
                  {errors.estimated_hours}
                </p>
              )}
            </div>
            <div>
              <label className="label">业务优先级</label>
              <select
                value={formData.priority_level}
                onChange={(e) => setFormData({ ...formData, priority_level: e.target.value })}
                className="select"
              >
                <option value="">请选择</option>
                <option value="critical">关键</option>
                <option value="high">高</option>
                <option value="medium">中</option>
                <option value="low">低</option>
              </select>
            </div>
          </div>

          {/* 阻塞原因（仅blocked状态显示） */}
          <div>
            <label className="label">阻塞原因</label>
            <input
              type="text"
              value={formData.blocked_reason}
              onChange={(e) => setFormData({ ...formData, blocked_reason: e.target.value })}
              className="input"
              placeholder="如果任务被阻塞，请说明原因"
            />
          </div>

          {/* 标签和备注 */}
          <div>
            <label className="label">标签</label>
            <input
              type="text"
              value={formData.tags}
              onChange={(e) => setFormData({ ...formData, tags: e.target.value })}
              className="input"
              placeholder="用逗号分隔多个标签"
            />
          </div>

          <div>
            <label className="label">备注</label>
            <textarea
              value={formData.notes}
              onChange={(e) => setFormData({ ...formData, notes: e.target.value })}
              className="input resize-none"
              rows={2}
              placeholder="备注信息"
            />
          </div>
        </form>

        {/* Footer */}
        <div className="modal-footer">
          <button type="button" onClick={onCancel} className="btn-secondary">
            取消
          </button>
          <button
            onClick={handleSubmit}
            disabled={!formData.title.trim() || loading || Object.keys(errors).length > 0}
            className="btn-primary"
          >
            {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
