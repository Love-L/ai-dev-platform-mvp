import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeft, Wand2, Save, Loader2, Sparkles, Wrench, Bug } from "lucide-react";
import {
  getRequirement,
  updateRequirement,
  generatePrdStreaming,
  listPrdSkills,
  enrichRequirement,
  createTask,
  listTasksByRequirement,
} from "../lib/tauri";
import type { Requirement, PrdSkill } from "../lib/types";
import { StatusBadge, PriorityBadge } from "../components/Badges";
import { showToast } from "../components/Toast";
import { useTasksStore } from "../stores";

/** 需求状态文案 */
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

/** 需求状态合法流转（与后端 validation::validate_requirement_status_transition 一致） */
const REQ_TRANSITIONS: Record<string, string[]> = {
  draft: ["reviewing"],
  reviewing: ["approved", "rejected"],
  approved: ["frozen", "in_development"],
  rejected: ["draft"],
  frozen: ["draft", "in_development"],
  in_development: ["testing"],
  testing: ["completed", "in_development"],
  completed: [],
};

/** 当前状态 + 合法流转目标（去重，当前状态排首位） */
function statusOptions(current: string): string[] {
  return Array.from(new Set([current, ...(REQ_TRANSITIONS[current] ?? [])]));
}

function nextReqStatuses(current: string): string[] {
  return REQ_TRANSITIONS[current] ?? [];
}

export default function RequirementDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [requirement, setRequirement] = useState<Requirement | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [enriching, setEnriching] = useState(false);
  /** 正在创建的任务类型（'' 表示空闲） */
  const [creatingTask, setCreatingTask] = useState<"" | "development" | "testing">("");
  const { fetchTasks } = useTasksStore();
  const [skills, setSkills] = useState<PrdSkill[]>([]);
  const [selectedSkillId, setSelectedSkillId] = useState<string>("");
  /** 流式生成 PRD 期间累积的增量文本（F049） */
  const [streamingText, setStreamingText] = useState("");

  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState("P1");
  const [status, setStatus] = useState("draft");

  // 扩展字段（F008）
  const [moduleName, setModuleName] = useState("");
  const [source, setSource] = useState("");
  const [productOwner, setProductOwner] = useState("");
  const [developer, setDeveloper] = useState("");
  const [tester, setTester] = useState("");
  const [businessPriority, setBusinessPriority] = useState("");
  const [estimatedEffort, setEstimatedEffort] = useState("");
  const [tags, setTags] = useState("");
  const [notes, setNotes] = useState("");

  useEffect(() => {
    if (id) loadRequirement(id);
    // 加载可用 Skill 列表
    listPrdSkills().then(setSkills).catch(() => {});
  }, [id]);

  // 订阅流式 PRD 增量（F049）。必须在调用命令前完成注册，且只处理当前需求的事件。
  useEffect(() => {
    if (!id) return;
    let unlisten: (() => void) | undefined;
    let disposed = false;

    listen<{ requirement_id: string; delta: string }>("prd:chunk", (event) => {
      if (event.payload.requirement_id !== id) return;
      setStreamingText((prev) => prev + event.payload.delta);
    })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [id]);

  async function loadRequirement(reqId: string) {
    try {
      const data = await getRequirement(reqId);
      if (data) {
        setRequirement(data);
        setTitle(data.title);
        setDescription(data.description || "");
        setPriority(data.priority);
        setStatus(data.status);
        setModuleName(data.module || "");
        setSource(data.source || "");
        setProductOwner(data.product_owner || "");
        setDeveloper(data.developer || "");
        setTester(data.tester || "");
        setBusinessPriority(data.business_priority || "");
        setEstimatedEffort(
          data.estimated_effort === undefined || data.estimated_effort === null
            ? ""
            : String(data.estimated_effort)
        );
        setTags(data.tags || "");
        setNotes(data.notes || "");
      }
    } catch (error) {
      console.error("加载需求失败:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSave(): Promise<boolean> {
    if (!id) return false;
    setSaving(true);
    try {
      await updateRequirement(id, {
        title,
        description,
        priority,
        status,
        module: moduleName || undefined,
        source: source || undefined,
        productOwner: productOwner || undefined,
        developer: developer || undefined,
        tester: tester || undefined,
        businessPriority: businessPriority || undefined,
        estimatedEffort: estimatedEffort === "" ? undefined : Number(estimatedEffort),
        tags: tags || undefined,
        notes: notes || undefined,
      });
      await loadRequirement(id);
      showToast("success", "需求已保存");
      return true;
    } catch (error) {
      console.error("保存失败:", error);
      showToast("error", "保存失败: " + (error instanceof Error ? error.message : String(error)));
      return false;
    } finally {
      setSaving(false);
    }
  }

  /** 手动变更需求状态：仅提交 status，成功后刷新表单（后端校验流转合法性） */
  async function handleStatusChange(next: string) {
    if (!id || !requirement || next === requirement.status || saving) return;
    setSaving(true);
    try {
      await updateRequirement(id, { status: next });
      await loadRequirement(id);
      showToast("success", `状态已更新为「${REQ_STATUS_LABEL[next] ?? next}」`);
    } catch (error) {
      console.error("状态更新失败:", error);
      showToast("error", "状态更新失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setSaving(false);
    }
  }

  /** F010 需求字段自动完善：调用 LLM 推断字段并回填表单（成功后自动保存） */
  async function handleEnrich() {
    if (!id) return;
    setEnriching(true);
    try {
      const result = (await enrichRequirement(id)) as Record<string, unknown>;
      const text = (v: unknown) =>
        v === undefined || v === null || v === "" ? "" : String(v);

      const suggestedModule = text(result.module);
      const suggestedTags = text(result.tags);
      const suggestedBusinessPriority = text(result.business_priority);
      const suggestedSource = text(result.source);
      const suggestedEffort = text(result.estimated_effort);

      if (suggestedModule) setModuleName(suggestedModule);
      if (suggestedTags) setTags(suggestedTags);
      if (suggestedBusinessPriority) setBusinessPriority(suggestedBusinessPriority);
      if (suggestedSource) setSource(suggestedSource);
      if (suggestedEffort) setEstimatedEffort(suggestedEffort);

      const changed = [
        suggestedModule && "模块",
        suggestedTags && "标签",
        suggestedBusinessPriority && "业务优先级",
        suggestedSource && "来源",
        suggestedEffort && "预估工作量",
      ].filter(Boolean);

      if (changed.length === 0) {
        showToast("warning", "未获得有效的完善建议");
      } else {
        // 自动保存：消除"回填了表单但未落库"的歧义（后端也会写入扩展字段）
        const saved = await handleSave();
        if (saved) {
          showToast("success", `已回填并保存：${changed.join("、")}`);
        }
        // 保存失败时 handleSave 已弹出错误提示，不再显示"已保存"造成误导
      }
    } catch (error) {
      console.error("智能完善失败:", error);
      showToast("error", "智能完善失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setEnriching(false);
    }
  }

  /**
   * 需求内容代入：把需求元信息 + 描述 + PRD 组装为任务描述，
   * 使任务详情（及以任务为上下文的 Agent 会话）拿到完整需求内容。
   */
  function buildRequirementContent(
    r: Requirement,
    type: "development" | "testing"
  ): string {
    const REQ_TYPE_LABEL: Record<string, string> = {
      feature: "功能",
      bug: "缺陷",
      optimization: "优化",
      tech_debt: "技术债",
    };
    const meta: string[] = [`优先级 ${r.priority}`];
    if (r.requirement_type) {
      meta.push(`类型 ${REQ_TYPE_LABEL[r.requirement_type] ?? r.requirement_type}`);
    }
    if (r.module) meta.push(`模块 ${r.module}`);
    if (r.target_version) meta.push(`目标版本 ${r.target_version}`);
    if (r.due_date) meta.push(`截止 ${r.due_date}`);

    const lines: string[] = [`【来源需求】${r.title}`, meta.join(" · ")];
    if (r.description) {
      lines.push("", "—— 需求描述 ——", r.description);
    }
    if (r.prd_content) {
      lines.push("", "—— 需求 PRD ——", r.prd_content);
    }
    lines.push("", type === "testing" ? "—— 测试要求 ——" : "—— 开发要求 ——");
    lines.push(
      type === "testing"
        ? "依据上方需求与 PRD 编写并执行测试，覆盖正常流与异常流。"
        : "依据上方需求与 PRD 完成开发交付。"
    );
    return lines.join("\n");
  }

  /**
   * 从需求一键创建开发/测试任务：需求内容（描述/PRD/元信息）代入任务。
   * 去重：同类型任务已存在时直接跳转，不重复创建。
   */
  async function handleCreateTask(type: "development" | "testing") {
    if (!id || !requirement || creatingTask) return;
    const label = type === "development" ? "开发" : "测试";
    setCreatingTask(type);
    try {
      const existing = await listTasksByRequirement(id);
      const dup = existing.find((t) => t.task_type === type);
      if (dup) {
        showToast("warning", `该需求已存在${label}任务，为你跳转`);
        navigate(`/tasks/${dup.id}`);
        return;
      }
      const task = await createTask(
        type === "testing" ? `【测试】${requirement.title}` : requirement.title,
        id,
        type,
        requirement.priority,
        // 执行人代入：开发任务→需求开发者，测试任务→需求测试者
        (type === "testing" ? requirement.tester : requirement.developer) || undefined,
        requirement.due_date || undefined,
        undefined,
        requirement.business_priority || undefined,
        undefined,
        requirement.tags || undefined,
        `由需求「${requirement.title}」一键创建`,
        buildRequirementContent(requirement, type)
      );
      await fetchTasks();
      showToast("success", `${label}任务已创建`);
      navigate(`/tasks/${task.id}`);
    } catch (error) {
      console.error("创建任务失败:", error);
      showToast(
        "error",
        `创建${label}任务失败: ` + (error instanceof Error ? error.message : String(error))
      );
    } finally {
      setCreatingTask("");
    }
  }

  async function handleGeneratePrd() {
    if (!id) return;
    setGenerating(true);
    setStreamingText("");
    try {
      // 流式生成：增量文本经 prd:chunk 事件实时累积，返回值是最终完整内容
      const prdContent = await generatePrdStreaming(id, selectedSkillId || undefined);
      setRequirement((prev) => (prev ? { ...prev, prd_content: prdContent } : null));
      setStreamingText("");
      showToast("success", "PRD生成完成");
    } catch (error) {
      console.error("生成PRD失败:", error);
      setStreamingText("");
      showToast(
        "error",
        "生成PRD失败: " + (error instanceof Error ? error.message : String(error))
      );
    } finally {
      setGenerating(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div style={{ color: 'var(--color-text-tertiary)', fontSize: '13px' }}>加载中...</div>
      </div>
    );
  }

  if (!requirement) {
    return (
      <div className="flex items-center justify-center h-full">
        <div style={{ color: 'var(--color-text-tertiary)', fontSize: '13px' }}>需求不存在</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5">
      {/* 顶部导航 */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => navigate("/requirements")}
          className="p-1.5 rounded-lg transition-colors"
          style={{ color: 'var(--color-text-tertiary)' }}
          onMouseEnter={(e) => { e.currentTarget.style.background = 'var(--color-bg-subtle)'; e.currentTarget.style.color = 'var(--color-text-secondary)'; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = 'var(--color-text-tertiary)'; }}
        >
          <ArrowLeft className="w-4 h-4" />
        </button>
        <div>
          <h1 className="text-xl font-semibold" style={{ color: 'var(--color-text)' }}>需求详情</h1>
          <p className="mt-0.5" style={{ color: 'var(--color-text-tertiary)', fontSize: '13px' }}>编辑需求并生成PRD文档</p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
        {/* 左侧：需求信息 */}
        <div className="lg:col-span-2 space-y-5">
          {/* 基本信息 */}
          <div className="panel">
            <div className="panel-header">
              <h2 className="panel-title">基本信息</h2>
              <button
                onClick={handleEnrich}
                disabled={enriching}
                className="btn-secondary text-xs"
                title="调用 AI 分析需求并自动回填模块、标签、业务优先级等字段"
              >
                {enriching ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Sparkles className="w-3.5 h-3.5" />
                )}
                {enriching ? "分析中..." : "智能完善"}
              </button>
            </div>
            <div className="space-y-4">
              <div>
                <label className="label">标题</label>
                <input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  className="input"
                />
              </div>
              <div>
                <label className="label">描述</label>
                <textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  className="input resize-none"
                  rows={4}
                  placeholder="详细描述需求内容..."
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="label">优先级</label>
                  <select
                    value={priority}
                    onChange={(e) => setPriority(e.target.value)}
                    className="select"
                  >
                    <option value="P0">P0 - 紧急</option>
                    <option value="P1">P1 - 高</option>
                    <option value="P2">P2 - 中</option>
                    <option value="P3">P3 - 低</option>
                  </select>
                </div>
                <div>
                  <label className="label">状态</label>
                  <select
                    value={status}
                    onChange={(e) => setStatus(e.target.value)}
                    className="select"
                    title="手动变更状态（仅列出当前状态的合法流转，保存时后端复核）"
                  >
                    {statusOptions(status).map((s) => (
                      <option key={s} value={s}>
                        {REQ_STATUS_LABEL[s] ?? s}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
              <div className="flex justify-end pt-2">
                <button onClick={handleSave} disabled={saving} className="btn-primary">
                  {saving ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Save className="w-4 h-4" />
                  )}
                  保存修改
                </button>
              </div>
            </div>
          </div>

          {/* 扩展信息（F008） */}
          <div className="panel">
            <h2 className="panel-title mb-4">扩展信息</h2>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="label">所属模块</label>
                  <input
                    type="text"
                    value={moduleName}
                    onChange={(e) => setModuleName(e.target.value)}
                    className="input"
                    placeholder="如：认证模块"
                  />
                </div>
                <div>
                  <label className="label">需求来源</label>
                  <select value={source} onChange={(e) => setSource(e.target.value)} className="select">
                    <option value="">未设置</option>
                    <option value="product">产品</option>
                    <option value="tech">技术</option>
                    <option value="customer">客户</option>
                    <option value="internal">内部</option>
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div>
                  <label className="label">产品负责人</label>
                  <input type="text" value={productOwner} onChange={(e) => setProductOwner(e.target.value)} className="input" />
                </div>
                <div>
                  <label className="label">开发负责人</label>
                  <input type="text" value={developer} onChange={(e) => setDeveloper(e.target.value)} className="input" />
                </div>
                <div>
                  <label className="label">测试负责人</label>
                  <input type="text" value={tester} onChange={(e) => setTester(e.target.value)} className="input" />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="label">业务优先级</label>
                  <select
                    value={businessPriority}
                    onChange={(e) => setBusinessPriority(e.target.value)}
                    className="select"
                  >
                    <option value="">未设置</option>
                    <option value="critical">紧急</option>
                    <option value="high">高</option>
                    <option value="medium">中</option>
                    <option value="low">低</option>
                  </select>
                </div>
                <div>
                  <label className="label">预估工作量（人天）</label>
                  <input
                    type="number"
                    min="0"
                    step="0.5"
                    value={estimatedEffort}
                    onChange={(e) => setEstimatedEffort(e.target.value)}
                    className="input"
                    placeholder="0"
                  />
                </div>
              </div>

              <div>
                <label className="label">标签</label>
                <input
                  type="text"
                  value={tags}
                  onChange={(e) => setTags(e.target.value)}
                  className="input"
                  placeholder="用逗号分隔多个标签"
                />
              </div>

              <div>
                <label className="label">备注</label>
                <textarea
                  value={notes}
                  onChange={(e) => setNotes(e.target.value)}
                  className="input resize-none"
                  rows={3}
                  placeholder="补充说明..."
                />
              </div>
            </div>
          </div>

          {/* PRD内容 */}
          <div className="panel">
            <div className="panel-header">
              <h2 className="panel-title">PRD文档</h2>
              <div className="flex items-center gap-3">
                {skills.length > 0 && (
                  <select
                    value={selectedSkillId}
                    onChange={(e) => setSelectedSkillId(e.target.value)}
                    className="select text-xs"
                    style={{ minWidth: "160px" }}
                  >
                    <option value="">默认模板</option>
                    {skills.map((skill) => (
                      <option key={skill.id} value={skill.id}>
                        {skill.name} ({skill.skill_type})
                      </option>
                    ))}
                  </select>
                )}
                <button
                  onClick={handleGeneratePrd}
                  disabled={generating}
                  className="btn-primary"
                >
                  {generating ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Wand2 className="w-4 h-4" />
                  )}
                  {requirement.prd_content ? "重新生成" : "生成PRD"}
                </button>
              </div>
            </div>

            {generating && streamingText ? (
              <div
                className="rounded-lg p-4 text-sm leading-relaxed whitespace-pre-wrap font-mono"
                style={{ background: 'var(--color-bg-subtle)', color: 'var(--color-text-secondary)' }}
              >
                {streamingText}
                <span className="animate-pulse" style={{ color: 'var(--color-primary)' }}>▍</span>
              </div>
            ) : requirement.prd_content ? (
              <div
                className="rounded-lg p-4 text-sm leading-relaxed whitespace-pre-wrap font-mono"
                style={{ background: 'var(--color-bg-subtle)', color: 'var(--color-text-secondary)' }}
              >
                {requirement.prd_content}
              </div>
            ) : (
              <div className="empty-state py-10">
                <Wand2 className="w-10 h-10 mb-3" style={{ color: 'var(--color-border)' }} />
                <p className="empty-state-title">尚未生成PRD文档</p>
                <p className="empty-state-desc">点击"生成PRD"按钮，AI将根据需求内容自动生成</p>
              </div>
            )}
          </div>
        </div>

        {/* 右侧：信息面板 */}
        <div className="space-y-5">
          {/* 元信息 */}
          <div className="panel">
            <h2 className="panel-title mb-3">元信息</h2>
            <dl className="space-y-2.5">
              <div className="flex justify-between text-sm">
                <dt style={{ color: 'var(--color-text-tertiary)' }}>创建时间</dt>
                <dd style={{ color: 'var(--color-text-secondary)' }}>{new Date(requirement.created_at).toLocaleString("zh-CN")}</dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: 'var(--color-text-tertiary)' }}>更新时间</dt>
                <dd style={{ color: 'var(--color-text-secondary)' }}>{new Date(requirement.updated_at).toLocaleString("zh-CN")}</dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: 'var(--color-text-tertiary)' }}>ID</dt>
                <dd className="text-xs font-mono truncate max-w-[120px]" style={{ color: 'var(--color-text-tertiary)' }}>{requirement.id}</dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: 'var(--color-text-tertiary)' }}>状态</dt>
                <dd><StatusBadge status={requirement.status} /></dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: 'var(--color-text-tertiary)' }}>优先级</dt>
                <dd><PriorityBadge priority={requirement.priority} /></dd>
              </div>
            </dl>
          </div>

          {/* 快捷操作 */}
          <div className="panel">
            <h2 className="panel-title mb-3">快捷操作</h2>
            <div className="space-y-2">
              {/* 手动变更状态（仅合法流转目标，立即生效） */}
              <div>
                <label className="label">变更状态</label>
                <select
                  value={requirement.status}
                  onChange={(e) => handleStatusChange(e.target.value)}
                  disabled={saving || nextReqStatuses(requirement.status).length === 0}
                  className="select"
                  title="手动变更状态（仅列出合法流转目标，后端校验）"
                >
                  <option value={requirement.status}>
                    {REQ_STATUS_LABEL[requirement.status] ?? requirement.status}（当前）
                  </option>
                  {nextReqStatuses(requirement.status).map((s) => (
                    <option key={s} value={s}>
                      {REQ_STATUS_LABEL[s] ?? s}
                    </option>
                  ))}
                </select>
                {nextReqStatuses(requirement.status).length === 0 && (
                  <p className="text-[11px] mt-1" style={{ color: "var(--color-text-tertiary)" }}>
                    已到终态，无可流转目标
                  </p>
                )}
              </div>
              <button
                onClick={() => handleCreateTask("development")}
                disabled={!!creatingTask}
                className="w-full btn-secondary justify-start"
                title="从当前需求创建开发任务并自动关联"
              >
                {creatingTask === "development" ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Wrench className="w-4 h-4" />
                )}
                创建开发任务
              </button>
              <button
                onClick={() => handleCreateTask("testing")}
                disabled={!!creatingTask}
                className="w-full btn-secondary justify-start"
                title="从当前需求创建测试任务并自动关联"
              >
                {creatingTask === "testing" ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Bug className="w-4 h-4" />
                )}
                创建测试任务
              </button>
              {!["approved", "frozen", "in_development", "testing", "completed"].includes(
                requirement.status
              ) && (
                <p className="text-[11px] pt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                  当前需求尚未通过评审，建议通过后创建任务
                </p>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
