import { useEffect, useState, type ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { format, formatDistanceToNow } from "date-fns";
import { zhCN } from "date-fns/locale";
import {
  FileText,
  Columns,
  CheckCircle,
  AlertCircle,
  ArrowRight,
  Bot,
  Coins,
  Plus,
  MessageSquare,
  GitBranch,
  FolderOpen,
  Rocket,
  Activity,
  Timer,
  TrendingUp,
  Zap,
} from "lucide-react";
import { useRequirementsStore, useTasksStore, useSessionsStore } from "../stores";
import { discoverAgents, getStatistics, getTokenBudget, listGitCommits, listProjects } from "../lib/tauri";
import type { GitCommitRecord } from "../lib/tauri";
import type { Agent, Project, StatisticsData, TokenBudget } from "../lib/types";
import { PriorityBadge, StatusBadge } from "../components/Badges";
import AnimatedNumber from "../components/AnimatedNumber";

/** 渐变色板（bento 卡片图标） */
const GRADIENT = {
  primary: "linear-gradient(135deg, #2563eb, #7c3aed)",
  violet: "linear-gradient(135deg, #8b5cf6, #d946ef)",
  green: "linear-gradient(135deg, #10b981, #06b6d4)",
  amber: "linear-gradient(135deg, #f59e0b, #f97316)",
  red: "linear-gradient(135deg, #ef4444, #ec4899)",
} as const;

/** 任务状态配色 */
const TASK_COLORS: Record<string, string> = {
  pending: "#f59e0b",
  in_progress: "#2563eb",
  completed: "#10b981",
  blocked: "#ef4444",
};

/** 需求状态配色与中文名 */
const REQ_STATUS_META: { key: string; label: string; color: string }[] = [
  { key: "draft", label: "草稿", color: "#94a3b8" },
  { key: "reviewing", label: "评审中", color: "#f59e0b" },
  { key: "approved", label: "已批准", color: "#10b981" },
];

/** 会话运行时长（started_at → now） */
function formatElapsed(startedAt: string): string {
  const ms = Date.now() - new Date(startedAt).getTime();
  if (!Number.isFinite(ms) || ms < 0) return "-";
  const mins = Math.floor(ms / 60000);
  if (mins < 1) return "<1 分钟";
  if (mins < 60) return `${mins} 分钟`;
  return `${Math.floor(mins / 60)} 小时 ${mins % 60} 分`;
}

/** Token 紧凑格式 */
function compactTokens(n?: number): string {
  if (!n) return "0";
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

interface BentoCardProps {
  children: ReactNode;
  className?: string;
  delay?: number;
  onClick?: () => void;
  title?: string;
}

/** bento 卡片：入场动画 + 悬浮微抬升 */
function BentoCard({ children, className = "", delay = 0, onClick, title }: BentoCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 14 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.28, delay, ease: "easeOut" }}
      className={`card p-4 flex flex-col ${className}`}
      title={title}
      onClick={onClick}
      style={{
        cursor: onClick ? "pointer" : "default",
        transition: "transform 0.18s ease, box-shadow 0.18s ease",
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.transform = "translateY(-2px)";
        e.currentTarget.style.boxShadow = "0 10px 30px rgba(0, 0, 0, 0.07)";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.transform = "";
        e.currentTarget.style.boxShadow = "";
      }}
    >
      {children}
    </motion.div>
  );
}

/** 渐变图标 chip */
function IconChip({ icon: Icon, gradient }: { icon: typeof FileText; gradient: string }) {
  return (
    <span
      className="inline-flex items-center justify-center w-8 h-8 rounded-xl flex-shrink-0"
      style={{ background: gradient }}
    >
      <Icon className="w-4 h-4" style={{ color: "#fff" }} strokeWidth={2} />
    </span>
  );
}

/** 卡片标题行 */
function CardHead({
  icon,
  gradient,
  title,
  extra,
}: {
  icon: typeof FileText;
  gradient: string;
  title: string;
  extra?: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between mb-3">
      <div className="flex items-center gap-2.5">
        <IconChip icon={icon} gradient={gradient} />
        <h2 className="text-sm font-semibold" style={{ color: "var(--color-text)" }}>
          {title}
        </h2>
      </div>
      {extra}
    </div>
  );
}

export default function Workspace() {
  const navigate = useNavigate();
  const { requirements, fetchRequirements } = useRequirementsStore();
  const { tasks, fetchTasks } = useTasksStore();
  const { sessions, fetchSessions, ensureStatusSubscription } = useSessionsStore();
  const [projects, setProjects] = useState<Project[]>([]);
  const [statistics, setStatistics] = useState<StatisticsData | null>(null);
  const [budget, setBudget] = useState<TokenBudget | null>(null);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [commits, setCommits] = useState<GitCommitRecord[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadData();
    // 会话状态变化自动刷新工作台态势（spawn/退出/终止均触发）
    let release: (() => void) | undefined;
    let disposed = false;
    ensureStatusSubscription().then((fn) => {
      if (disposed) fn();
      else release = fn;
    });
    return () => {
      disposed = true;
      release?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function loadData() {
    try {
      await Promise.all([
        fetchRequirements(),
        fetchTasks(),
        fetchSessions(),
        listProjects().then(setProjects),
        getStatistics().then(setStatistics),
        getTokenBudget()
          .then(setBudget)
          .catch(() => null),
        discoverAgents()
          .then(setAgents)
          .catch(() => null),
        listGitCommits(undefined, undefined, 4)
          .then(setCommits)
          .catch(() => null),
      ]);
    } catch (error) {
      console.error("加载工作台数据失败:", error);
    } finally {
      setLoading(false);
    }
  }

  const stats = {
    totalRequirements: requirements.length,
    draftRequirements: requirements.filter((r) => r.status === "draft").length,
    reviewingRequirements: requirements.filter((r) => r.status === "reviewing").length,
    approvedRequirements: requirements.filter((r) => r.status === "approved").length,
    totalTasks: tasks.length,
    pendingTasks: tasks.filter((t) => t.status === "pending").length,
    inProgressTasks: tasks.filter((t) => t.status === "in_progress").length,
    completedTasks: tasks.filter((t) => t.status === "completed").length,
    blockedTasks: tasks.filter((t) => t.status === "blocked").length,
  };

  const completionRate = stats.totalTasks > 0 ? stats.completedTasks / stats.totalTasks : 0;
  const completionPct = Math.round(completionRate * 100);

  /** 运行中的会话（session:status 事件驱动，自动刷新） */
  const runningSessions = sessions.filter((s) => s.status === "running");

  /** 近 7 天 Token 趋势（后端补零正序）与本月预算 */
  const trend = statistics?.tokens.by_day ?? [];
  const trendMax = Math.max(1, ...trend.map((d) => d.tokens));
  const budgetRatio = budget && budget.monthly_limit > 0 ? budget.current_usage / budget.monthly_limit : null;
  const budgetWarned = budgetRatio !== null && budgetRatio >= (budget?.alert_threshold ?? 0.8);

  const blockedList = tasks.filter((t) => t.status === "blocked").slice(0, 3);

  const taskSegments = [
    { key: "pending", label: "待处理", count: stats.pendingTasks, color: TASK_COLORS.pending },
    { key: "in_progress", label: "进行中", count: stats.inProgressTasks, color: TASK_COLORS.in_progress },
    { key: "completed", label: "已完成", count: stats.completedTasks, color: TASK_COLORS.completed },
    { key: "blocked", label: "阻塞", count: stats.blockedTasks, color: TASK_COLORS.blocked },
  ];

  const otherRequirements =
    stats.totalRequirements - stats.draftRequirements - stats.reviewingRequirements - stats.approvedRequirements;
  const reqSegments = [
    { label: "草稿", count: stats.draftRequirements, color: REQ_STATUS_META[0].color },
    { label: "评审中", count: stats.reviewingRequirements, color: REQ_STATUS_META[1].color },
    { label: "已批准", count: stats.approvedRequirements, color: REQ_STATUS_META[2].color },
    { label: "其它", count: otherRequirements, color: "#8b5cf6" },
  ];

  const recentActivity = [
    ...requirements.slice(0, 5).map((r) => ({
      id: r.id,
      type: "requirement" as const,
      title: r.title,
      status: r.status,
      priority: r.priority,
      created_at: r.created_at,
      link: `/requirements/${r.id}`,
    })),
    ...tasks.slice(0, 5).map((t) => ({
      id: t.id,
      type: "task" as const,
      title: t.title,
      status: t.status,
      priority: t.priority,
      created_at: t.created_at,
      link: `/tasks/${t.id}`,
    })),
  ]
    .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
    .slice(0, 8);

  const quickActions = [
    { label: "新建需求", icon: Plus, nav: "/requirements", primary: true },
    { label: "任务看板", icon: Columns, nav: "/board" },
    { label: "Agent 会话", icon: MessageSquare, nav: "/sessions" },
    { label: "代码提交", icon: GitBranch, nav: "/commits" },
  ];

  const hour = new Date().getHours();
  const greeting = hour < 6 ? "夜深了" : hour < 12 ? "早上好" : hour < 14 ? "中午好" : hour < 18 ? "下午好" : "晚上好";

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
      {/* Hero：问候 + 日期 + 快捷动作 */}
      <motion.div
        initial={{ opacity: 0, y: -8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.25 }}
        className="flex flex-wrap items-center justify-between gap-4"
      >
        <div>
          <h1
            className="text-2xl font-bold"
            style={{
              background: "linear-gradient(90deg, var(--color-text) 30%, var(--color-primary))",
              WebkitBackgroundClip: "text",
              WebkitTextFillColor: "transparent",
            }}
          >
            {greeting}，欢迎回来
          </h1>
          <p className="text-sm mt-1" style={{ color: "var(--color-text-tertiary)" }}>
            {format(new Date(), "yyyy年M月d日 EEEE", { locale: zhCN })} · 项目概览与快速操作
          </p>
        </div>
        <div className="flex items-center gap-2">
          {quickActions.map((action) => (
            <button
              key={action.label}
              onClick={() => navigate(action.nav)}
              className={`text-xs px-3 py-2 rounded-xl flex items-center gap-1.5 transition-all ${
                action.primary ? "btn-primary" : "btn-secondary"
              }`}
            >
              <action.icon className="w-3.5 h-3.5" />
              {action.label}
            </button>
          ))}
        </div>
      </motion.div>

      {/* 运行中的会话（实时态势，session:status 驱动自动刷新） */}
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.26, delay: 0.04, ease: "easeOut" }}
        className="panel"
      >
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2.5">
            <IconChip icon={Zap} gradient={GRADIENT.green} />
            <h2 className="text-sm font-semibold" style={{ color: "var(--color-text)" }}>
              运行中的会话
            </h2>
            {runningSessions.length > 0 && (
              <span
                className="text-[10px] px-1.5 py-0.5 rounded-full font-medium"
                style={{ background: "var(--color-success-light)", color: "var(--color-success)" }}
              >
                {runningSessions.length} 个 Agent 在工作
              </span>
            )}
          </div>
          <button
            onClick={() => navigate("/sessions")}
            className="text-xs flex items-center gap-1"
            style={{ color: "var(--color-primary)" }}
          >
            全部会话 <ArrowRight className="w-3.5 h-3.5" />
          </button>
        </div>
        {runningSessions.length === 0 ? (
          <div className="flex items-center justify-between py-3">
            <div className="flex items-center gap-2.5">
              <span className="w-2 h-2 rounded-full" style={{ background: "var(--color-border)" }} aria-hidden />
              <div>
                <p className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                  暂无运行中的会话
                </p>
                <p className="text-xs mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                  启动 Agent 后，执行状态会实时显示在这里
                </p>
              </div>
            </div>
            <button onClick={() => navigate("/agents")} className="btn-secondary text-xs">
              去启动 Agent
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
            {runningSessions.slice(0, 4).map((s) => (
              <button
                key={s.id}
                onClick={() => navigate(`/sessions/${s.id}`)}
                className="text-left rounded-xl px-3.5 py-3"
                style={{
                  background: "var(--color-primary-subtle)",
                  border: "1px solid var(--color-primary)",
                  transition: "transform 0.15s ease",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.transform = "translateY(-2px)")}
                onMouseLeave={(e) => (e.currentTarget.style.transform = "")}
              >
                <div className="flex items-center gap-2">
                  <span
                    className="w-2 h-2 rounded-full animate-pulse shrink-0"
                    style={{ background: "var(--color-success)" }}
                    aria-hidden
                  />
                  <p className="text-sm font-medium truncate" style={{ color: "var(--color-text)" }}>
                    {s.title || `会话 ${s.id.slice(0, 8)}`}
                  </p>
                </div>
                <div
                  className="flex items-center gap-3 mt-2 text-[11px]"
                  style={{ color: "var(--color-text-tertiary)" }}
                >
                  <span className="flex items-center gap-1 truncate">
                    <Bot className="w-3 h-3 shrink-0" />
                    {s.agent_name || "Agent"}
                  </span>
                  <span className="flex items-center gap-1">
                    <Timer className="w-3 h-3" />
                    {s.started_at ? formatElapsed(s.started_at) : "-"}
                  </span>
                  <span className="ml-auto shrink-0">{compactTokens(s.total_tokens)} tok</span>
                </div>
              </button>
            ))}
          </div>
        )}
      </motion.div>

      {/* Bento 网格 */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* A：进度总览（环形完成率 + 核心指标） */}
        <BentoCard className="md:col-span-2" delay={0.04} onClick={() => navigate("/board")} title="查看任务看板">
          <CardHead
            icon={Activity}
            gradient={GRADIENT.primary}
            title="进度总览"
            extra={
              <span
                className="text-[11px] px-2 py-0.5 rounded-full"
                style={{ background: "var(--color-primary-subtle)", color: "var(--color-primary)" }}
              >
                实时
              </span>
            }
          />
          <div className="flex flex-col sm:flex-row items-center gap-6 pt-1">
            {/* 环形完成率 */}
            <div className="relative flex-shrink-0" style={{ width: 132, height: 132 }}>
              <svg width="132" height="132" viewBox="0 0 132 132">
                <defs>
                  <linearGradient id="ws-grad" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stopColor="#2563eb" />
                    <stop offset="100%" stopColor="#8b5cf6" />
                  </linearGradient>
                </defs>
                <circle cx="66" cy="66" r="54" fill="none" stroke="var(--color-border-subtle)" strokeWidth="11" />
                <motion.circle
                  cx="66"
                  cy="66"
                  r="54"
                  fill="none"
                  stroke="url(#ws-grad)"
                  strokeWidth="11"
                  strokeLinecap="round"
                  strokeDasharray={2 * Math.PI * 54}
                  initial={{ strokeDashoffset: 2 * Math.PI * 54 }}
                  animate={{ strokeDashoffset: 2 * Math.PI * 54 * (1 - completionRate) }}
                  transition={{ duration: 1.1, ease: "easeOut" }}
                  transform="rotate(-90 66 66)"
                />
              </svg>
              <div className="absolute inset-0 flex flex-col items-center justify-center">
                <span className="text-2xl font-bold" style={{ color: "var(--color-text)" }}>
                  <AnimatedNumber value={completionPct} suffix="%" duration={1.2} />
                </span>
                <span className="text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
                  任务完成率
                </span>
              </div>
            </div>

            {/* 核心指标 */}
            <div className="grid grid-cols-2 gap-3 flex-1 w-full">
              {[
                {
                  label: "需求总数",
                  value: stats.totalRequirements,
                  hint: `${stats.draftRequirements} 草稿 · ${stats.reviewingRequirements} 评审中`,
                  color: "var(--color-text)",
                },
                {
                  label: "任务总数",
                  value: stats.totalTasks,
                  hint: `${stats.pendingTasks} 待处理`,
                  color: "var(--color-text)",
                },
                { label: "进行中", value: stats.inProgressTasks, hint: "正在推进", color: "#2563eb" },
                {
                  label: "已完成",
                  value: stats.completedTasks,
                  hint: `已批准需求 ${stats.approvedRequirements}`,
                  color: "#10b981",
                },
              ].map((m) => (
                <div key={m.label} className="rounded-xl px-3.5 py-3" style={{ background: "var(--color-bg-subtle)" }}>
                  <p className="text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
                    {m.label}
                  </p>
                  <p className="text-xl font-bold mt-0.5" style={{ color: m.color }}>
                    <AnimatedNumber value={m.value} duration={1} />
                  </p>
                  <p className="text-[10px] mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                    {m.hint}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </BentoCard>

        {/* B：任务分布 */}
        <BentoCard delay={0.08} onClick={() => navigate("/board")} title="查看任务看板">
          <CardHead icon={Columns} gradient={GRADIENT.violet} title="任务分布" />
          <div
            className="flex h-2.5 rounded-full overflow-hidden mb-4"
            style={{ background: "var(--color-bg-subtle)" }}
          >
            {taskSegments.map((seg) =>
              seg.count > 0 ? (
                <div
                  key={seg.key}
                  style={{
                    width: `${(seg.count / Math.max(stats.totalTasks, 1)) * 100}%`,
                    background: seg.color,
                    transition: "width 0.6s ease",
                  }}
                />
              ) : null,
            )}
            {stats.totalTasks === 0 && <div className="w-full" style={{ background: "var(--color-bg-subtle)" }} />}
          </div>
          <div className="space-y-2.5">
            {taskSegments.map((seg) => (
              <div key={seg.key} className="flex items-center justify-between text-xs">
                <span className="flex items-center gap-2" style={{ color: "var(--color-text-secondary)" }}>
                  <span className="w-2 h-2 rounded-full" style={{ background: seg.color }} />
                  {seg.label}
                </span>
                <span className="font-medium" style={{ color: "var(--color-text)" }}>
                  {seg.count}
                  <span className="ml-1 text-[10px]" style={{ color: "var(--color-text-tertiary)" }}>
                    {stats.totalTasks > 0 ? Math.round((seg.count / stats.totalTasks) * 100) : 0}%
                  </span>
                </span>
              </div>
            ))}
          </div>
        </BentoCard>

        {/* C：需求状态 */}
        <BentoCard delay={0.12} onClick={() => navigate("/requirements")} title="查看全部需求">
          <CardHead icon={FileText} gradient={GRADIENT.green} title="需求状态" />
          <div className="space-y-3">
            {reqSegments.map((seg) => (
              <div key={seg.label}>
                <div className="flex items-center justify-between text-xs mb-1">
                  <span style={{ color: "var(--color-text-secondary)" }}>{seg.label}</span>
                  <span className="font-medium" style={{ color: "var(--color-text)" }}>
                    {seg.count}
                  </span>
                </div>
                <div className="h-1.5 rounded-full overflow-hidden" style={{ background: "var(--color-bg-subtle)" }}>
                  <div
                    style={{
                      width: `${stats.totalRequirements > 0 ? (seg.count / stats.totalRequirements) * 100 : 0}%`,
                      height: "100%",
                      background: seg.color,
                      borderRadius: "9999px",
                      transition: "width 0.6s ease",
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
          <div className="mt-auto pt-3 text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
            共 {stats.totalRequirements} 条需求
          </div>
        </BentoCard>

        {/* D：阻塞提醒 */}
        <BentoCard delay={0.16} onClick={() => navigate("/board")} title="查看任务看板">
          <CardHead
            icon={stats.blockedTasks > 0 ? AlertCircle : CheckCircle}
            gradient={stats.blockedTasks > 0 ? GRADIENT.red : GRADIENT.green}
            title="阻塞提醒"
          />
          {stats.blockedTasks > 0 ? (
            <>
              <div className="flex items-baseline gap-2">
                <span className="text-3xl font-bold" style={{ color: "#ef4444" }}>
                  <AnimatedNumber value={stats.blockedTasks} duration={0.8} />
                </span>
                <span className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                  个任务被阻塞
                </span>
              </div>
              <div className="mt-3 space-y-1.5">
                {blockedList.map((t) => (
                  <p key={t.id} className="text-xs truncate" style={{ color: "var(--color-text-secondary)" }}>
                    · {t.title}
                  </p>
                ))}
              </div>
              <p className="mt-auto pt-3 text-[11px] flex items-center gap-1" style={{ color: "#ef4444" }}>
                需要关注 <ArrowRight className="w-3 h-3" />
              </p>
            </>
          ) : (
            <div className="flex flex-col items-center justify-center py-4 gap-2">
              <CheckCircle className="w-8 h-8" style={{ color: "#10b981" }} strokeWidth={1.5} />
              <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                一切顺利，无阻塞任务
              </p>
            </div>
          )}
        </BentoCard>

        {/* T：Token 消耗与预算（含 7 天趋势） */}
        <BentoCard delay={0.2} onClick={() => navigate("/cost")} title="查看成本明细">
          <CardHead icon={Coins} gradient={GRADIENT.amber} title="Token 与预算" />
          <div className="space-y-2.5">
            <div>
              <p className="text-xl font-bold" style={{ color: "var(--color-text)" }}>
                {compactTokens(statistics?.tokens.total)}{" "}
                <span className="text-[11px] font-normal" style={{ color: "var(--color-text-tertiary)" }}>
                  累计 tokens
                </span>
              </p>
              <p className="text-[11px] mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                累计费用 ${(statistics?.tokens.cost || 0).toFixed(4)}
              </p>
            </div>

            {/* 本月预算进度 */}
            {budgetRatio !== null ? (
              <div>
                <div className="flex items-center justify-between text-[11px] mb-1">
                  <span style={{ color: "var(--color-text-secondary)" }}>本月预算</span>
                  <span style={{ color: budgetWarned ? "var(--color-danger)" : "var(--color-text-secondary)" }}>
                    {compactTokens(budget?.current_usage)} / {compactTokens(budget?.monthly_limit)}
                    {budgetWarned ? " · 接近上限" : ""}
                  </span>
                </div>
                <div className="h-1.5 rounded-full overflow-hidden" style={{ background: "var(--color-bg-subtle)" }}>
                  <div
                    style={{
                      width: `${Math.min(100, budgetRatio * 100)}%`,
                      height: "100%",
                      borderRadius: "9999px",
                      background: budgetWarned ? "var(--color-danger)" : "linear-gradient(90deg, #f59e0b, #f97316)",
                      transition: "width 0.6s ease",
                    }}
                  />
                </div>
              </div>
            ) : (
              <p className="text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
                未设置预算 · 在设置中开启月度额度
              </p>
            )}

            {/* 近 7 天迷你趋势 */}
            <div>
              <p
                className="text-[11px] mb-1.5 flex items-center gap-1"
                style={{ color: "var(--color-text-secondary)" }}
              >
                <TrendingUp className="w-3 h-3" /> 近 7 天用量
              </p>
              {trend.length === 0 || trend.every((d) => d.tokens === 0) ? (
                <p className="text-[11px] py-2" style={{ color: "var(--color-text-tertiary)" }}>
                  近 7 天暂无用量
                </p>
              ) : (
                <div className="flex items-end gap-1 h-10">
                  {trend.map((d, i) => {
                    const isToday = i === trend.length - 1;
                    const h = Math.max(6, Math.round((d.tokens / trendMax) * 100));
                    return (
                      <div
                        key={d.date}
                        title={`${d.date}: ${d.tokens.toLocaleString()} tokens · $${d.cost.toFixed(4)}`}
                        className="flex-1 rounded-t"
                        style={{
                          height: `${h}%`,
                          background: isToday ? "var(--color-primary)" : "var(--color-primary-subtle)",
                          border: isToday ? "none" : "1px solid var(--color-border-subtle)",
                          borderBottom: "none",
                          transition: "height 0.5s ease",
                        }}
                      />
                    );
                  })}
                </div>
              )}
            </div>
          </div>
          <div className="mt-auto pt-3 text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
            今日 {compactTokens(trend[trend.length - 1]?.tokens)} tokens
          </div>
        </BentoCard>

        {/* E：项目 */}
        <BentoCard delay={0.2} onClick={() => navigate("/projects")} title="查看全部项目">
          <CardHead
            icon={FolderOpen}
            gradient={GRADIENT.amber}
            title="项目"
            extra={<ArrowRight className="w-3.5 h-3.5" style={{ color: "var(--color-text-tertiary)" }} />}
          />
          {projects.length === 0 ? (
            <div className="py-4 text-center text-xs" style={{ color: "var(--color-text-tertiary)" }}>
              暂无项目
            </div>
          ) : (
            <div className="space-y-2">
              {projects.slice(0, 4).map((project) => (
                <div
                  key={project.id}
                  onClick={(e) => {
                    e.stopPropagation();
                    navigate(`/projects/${project.id}`);
                  }}
                  className="flex items-center gap-2.5 px-2 py-2 rounded-lg cursor-pointer"
                  style={{ background: "var(--color-bg-subtle)", transition: "background 0.15s ease" }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-primary-subtle)")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "var(--color-bg-subtle)")}
                >
                  <span
                    className="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-semibold flex-shrink-0"
                    style={{ background: GRADIENT.primary, color: "#fff" }}
                  >
                    {project.name.charAt(0)}
                  </span>
                  <span className="text-xs font-medium truncate flex-1" style={{ color: "var(--color-text)" }}>
                    {project.name}
                  </span>
                  <span
                    className="w-1.5 h-1.5 rounded-full"
                    style={{ background: (project.status || "active") === "active" ? "#10b981" : "#94a3b8" }}
                  />
                </div>
              ))}
            </div>
          )}
        </BentoCard>

        {/* G：Agent 状态 */}
        <BentoCard delay={0.24} onClick={() => navigate("/agents")} title="管理 Agent">
          <CardHead icon={Bot} gradient={GRADIENT.primary} title="Agent 状态" />
          {agents.length === 0 ? (
            <div className="py-4 text-center">
              <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                未检测到 Agent
              </p>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  navigate("/agents");
                }}
                className="btn-secondary text-xs mt-2"
              >
                去配置
              </button>
            </div>
          ) : (
            <div className="space-y-1.5">
              {agents.slice(0, 4).map((agent) => (
                <div
                  key={agent.id}
                  className="flex items-center justify-between px-2 py-1.5 rounded-lg"
                  style={{ background: "var(--color-bg-subtle)" }}
                >
                  <div className="min-w-0 flex-1">
                    <p className="text-xs font-medium truncate" style={{ color: "var(--color-text)" }}>
                      {agent.display_name}
                    </p>
                    <p className="text-[10px] truncate" style={{ color: "var(--color-text-tertiary)" }}>
                      {agent.version || "版本未知"}
                    </p>
                  </div>
                  <StatusBadge status={agent.status} />
                </div>
              ))}
            </div>
          )}
          <div className="mt-auto pt-2 text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
            共 {agents.length} 个 Agent
          </div>
        </BentoCard>

        {/* H：最近提交 */}
        <BentoCard delay={0.28} onClick={() => navigate("/commits")} title="查看提交记录">
          <CardHead icon={GitBranch} gradient={GRADIENT.violet} title="最近提交" />
          {commits.length === 0 ? (
            <div className="py-4 text-center">
              <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                暂无提交记录
              </p>
              <p className="text-[11px] mt-1" style={{ color: "var(--color-text-tertiary)" }}>
                Agent 完成任务提交代码后将显示在这里
              </p>
            </div>
          ) : (
            <div className="space-y-1.5">
              {commits.slice(0, 4).map((c) => (
                <div
                  key={c.short_hash}
                  className="flex items-center gap-2 px-2 py-1.5 rounded-lg"
                  style={{ background: "var(--color-bg-subtle)" }}
                >
                  <span
                    className="font-mono text-[10px] px-1 py-0.5 rounded shrink-0"
                    style={{ background: "var(--color-primary-subtle)", color: "var(--color-primary)" }}
                  >
                    {c.short_hash}
                  </span>
                  <p className="text-xs truncate flex-1" style={{ color: "var(--color-text)" }}>
                    {c.message || "(无提交信息)"}
                  </p>
                  <span className="text-[10px] shrink-0" style={{ color: "var(--color-text-tertiary)" }}>
                    {c.files_changed} 文件
                  </span>
                </div>
              ))}
            </div>
          )}
          <div className="mt-auto pt-2 text-[11px] truncate" style={{ color: "var(--color-text-tertiary)" }}>
            {commits[0]?.branch_name ? `分支 ${commits[0].branch_name}` : ""}
          </div>
        </BentoCard>

        {/* F：最近动态（时间线，整行） */}
        <BentoCard className="md:col-span-3" delay={0.24}>
          <CardHead
            icon={Rocket}
            gradient={GRADIENT.violet}
            title="最近动态"
            extra={
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  navigate("/board");
                }}
                className="text-xs flex items-center gap-1"
                style={{ color: "var(--color-primary)" }}
              >
                查看全部 <ArrowRight className="w-3 h-3" />
              </button>
            }
          />
          {recentActivity.length === 0 ? (
            <div className="py-5 text-center text-sm" style={{ color: "var(--color-text-tertiary)" }}>
              暂无活动记录，从「新建需求」开始吧
            </div>
          ) : (
            <div className="relative">
              {/* 时间线主轴 */}
              <div
                className="absolute left-[5px] top-2 bottom-2 w-px"
                style={{ background: "linear-gradient(var(--color-border), transparent)" }}
              />
              <div className="space-y-3">
                {recentActivity.map((item) => (
                  <div
                    key={item.id}
                    onClick={() => navigate(item.link)}
                    className="flex items-center gap-3 cursor-pointer group"
                  >
                    <span
                      className="w-[11px] h-[11px] rounded-full flex-shrink-0"
                      style={{
                        background:
                          item.type === "requirement"
                            ? "var(--color-primary)"
                            : "linear-gradient(135deg, #8b5cf6, #d946ef)",
                        boxShadow: "0 0 0 3px var(--color-primary-subtle)",
                      }}
                    />
                    <div className="flex-1 min-w-0 flex items-center justify-between gap-3">
                      <div className="min-w-0">
                        <p className="text-sm truncate group-hover:underline" style={{ color: "var(--color-text)" }}>
                          {item.title}
                        </p>
                        <p className="text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
                          {item.type === "requirement" ? "需求" : "任务"} ·{" "}
                          {formatDistanceToNow(new Date(item.created_at), {
                            addSuffix: true,
                            locale: zhCN,
                          })}
                        </p>
                      </div>
                      <div className="flex items-center gap-2 flex-shrink-0">
                        <PriorityBadge priority={item.priority} />
                        <StatusBadge status={item.status} />
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </BentoCard>
      </div>
    </div>
  );
}
