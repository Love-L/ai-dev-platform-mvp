import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, Save, Loader2, Plus, Trash2, X, Settings, Zap, Plug, FileText, Columns } from "lucide-react";
import {
  getProject,
  updateProject,
  listProjectSkills,
  addProjectSkill,
  removeProjectSkill,
  listProjectMcps,
  addProjectMcp,
  removeProjectMcp,
  listPrdSkills,
  listRequirements,
  listTasksByProject,
  listStageSkillConfigs,
  upsertStageSkillConfig,
  removeStageSkillConfig,
  getPipelineStages,
  getStageSkillConfig,
} from "../lib/tauri";
import type {
  Project,
  ProjectSkill,
  ProjectMcp,
  PrdSkill,
  Requirement,
  Task,
  StageSkillConfig,
  PipelineStageOption,
} from "../lib/types";
import { PriorityBadge, StatusBadge, TaskTypeBadge } from "../components/Badges";

type Tab = "overview" | "requirements" | "tasks" | "skills" | "mcps" | "pipeline";

export default function ProjectDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [project, setProject] = useState<Project | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("overview");

  // 基本信息
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [key, setKey] = useState("");
  const [status, setStatus] = useState("active");

  // Skill 配置
  const [projectSkills, setProjectSkills] = useState<ProjectSkill[]>([]);
  const [allSkills, setAllSkills] = useState<PrdSkill[]>([]);
  const [showAddSkill, setShowAddSkill] = useState(false);

  // MCP 配置
  const [projectMcps, setProjectMcps] = useState<ProjectMcp[]>([]);
  const [showAddMcp, setShowAddMcp] = useState(false);
  const [newMcpName, setNewMcpName] = useState("");
  const [newMcpEndpoint, setNewMcpEndpoint] = useState("");
  const [newMcpDesc, setNewMcpDesc] = useState("");

  // 需求和任务
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);

  // 环节 Skill 配置
  const [stageConfigs, setStageConfigs] = useState<StageSkillConfig[]>([]);
  const [pipelineStages, setPipelineStages] = useState<PipelineStageOption[]>([]);

  useEffect(() => {
    if (id) loadAll(id);
  }, [id]);

  async function loadAll(projectId: string) {
    try {
      const [proj, skills, mcps, allSk, reqs, projectTasks, stages, pipelineStagesList] = await Promise.all([
        getProject(projectId),
        listProjectSkills(projectId),
        listProjectMcps(projectId),
        listPrdSkills(),
        listRequirements(),
        listTasksByProject(projectId),
        listStageSkillConfigs(projectId),
        getPipelineStages(),
      ]);
      if (proj) {
        setProject(proj);
        setName(proj.name);
        setDescription(proj.description || "");
        setKey(proj.key || "");
        setStatus(proj.status || "active");
      }
      setProjectSkills(skills);
      setProjectMcps(mcps);
      setAllSkills(allSk);
      setStageConfigs(stages);
      setPipelineStages(pipelineStagesList);
      // 过滤属于当前项目的需求
      setRequirements(reqs.filter((r) => r.project_id === projectId));
      setTasks(projectTasks);
    } catch (error) {
      console.error("加载项目详情失败:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSave() {
    if (!id) return;
    setSaving(true);
    try {
      await updateProject(id, { name, description, key, status });
    } catch (error) {
      console.error("保存失败:", error);
    } finally {
      setSaving(false);
    }
  }

  async function handleAddSkill(skillId: string) {
    if (!id) return;
    try {
      await addProjectSkill(id, skillId, true);
      setShowAddSkill(false);
      const skills = await listProjectSkills(id);
      setProjectSkills(skills);
    } catch (error) {
      console.error("添加Skill失败:", error);
    }
  }

  async function handleRemoveSkill(skillConfigId: string) {
    if (!id) return;
    try {
      await removeProjectSkill(skillConfigId);
      const skills = await listProjectSkills(id);
      setProjectSkills(skills);
    } catch (error) {
      console.error("移除Skill失败:", error);
    }
  }

  async function handleAddMcp() {
    if (!id || !newMcpName.trim() || !newMcpEndpoint.trim()) return;
    try {
      await addProjectMcp(id, newMcpName, newMcpEndpoint, newMcpDesc || undefined, true);
      setShowAddMcp(false);
      setNewMcpName("");
      setNewMcpEndpoint("");
      setNewMcpDesc("");
      const mcps = await listProjectMcps(id);
      setProjectMcps(mcps);
    } catch (error) {
      console.error("添加MCP失败:", error);
    }
  }

  async function handleRemoveMcp(mcpConfigId: string) {
    if (!id) return;
    try {
      await removeProjectMcp(mcpConfigId);
      const mcps = await listProjectMcps(id);
      setProjectMcps(mcps);
    } catch (error) {
      console.error("移除MCP失败:", error);
    }
  }

  async function handleStageSkillChange(stage: string, skillId: string) {
    if (!id || !skillId) return;
    try {
      await upsertStageSkillConfig(id, stage, skillId, true);

      // 只回读被修改的单个环节，避免整表刷新
      const updated = await getStageSkillConfig(id, stage);
      if (updated) {
        setStageConfigs((prev) => {
          const rest = prev.filter((c) => c.stage !== stage);
          return [...rest, updated].sort((a, b) => a.stage.localeCompare(b.stage));
        });
      } else {
        setStageConfigs(await listStageSkillConfigs(id));
      }
    } catch (error) {
      console.error("保存环节配置失败:", error);
    }
  }

  async function handleRemoveStageSkill(configId: string) {
    if (!id) return;
    try {
      await removeStageSkillConfig(configId);
      const configs = await listStageSkillConfigs(id);
      setStageConfigs(configs);
    } catch (error) {
      console.error("删除环节配置失败:", error);
    }
  }

  const tabs: { id: Tab; label: string; icon: typeof Settings }[] = [
    { id: "overview", label: "概览", icon: Settings },
    { id: "requirements", label: `需求 (${requirements.length})`, icon: FileText },
    { id: "tasks", label: `任务 (${tasks.length})`, icon: Columns },
    { id: "skills", label: "Skill 配置", icon: Zap },
    { id: "mcps", label: "MCP 配置", icon: Plug },
    { id: "pipeline", label: "流水线配置", icon: Settings },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">加载中...</div>
      </div>
    );
  }

  if (!project) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">项目不存在</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5">
      {/* 顶部导航 */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => navigate("/projects")}
          className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
        </button>
        <div className="flex-1">
          <h1 className="text-xl font-semibold text-gray-900">{project.name}</h1>
          <p className="text-sm text-gray-500 mt-0.5">
            {project.key && <span className="font-mono mr-2">[{project.key}]</span>}
            {project.description || "无描述"}
          </p>
        </div>
      </div>

      {/* Tab 栏 */}
      <div className="border-b border-gray-200">
        <div className="flex gap-0">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
                activeTab === tab.id
                  ? "border-blue-600 text-blue-700"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              <tab.icon className="w-4 h-4" />
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* 概览 Tab */}
      {activeTab === "overview" && (
        <div className="panel max-w-2xl">
          <h2 className="panel-title mb-4">基本信息</h2>
          <div className="space-y-4">
            <div>
              <label className="label">项目名称</label>
              <input type="text" value={name} onChange={(e) => setName(e.target.value)} className="input" />
            </div>
            <div>
              <label className="label">项目描述</label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="input resize-none"
                rows={3}
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="label">项目标识</label>
                <input
                  type="text"
                  value={key}
                  onChange={(e) => setKey(e.target.value.toUpperCase())}
                  className="input"
                  maxLength={10}
                />
              </div>
              <div>
                <label className="label">状态</label>
                <select value={status} onChange={(e) => setStatus(e.target.value)} className="select">
                  <option value="active">进行中</option>
                  <option value="paused">已暂停</option>
                  <option value="archived">已归档</option>
                </select>
              </div>
            </div>
            <div className="flex justify-end pt-2">
              <button onClick={handleSave} disabled={saving} className="btn-primary">
                {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
                保存修改
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 需求 Tab */}
      {activeTab === "requirements" && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-500">项目关联的需求列表</p>
            <button onClick={() => navigate("/requirements")} className="btn-secondary text-sm">
              <Plus className="w-4 h-4" /> 新建需求
            </button>
          </div>

          {requirements.length === 0 ? (
            <div className="card p-8 text-center">
              <FileText className="w-8 h-8 text-gray-300 mx-auto mb-2" />
              <p className="text-sm text-gray-500">暂无需求</p>
            </div>
          ) : (
            <div className="card divide-y divide-gray-100">
              {requirements.map((req) => (
                <div
                  key={req.id}
                  onClick={() => navigate(`/requirements/${req.id}`)}
                  className="flex items-center justify-between px-4 py-3 hover:bg-gray-50 cursor-pointer"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 truncate">{req.title}</p>
                    <p className="text-xs text-gray-400 mt-0.5">
                      {req.requirement_type || "未分类"} · {new Date(req.created_at).toLocaleDateString()}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 ml-4">
                    <PriorityBadge priority={req.priority} />
                    <StatusBadge status={req.status} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* 任务 Tab */}
      {activeTab === "tasks" && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-500">项目关联的任务列表</p>
            <button onClick={() => navigate("/board")} className="btn-secondary text-sm">
              <Plus className="w-4 h-4" /> 新建任务
            </button>
          </div>

          {tasks.length === 0 ? (
            <div className="card p-8 text-center">
              <Columns className="w-8 h-8 text-gray-300 mx-auto mb-2" />
              <p className="text-sm text-gray-500">暂无任务</p>
            </div>
          ) : (
            <div className="card divide-y divide-gray-100">
              {tasks.map((task) => (
                <div
                  key={task.id}
                  onClick={() => navigate(`/tasks/${task.id}`)}
                  className="flex items-center justify-between px-4 py-3 hover:bg-gray-50 cursor-pointer"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 truncate">{task.title}</p>
                    <p className="text-xs text-gray-400 mt-0.5">
                      {task.assignee || "未分配"} · {new Date(task.created_at).toLocaleDateString()}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 ml-4">
                    <TaskTypeBadge type={task.task_type} />
                    <PriorityBadge priority={task.priority} />
                    <StatusBadge status={task.status} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Skill 配置 Tab */}
      {activeTab === "skills" && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-500">配置项目默认使用的 Skill，创建任务时将自动继承</p>
            <button onClick={() => setShowAddSkill(true)} className="btn-primary text-sm">
              <Plus className="w-4 h-4" /> 添加 Skill
            </button>
          </div>

          {projectSkills.length === 0 ? (
            <div className="card p-8 text-center">
              <Zap className="w-8 h-8 text-gray-300 mx-auto mb-2" />
              <p className="text-sm text-gray-500">尚未配置 Skill</p>
            </div>
          ) : (
            <div className="card divide-y divide-gray-100">
              {projectSkills.map((ps) => {
                const skill = allSkills.find((s) => s.id === ps.skill_id);
                return (
                  <div key={ps.id} className="flex items-center justify-between px-4 py-3">
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded bg-purple-50 flex items-center justify-center">
                        <Zap className="w-4 h-4 text-purple-600" />
                      </div>
                      <div>
                        <p className="text-sm font-medium text-gray-900">{skill?.name || ps.skill_id}</p>
                        <p className="text-xs text-gray-400">{skill?.skill_type || "未知类型"}</p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {ps.is_default && (
                        <span className="text-[10px] bg-blue-50 text-blue-700 px-1.5 py-0.5 rounded">默认启用</span>
                      )}
                      <button
                        onClick={() => handleRemoveSkill(ps.id)}
                        className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* 添加 Skill 弹窗 */}
          {showAddSkill && (
            <div className="modal-overlay" onClick={() => setShowAddSkill(false)}>
              <div className="modal-content" onClick={(e) => e.stopPropagation()}>
                <div className="modal-header">
                  <div className="flex items-center justify-between">
                    <h3 className="text-base font-semibold text-gray-900">添加 Skill</h3>
                    <button
                      onClick={() => setShowAddSkill(false)}
                      className="p-1 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                </div>
                <div className="modal-body">
                  <div className="space-y-2 max-h-60 overflow-y-auto">
                    {allSkills
                      .filter((s) => !projectSkills.some((ps) => ps.skill_id === s.id))
                      .map((skill) => (
                        <button
                          key={skill.id}
                          onClick={() => handleAddSkill(skill.id)}
                          className="w-full flex items-center gap-3 p-3 rounded-lg border border-gray-200 hover:border-blue-300 hover:bg-blue-50 transition-all text-left"
                        >
                          <Zap className="w-4 h-4 text-purple-600 flex-shrink-0" />
                          <div>
                            <p className="text-sm font-medium text-gray-900">{skill.name}</p>
                            <p className="text-xs text-gray-400">
                              {skill.skill_type} · {skill.description?.slice(0, 50)}
                            </p>
                          </div>
                        </button>
                      ))}
                    {allSkills.filter((s) => !projectSkills.some((ps) => ps.skill_id === s.id)).length === 0 && (
                      <p className="text-sm text-gray-400 text-center py-4">所有 Skill 已添加</p>
                    )}
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* MCP 配置 Tab */}
      {activeTab === "mcps" && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-500">配置项目默认连接的 MCP 服务，会话启动时将自动连接</p>
            <button onClick={() => setShowAddMcp(true)} className="btn-primary text-sm">
              <Plus className="w-4 h-4" /> 添加 MCP
            </button>
          </div>

          {projectMcps.length === 0 ? (
            <div className="card p-8 text-center">
              <Plug className="w-8 h-8 text-gray-300 mx-auto mb-2" />
              <p className="text-sm text-gray-500">尚未配置 MCP</p>
            </div>
          ) : (
            <div className="card divide-y divide-gray-100">
              {projectMcps.map((pm) => (
                <div key={pm.id} className="flex items-center justify-between px-4 py-3">
                  <div className="flex items-center gap-3">
                    <div className="w-8 h-8 rounded bg-green-50 flex items-center justify-center">
                      <Plug className="w-4 h-4 text-green-600" />
                    </div>
                    <div>
                      <p className="text-sm font-medium text-gray-900">{pm.mcp_name}</p>
                      <p className="text-xs text-gray-400 font-mono">{pm.endpoint}</p>
                      {pm.description && <p className="text-xs text-gray-400">{pm.description}</p>}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {pm.is_default && (
                      <span className="text-[10px] bg-blue-50 text-blue-700 px-1.5 py-0.5 rounded">默认启用</span>
                    )}
                    <button
                      onClick={() => handleRemoveMcp(pm.id)}
                      className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* 添加 MCP 弹窗 */}
          {showAddMcp && (
            <div className="modal-overlay" onClick={() => setShowAddMcp(false)}>
              <div className="modal-content" onClick={(e) => e.stopPropagation()}>
                <div className="modal-header">
                  <div className="flex items-center justify-between">
                    <h3 className="text-base font-semibold text-gray-900">添加 MCP</h3>
                    <button
                      onClick={() => setShowAddMcp(false)}
                      className="p-1 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-gray-100"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                </div>
                <div className="modal-body space-y-4">
                  <div>
                    <label className="label">MCP 名称 *</label>
                    <input
                      type="text"
                      value={newMcpName}
                      onChange={(e) => setNewMcpName(e.target.value)}
                      placeholder="如 modao、feishu"
                      className="input"
                    />
                  </div>
                  <div>
                    <label className="label">服务地址 *</label>
                    <input
                      type="text"
                      value={newMcpEndpoint}
                      onChange={(e) => setNewMcpEndpoint(e.target.value)}
                      placeholder="如 wss://modao-mcp.example.com"
                      className="input"
                    />
                  </div>
                  <div>
                    <label className="label">描述</label>
                    <input
                      type="text"
                      value={newMcpDesc}
                      onChange={(e) => setNewMcpDesc(e.target.value)}
                      placeholder="用途说明"
                      className="input"
                    />
                  </div>
                </div>
                <div className="modal-footer">
                  <button onClick={() => setShowAddMcp(false)} className="btn-secondary">
                    取消
                  </button>
                  <button
                    onClick={handleAddMcp}
                    disabled={!newMcpName.trim() || !newMcpEndpoint.trim()}
                    className="btn-primary"
                  >
                    添加
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* 流水线配置 Tab */}
      {activeTab === "pipeline" && (
        <div className="space-y-4">
          <p className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
            配置每个环节默认使用的 Skill，新需求将自动应用对应环节的配置
          </p>

          <div className="card divide-y" style={{ borderColor: "var(--color-border)" }}>
            {pipelineStages.map((stage) => {
              const config = stageConfigs.find((c) => c.stage === stage.value);
              const configuredSkill = config ? allSkills.find((s) => s.id === config.skill_id) : null;

              return (
                <div key={stage.value} className="flex items-center justify-between px-4 py-3">
                  <div className="flex items-center gap-3">
                    <div
                      className="w-8 h-8 rounded flex items-center justify-center"
                      style={{ background: "var(--color-primary-subtle)" }}
                    >
                      <Settings className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
                    </div>
                    <div>
                      <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                        {stage.label}
                      </p>
                      <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                        {configuredSkill ? `使用: ${configuredSkill.name}` : "未配置"}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <select
                      value={config?.skill_id || ""}
                      onChange={(e) => handleStageSkillChange(stage.value, e.target.value)}
                      className="select text-xs"
                      style={{ minWidth: "150px" }}
                    >
                      <option value="">不配置</option>
                      {allSkills.map((skill) => (
                        <option key={skill.id} value={skill.id}>
                          {skill.name} ({skill.skill_type})
                        </option>
                      ))}
                    </select>
                    {config && (
                      <button
                        onClick={() => handleRemoveStageSkill(config.id)}
                        className="p-1.5 rounded-lg transition-colors"
                        style={{ color: "var(--color-text-tertiary)" }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.color = "var(--color-danger)";
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.color = "var(--color-text-tertiary)";
                        }}
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
