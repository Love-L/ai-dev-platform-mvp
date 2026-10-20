import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { motion } from "framer-motion";
import { Plus, Search, Folder, X, Trash2 } from "lucide-react";
import { useProjectsStore } from "../stores";
import { showToast } from "../components/Toast";

export default function Projects() {
  const { projects, loading, fetchProjects, addProject, removeProject } = useProjectsStore();
  const [searchTerm, setSearchTerm] = useState("");
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [newKey, setNewKey] = useState("");

  useEffect(() => {
    fetchProjects();
  }, []);

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

  async function handleDelete(id: string) {
    if (!confirm("确定要删除这个项目吗？")) return;
    try {
      await removeProject(id);
    } catch (error) {
      console.error("删除项目失败:", error);
      showToast("error", `删除项目失败: ${error}`);
    }
  }

  const filtered = projects.filter(
    (p) =>
      !searchTerm ||
      p.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      p.description?.toLowerCase().includes(searchTerm.toLowerCase()),
  );

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
            共 {projects.length} 个项目
          </p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建项目
        </button>
      </motion.div>

      <div className="flex gap-3">
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
      </div>

      <div className="space-y-3">
        {filtered.length === 0 ? (
          <div className="card p-10 text-center">
            <Folder className="w-10 h-10 mx-auto mb-3" style={{ color: "var(--color-text-tertiary)" }} />
            <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
              暂无项目
            </p>
            <p className="text-xs mt-1" style={{ color: "var(--color-text-tertiary)" }}>
              点击"新建项目"开始创建
            </p>
          </div>
        ) : (
          filtered.map((project) => (
            <Link
              key={project.id}
              to={`/projects/${project.id}`}
              className="card p-4 block group"
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
              <div className="flex items-start justify-between">
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
                            : project.status === "archived"
                              ? "已归档"
                              : project.status || "进行中"}
                      </span>
                    </div>
                    {project.description && (
                      <p className="text-xs mt-1 line-clamp-1" style={{ color: "var(--color-text-tertiary)" }}>
                        {project.description}
                      </p>
                    )}
                    <p className="text-[11px] mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
                      创建于 {new Date(project.created_at).toLocaleDateString("zh-CN")}
                    </p>
                  </div>
                </div>
                <button
                  onClick={(e) => {
                    e.preventDefault();
                    handleDelete(project.id);
                  }}
                  className="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                  style={{ color: "var(--color-text-tertiary)" }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.color = "var(--color-danger)";
                    e.currentTarget.style.background = "var(--color-danger-light)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.color = "";
                    e.currentTarget.style.background = "";
                  }}
                  aria-label={`删除项目 ${project.name}`}
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            </Link>
          ))
        )}
      </div>

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
