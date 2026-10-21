import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  ArrowLeft,
  Wand2,
  Save,
  Loader2,
  Sparkles,
  Wrench,
  Bug,
  History,
  FileText,
  ListChecks,
  Paperclip,
  Plus,
  X,
  ArrowUpCircle,
} from "lucide-react";
import {
  getRequirement,
  updateRequirement,
  generatePrdStreaming,
  listPrdSkills,
  enrichRequirement,
  createTask,
  listTasksByRequirement,
  listPrdVersions,
  restorePrdVersion,
  generateTaskBreakdown,
  getRequirementTransitions,
  type PrdVersion,
  type BreakdownTask,
} from "../lib/tauri";
import type { Requirement, PrdSkill, Task } from "../lib/types";
import { StatusBadge, PriorityBadge } from "../components/Badges";
import ConfirmDialog from "../components/ConfirmDialog";
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

/**
 * 状态机兜底表：仅在后端 `get_requirement_transitions` 拉取失败（纯浏览器预览等）时使用。
 * 正常运行以后端下发为唯一真源（与 validation::requirement_transitions 同源）。
 */
const REQ_TRANSITIONS_FALLBACK: Record<string, string[]> = {
  draft: ["reviewing"],
  reviewing: ["approved", "rejected"],
  approved: ["frozen", "in_development"],
  rejected: ["draft"],
  frozen: ["draft", "in_development"],
  in_development: ["testing"],
  testing: ["completed", "in_development"],
  completed: [],
};

/** 解析 attachments 字段：JSON 数组优先，退化兼容逗号分隔/换行分隔 */
function parseAttachmentsField(raw?: string | null): string[] {
  if (!raw || !raw.trim()) return [];
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed.map(String).filter(Boolean);
  } catch {
    // 非 JSON，走分隔符兼容
  }
  return raw
    .split(/[,\n]/)
    .map((s) => s.trim())
    .filter(Boolean);
}

/** 当前状态 + 合法流转目标（去重，当前状态排首位） */
function statusOptions(current: string, table: Record<string, string[]>): string[] {
  return Array.from(new Set([current, ...(table[current] ?? [])]));
}

function nextReqStatuses(current: string, table: Record<string, string[]>): string[] {
  return table[current] ?? [];
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

  // ---- P0-② PRD 版本保护 ----
  const [prdView, setPrdView] = useState<"preview" | "source">("preview");
  const [versions, setVersions] = useState<PrdVersion[]>([]);
  const [showVersions, setShowVersions] = useState(false);
  const [confirmRegenerate, setConfirmRegenerate] = useState(false);
  const [restoringId, setRestoringId] = useState("");

  // ---- P1-⑪ 智能拆解 ----
  const [breakdownTasks, setBreakdownTasks] = useState<BreakdownTask[] | null>(null);
  const [breakdownChecked, setBreakdownChecked] = useState<Set<number>>(new Set());
  const [breakingDown, setBreakingDown] = useState(false);
  const [creatingBulk, setCreatingBulk] = useState(false);

  // ---- P2 状态机下发 + 任务联动 + 附件 ----
  const [transitions, setTransitions] = useState<Record<string, string[]>>(REQ_TRANSITIONS_FALLBACK);
  const [reqTasks, setReqTasks] = useState<Task[]>([]);
  const [attachments, setAttachments] = useState<string[]>([]);
  const [newAttachmentPath, setNewAttachmentPath] = useState("");

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
    listPrdSkills()
      .then(setSkills)
      .catch(() => {});
    // 状态机以后端为唯一真源（拉取失败退回兜底表）
    getRequirementTransitions()
      .then((table) => {
        if (Object.keys(table).length > 0) setTransitions(table);
      })
      .catch(() => {});
  }, [id]);

  /** 该需求的关联任务（联动提示：全部完成时引导推进需求状态） */
  async function loadReqTasks(reqId: string) {
    try {
      setReqTasks(await listTasksByRequirement(reqId));
    } catch {
      // 拉取失败不阻断页面
    }
  }

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
          data.estimated_effort === undefined || data.estimated_effort === null ? "" : String(data.estimated_effort),
        );
        setTags(data.tags || "");
        setNotes(data.notes || "");
        // 附件字段：JSON 数组优先，退化兼容逗号分隔
        setAttachments(parseAttachmentsField(data.attachments));
      }
      loadReqTasks(reqId);
    } catch (error) {
      console.error("加载需求失败:", error);
    } finally {
      setLoading(false);
    }
  }

  // ---- P0-② 版本保护 ----

  /** 打开 PRD 前先确认：重新生成会覆盖当前版本（旧版自动留档为快照，可恢复） */
  function requestGeneratePrd() {
    if (requirement?.prd_content) setConfirmRegenerate(true);
    else void handleGeneratePrd();
  }

  async function handleLoadVersions() {
    if (!id) return;
    setShowVersions((v) => !v);
    if (showVersions) return;
    try {
      setVersions(await listPrdVersions(id));
    } catch (error) {
      showToast("error", "加载版本历史失败: " + (error instanceof Error ? error.message : String(error)));
    }
  }

  async function handleRestore(versionId: string) {
    if (!id) return;
    setRestoringId(versionId);
    try {
      const updated = await restorePrdVersion(id, versionId);
      setRequirement(updated);
      setStatus(updated.status);
      showToast("success", "已恢复该版本的 PRD");
      setShowVersions(false);
    } catch (error) {
      showToast("error", "恢复失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setRestoringId("");
    }
  }

  // ---- P1-⑪ 智能拆解 ----

  async function handleBreakdown() {
    if (!id) return;
    setBreakingDown(true);
    try {
      const tasks = await generateTaskBreakdown(id);
      setBreakdownTasks(tasks);
      setBreakdownChecked(new Set(tasks.map((_, i) => i)));
    } catch (error) {
      showToast("error", "智能拆解失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setBreakingDown(false);
    }
  }

  /** 勾选的候选任务批量落库（requirement_id 关联 + 需求内容代入任务描述） */
  async function handleCreateBreakdownTasks() {
    if (!id || !requirement || !breakdownTasks || creatingBulk) return;
    const picked = breakdownTasks.filter((_, i) => breakdownChecked.has(i));
    if (picked.length === 0) return;
    setCreatingBulk(true);
    try {
      for (const t of picked) {
        await createTask(
          t.title,
          id,
          t.task_type,
          t.priority,
          undefined,
          requirement.due_date || undefined,
          undefined,
          requirement.business_priority || undefined,
          undefined,
          requirement.tags || undefined,
          `由需求「${requirement.title}」智能拆解创建`,
          `${t.description}\n\n${buildRequirementContent(requirement, t.task_type === "testing" ? "testing" : "development")}`,
        );
      }
      await fetchTasks();
      showToast("success", `已创建 ${picked.length} 个任务`);
      setBreakdownTasks(null);
      navigate("/board");
    } catch (error) {
      console.error("批量创建任务失败:", error);
      showToast("error", "批量创建任务失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setCreatingBulk(false);
    }
  }

  // ---- P2 附件（路径管理，Agent 可读取） ----

  function handleAddAttachment() {
    const path = newAttachmentPath.trim();
    if (!path) return;
    if (attachments.includes(path)) {
      showToast("warning", "该附件已在列表中");
      return;
    }
    setAttachments([...attachments, path]);
    setNewAttachmentPath("");
  }

  async function handleSaveAttachments(next: string[]) {
    if (!id) return;
    setAttachments(next);
    try {
      await updateRequirement(id, { attachments: JSON.stringify(next) });
      showToast("success", "附件已保存");
    } catch (error) {
      showToast("error", "附件保存失败: " + (error instanceof Error ? error.message : String(error)));
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
      const text = (v: unknown) => (v === undefined || v === null || v === "" ? "" : String(v));

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
  function buildRequirementContent(r: Requirement, type: "development" | "testing"): string {
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
        : "依据上方需求与 PRD 完成开发交付。",
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
        buildRequirementContent(requirement, type),
      );
      await fetchTasks();
      showToast("success", `${label}任务已创建`);
      navigate(`/tasks/${task.id}`);
    } catch (error) {
      console.error("创建任务失败:", error);
      showToast("error", `创建${label}任务失败: ` + (error instanceof Error ? error.message : String(error)));
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
      showToast("error", "生成PRD失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setGenerating(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>加载中...</div>
      </div>
    );
  }

  if (!requirement) {
    return (
      <div className="flex items-center justify-center h-full">
        <div style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>需求不存在</div>
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
          style={{ color: "var(--color-text-tertiary)" }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = "var(--color-bg-subtle)";
            e.currentTarget.style.color = "var(--color-text-secondary)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "transparent";
            e.currentTarget.style.color = "var(--color-text-tertiary)";
          }}
        >
          <ArrowLeft className="w-4 h-4" />
        </button>
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            需求详情
          </h1>
          <p className="mt-0.5" style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>
            编辑需求并生成PRD文档
          </p>
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
                {enriching ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Sparkles className="w-3.5 h-3.5" />}
                {enriching ? "分析中..." : "智能完善"}
              </button>
            </div>
            <div className="space-y-4">
              <div>
                <label className="label">标题</label>
                <input type="text" value={title} onChange={(e) => setTitle(e.target.value)} className="input" />
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
                  <select value={priority} onChange={(e) => setPriority(e.target.value)} className="select">
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
                    {statusOptions(status, transitions).map((s) => (
                      <option key={s} value={s}>
                        {REQ_STATUS_LABEL[s] ?? s}
                      </option>
                    ))}
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
                  <input
                    type="text"
                    value={productOwner}
                    onChange={(e) => setProductOwner(e.target.value)}
                    className="input"
                  />
                </div>
                <div>
                  <label className="label">开发负责人</label>
                  <input
                    type="text"
                    value={developer}
                    onChange={(e) => setDeveloper(e.target.value)}
                    className="input"
                  />
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

          {/* PRD内容（P0 快照保护 + P1 Markdown 渲染） */}
          <div className="panel">
            <div className="panel-header">
              <h2 className="panel-title">PRD文档</h2>
              <div className="flex items-center gap-2">
                {requirement.prd_content && (
                  <>
                    {/* 预览 / 源码切换 */}
                    <div
                      className="flex rounded-lg overflow-hidden border"
                      style={{ borderColor: "var(--color-border)" }}
                    >
                      {(["preview", "source"] as const).map((mode) => (
                        <button
                          key={mode}
                          onClick={() => setPrdView(mode)}
                          className="px-2.5 py-1 text-xs font-medium"
                          style={{
                            background: prdView === mode ? "var(--color-primary-subtle)" : "transparent",
                            color: prdView === mode ? "var(--color-primary)" : "var(--color-text-tertiary)",
                          }}
                        >
                          {mode === "preview" ? "预览" : "源码"}
                        </button>
                      ))}
                    </div>
                    {/* 版本历史 */}
                    <button
                      onClick={handleLoadVersions}
                      className="btn-secondary text-xs"
                      title="查看 PRD 快照历史（每次生成自动留档）"
                    >
                      <History className="w-3.5 h-3.5" />
                      历史
                    </button>
                  </>
                )}
                {skills.length > 0 && (
                  <select
                    value={selectedSkillId}
                    onChange={(e) => setSelectedSkillId(e.target.value)}
                    className="select text-xs"
                    style={{ minWidth: "150px" }}
                  >
                    <option value="">默认模板</option>
                    {skills.map((skill) => (
                      <option key={skill.id} value={skill.id}>
                        {skill.name} ({skill.skill_type})
                      </option>
                    ))}
                  </select>
                )}
                <button onClick={requestGeneratePrd} disabled={generating} className="btn-primary text-xs">
                  {generating ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Wand2 className="w-3.5 h-3.5" />}
                  {requirement.prd_content ? "重新生成" : "生成PRD"}
                </button>
              </div>
            </div>

            {/* 版本历史下拉 */}
            {showVersions && (
              <div
                className="mb-3 rounded-lg border divide-y"
                style={{ borderColor: "var(--color-border)", background: "var(--color-bg-subtle)" }}
              >
                {versions.length === 0 ? (
                  <p className="px-3 py-2.5 text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                    暂无快照（历史版本在每次生成时自动留档）
                  </p>
                ) : (
                  versions.slice(0, 10).map((v) => (
                    <div key={v.id} className="flex items-center gap-2 px-3 py-2">
                      <FileText className="w-3.5 h-3.5 shrink-0" style={{ color: "var(--color-text-tertiary)" }} />
                      <div className="min-w-0 flex-1">
                        <p className="text-xs truncate" style={{ color: "var(--color-text-secondary)" }}>
                          {v.content.split("\n")[0]?.slice(0, 60) || "(空版本)"}
                        </p>
                        <p className="text-[10px]" style={{ color: "var(--color-text-tertiary)" }}>
                          {new Date(v.created_at).toLocaleString("zh-CN")}
                        </p>
                      </div>
                      <button
                        onClick={() => handleRestore(v.id)}
                        disabled={!!restoringId}
                        className="text-xs px-2 py-1 rounded-md"
                        style={{ color: "var(--color-primary)" }}
                        title="恢复此版本为当前 PRD"
                      >
                        {restoringId === v.id ? <Loader2 className="w-3 h-3 animate-spin inline" /> : "恢复"}
                      </button>
                    </div>
                  ))
                )}
              </div>
            )}

            {generating && streamingText ? (
              <div
                className="rounded-lg p-4 text-sm leading-relaxed whitespace-pre-wrap font-mono"
                style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-secondary)" }}
              >
                {streamingText}
                <span className="animate-pulse" style={{ color: "var(--color-primary)" }}>
                  ▍
                </span>
              </div>
            ) : requirement.prd_content ? (
              prdView === "preview" ? (
                <div
                  className="rounded-lg p-4 text-sm leading-relaxed prd-markdown"
                  style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-secondary)" }}
                >
                  <ReactMarkdown remarkPlugins={[remarkGfm]}>{requirement.prd_content}</ReactMarkdown>
                </div>
              ) : (
                <div
                  className="rounded-lg p-4 text-sm leading-relaxed whitespace-pre-wrap font-mono"
                  style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-secondary)" }}
                >
                  {requirement.prd_content}
                </div>
              )
            ) : (
              <div className="empty-state py-10">
                <Wand2 className="w-10 h-10 mb-3" style={{ color: "var(--color-border)" }} />
                <p className="empty-state-title">尚未生成PRD文档</p>
                <p className="empty-state-desc">点击"生成PRD"按钮，AI将根据需求内容自动生成</p>
              </div>
            )}
          </div>

          {/* P2 附件（路径管理，随任务/会话上下文供 Agent 读取） */}
          <div className="panel">
            <div className="panel-header">
              <h2 className="panel-title flex items-center gap-2">
                <Paperclip className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
                附件
              </h2>
            </div>
            {attachments.length > 0 && (
              <div className="space-y-1.5 mb-3">
                {attachments.map((path, i) => (
                  <div
                    key={`${path}-${i}`}
                    className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-xs"
                    style={{ background: "var(--color-bg-subtle)" }}
                  >
                    <FileText className="w-3.5 h-3.5 shrink-0" style={{ color: "var(--color-text-tertiary)" }} />
                    <span
                      className="truncate flex-1 font-mono"
                      style={{ color: "var(--color-text-secondary)" }}
                      title={path}
                    >
                      {path.split(/[\\/]/).pop()}
                    </span>
                    <span
                      className="truncate max-w-[220px] text-[10px]"
                      style={{ color: "var(--color-text-tertiary)" }}
                      title={path}
                    >
                      {path}
                    </span>
                    <button
                      onClick={() => handleSaveAttachments(attachments.filter((_, j) => j !== i))}
                      className="p-1 rounded"
                      style={{ color: "var(--color-text-tertiary)" }}
                      title="移除"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                ))}
              </div>
            )}
            <div className="flex gap-2">
              <input
                type="text"
                value={newAttachmentPath}
                onChange={(e) => setNewAttachmentPath(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleAddAttachment();
                }}
                placeholder="粘贴附件绝对路径（如 D:\design\原型图.png）"
                className="input text-xs flex-1"
              />
              <button
                onClick={() => {
                  handleAddAttachment();
                  if (newAttachmentPath.trim()) handleSaveAttachments([...attachments, newAttachmentPath.trim()]);
                }}
                disabled={!newAttachmentPath.trim()}
                className="btn-secondary text-xs"
              >
                <Plus className="w-3.5 h-3.5" />
                添加
              </button>
            </div>
            <p className="text-[11px] mt-2" style={{ color: "var(--color-text-tertiary)" }}>
              附件为本机文件路径，Agent 执行任务时可直接读取；从任务一键创建会话时随任务描述带入。
            </p>
          </div>
        </div>

        {/* 右侧：信息面板 */}
        <div className="space-y-5">
          {/* 元信息 */}
          <div className="panel">
            <h2 className="panel-title mb-3">元信息</h2>
            <dl className="space-y-2.5">
              <div className="flex justify-between text-sm">
                <dt style={{ color: "var(--color-text-tertiary)" }}>创建时间</dt>
                <dd style={{ color: "var(--color-text-secondary)" }}>
                  {new Date(requirement.created_at).toLocaleString("zh-CN")}
                </dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: "var(--color-text-tertiary)" }}>更新时间</dt>
                <dd style={{ color: "var(--color-text-secondary)" }}>
                  {new Date(requirement.updated_at).toLocaleString("zh-CN")}
                </dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: "var(--color-text-tertiary)" }}>ID</dt>
                <dd
                  className="text-xs font-mono truncate max-w-[120px]"
                  style={{ color: "var(--color-text-tertiary)" }}
                >
                  {requirement.id}
                </dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: "var(--color-text-tertiary)" }}>状态</dt>
                <dd>
                  <StatusBadge status={requirement.status} />
                </dd>
              </div>
              <div className="flex justify-between text-sm">
                <dt style={{ color: "var(--color-text-tertiary)" }}>优先级</dt>
                <dd>
                  <PriorityBadge priority={requirement.priority} />
                </dd>
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
                  disabled={saving || nextReqStatuses(requirement.status, transitions).length === 0}
                  className="select"
                  title="手动变更状态（仅列出合法流转目标，后端校验）"
                >
                  <option value={requirement.status}>
                    {REQ_STATUS_LABEL[requirement.status] ?? requirement.status}（当前）
                  </option>
                  {nextReqStatuses(requirement.status, transitions).map((s) => (
                    <option key={s} value={s}>
                      {REQ_STATUS_LABEL[s] ?? s}
                    </option>
                  ))}
                </select>
                {nextReqStatuses(requirement.status, transitions).length === 0 && (
                  <p className="text-[11px] mt-1" style={{ color: "var(--color-text-tertiary)" }}>
                    已到终态，无可流转目标
                  </p>
                )}
              </div>
              {/* P1-⑪ 智能拆解（LLM 按 PRD 出清单，勾选确认后批量落库） */}
              <button
                onClick={handleBreakdown}
                disabled={breakingDown || !!creatingBulk}
                className="w-full btn-primary justify-start"
                title="读取 PRD，由 AI 拆解为多条可执行任务（勾选确认后创建）"
              >
                {breakingDown ? <Loader2 className="w-4 h-4 animate-spin" /> : <ListChecks className="w-4 h-4" />}
                {breakingDown ? "AI 拆解中..." : "智能拆解任务"}
              </button>

              {/* P2 状态↔任务联动：任务全部完成时引导推进需求状态 */}
              {reqTasks.length > 0 &&
                reqTasks.every((t) => t.status === "completed") &&
                (requirement.status === "in_development" || requirement.status === "testing") && (
                  <div
                    className="rounded-lg px-3 py-2.5 text-xs space-y-2"
                    style={{ background: "var(--color-success-light)", color: "var(--color-text-secondary)" }}
                  >
                    <p className="flex items-center gap-1.5">
                      <ArrowUpCircle className="w-3.5 h-3.5" style={{ color: "var(--color-success)" }} />
                      该需求的 {reqTasks.length} 个任务已全部完成
                    </p>
                    <button
                      onClick={() =>
                        handleStatusChange(requirement.status === "in_development" ? "testing" : "completed")
                      }
                      disabled={saving}
                      className="btn-secondary text-xs w-full"
                    >
                      推进到「
                      {REQ_STATUS_LABEL[requirement.status === "in_development" ? "testing" : "completed"] ?? ""}」
                    </button>
                  </div>
                )}

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
              {!["approved", "frozen", "in_development", "testing", "completed"].includes(requirement.status) && (
                <p className="text-[11px] pt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                  当前需求尚未通过评审，建议通过后创建任务
                </p>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* P0 重新生成确认：覆盖当前 PRD（旧版自动留档为快照） */}
      {confirmRegenerate && (
        <ConfirmDialog
          title="重新生成 PRD"
          message="将基于当前需求与 PRD 模板生成新版本，覆盖现有内容。\n当前版本会自动保留为历史快照，可随时在「历史」中查看与恢复。"
          confirmLabel="生成新版本"
          onConfirm={() => {
            setConfirmRegenerate(false);
            void handleGeneratePrd();
          }}
          onCancel={() => setConfirmRegenerate(false)}
        />
      )}

      {/* P1 智能拆解清单：勾选确认后批量创建任务 */}
      {breakdownTasks && (
        <div className="modal-overlay" onClick={() => !creatingBulk && setBreakdownTasks(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 640 }}>
            <div className="modal-header">
              <h3 className="text-base font-semibold flex items-center gap-2" style={{ color: "var(--color-text)" }}>
                <ListChecks className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
                智能拆解清单（{breakdownChecked.size}/{breakdownTasks.length} 已选）
              </h3>
            </div>
            <div className="modal-body space-y-2 max-h-[420px] overflow-y-auto">
              {breakdownTasks.length === 0 ? (
                <p className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
                  未拆解出任务，请补充 PRD 后重试。
                </p>
              ) : (
                breakdownTasks.map((t, i) => (
                  <label
                    key={`${t.title}-${i}`}
                    className="flex items-start gap-2.5 p-2.5 rounded-lg cursor-pointer"
                    style={{
                      background: breakdownChecked.has(i) ? "var(--color-primary-subtle)" : "var(--color-bg-subtle)",
                      border: `1px solid ${breakdownChecked.has(i) ? "var(--color-primary)" : "transparent"}`,
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={breakdownChecked.has(i)}
                      onChange={(e) => {
                        const next = new Set(breakdownChecked);
                        if (e.target.checked) next.add(i);
                        else next.delete(i);
                        setBreakdownChecked(next);
                      }}
                      className="mt-1"
                      style={{ accentColor: "var(--color-primary)" }}
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <p className="text-sm font-medium truncate" style={{ color: "var(--color-text)" }}>
                          {t.title}
                        </p>
                        <span
                          className="text-[10px] px-1.5 py-0.5 rounded shrink-0"
                          style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-tertiary)" }}
                        >
                          {t.task_type === "testing" ? "测试" : t.task_type === "bug_fix" ? "修复" : "开发"}
                        </span>
                        <span
                          className="text-[10px] px-1.5 py-0.5 rounded shrink-0"
                          style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-tertiary)" }}
                        >
                          {t.priority}
                        </span>
                      </div>
                      {t.description && (
                        <p className="text-xs mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                          {t.description}
                        </p>
                      )}
                    </div>
                  </label>
                ))
              )}
            </div>
            <div className="modal-footer">
              <button onClick={() => setBreakdownTasks(null)} disabled={creatingBulk} className="btn-secondary">
                取消
              </button>
              <button
                onClick={handleCreateBreakdownTasks}
                disabled={creatingBulk || breakdownChecked.size === 0}
                className="btn-primary"
              >
                {creatingBulk ? <Loader2 className="w-4 h-4 animate-spin" /> : <Plus className="w-4 h-4" />}
                创建选中任务（{breakdownChecked.size}）
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
