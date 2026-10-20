import { useState, useEffect } from "react";
import { X, Save, Loader2 } from "lucide-react";
import type { Requirement } from "../lib/types";
import { listProjects } from "../lib/tauri";

interface RequirementFormProps {
  requirement?: Requirement;
  onSubmit: (data: Partial<Requirement>) => Promise<void>;
  onCancel: () => void;
}

export default function RequirementForm({ requirement, onSubmit, onCancel }: RequirementFormProps) {
  const [loading, setLoading] = useState(false);
  const [projects, setProjects] = useState<Array<{ id: string; name: string }>>([]);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formData, setFormData] = useState({
    title: requirement?.title || "",
    description: requirement?.description || "",
    priority: requirement?.priority || "P1",
    project_id: (requirement as any)?.project_id || "",
    requirement_type: requirement?.requirement_type || "feature",
    module: requirement?.module || "",
    source: requirement?.source || "",
    source_detail: requirement?.source_detail || "",
    product_owner: requirement?.product_owner || "",
    developer: requirement?.developer || "",
    tester: requirement?.tester || "",
    business_priority: requirement?.business_priority || "",
    estimated_effort: requirement?.estimated_effort || "",
    tags: requirement?.tags || "",
    notes: requirement?.notes || "",
  });

  useEffect(() => {
    listProjects().then(setProjects).catch(() => {});
  }, []);

  function validate(): boolean {
    const errs: Record<string, string> = {};
    const title = formData.title.trim();
    if (!title) {
      errs.title = "标题不能为空";
    } else if (title.length > 200) {
      errs.title = `标题不能超过 200 字（当前 ${title.length}）`;
    }
    if (formData.estimated_effort) {
      const effort = Number(formData.estimated_effort);
      if (isNaN(effort) || effort < 0) {
        errs.estimated_effort = "预估工作量必须为非负数";
      } else if (effort > 1000) {
        errs.estimated_effort = "预估工作量不能超过 1000 人天";
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
        project_id: formData.project_id || undefined,
        estimated_effort: formData.estimated_effort ? Number(formData.estimated_effort) : undefined,
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content w-full max-w-2xl max-h-[90vh]">
        {/* Header */}
        <div className="modal-header">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold" style={{ color: "var(--color-text)" }}>
              {requirement ? "编辑需求" : "新建需求"}
            </h2>
            <button onClick={onCancel} className="p-1 rounded-lg transition-colors" style={{ color: "var(--color-text-tertiary)" }}>
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
              placeholder="输入需求标题"
              required
              maxLength={200}
            />
            {errors.title && (
              <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>{errors.title}</p>
            )}
          </div>

          {/* 所属项目 */}
          {projects.length > 0 && (
            <div>
              <label className="label">所属项目</label>
              <select
                value={formData.project_id}
                onChange={(e) => setFormData({ ...formData, project_id: e.target.value })}
                className="select"
              >
                <option value="">请选择项目（可选）</option>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
            </div>
          )}

          {/* 描述 */}
          <div>
            <label className="label">描述</label>
            <textarea
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              className="input resize-none"
              rows={3}
              placeholder="输入需求描述"
            />
          </div>

          {/* 优先级和类型 */}
          <div className="grid grid-cols-2 gap-4">
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
            <div>
              <label className="label">类型</label>
              <select
                value={formData.requirement_type}
                onChange={(e) => setFormData({ ...formData, requirement_type: e.target.value as "feature" | "bug" | "optimization" | "tech_debt" })}
                className="select"
              >
                <option value="feature">功能需求</option>
                <option value="bug">缺陷</option>
                <option value="optimization">优化</option>
                <option value="tech_debt">技术债务</option>
              </select>
            </div>
          </div>

          {/* 模块和来源 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="label">模块</label>
              <input
                type="text"
                value={formData.module}
                onChange={(e) => setFormData({ ...formData, module: e.target.value })}
                className="input"
                placeholder="所属模块"
              />
            </div>
            <div>
              <label className="label">来源</label>
              <select
                value={formData.source}
                onChange={(e) => setFormData({ ...formData, source: e.target.value })}
                className="select"
              >
                <option value="">请选择</option>
                <option value="product">产品</option>
                <option value="tech">技术</option>
                <option value="customer">客户</option>
                <option value="internal">内部</option>
              </select>
            </div>
          </div>

          {/* 负责人 */}
          <div className="grid grid-cols-3 gap-4">
            <div>
              <label className="label">产品负责人</label>
              <input
                type="text"
                value={formData.product_owner}
                onChange={(e) => setFormData({ ...formData, product_owner: e.target.value })}
                className="input"
                placeholder="产品负责人"
              />
            </div>
            <div>
              <label className="label">开发负责人</label>
              <input
                type="text"
                value={formData.developer}
                onChange={(e) => setFormData({ ...formData, developer: e.target.value })}
                className="input"
                placeholder="开发负责人"
              />
            </div>
            <div>
              <label className="label">测试负责人</label>
              <input
                type="text"
                value={formData.tester}
                onChange={(e) => setFormData({ ...formData, tester: e.target.value })}
                className="input"
                placeholder="测试负责人"
              />
            </div>
          </div>

          {/* 工作量 */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="label">预估工作量（人天）</label>
              <input
                type="number"
                value={formData.estimated_effort}
                onChange={(e) => setFormData({ ...formData, estimated_effort: e.target.value })}
                className="input"
                style={errors.estimated_effort ? { borderColor: "var(--color-danger)" } : undefined}
                placeholder="0"
                min="0"
                step="0.5"
              />
              {errors.estimated_effort && (
                <p className="text-xs mt-1" style={{ color: "var(--color-danger)" }}>{errors.estimated_effort}</p>
              )}
            </div>
            <div>
              <label className="label">业务优先级</label>
              <select
                value={formData.business_priority}
                onChange={(e) => setFormData({ ...formData, business_priority: e.target.value })}
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
            {loading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Save className="w-4 h-4" />
            )}
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
