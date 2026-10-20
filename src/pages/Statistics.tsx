import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { BarChart3, CheckCircle, Clock, Coins } from "lucide-react";
import {
  getStatistics,
  listTasks,
  getSkillUsageStats,
  getMcpUsageStats,
  type SkillUsageStat,
  type McpUsageStat,
} from "../lib/tauri";
import type { StatisticsData, Task } from "../lib/types";

export default function Statistics() {
  const [stats, setStats] = useState<StatisticsData | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [skillStats, setSkillStats] = useState<SkillUsageStat[]>([]);
  const [mcpStats, setMcpStats] = useState<McpUsageStat[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    try {
      const [statsData, tasksData, skillData, mcpData] = await Promise.all([
        getStatistics(),
        listTasks(),
        getSkillUsageStats().catch(() => [] as SkillUsageStat[]),
        getMcpUsageStats().catch(() => [] as McpUsageStat[]),
      ]);
      setStats(statsData);
      setTasks(tasksData);
      setSkillStats(skillData);
      setMcpStats(mcpData);
    } catch (error) {
      console.error("加载数据失败:", error);
    } finally {
      setLoading(false);
    }
  }

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

  const completionRate = stats?.tasks.total ? Math.round((stats.tasks.completed / stats.tasks.total) * 100) : 0;

  const byPriority = {
    P0: tasks.filter((t) => t.priority === "P0").length,
    P1: tasks.filter((t) => t.priority === "P1").length,
    P2: tasks.filter((t) => t.priority === "P2").length,
    P3: tasks.filter((t) => t.priority === "P3").length,
  };

  const statCards = [
    {
      icon: BarChart3,
      label: "总任务数",
      value: stats?.tasks.total || 0,
      color: "var(--color-primary)",
      bg: "var(--color-primary-subtle)",
    },
    {
      icon: CheckCircle,
      label: "完成率",
      value: `${completionRate}%`,
      color: "var(--color-success)",
      bg: "var(--color-success-light)",
    },
    {
      icon: Clock,
      label: "进行中",
      value: stats?.tasks.in_progress || 0,
      color: "var(--color-warning)",
      bg: "var(--color-warning-light)",
    },
    {
      icon: Coins,
      label: "Token 消耗",
      value: (stats?.tokens.total || 0).toLocaleString(),
      color: "#7c3aed",
      bg: "rgba(124, 58, 237, 0.08)",
    },
  ];

  return (
    <div className="p-6 space-y-5">
      {/* 页面标题 */}
      <motion.div initial={{ opacity: 0, y: -6 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.2 }}>
        <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
          统计报表
        </h1>
        <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
          多维度任务执行效率统计
        </p>
      </motion.div>

      {/* 概览卡片 */}
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
                  {card.value}
                </p>
              </div>
            </div>
          </motion.div>
        ))}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
        {/* 任务状态分布 */}
        <div className="panel">
          <h2 className="panel-title mb-4">任务状态分布</h2>
          <div className="space-y-3">
            <ProgressRow
              label="已完成"
              count={stats?.tasks.completed || 0}
              total={stats?.tasks.total || 0}
              color="var(--color-success)"
            />
            <ProgressRow
              label="进行中"
              count={stats?.tasks.in_progress || 0}
              total={stats?.tasks.total || 0}
              color="var(--color-primary)"
            />
            <ProgressRow
              label="待处理"
              count={stats?.tasks.pending || 0}
              total={stats?.tasks.total || 0}
              color="var(--color-text-tertiary)"
            />
          </div>
        </div>

        {/* 任务类型分布 */}
        <div className="panel">
          <h2 className="panel-title mb-4">任务类型分布</h2>
          <div className="space-y-3">
            <ProgressRow
              label="开发任务"
              count={stats?.tasks.by_type.development || 0}
              total={stats?.tasks.total || 0}
              color="var(--color-primary)"
            />
            <ProgressRow
              label="测试任务"
              count={stats?.tasks.by_type.testing || 0}
              total={stats?.tasks.total || 0}
              color="#7c3aed"
            />
            <ProgressRow
              label="Bug修复"
              count={stats?.tasks.by_type.bug_fix || 0}
              total={stats?.tasks.total || 0}
              color="var(--color-danger)"
            />
          </div>
        </div>

        {/* 优先级分布 */}
        <div className="panel">
          <h2 className="panel-title mb-4">优先级分布</h2>
          <div className="space-y-3">
            <ProgressRow label="P0 紧急" count={byPriority.P0} total={tasks.length} color="var(--color-danger)" />
            <ProgressRow label="P1 高" count={byPriority.P1} total={tasks.length} color="var(--color-warning)" />
            <ProgressRow label="P2 中" count={byPriority.P2} total={tasks.length} color="var(--color-primary)" />
            <ProgressRow label="P3 低" count={byPriority.P3} total={tasks.length} color="var(--color-text-tertiary)" />
          </div>
        </div>

        {/* Token 统计 */}
        <div className="panel">
          <h2 className="panel-title mb-4">Token 消耗统计</h2>
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span style={{ color: "var(--color-text-tertiary)" }}>输入 Token</span>
              <span className="font-medium" style={{ color: "var(--color-text-secondary)" }}>
                {(stats?.tokens.input || 0).toLocaleString()}
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span style={{ color: "var(--color-text-tertiary)" }}>输出 Token</span>
              <span className="font-medium" style={{ color: "var(--color-text-secondary)" }}>
                {(stats?.tokens.output || 0).toLocaleString()}
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span style={{ color: "var(--color-text-tertiary)" }}>总消耗</span>
              <span className="font-medium" style={{ color: "var(--color-text-secondary)" }}>
                {(stats?.tokens.total || 0).toLocaleString()}
              </span>
            </div>
            <div
              className="flex justify-between text-sm pt-2"
              style={{ borderTop: "1px solid var(--color-border-subtle)" }}
            >
              <span style={{ color: "var(--color-text-tertiary)" }}>总费用</span>
              <span className="font-semibold" style={{ color: "var(--color-text)" }}>
                ${(stats?.tokens.cost || 0).toFixed(2)}
              </span>
            </div>
          </div>
        </div>

        {/* Skill 使用统计（F036） */}
        <div className="panel">
          <h2 className="panel-title mb-4">Skill 使用统计</h2>
          {skillStats.length === 0 ? (
            <p className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
              暂无 Skill 调用记录
            </p>
          ) : (
            <div className="space-y-3">
              {skillStats.map((item) => (
                <div key={item.skill_name} className="flex items-center justify-between text-sm">
                  <div className="min-w-0">
                    <p className="truncate" style={{ color: "var(--color-text-secondary)" }}>
                      {item.skill_name}
                    </p>
                    <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                      {item.skill_type || "未分类"} · {item.total_tokens.toLocaleString()} tokens
                    </p>
                  </div>
                  <div className="text-right shrink-0 ml-3">
                    <p className="font-medium" style={{ color: "var(--color-text)" }}>
                      {item.use_count} 次
                    </p>
                    <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                      成功率 {item.use_count > 0 ? Math.round((item.success_count / item.use_count) * 100) : 0}%
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* MCP 使用统计（F040） */}
        <div className="panel">
          <h2 className="panel-title mb-4">MCP 使用统计</h2>
          {mcpStats.length === 0 ? (
            <p className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>
              暂无 MCP 调用记录
            </p>
          ) : (
            <div className="space-y-3">
              {mcpStats.map((item) => (
                <div key={item.mcp_name} className="flex items-center justify-between text-sm">
                  <div className="min-w-0">
                    <p className="truncate" style={{ color: "var(--color-text-secondary)" }}>
                      {item.mcp_name}
                    </p>
                    <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                      平均耗时 {Math.round(item.avg_duration ?? 0)} ms
                    </p>
                  </div>
                  <div className="text-right shrink-0 ml-3">
                    <p className="font-medium" style={{ color: "var(--color-text)" }}>
                      {item.call_count} 次
                    </p>
                    <p className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                      成功率 {item.call_count > 0 ? Math.round((item.success_count / item.call_count) * 100) : 0}%
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ProgressRow({ label, count, total, color }: { label: string; count: number; total: number; color: string }) {
  const percentage = total > 0 ? Math.round((count / total) * 100) : 0;

  return (
    <div>
      <div className="flex justify-between text-sm mb-1.5">
        <span style={{ color: "var(--color-text-secondary)" }}>{label}</span>
        <span style={{ color: "var(--color-text-tertiary)" }}>
          {count} ({percentage}%)
        </span>
      </div>
      <div className="w-full rounded-full h-2" style={{ background: "var(--color-border-subtle)" }}>
        <div
          className="h-2 rounded-full"
          style={{ width: `${percentage}%`, background: color, transition: "width 0.5s cubic-bezier(0.2, 0, 0, 1)" }}
        />
      </div>
    </div>
  );
}
