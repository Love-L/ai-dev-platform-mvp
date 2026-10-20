import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  ArrowLeft,
  Activity,
  Sparkles,
  RefreshCw,
  RotateCcw,
  AlertTriangle,
  AlertCircle,
  Info,
  CheckCircle2,
  Clock,
  Zap,
  FileText,
  ChevronDown,
  ChevronUp,
  X,
  Loader2,
  Lightbulb,
  GitBranch,
} from "lucide-react";
import type {
  PrdSkill,
  SkillExecutionLog,
  SkillOptimizeDraft,
  SkillScore,
  SkillScoreDimension,
  SkillSuggestion,
  SkillVersion,
} from "../lib/types";
import {
  getPrdSkill,
  getSkillExecutionLogsBySkill,
  getSkillScoreHistory,
  judgeSkill,
  analyzeSkills,
  listSkillSuggestions,
  listSkillVersions,
  optimizeSkill,
  applySkillDraft,
  applySkillSuggestion,
  revertSkillVersion,
} from "../lib/tauri";
import { showToast } from "../components/Toast";

type TabKey = "overview" | "scores" | "versions" | "logs";

const SEVERITY_ICON = {
  HIGH: AlertTriangle,
  MEDIUM: AlertCircle,
  LOW: Info,
} as const;

const SEVERITY_CLASS = {
  HIGH: "text-red-600 bg-red-50",
  MEDIUM: "text-amber-600 bg-amber-50",
  LOW: "text-blue-600 bg-blue-50",
} as const;

function scoreColor(total: number): string {
  if (total >= 80) return "#16a34a";
  if (total >= 60) return "#d97706";
  return "#dc2626";
}

/** 评分环：SVG 圆环 + 中心总分 */
function ScoreRing({ total }: { total: number }) {
  const r = 44;
  const c = 2 * Math.PI * r;
  const pct = Math.max(0, Math.min(100, total));
  return (
    <svg width="110" height="110" viewBox="0 0 110 110" className="shrink-0">
      <circle cx="55" cy="55" r={r} fill="none" stroke="#e5e7eb" strokeWidth="10" />
      <circle
        cx="55"
        cy="55"
        r={r}
        fill="none"
        stroke={scoreColor(pct)}
        strokeWidth="10"
        strokeLinecap="round"
        strokeDasharray={`${(c * pct) / 100} ${c}`}
        transform="rotate(-90 55 55)"
      />
      <text x="55" y="52" textAnchor="middle" fontSize="26" fontWeight="700" fill="#111827">
        {total}
      </text>
      <text x="55" y="72" textAnchor="middle" fontSize="11" fill="#6b7280">
        / 100
      </text>
    </svg>
  );
}

/** 简易折线图：版本 ↔ 总分（内联 SVG，无第三方依赖） */
function ScoreTrend({ history }: { history: SkillScore[] }) {
  if (history.length === 0) return null;
  const w = 560;
  const h = 140;
  const pad = 28;
  const max = 100;
  const min = Math.max(0, Math.min(...history.map((s) => s.total_score)) - 10);
  const stepX =
    history.length > 1 ? (w - pad * 2) / (history.length - 1) : 0;
  const y = (v: number) => h - pad - ((v - min) / (max - min)) * (h - pad * 2);
  const points = history
    .map((s, i) => `${pad + i * stepX},${y(s.total_score)}`)
    .join(" ");
  return (
    <svg width="100%" viewBox={`0 0 ${w} ${h}`} className="max-w-[560px]">
      <line x1={pad} y1={h - pad} x2={w - pad} y2={h - pad} stroke="#e5e7eb" />
      <line x1={pad} y1={pad} x2={pad} y2={h - pad} stroke="#e5e7eb" />
      <polyline points={points} fill="none" stroke="#2563eb" strokeWidth="2" />
      {history.map((s, i) => (
        <g key={s.id}>
          <circle cx={pad + i * stepX} cy={y(s.total_score)} r="4" fill="#2563eb" />
          <text
            x={pad + i * stepX}
            y={h - pad + 14}
            textAnchor="middle"
            fontSize="10"
            fill="#6b7280"
          >
            v{s.version}
          </text>
          <text
            x={pad + i * stepX}
            y={y(s.total_score) - 8}
            textAnchor="middle"
            fontSize="10"
            fill="#374151"
          >
            {s.total_score}
          </text>
        </g>
      ))}
    </svg>
  );
}

/** 行级左右对照 diff（下标对齐，变化行高亮） */
function TemplateDiff({ oldText, newText }: { oldText: string; newText: string }) {
  const oldLines = oldText.split("\n");
  const newLines = newText.split("\n");
  const rows = Math.max(oldLines.length, newLines.length);
  return (
    <div className="grid grid-cols-2 gap-3 text-xs font-mono">
      <div className="border rounded-lg overflow-hidden">
        <div className="px-3 py-1.5 bg-gray-100 text-gray-600 font-sans font-medium">
          当前模板（v 旧）
        </div>
        <pre className="p-3 max-h-[45vh] overflow-auto whitespace-pre-wrap leading-5">
          {Array.from({ length: rows }).map((_, i) => {
            const line = oldLines[i] ?? "";
            const changed = line !== (newLines[i] ?? "");
            return (
              <div key={i} className={changed ? "bg-red-50 text-red-800" : ""}>
                {line || " "}
              </div>
            );
          })}
        </pre>
      </div>
      <div className="border rounded-lg overflow-hidden">
        <div className="px-3 py-1.5 bg-blue-50 text-blue-700 font-sans font-medium">
          优化草稿（新）
        </div>
        <pre className="p-3 max-h-[45vh] overflow-auto whitespace-pre-wrap leading-5">
          {Array.from({ length: rows }).map((_, i) => {
            const line = newLines[i] ?? "";
            const changed = line !== (oldLines[i] ?? "");
            return (
              <div key={i} className={changed ? "bg-green-50 text-green-800" : ""}>
                {line || " "}
              </div>
            );
          })}
        </pre>
      </div>
    </div>
  );
}

export default function SkillEvolution() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [skill, setSkill] = useState<PrdSkill | null>(null);
  const [history, setHistory] = useState<SkillScore[]>([]);
  const [suggestions, setSuggestions] = useState<SkillSuggestion[]>([]);
  const [versions, setVersions] = useState<SkillVersion[]>([]);
  const [logs, setLogs] = useState<SkillExecutionLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<TabKey>("overview");

  const [judging, setJudging] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [optimizing, setOptimizing] = useState(false);

  const [draft, setDraft] = useState<SkillOptimizeDraft | null>(null);
  const [draftSource, setDraftSource] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);

  const [expandedLog, setExpandedLog] = useState<string | null>(null);
  const [expandedVersion, setExpandedVersion] = useState<number | null>(null);

  const latest = history.length > 0 ? history[history.length - 1] : null;
  const latestDims: SkillScoreDimension[] = useMemo(() => {
    if (!latest) return [];
    try {
      return JSON.parse(latest.dimensions_json);
    } catch {
      return [];
    }
  }, [latest]);

  const ratchetBadge = useMemo(() => {
    const hasRevert = suggestions.some(
      (s) => s.rule_type === "ratchet_revert" && s.status === "pending"
    );
    const hasOsc = suggestions.some(
      (s) => s.rule_type === "oscillation" && s.status === "pending"
    );
    if (hasRevert)
      return { label: "below_peak", cls: "bg-amber-50 text-amber-700" };
    if (hasOsc)
      return { label: "oscillating", cls: "bg-orange-50 text-orange-700" };
    if (latest) return { label: "keep", cls: "bg-green-50 text-green-700" };
    return null;
  }, [suggestions, latest]);

  const loadAll = useCallback(async () => {
    if (!id) return;
    try {
      const [s, h, sg, v, l] = await Promise.all([
        getPrdSkill(id),
        getSkillScoreHistory(id),
        listSkillSuggestions(id),
        listSkillVersions(id),
        getSkillExecutionLogsBySkill(id, 50),
      ]);
      setSkill(s);
      setHistory(h);
      setSuggestions(sg);
      setVersions(v);
      setLogs(l);
    } catch (e) {
      console.error("加载进化数据失败:", e);
      showToast("error", "加载进化数据失败");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  async function handleJudge() {
    if (!id) return;
    setJudging(true);
    try {
      const score = await judgeSkill(id);
      showToast(
        "success",
        `评分完成：结构 ${score.structure_score}/60` +
          (score.effect_score != null
            ? ` · 效果 ${score.effect_score}/40`
            : " · 仅结构分")
      );
      await loadAll();
    } catch (e) {
      showToast("error", `评分失败：${e}`);
    } finally {
      setJudging(false);
    }
  }

  async function handleAnalyze() {
    if (!id) return;
    setAnalyzing(true);
    try {
      await analyzeSkills();
      const sg = await listSkillSuggestions(id);
      setSuggestions(sg);
      showToast("success", `诊断完成：${sg.filter((s) => s.status === "pending").length} 条待处理建议`);
    } catch (e) {
      showToast("error", `诊断失败：${e}`);
    } finally {
      setAnalyzing(false);
    }
  }

  async function handleOptimize() {
    if (!id || !skill) return;
    if (skill.is_builtin) {
      showToast("error", "内置 Skill 不可优化，请先创建自定义版本");
      return;
    }
    setOptimizing(true);
    try {
      const d = await optimizeSkill(id);
      setDraft(d);
      setDraftSource(null);
    } catch (e) {
      showToast("error", `AI 优化失败：${e}`);
    } finally {
      setOptimizing(false);
    }
  }

  async function handleOptimizeFromSuggestion(sg: SkillSuggestion) {
    if (!id || !skill) return;
    if (skill.is_builtin) {
      showToast("error", "内置 Skill 不可优化，请先创建自定义版本");
      return;
    }
    setOptimizing(true);
    try {
      const d = await optimizeSkill(id);
      setDraft(d);
      setDraftSource(sg.id);
    } catch (e) {
      showToast("error", `AI 优化失败：${e}`);
    } finally {
      setOptimizing(false);
    }
  }

  async function handleApplyDraft() {
    if (!id || !skill || !draft) return;
    setApplying(true);
    try {
      const before = latest?.total_score ?? null;
      if (draftSource) {
        await applySkillSuggestion(draftSource, draft.new_template);
      } else {
        await applySkillDraft(id, draft.new_template);
      }
      setDraft(null);
      setDraftSource(null);
      // 应用后自动评分（棘轮判定）
      try {
        const score = await judgeSkill(id);
        const delta =
          before != null ? `，评分 ${before} → ${score.total_score}` : "";
        const regressed =
          before != null && score.total_score < before;
        showToast(
          regressed ? "error" : "success",
          regressed
            ? `已应用但分数下降${delta}，建议回退`
            : `已应用为新版本（v${score.version}）${delta}`
        );
      } catch {
        showToast("success", "已应用为新版本");
      }
      await loadAll();
    } catch (e) {
      showToast("error", `应用失败：${e}`);
    } finally {
      setApplying(false);
    }
  }

  async function handleRevert(version: number) {
    if (!id) return;
    if (!window.confirm(`确认回退到 v${version}？将以该版本模板创建新版本。`))
      return;
    try {
      const updated = await revertSkillVersion(id, version);
      showToast("success", `已回退，当前版本 v${updated.version}`);
      await loadAll();
    } catch (e) {
      showToast("error", `回退失败：${e}`);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">加载中...</div>
      </div>
    );
  }

  if (!skill) {
    return (
      <div className="p-6">
        <div className="card p-8 text-center text-sm text-gray-500">
          Skill 不存在
          <button className="ml-3 text-blue-600" onClick={() => navigate("/skills")}>
            返回 Skill 管理
          </button>
        </div>
      </div>
    );
  }

  const pendingSuggestions = suggestions.filter((s) => s.status === "pending");
  const builtin = skill.is_builtin;

  return (
    <div className="p-6 space-y-5">
      {/* 头部 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate("/skills")}
            className="p-2 text-gray-400 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
            title="返回 Skill 管理"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-xl font-semibold text-gray-900">{skill.name}</h1>
              <span className="px-2 py-0.5 bg-gray-100 text-gray-600 text-xs font-medium rounded">
                {skill.skill_type}
              </span>
              {builtin && (
                <span className="px-2 py-0.5 bg-blue-50 text-blue-700 text-xs font-medium rounded">
                  内置
                </span>
              )}
              {(skill.version ?? 1) > 1 && (
                <span className="px-2 py-0.5 bg-green-50 text-green-700 text-xs font-medium rounded">
                  v{skill.version}
                </span>
              )}
              {ratchetBadge && (
                <span
                  className={`px-2 py-0.5 text-xs font-medium rounded ${ratchetBadge.cls}`}
                  title={
                    ratchetBadge.label === "below_peak"
                      ? "分数低于历史峰值，可回退"
                      : ratchetBadge.label === "oscillating"
                      ? "维度分数振荡，建议人工介入"
                      : "棘轮正常（分数只升不降）"
                  }
                >
                  {ratchetBadge.label}
                </span>
              )}
            </div>
            <p className="text-sm text-gray-500 mt-0.5">
              {skill.description || "Skill 进化视图：评分 · 建议 · 优化 · 版本"}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleAnalyze}
            disabled={analyzing}
            className="px-3 py-2 text-sm border rounded-lg text-gray-700 hover:bg-gray-50 disabled:opacity-50 flex items-center gap-1.5"
          >
            {analyzing ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Lightbulb className="w-4 h-4" />
            )}
            立即诊断
          </button>
          <button
            onClick={handleJudge}
            disabled={judging}
            className="px-3 py-2 text-sm border rounded-lg text-gray-700 hover:bg-gray-50 disabled:opacity-50 flex items-center gap-1.5"
          >
            {judging ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            立即体检
          </button>
          <button
            onClick={handleOptimize}
            disabled={optimizing || builtin}
            title={builtin ? "内置 Skill 不可优化，请先创建自定义版本" : "AI 优化模板"}
            className="btn-primary disabled:opacity-50 flex items-center gap-1.5"
          >
            {optimizing ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Sparkles className="w-4 h-4" />
            )}
            AI 优化
          </button>
        </div>
      </div>

      {/* 评分卡 */}
      <div className="card p-5 flex items-center gap-6">
        <ScoreRing total={latest?.total_score ?? 0} />
        <div className="flex-1">
          {latest ? (
            <>
              <div className="flex items-center gap-4 text-sm text-gray-700">
                <span>
                  结构分{" "}
                  <b className="text-gray-900">
                    {latest.structure_score}
                  </b>
                  /60
                </span>
                <span>
                  效果分{" "}
                  <b className="text-gray-900">
                    {latest.effect_score ?? "—"}
                  </b>
                  /40
                </span>
                <span className="text-gray-400">
                  v{latest.version} · {latest.judged_at.slice(0, 16).replace("T", " ")}
                </span>
              </div>
              {latest.judge_reason && (
                <p className="text-xs text-gray-500 mt-1.5">
                  Judge：{latest.judge_reason}
                </p>
              )}
              <div className="grid grid-cols-3 gap-x-4 gap-y-1 mt-3">
                {latestDims.map((d) => (
                  <div key={d.key} className="text-xs">
                    <div className="flex justify-between text-gray-500">
                      <span>{d.label}</span>
                      <span className="text-gray-800 font-medium">
                        {d.score}/{d.max}
                      </span>
                    </div>
                    <div className="h-1.5 bg-gray-100 rounded mt-0.5">
                      <div
                        className="h-1.5 rounded"
                        style={{
                          width: `${(d.score / d.max) * 100}%`,
                          background: scoreColor((d.score / d.max) * 100),
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="text-sm text-gray-500">
              暂无评分记录。点击「立即体检」计算结构分（效果分需执行产出与 API Key）。
            </div>
          )}
        </div>
      </div>

      {/* Tab 导航 */}
      <div className="flex gap-1 border-b">
        {(
          [
            ["overview", "概览", Lightbulb],
            ["scores", "评分历史", Activity],
            ["versions", "版本历史", GitBranch],
            ["logs", "执行日志", FileText],
          ] as [TabKey, string, typeof Lightbulb][]
        ).map(([key, label, Icon]) => (
          <button
            key={key}
            onClick={() => setTab(key)}
            className={`px-4 py-2 text-sm flex items-center gap-1.5 border-b-2 -mb-px transition-colors ${
              tab === key
                ? "border-blue-600 text-blue-600 font-medium"
                : "border-transparent text-gray-500 hover:text-gray-800"
            }`}
          >
            <Icon className="w-4 h-4" />
            {label}
            {key === "overview" && pendingSuggestions.length > 0 && (
              <span className="ml-1 px-1.5 py-0.5 text-[10px] bg-red-100 text-red-700 rounded-full">
                {pendingSuggestions.length}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* 概览 Tab：建议列表 */}
      {tab === "overview" && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium text-gray-700">优化建议</h3>
            <span className="text-xs text-gray-400">
              规则诊断 + 静态体检 + 棘轮/振荡
            </span>
          </div>
          {suggestions.length === 0 ? (
            <div className="card p-10 text-center">
              <CheckCircle2 className="w-8 h-8 text-green-400 mx-auto mb-2" />
              <p className="text-sm text-gray-500">
                暂无建议。点击「立即诊断」运行 L1 规则引擎
              </p>
            </div>
          ) : (
            suggestions.map((s) => {
              const Icon = SEVERITY_ICON[s.severity as keyof typeof SEVERITY_ICON] ?? Info;
              return (
                <div
                  key={s.id}
                  className="card p-4 flex items-start gap-3"
                  style={{ opacity: s.status === "pending" ? 1 : 0.55 }}
                >
                  <div
                    className={`w-7 h-7 rounded-full flex items-center justify-center shrink-0 ${
                      SEVERITY_CLASS[s.severity as keyof typeof SEVERITY_CLASS] ??
                      "text-gray-500 bg-gray-100"
                    }`}
                  >
                    <Icon className="w-4 h-4" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-[11px] font-mono text-gray-400">
                        {s.rule_type}
                      </span>
                      <span
                        className={`px-1.5 py-0.5 text-[10px] rounded ${
                          s.status === "pending"
                            ? "bg-amber-50 text-amber-700"
                            : s.status === "applied"
                            ? "bg-blue-50 text-blue-700"
                            : s.status === "verified"
                            ? "bg-green-50 text-green-700"
                            : "bg-gray-100 text-gray-500"
                        }`}
                      >
                        {s.status}
                      </span>
                      {s.applied_version != null && (
                        <span className="text-[10px] text-gray-400">
                          v{s.applied_version}
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-gray-700 mt-0.5">{s.message}</p>
                  </div>
                  {s.status === "pending" && !builtin && (
                    <button
                      onClick={() => handleOptimizeFromSuggestion(s)}
                      disabled={optimizing}
                      className="px-2.5 py-1.5 text-xs border rounded-lg text-blue-600 hover:bg-blue-50 disabled:opacity-50 flex items-center gap-1 shrink-0"
                    >
                      {optimizing ? (
                        <Loader2 className="w-3 h-3 animate-spin" />
                      ) : (
                        <Sparkles className="w-3 h-3" />
                      )}
                      AI 优化草稿
                    </button>
                  )}
                </div>
              );
            })
          )}
        </div>
      )}

      {/* 评分历史 Tab */}
      {tab === "scores" && (
        <div className="card p-5 space-y-4">
          {history.length === 0 ? (
            <div className="p-8 text-center text-sm text-gray-500">
              暂无评分历史，点击「立即体检」开始
            </div>
          ) : (
            <>
              <ScoreTrend history={history} />
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-left text-xs text-gray-400 border-b">
                    <th className="py-2">版本</th>
                    <th>结构分</th>
                    <th>效果分</th>
                    <th>总分</th>
                    <th>评语</th>
                    <th className="text-right">评分时间</th>
                  </tr>
                </thead>
                <tbody>
                  {[...history].reverse().map((s) => (
                    <tr key={s.id} className="border-b last:border-0">
                      <td className="py-2 font-medium">v{s.version}</td>
                      <td>{s.structure_score}/60</td>
                      <td>{s.effect_score ?? "—"}/40</td>
                      <td>
                        <b style={{ color: scoreColor(s.total_score) }}>
                          {s.total_score}
                        </b>
                      </td>
                      <td className="text-xs text-gray-500 max-w-[240px] truncate">
                        {s.judge_reason || "—"}
                      </td>
                      <td className="text-right text-xs text-gray-400">
                        {s.judged_at.slice(0, 16).replace("T", " ")}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </div>
      )}

      {/* 版本历史 Tab */}
      {tab === "versions" && (
        <div className="card divide-y">
          {versions.length === 0 ? (
            <div className="p-8 text-center text-sm text-gray-500">
              暂无版本快照（历史版本在创建/编辑时自动记录）
            </div>
          ) : (
            [...versions].reverse().map((v) => (
              <div key={v.id} className="p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <GitBranch className="w-4 h-4 text-gray-400" />
                    <span className="text-sm font-medium">v{v.version}</span>
                    {v.version === (skill.version ?? 1) && (
                      <span className="px-1.5 py-0.5 text-[10px] bg-green-50 text-green-700 rounded">
                        当前
                      </span>
                    )}
                    {v.note && (
                      <span className="text-xs text-gray-400">{v.note}</span>
                    )}
                    <span className="text-xs text-gray-300">
                      {v.created_at.slice(0, 16).replace("T", " ")}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() =>
                        setExpandedVersion(expandedVersion === v.version ? null : v.version)
                      }
                      className="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded"
                      title="查看模板"
                    >
                      {expandedVersion === v.version ? (
                        <ChevronUp className="w-4 h-4" />
                      ) : (
                        <ChevronDown className="w-4 h-4" />
                      )}
                    </button>
                    {!builtin && v.version !== (skill.version ?? 1) && (
                      <button
                        onClick={() => handleRevert(v.version)}
                        className="px-2.5 py-1.5 text-xs border rounded-lg text-amber-600 hover:bg-amber-50 flex items-center gap-1"
                      >
                        <RotateCcw className="w-3 h-3" />
                        回退到此版本
                      </button>
                    )}
                  </div>
                </div>
                {expandedVersion === v.version && (
                  <pre className="mt-3 text-xs bg-gray-50 rounded-lg p-3 max-h-64 overflow-auto whitespace-pre-wrap font-mono border">
                    {v.template}
                  </pre>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {/* 执行日志 Tab */}
      {tab === "logs" && (
        <div className="card divide-y">
          {logs.length === 0 ? (
            <div className="p-8 text-center text-sm text-gray-500">
              暂无执行记录
            </div>
          ) : (
            logs.map((log) => (
              <div key={log.id} className="p-4">
                <div
                  className="flex items-center justify-between cursor-pointer"
                  onClick={() =>
                    setExpandedLog(expandedLog === log.id ? null : log.id)
                  }
                >
                  <div className="flex items-center gap-3 text-sm">
                    {log.status === "success" ? (
                      <CheckCircle2 className="w-4 h-4 text-green-500" />
                    ) : (
                      <AlertTriangle className="w-4 h-4 text-red-500" />
                    )}
                    <span className="font-medium">{log.execution_phase}</span>
                    <span className="text-gray-400 text-xs">{log.status}</span>
                    {log.duration_ms != null && (
                      <span className="text-gray-400 text-xs flex items-center gap-1">
                        <Clock className="w-3 h-3" />
                        {(log.duration_ms / 1000).toFixed(1)}s
                      </span>
                    )}
                    <span className="text-gray-400 text-xs flex items-center gap-1">
                      <Zap className="w-3 h-3" />
                      {log.token_usage} tokens
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-gray-400">
                    {log.executed_at.slice(0, 16).replace("T", " ")}
                    {expandedLog === log.id ? (
                      <ChevronUp className="w-4 h-4" />
                    ) : (
                      <ChevronDown className="w-4 h-4" />
                    )}
                  </div>
                </div>
                {expandedLog === log.id && (
                  <div className="mt-2 text-xs bg-gray-50 rounded-lg p-3 space-y-2">
                    {log.error_message && (
                      <div className="text-red-600">
                        <b>错误：</b>
                        {log.error_message}
                      </div>
                    )}
                    {log.output_summary && (
                      <div className="text-gray-600">
                        <b>输出：</b>
                        {log.output_summary}
                      </div>
                    )}
                    {log.input_params && (
                      <div className="text-gray-500 font-mono break-all">
                        <b>输入：</b>
                        {log.input_params}
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {/* AI 优化 diff 弹窗（人在回路） */}
      {draft && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-6">
          <div className="bg-white rounded-xl w-full max-w-5xl max-h-[85vh] overflow-hidden flex flex-col">
            <div className="flex items-center justify-between px-6 py-4 border-b">
              <h2 className="text-lg font-semibold flex items-center gap-2">
                <Sparkles className="w-5 h-5 text-blue-600" />
                AI 优化预览
              </h2>
              <button
                onClick={() => setDraft(null)}
                className="p-1 hover:bg-gray-100 rounded"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <div className="p-6 space-y-4 overflow-y-auto">
              <div className="flex items-start gap-2 text-sm bg-blue-50 text-blue-700 rounded-lg p-3">
                <Sparkles className="w-4 h-4 mt-0.5 shrink-0" />
                <div>
                  <b>改动要点：</b>
                  {draft.change_summary || "（无）"}
                  {draft.target_dimensions?.length > 0 && (
                    <div className="mt-1 flex gap-1.5 flex-wrap">
                      {draft.target_dimensions.map((d) => (
                        <span
                          key={d}
                          className="px-1.5 py-0.5 text-[10px] bg-blue-100 rounded"
                        >
                          {d}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
              <TemplateDiff oldText={skill.template} newText={draft.new_template} />
            </div>
            <div className="flex justify-end gap-3 px-6 py-4 border-t bg-gray-50">
              <button
                onClick={() => setDraft(null)}
                className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg"
              >
                取消
              </button>
              <button
                onClick={handleApplyDraft}
                disabled={applying || builtin}
                className="btn-primary disabled:opacity-50 flex items-center gap-1.5"
              >
                {applying ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <CheckCircle2 className="w-4 h-4" />
                )}
                应用为新版本（v{(skill.version ?? 1) + 1}）
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
