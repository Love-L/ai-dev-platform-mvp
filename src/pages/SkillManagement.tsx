import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, Trash2, Edit3, Copy, Zap, FileText, ChevronDown, ChevronUp, X, Save, Activity } from "lucide-react";
import type { PrdSkill } from "../lib/types";
import { listPrdSkills, createPrdSkill, updatePrdSkill, deletePrdSkill, createCustomSkillFromBuiltin } from "../lib/tauri";
import { showToast } from "../components/Toast";

export default function SkillManagement() {
  const navigate = useNavigate();
  const [skills, setSkills] = useState<PrdSkill[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedSkill, setExpandedSkill] = useState<string | null>(null);

  // 弹窗状态
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [selectedSkill, setSelectedSkill] = useState<PrdSkill | null>(null);

  // 表单状态
  const [formData, setFormData] = useState({
    name: "",
    skill_type: "feature",
    description: "",
    template: "",
  });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadSkills();
  }, []);

  async function loadSkills() {
    try {
      const data = await listPrdSkills();
      setSkills(data);
    } catch (error) {
      console.error("加载Skill列表失败:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleCreate() {
    if (!formData.name.trim() || !formData.template.trim()) return;
    setSaving(true);
    try {
      await createPrdSkill(
        formData.name,
        formData.skill_type,
        formData.description || null,
        formData.template
      );
      setShowCreateModal(false);
      setFormData({ name: "", skill_type: "feature", description: "", template: "" });
      showToast("success", "Skill创建成功");
      await loadSkills();
    } catch (error) {
      console.error("创建Skill失败:", error);
      showToast("error", "创建Skill失败");
    } finally {
      setSaving(false);
    }
  }

  async function handleUpdate() {
    if (!selectedSkill) return;
    setSaving(true);
    try {
      await updatePrdSkill(
        selectedSkill.id,
        formData.name || null,
        formData.description || null,
        formData.template || null
      );
      setShowEditModal(false);
      setSelectedSkill(null);
      setFormData({ name: "", skill_type: "feature", description: "", template: "" });
      showToast("success", "Skill更新成功");
      await loadSkills();
    } catch (error) {
      console.error("更新Skill失败:", error);
      showToast("error", "更新Skill失败");
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!selectedSkill) return;
    setSaving(true);
    try {
      await deletePrdSkill(selectedSkill.id);
      setShowDeleteModal(false);
      setSelectedSkill(null);
      showToast("success", "Skill删除成功");
      await loadSkills();
    } catch (error) {
      console.error("删除Skill失败:", error);
      showToast("error", "删除Skill失败");
    } finally {
      setSaving(false);
    }
  }

  async function handleCreateCustomVersion(skill: PrdSkill) {
    const customName = `自定义 ${skill.name}`;
    try {
      await createCustomSkillFromBuiltin(skill.id, customName, null);
      showToast("success", "自定义版本创建成功");
      await loadSkills();
    } catch (error) {
      console.error("创建自定义版本失败:", error);
      showToast("error", "创建自定义版本失败");
    }
  }

  function openCreateModal() {
    setFormData({ name: "", skill_type: "feature", description: "", template: "" });
    setShowCreateModal(true);
  }

  function openEditModal(skill: PrdSkill) {
    setSelectedSkill(skill);
    setFormData({
      name: skill.name,
      skill_type: skill.skill_type,
      description: skill.description || "",
      template: skill.template,
    });
    setShowEditModal(true);
  }

  function openDeleteModal(skill: PrdSkill) {
    setSelectedSkill(skill);
    setShowDeleteModal(true);
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">加载中...</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5">
      {/* 页面标题 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-gray-900">Skill 管理</h1>
          <p className="text-sm text-gray-500 mt-0.5">管理 PRD 生成模板和技能配置</p>
        </div>
        <button onClick={openCreateModal} className="btn-primary">
          <Plus className="w-4 h-4" />
          新建 Skill
        </button>
      </div>

      {/* Skill 列表 */}
      {skills.length === 0 ? (
        <div className="card p-12 text-center">
          <Zap className="w-10 h-10 text-gray-300 mx-auto mb-3" />
          <p className="text-sm text-gray-500">暂无 Skill 模板</p>
        </div>
      ) : (
        <div className="space-y-3">
          {skills.map((skill) => (
            <div key={skill.id} className="card overflow-hidden">
              {/* Skill 头部 */}
              <div
                className="flex items-center justify-between p-4 cursor-pointer hover:bg-gray-50 transition-colors"
                onClick={() => setExpandedSkill(expandedSkill === skill.id ? null : skill.id)}
              >
                <div className="flex items-center gap-3">
                  <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${
                    skill.is_builtin ? "bg-blue-50" : "bg-purple-50"
                  }`}>
                    {skill.is_builtin ? (
                      <Zap className="w-5 h-5 text-blue-600" />
                    ) : (
                      <FileText className="w-5 h-5 text-purple-600" />
                    )}
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <h3 className="text-sm font-medium text-gray-900">{skill.name}</h3>
                      <span className="px-2 py-0.5 bg-gray-100 text-gray-600 text-xs font-medium rounded">
                        {skill.skill_type}
                      </span>
                      {skill.is_builtin && (
                        <span className="px-2 py-0.5 bg-blue-50 text-blue-700 text-xs font-medium rounded">
                          内置
                        </span>
                      )}
                      {skill.version && skill.version > 1 && (
                        <span className="px-2 py-0.5 bg-green-50 text-green-700 text-xs font-medium rounded">
                          v{skill.version}
                        </span>
                      )}
                      {skill.parent_id && (
                        <span className="px-2 py-0.5 bg-purple-50 text-purple-700 text-xs font-medium rounded">
                          自定义版本
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-gray-400 mt-0.5">
                      {skill.description || "无描述"}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {skill.is_builtin && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleCreateCustomVersion(skill);
                      }}
                      className="p-1.5 text-gray-400 hover:text-purple-600 hover:bg-purple-50 rounded-lg transition-colors"
                      title="基于此创建自定义版本"
                    >
                      <Copy className="w-4 h-4" />
                    </button>
                  )}
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      navigate(`/skills/${skill.id}/evolution`);
                    }}
                    className="p-1.5 text-gray-400 hover:text-emerald-600 hover:bg-emerald-50 rounded-lg transition-colors"
                    title="查看 Skill 进化（评分 · 建议 · 优化 · 版本）"
                  >
                    <Activity className="w-4 h-4" />
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      openEditModal(skill);
                    }}
                    className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
                    title="编辑"
                  >
                    <Edit3 className="w-4 h-4" />
                  </button>
                  {!skill.is_builtin && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        openDeleteModal(skill);
                      }}
                      className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                      title="删除"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  )}
                  {expandedSkill === skill.id ? (
                    <ChevronUp className="w-4 h-4 text-gray-400" />
                  ) : (
                    <ChevronDown className="w-4 h-4 text-gray-400" />
                  )}
                </div>
              </div>

              {/* Skill 模板内容 */}
              {expandedSkill === skill.id && (
                <div className="border-t border-gray-100 p-4 bg-gray-50">
                  <div className="flex items-center justify-between mb-2">
                    <h4 className="text-xs font-medium text-gray-500">模板内容</h4>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        navigate(`/skills/${skill.id}/evolution`);
                      }}
                      className="text-xs text-emerald-600 hover:text-emerald-700 hover:underline"
                    >
                      查看进化历程 →
                    </button>
                  </div>
                  <pre className="text-xs text-gray-600 whitespace-pre-wrap bg-white rounded-lg p-3 border border-gray-200 max-h-64 overflow-auto font-mono">
                    {skill.template}
                  </pre>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* 创建弹窗 */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl w-full max-w-2xl max-h-[80vh] overflow-hidden">
            <div className="flex items-center justify-between px-6 py-4 border-b">
              <h2 className="text-lg font-semibold">新建 Skill</h2>
              <button onClick={() => setShowCreateModal(false)} className="p-1 hover:bg-gray-100 rounded">
                <X className="w-5 h-5" />
              </button>
            </div>
            <div className="p-6 space-y-4 overflow-y-auto max-h-[60vh]">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">名称 *</label>
                <input
                  type="text"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  placeholder="输入Skill名称"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">类型</label>
                <select
                  value={formData.skill_type}
                  onChange={(e) => setFormData({ ...formData, skill_type: e.target.value })}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="feature">功能需求</option>
                  <option value="bugfix">Bug修复</option>
                  <option value="optimization">技术优化</option>
                  <option value="refactor">架构重构</option>
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">描述</label>
                <input
                  type="text"
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  placeholder="输入描述（可选）"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">模板内容 *</label>
                <textarea
                  value={formData.template}
                  onChange={(e) => setFormData({ ...formData, template: e.target.value })}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 font-mono text-sm"
                  rows={12}
                  placeholder="输入模板内容"
                />
              </div>
            </div>
            <div className="flex justify-end gap-3 px-6 py-4 border-t bg-gray-50">
              <button
                onClick={() => setShowCreateModal(false)}
                className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg"
              >
                取消
              </button>
              <button
                onClick={handleCreate}
                disabled={saving || !formData.name.trim() || !formData.template.trim()}
                className="btn-primary disabled:opacity-50"
              >
                <Save className="w-4 h-4" />
                {saving ? "保存中..." : "创建"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 编辑弹窗 */}
      {showEditModal && selectedSkill && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl w-full max-w-2xl max-h-[80vh] overflow-hidden">
            <div className="flex items-center justify-between px-6 py-4 border-b">
              <h2 className="text-lg font-semibold">编辑 Skill</h2>
              <button onClick={() => setShowEditModal(false)} className="p-1 hover:bg-gray-100 rounded">
                <X className="w-5 h-5" />
              </button>
            </div>
            <div className="p-6 space-y-4 overflow-y-auto max-h-[60vh]">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">名称</label>
                <input
                  type="text"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">描述</label>
                <input
                  type="text"
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">模板内容</label>
                <textarea
                  value={formData.template}
                  onChange={(e) => setFormData({ ...formData, template: e.target.value })}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 font-mono text-sm"
                  rows={12}
                />
              </div>
            </div>
            <div className="flex justify-end gap-3 px-6 py-4 border-t bg-gray-50">
              <button
                onClick={() => setShowEditModal(false)}
                className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg"
              >
                取消
              </button>
              <button
                onClick={handleUpdate}
                disabled={saving || !formData.name.trim() || !formData.template.trim()}
                className="btn-primary disabled:opacity-50"
              >
                <Save className="w-4 h-4" />
                {saving ? "保存中..." : "保存"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 删除确认弹窗 */}
      {showDeleteModal && selectedSkill && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl w-full max-w-md overflow-hidden">
            <div className="px-6 py-4">
              <h2 className="text-lg font-semibold">删除 Skill</h2>
              <p className="text-sm text-gray-500 mt-2">
                确定要删除 Skill "{selectedSkill.name}" 吗？此操作不可撤销。
              </p>
            </div>
            <div className="flex justify-end gap-3 px-6 py-4 border-t bg-gray-50">
              <button
                onClick={() => setShowDeleteModal(false)}
                className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg"
              >
                取消
              </button>
              <button
                onClick={handleDelete}
                disabled={saving}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
              >
                {saving ? "删除中..." : "删除"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
