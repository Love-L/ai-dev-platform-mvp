import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Plus, Search, Trash2, Eye } from "lucide-react";
import { useRequirementsStore, useProjectsStore } from "../stores";
import { StatusBadge, PriorityBadge, ReqTypeBadge } from "../components/Badges";
import RequirementForm from "../components/RequirementForm";
import ConfirmDialog from "../components/ConfirmDialog";
import { createRequirement } from "../lib/tauri";
import type { Requirement } from "../lib/types";
import { showToast } from "../components/Toast";

export default function Requirements() {
  const navigate = useNavigate();
  const { requirements, loading, fetchRequirements, removeRequirement } = useRequirementsStore();
  const { projects, fetchProjects } = useProjectsStore();
  const [searchTerm, setSearchTerm] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("");
  const [typeFilter, setTypeFilter] = useState<string>("");
  const [projectFilter, setProjectFilter] = useState<string>("");
  const [confirmDelete, setConfirmDelete] = useState<Requirement | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);

  useEffect(() => {
    fetchRequirements();
    fetchProjects();
  }, []);

  async function handleConfirmDelete() {
    if (!confirmDelete) return;
    try {
      await removeRequirement(confirmDelete.id);
      showToast("success", "已删除需求");
    } catch (error) {
      console.error("删除失败:", error);
      // 后端对关联任务有校验（人话提示），直接透出
      showToast("error", `删除失败: ${error}`);
    } finally {
      setConfirmDelete(null);
    }
  }

  async function handleCreate(data: Partial<Requirement>) {
    try {
      const req = await createRequirement({
        title: data.title ?? "",
        description: data.description,
        priority: data.priority,
        projectId: data.project_id,
        module: data.module,
        source: data.source,
        sourceDetail: data.source_detail,
        productOwner: data.product_owner,
        developer: data.developer,
        tester: data.tester,
        businessPriority: data.business_priority,
        estimatedEffort: data.estimated_effort,
        tags: data.tags,
        notes: data.notes,
      });
      setShowCreateModal(false);
      await fetchRequirements();
      navigate(`/requirements/${req.id}`);
    } catch (error) {
      console.error("创建失败:", error);
      showToast("error", `创建需求失败: ${error}`);
    }
  }

  const filtered = requirements.filter((req) => {
    const matchSearch =
      !searchTerm ||
      req.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      req.description?.toLowerCase().includes(searchTerm.toLowerCase());
    const matchStatus = !statusFilter || req.status === statusFilter;
    const matchType = !typeFilter || req.requirement_type === typeFilter;
    const matchProject =
      !projectFilter || (projectFilter === "__none__" ? !req.project_id : req.project_id === projectFilter);
    return matchSearch && matchStatus && matchType && matchProject;
  });

  if (loading) {
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
            需求管理
          </h1>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
            共 {requirements.length} 个需求
          </p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建需求
        </button>
      </motion.div>

      {/* 筛选栏 */}
      <div className="flex flex-col sm:flex-row gap-3">
        <div className="flex-1 relative">
          <Search
            className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2"
            style={{ color: "var(--color-text-tertiary)" }}
          />
          <input
            type="text"
            placeholder="搜索需求..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="input pl-9"
          />
        </div>
        <select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
          className="select w-full sm:w-32"
        >
          <option value="">全部状态</option>
          <option value="draft">草稿</option>
          <option value="reviewing">评审中</option>
          <option value="approved">已批准</option>
          <option value="rejected">已拒绝</option>
          <option value="frozen">已冻结</option>
          <option value="in_development">开发中</option>
          <option value="testing">测试中</option>
          <option value="completed">已完成</option>
        </select>
        <select value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)} className="select w-full sm:w-32">
          <option value="">全部类型</option>
          <option value="feature">功能需求</option>
          <option value="bug">Bug</option>
          <option value="optimization">技术优化</option>
          <option value="tech_debt">技术债</option>
        </select>
        <select
          value={projectFilter}
          onChange={(e) => setProjectFilter(e.target.value)}
          className="select w-full sm:w-40"
        >
          <option value="">全部项目</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
          <option value="__none__">未归属项目</option>
        </select>
      </div>

      {/* 需求列表 */}
      <div className="card divide-y" style={{ borderColor: "var(--color-border-subtle)" }}>
        {filtered.length === 0 ? (
          <div className="empty-state">
            <FileText className="empty-state-icon" />
            <p className="empty-state-title">暂无需求</p>
            <p className="empty-state-desc">点击"新建需求"开始创建</p>
          </div>
        ) : (
          filtered.map((req) => (
            <div
              key={req.id}
              className="flex items-center justify-between px-4 py-3 transition-colors group cursor-pointer"
              style={{ transition: "background var(--transition-fast)" }}
              onClick={() => navigate(`/requirements/${req.id}`)}
              onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-primary-subtle)")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "")}
            >
              <div className="flex items-center gap-4 min-w-0 flex-1">
                <div className="min-w-0 flex-1">
                  <Link
                    to={`/requirements/${req.id}`}
                    onClick={(e) => e.stopPropagation()}
                    className="text-sm font-medium transition-colors hover:underline"
                    style={{ color: "var(--color-text)" }}
                  >
                    {req.title}
                  </Link>
                  {req.description && (
                    <p className="text-xs mt-0.5 line-clamp-1" style={{ color: "var(--color-text-tertiary)" }}>
                      {req.description}
                    </p>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-2 ml-4">
                <ReqTypeBadge type={req.requirement_type || "feature"} />
                <StatusBadge status={req.status} />
                <PriorityBadge priority={req.priority} />
                {/* 详情入口（原仅标题可点，不易发现）：显式按钮 + 整行可点 */}
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    navigate(`/requirements/${req.id}`);
                  }}
                  className="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                  style={{ color: "var(--color-text-tertiary)" }}
                  aria-label={`查看需求详情 ${req.title}`}
                  title="查看详情"
                >
                  <Eye className="w-3.5 h-3.5" />
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setConfirmDelete(req);
                  }}
                  className="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                  style={{ color: "var(--color-text-tertiary)" }}
                  aria-label={`删除需求 ${req.title}`}
                  title="删除"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      {/* 删除确认 */}
      {confirmDelete && (
        <ConfirmDialog
          danger
          title="删除需求"
          message={`确定要删除「${confirmDelete.title}」吗？\n该需求下存在关联任务时将无法删除，请先处理任务。PRD 内容会一并删除且不可恢复。`}
          confirmLabel="删除"
          onConfirm={handleConfirmDelete}
          onCancel={() => setConfirmDelete(null)}
        />
      )}

      {/* 创建弹窗 */}
      {showCreateModal && <RequirementForm onSubmit={handleCreate} onCancel={() => setShowCreateModal(false)} />}
    </div>
  );
}

function FileText({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="16" x2="8" y1="13" y2="13" />
      <line x1="16" x2="8" y1="17" y2="17" />
      <polyline points="10 9 9 9 8 9" />
    </svg>
  );
}
