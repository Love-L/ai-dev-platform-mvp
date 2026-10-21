import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { motion } from "framer-motion";
import { Archive, ArchiveRestore, Plus, Search, Folder, X, Trash2 } from "lucide-react";
import { useProjectsStore } from "../stores";
import { getProjectsOverview, updateProject, type ProjectOverviewRow } from "../lib/tauri";
import type { Project } from "../lib/types";
import ConfirmDialog from "../components/ConfirmDialog";
import { showToast } from "../components/Toast";

export default function Projects() {
  const { projects, loading, fetchProjects, addProject, removeProject } = useProjectsStore();
  const [searchTerm, setSearchTerm] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [overviewMap, setOverviewMap] = useState<Record<string, ProjectOverviewRow>>({});
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<Project | null>(null);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [newKey, setNewKey] = useState("");

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    await fetchProjects();
    try {
      const rows = await getProjectsOverview();
      setOverviewMap(Object.fromEntries(rows.map((r) => [r.project_id, r])));
    } catch (error) {
      console.error("加载项目总览失败:", error);
    }
  }

  async function handleCreate() {
    if (!newName.trim()) return;
    try {
      await addProject(newName, newDescription || undefined, newKey || undefined);
      setShowCreateModal(false);
      setNewName("");
      setNewDescription("");
      setNewKey("");
    } catch (error) {
      console.error("创建项目失败:", error);
      showToast("error", `创建项目失败: ${error}`);
    }
  }

  async function handleToggleArchive(project: Project) {
    const next = project.status === "archived" ? "active" : "archived";
    try {
      await updateProject(project.id, { status: next });
      showToast("success", next === "archived" ? `已归档「${project.name}」` : `已恢复「${project.name}」`);
      await loadData();
    } catch (error) {
      console.error("归档项目失败:", error);
      showToast("error", `归档失败: ${error}`);
    }
  }

  async function handleConfirmDelete() {
    if (!confirmDelete) return;
    try {
      await removeProject(confirmDelete.id);
      showToast("success", `已删除「${confirmDelete.name}」`);
    } catch (error) {
      // 后端对关联数据有校验（人话提示），直接透出
      console.error("删除项目失败:", error);
      showToast("error", `删除失败: ${error}`);
    } finally {
      setConfirmDelete(null);
    }
  }

  const visible = projects.filter((p) => showArchived || p.status !== "archived");
  const filtered = visible.filter(
    (p) =>
      !searchTerm ||
      p.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      p.description?.toLowerCase().includes(searchTerm.toLowerCase()),
  );
  const archivedCount = projects.filter((p) => p.status === "archived").length;

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
      <motion.div
        initial={{ opacity: 0, y: -6 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2 }}
        className="flex items-center justify-between"
      >
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            项目管理
          </h1>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
            共 {visible.length} 个项目{archivedCount > 0 && !showArchived ? `（另有 ${archivedCount} 个已归档）` : ""}
          </p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建项目
        </button>
      </motion.div>

      <div className="flex gap-3 items-center">
        <div className="flex-1 relative">
          <Search
            className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2"
            style={{ color: "var(--color-text-tertiary)" }}
          />
          <input
            type="text"
            placeholder="搜索项目..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="input pl-9"
          />
        </div>
        {archivedCount > 0 && (
          <label
            className="flex items-center gap-2 text-xs cursor-pointer select-none"
            style={{ color: "var(--color-text-secondary)" }}
          >
            <input
              type="checkbox"
              checked={showArchived}
              onChange={(e) => setShowArchived(e.target.checked)}
              style={{ accentColor: "var(--color-primary)" }}
            />
            显示已归档（{archivedCount}）
          </label>
        )}
      </div>

      <div className="space-y-3">
        {filtered.length === 0 ? (
          <div className="card p-10 text-center">
            <Folder className="w-10 h-10 mx-auto mb-3" style={{ color: "var(--color-text-tertiary)" }} />
            <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
              {searchTerm || projects.length > 0 ? "没有匹配的项目" : "暂无项目"}
            </p>
            <p className="text-xs mt-1" style={{ color: "var(--color-text-tertiary)" }}>
              {searchTerm ? "换个关键词试试" : "点击「新建项目」开始创建"}
            </p>
          </div>
        ) : (
          filtered.map((project) => {
            const ov = overviewMap[project.id];
            const reqPct = ov && ov.req_total > 0 ? Math.round((ov.req_completed / ov.req_total) * 100) : 0;
            const taskPct = ov && ov.task_total > 0 ? Math.round((ov.task_completed / ov.task_total) * 100) : 0;
            const isArchived = project.status === "archived";
            return (
              <Link
                key={project.id}
                to={`/projects/${project.id}`}
                className="card p-4 block group"
                style={{
                  opacity: isArchived ? 0.65 : 1,
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
                <div className="flex items-start justify-between gap-4">
                  <div className="flex items-start gap-3 min-w-0 flex-1">
                    <div
                      className="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0"
                      style={{ background: "var(--color-primary-subtle)" }}
                    >
                      <Folder className="w-5 h-5" style={{ color: "var(--color-primary)" }} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <h3
                          className="text-sm font-semibold group-hover:text-primary transition-colors"
                          style={{ color: "var(--color-text)" }}
                        >
                          {project.name}
                        </h3>
                        {project.key && (
                          <span
                            className="text-[10px] font-mono px-1.5 py-0.5 rounded"
                            style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-tertiary)" }}
                          >
                            {project.key}
                          </span>
                        )}
                        <span
                          className="text-[10px] px-1.5 py-0.5 rounded-full"
                          style={{
                            background:
                              project.status === "active"
                                ? "var(--color-success-light)"
                                : project.status === "paused"
                                  ? "var(--color-warning-light)"
                                  : "var(--color-bg-subtle)",
                            color:
                              project.status === "active"
                                ? "var(--color-success)"
                                : project.status === "paused"
                                  ? "var(--color-warning)"
                                  : "var(--color-text-tertiary)",
                          }}
                        >
                          {project.status === "active"
                            ? "进行中"
                            : project.status === "paused"
                              ? "已暂停"
                              : isArchived
                                ? "已归档"
                                : project.status || "进行中"}
                        </span>
                      </div>
                      {project.description && (
                        <p className="text-xs mt-1 line-clamp-1" style={{ color: "var(--color-text-tertiary)" }}>
                          {project.description}
                        </p>
                      )}
                      {/* 进度概览：需求 / 任务 双进度条 */}
                      {ov ? (
                        <div className="mt-2 space-y-1.5 max-w-md">
                          <div className="flex items-center gap-2 text-[11px]">
                            <span className="w-8 shrink-0" style={{ color: "var(--color-text-tertiary)" }}>
                              需求
                            </span>
                            <div
                              className="flex-1 h-1.5 rounded-full overflow-hidden"
                              style={{ background: "var(--color-bg-subtle)" }}
                            >
                              <div
                                style={{
                                  width: `${reqPct}%`,
                                  height: "100%",
                                  background: "var(--color-primary)",
                                  borderRadius: "9999px",
                                  transition: "width 0.5s ease",
                                }}
                              />
                            </div>
                            <span className="w-14 text-right shrink-0" style={{ color: "var(--color-text-secondary)" }}>
                              {ov.req_completed}/{ov.req_total}
                            </span>
                          </div>
                          <div className="flex items-center gap-2 text-[11px]">
                            <span className="w-8 shrink-0" style={{ color: "var(--color-text-tertiary)" }}>
                              任务
                            </span>
                            <div
                              className="flex-1 h-1.5 rounded-full overflow-hidden"
                              style={{ background: "var(--color-bg-subtle)" }}
                            >
                              <div
                                style={{
                                  width: `${taskPct}%`,
                                  height: "100%",
                                  background: "var(--color-success)",
                                  borderRadius: "9999px",
                                  transition: "width 0.5s ease",
                                }}
                              />
                            </div>
                            <span className="w-14 text-right shrink-0" style={{ color: "var(--color-text-secondary)" }}>
                              {ov.task_completed}/{ov.task_total}
                            </span>
                          </div>
                        </div>
                      ) : (
                        <p className="text-[11px] mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
                          创建于 {new Date(project.created_at).toLocaleDateString("zh-CN")}
                        </p>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-all flex-shrink-0">
                    <button
                      onClick={(e) => {
                        e.preventDefault();
                        handleToggleArchive(project);
                      }}
                      className="p-1.5 rounded-lg"
                      style={{ color: "var(--color-text-tertiary)" }}
                      title={isArchived ? "取消归档" : "归档项目（保留全部数据，从默认列表隐藏）"}
                      aria-label={isArchived ? `取消归档 ${project.name}` : `归档项目 ${project.name}`}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.color = "var(--color-primary)";
                        e.currentTarget.style.background = "var(--color-primary-subtle)";
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.color = "";
                        e.currentTarget.style.background = "";
                      }}
                    >
                      {isArchived ? <ArchiveRestore className="w-3.5 h-3.5" /> : <Archive className="w-3.5 h-3.5" />}
                    </button>
                    <button
                      onClick={(e) => {
                        e.preventDefault();
                        setConfirmDelete(project);
                      }}
                      className="p-1.5 rounded-lg"
                      style={{ color: "var(--color-text-tertiary)" }}
                      aria-label={`删除项目 ${project.name}`}
                      title="删除项目"
                      onMouseEnter={(e) => {
                        e.currentTarget.style.color = "var(--color-danger)";
                        e.currentTarget.style.background = "var(--color-danger-light)";
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.color = "";
                        e.currentTarget.style.background = "";
                      }}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </Link>
            );
          })
        )}
      </div>

      {confirmDelete && (
        <ConfirmDialog
          danger
          title="删除项目"
          message={`确定要删除「${confirmDelete.name}」吗？\n项目下存在需求、任务、Skill 或 MCP 配置时无法删除；此类项目建议先「归档」（保留数据、从列表隐藏）。`}
          confirmLabel="删除"
          onConfirm={handleConfirmDelete}
          onCancel={() => setConfirmDelete(null)}
        />
      )}

      {showCreateModal && (
        <div className="modal-overlay" onClick={() => setShowCreateModal(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <div className="flex items-center justify-between">
                <h3 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                  新建项目
                </h3>
                <button
                  onClick={() => setShowCreateModal(false)}
                  className="p-1 rounded-lg transition-colors"
                  style={{ color: "var(--color-text-tertiary)", transition: "all var(--transition-fast)" }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.color = "var(--color-text-secondary)";
                    e.currentTarget.style.background = "var(--color-bg-subtle)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.color = "";
                    e.currentTarget.style.background = "";
                  }}
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>
            <div className="modal-body space-y-4">
              <div>
                <label className="label">项目名称 *</label>
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="输入项目名称"
                  className="input"
                  autoFocus
                />
              </div>
              <div>
                <label className="label">项目描述</label>
                <textarea
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  placeholder="描述项目目标和范围"
                  rows={3}
                  className="input resize-none"
                />
              </div>
              <div>
                <label className="label">项目标识（前缀）</label>
                <input
                  type="text"
                  value={newKey}
                  onChange={(e) => setNewKey(e.target.value.toUpperCase())}
                  placeholder="如 AI、WEB（用于生成需求编号）"
                  className="input"
                  maxLength={10}
                />
              </div>
            </div>
            <div className="modal-footer">
              <button onClick={() => setShowCreateModal(false)} className="btn-secondary">
                取消
              </button>
              <button onClick={handleCreate} disabled={!newName.trim()} className="btn-primary">
                创建
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
