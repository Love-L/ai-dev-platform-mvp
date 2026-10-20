import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { motion } from "framer-motion";
import { FileText, Columns, ArrowRight, Coins, CheckCircle, TrendingUp } from "lucide-react";
import { discoverAgents, getStatistics } from "../lib/tauri";
import type { Agent, StatisticsData } from "../lib/types";
import { useRequirementsStore, useTasksStore } from "../stores";
import AnimatedNumber from "../components/AnimatedNumber";
import { StatCardSkeleton, CardSkeleton } from "../components/Skeleton";
import { StatusBadge, TaskTypeBadge } from "../components/Badges";

export default function Dashboard() {
  const { requirements, loading: reqLoading, fetchRequirements } = useRequirementsStore();
  const { tasks, loading: taskLoading, fetchTasks } = useTasksStore();
  const [agents, setAgents] = useState<Agent[]>([]);
  const [stats, setStats] = useState<StatisticsData | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchRequirements();
    fetchTasks();
    loadData();
  }, []);

  async function loadData() {
    try {
      const [agts, statsData] = await Promise.all([discoverAgents(), getStatistics()]);
      setAgents(agts);
      setStats(statsData);
    } catch (error) {
      console.error("加载数据失败:", error);
    } finally {
      setLoading(false);
    }
  }

  const loadingState = reqLoading || taskLoading || loading;

  if (loadingState) {
    return (
      <div className="p-6 space-y-6">
        <div className="space-y-2">
          <div className="h-7 rounded w-32 animate-pulse" style={{ background: "var(--color-bg-subtle)" }} />
          <div className="h-4 rounded w-48 animate-pulse" style={{ background: "var(--color-bg-subtle)" }} />
        </div>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <StatCardSkeleton key={i} />
          ))}
        </div>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <CardSkeleton key={i} />
          ))}
        </div>
      </div>
    );
  }

  const statCards = [
    {
      icon: FileText,
      label: "需求总数",
      value: stats?.requirements.total || 0,
      color: "var(--color-primary)",
      bg: "var(--color-primary-subtle)",
    },
    {
      icon: Columns,
      label: "任务总数",
      value: stats?.tasks.total || 0,
      color: "var(--color-success)",
      bg: "var(--color-success-light)",
    },
    {
      icon: CheckCircle,
      label: "完成率",
      value: `${stats?.tasks.total ? Math.round((stats.tasks.completed / stats.tasks.total) * 100) : 0}%`,
      color: "#7c3aed",
      bg: "rgba(124, 58, 237, 0.08)",
    },
    {
      icon: Coins,
      label: "Token 消耗",
      value: (stats?.tokens.total || 0).toLocaleString(),
      color: "var(--color-warning)",
      bg: "var(--color-warning-light)",
    },
  ];

  return (
    <div className="p-6 space-y-6">
      {/* 页面标题 */}
      <motion.div initial={{ opacity: 0, y: -6 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2 }}>
        <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
          仪表盘
        </h1>
        <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
          AI研发协作平台概览
        </p>
      </motion.div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        {statCards.map((card, i) => (
          <motion.div
            key={card.label}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.2, delay: i * 0.04 }}
            className="card p-4"
          >
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg" style={{ background: card.bg }}>
                <card.icon className="w-5 h-5" style={{ color: card.color }} strokeWidth={1.75} />
              </div>
              <div>
                <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                  {card.label}
                </p>
                <p className="text-lg font-semibold" style={{ color: "var(--color-text)" }}>
                  {typeof card.value === "number" ? <AnimatedNumber value={card.value} duration={1.5} /> : card.value}
                </p>
              </div>
            </div>
          </motion.div>
        ))}
      </div>

      {/* 快捷入口 */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* 最近需求 */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2, delay: 0.16 }}
          className="panel"
        >
          <div className="panel-header">
            <h2 className="panel-title">最近需求</h2>
            <Link
              to="/requirements"
              className="text-xs flex items-center gap-1"
              style={{ color: "var(--color-primary)" }}
            >
              查看全部
              <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          </div>
          {requirements.length === 0 ? (
            <div className="empty-state py-8">
              <FileText className="empty-state-icon" />
              <p className="empty-state-desc">暂无需求</p>
            </div>
          ) : (
            <div className="space-y-0.5">
              {requirements.slice(0, 5).map((req) => (
                <Link
                  key={req.id}
                  to={`/requirements/${req.id}`}
                  className="flex items-center justify-between p-2.5 rounded-lg transition-colors group"
                  style={{ transition: "background var(--transition-fast)" }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-primary-subtle)")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "")}
                >
                  <div className="min-w-0 flex-1">
                    <p
                      className="text-sm font-medium truncate transition-colors"
                      style={{ color: "var(--color-text)" }}
                    >
                      {req.title}
                    </p>
                    <p className="text-xs mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                      优先级 {req.priority}
                    </p>
                  </div>
                  <StatusBadge status={req.status} />
                </Link>
              ))}
            </div>
          )}
        </motion.div>

        {/* 进行中的任务 */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2, delay: 0.2 }}
          className="panel"
        >
          <div className="panel-header">
            <h2 className="panel-title">进行中的任务</h2>
            <Link to="/board" className="text-xs flex items-center gap-1" style={{ color: "var(--color-primary)" }}>
              查看看板
              <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          </div>
          {tasks.filter((t) => t.status === "in_progress").length === 0 ? (
            <div className="empty-state py-8">
              <Columns className="empty-state-icon" />
              <p className="empty-state-desc">暂无进行中的任务</p>
            </div>
          ) : (
            <div className="space-y-0.5">
              {tasks
                .filter((t) => t.status === "in_progress")
                .slice(0, 5)
                .map((task) => (
                  <div
                    key={task.id}
                    className="flex items-center justify-between p-2.5 rounded-lg transition-colors"
                    style={{ transition: "background var(--transition-fast)" }}
                  >
                    <div className="min-w-0 flex-1">
                      <p className="text-sm font-medium truncate" style={{ color: "var(--color-text)" }}>
                        {task.title}
                      </p>
                      <p className="text-xs mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                        {task.agent_type || "未分配"}
                      </p>
                    </div>
                    <TaskTypeBadge type={task.task_type} />
                  </div>
                ))}
            </div>
          )}
        </motion.div>

        {/* Agent 状态 */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2, delay: 0.24 }}
          className="panel"
        >
          <div className="panel-header">
            <h2 className="panel-title">Agent 状态</h2>
            <Link to="/agents" className="text-xs flex items-center gap-1" style={{ color: "var(--color-primary)" }}>
              管理
              <ArrowRight className="w-3.5 h-3.5" />
            </Link>
          </div>
          {agents.length === 0 ? (
            <div className="empty-state py-8">
              <TrendingUp className="empty-state-icon" />
              <p className="empty-state-desc">未检测到 Agent</p>
            </div>
          ) : (
            <div className="space-y-0.5">
              {agents.map((agent) => (
                <div
                  key={agent.id}
                  className="flex items-center justify-between p-2.5 rounded-lg transition-colors"
                  style={{ transition: "background var(--transition-fast)" }}
                >
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                      {agent.display_name}
                    </p>
                    <p className="text-xs mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
                      {agent.version || "版本未知"}
                    </p>
                  </div>
                  <StatusBadge status={agent.status} />
                </div>
              ))}
            </div>
          )}
        </motion.div>
      </div>
    </div>
  );
}
