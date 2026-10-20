import { useEffect, useState } from "react";
import { Coins, TrendingUp, RefreshCw, Settings, AlertTriangle, X } from "lucide-react";
import {
  getTokenUsageSummary,
  getTokenUsageTrend,
  getTokenUsageByModel,
  getTokenBudget,
  setTokenBudget,
  syncLocalUsage,
} from "../lib/tauri";
import type { TokenUsageSummary, DailyUsage, ModelUsage, TokenBudget } from "../lib/types";

export default function Cost() {
  const [summary, setSummary] = useState<TokenUsageSummary | null>(null);
  const [trend, setTrend] = useState<DailyUsage[]>([]);
  const [modelUsage, setModelUsage] = useState<ModelUsage[]>([]);
  const [budget, setBudget] = useState<TokenBudget | null>(null);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [showBudgetModal, setShowBudgetModal] = useState(false);
  const [budgetLimit, setBudgetLimit] = useState("2000000");
  const [budgetThreshold, setBudgetThreshold] = useState("0.8");

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    try {
      const [sum, trnd, models, bgt] = await Promise.all([
        getTokenUsageSummary(),
        getTokenUsageTrend(30),
        getTokenUsageByModel(),
        getTokenBudget(),
      ]);
      setSummary(sum);
      setTrend(trnd);
      setModelUsage(models);
      setBudget(bgt);
      if (bgt) {
        setBudgetLimit(bgt.monthly_limit.toString());
        setBudgetThreshold(bgt.alert_threshold.toString());
      }
    } catch (error) {
      console.error("加载数据失败:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSync() {
    setSyncing(true);
    try {
      await syncLocalUsage();
      await loadData();
    } catch (error) {
      console.error("同步失败:", error);
    } finally {
      setSyncing(false);
    }
  }

  async function handleSaveBudget() {
    try {
      await setTokenBudget(parseInt(budgetLimit), parseFloat(budgetThreshold));
      setShowBudgetModal(false);
      await loadData();
    } catch (error) {
      console.error("保存预算失败:", error);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>加载中...</div>
      </div>
    );
  }

  // 优先使用后端维护的当月累计用量（跨月自动重置），缺失时回退到总消耗估算
  const currentUsage = budget?.current_usage ?? summary?.total_tokens ?? 0;
  const budgetUsagePercent = budget
    ? Math.min(budget.usage_ratio !== undefined ? budget.usage_ratio : currentUsage / budget.monthly_limit, 1)
    : 0;
  const isOverBudget =
    budget?.exceeded !== undefined ? budget.exceeded : budgetUsagePercent >= (budget?.alert_threshold || 0.8);

  const statCards = [
    {
      icon: Coins,
      label: "总消耗",
      value: (summary?.total_tokens || 0).toLocaleString(),
      sub: "tokens",
      color: "var(--color-primary)",
      bg: "var(--color-primary-light)",
    },
    {
      icon: TrendingUp,
      label: "输入 Token",
      value: (summary?.input_tokens || 0).toLocaleString(),
      sub: "input tokens",
      color: "var(--color-success)",
      bg: "var(--color-success-light)",
    },
    {
      icon: TrendingUp,
      label: "输出 Token",
      value: (summary?.output_tokens || 0).toLocaleString(),
      sub: "output tokens",
      color: "#7c3aed",
      bg: "rgba(124, 58, 237, 0.06)",
    },
    {
      icon: Coins,
      label: "总费用",
      value: `$${(summary?.total_cost || 0).toFixed(2)}`,
      sub: `${summary?.record_count || 0} 条记录`,
      color: "var(--color-warning)",
      bg: "var(--color-warning-light)",
    },
  ];

  return (
    <div className="p-6 space-y-5">
      {/* 页面标题 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            Token 消耗
          </h1>
          <p className="mt-0.5" style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>
            追踪和管理 LLM Token 消耗
          </p>
        </div>
        <div className="flex gap-2">
          <button onClick={handleSync} disabled={syncing} className="btn-primary">
            <RefreshCw className={`w-4 h-4 ${syncing ? "animate-spin" : ""}`} />
            同步本地数据
          </button>
          <button onClick={() => setShowBudgetModal(true)} className="btn-secondary">
            <Settings className="w-4 h-4" />
            预算设置
          </button>
        </div>
      </div>

      {/* 预算概览 */}
      {budget && (
        <div
          className="panel"
          style={{
            borderColor: isOverBudget ? "rgba(220, 38, 38, 0.2)" : "rgba(37, 99, 235, 0.2)",
            background: isOverBudget ? "rgba(220, 38, 38, 0.03)" : "rgba(37, 99, 235, 0.03)",
          }}
        >
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              {isOverBudget && <AlertTriangle className="w-4 h-4" style={{ color: "var(--color-danger)" }} />}
              <span className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                {isOverBudget ? "预算警告" : "预算使用"}
              </span>
            </div>
            <span style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>
              {currentUsage.toLocaleString()} / {budget.monthly_limit.toLocaleString()} tokens
            </span>
          </div>
          <div className="w-full rounded-full h-2" style={{ background: "var(--color-bg-muted)" }}>
            <div
              className="h-2 rounded-full transition-all"
              style={{
                width: `${budgetUsagePercent * 100}%`,
                background: isOverBudget ? "var(--color-danger)" : "var(--color-primary)",
              }}
            />
          </div>
          <div className="flex justify-between mt-2" style={{ fontSize: "11px", color: "var(--color-text-tertiary)" }}>
            <span>{(budgetUsagePercent * 100).toFixed(1)}% 已使用</span>
            <span>预计费用: ${(summary?.total_cost || 0).toFixed(2)}</span>
          </div>
        </div>
      )}

      {/* 统计卡片 */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        {statCards.map((card) => (
          <div key={card.label} className="card p-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg" style={{ background: card.bg }}>
                <card.icon className="w-5 h-5" style={{ color: card.color }} strokeWidth={1.75} />
              </div>
              <div>
                <p style={{ color: "var(--color-text-tertiary)", fontSize: "12px" }}>{card.label}</p>
                <p className="text-lg font-semibold" style={{ color: "var(--color-text)" }}>
                  {card.value}
                </p>
              </div>
            </div>
            <p className="mt-2" style={{ color: "var(--color-text-tertiary)", fontSize: "11px" }}>
              {card.sub}
            </p>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
        {/* 消耗趋势 */}
        <div className="panel">
          <h2 className="panel-title mb-4">近 30 天消耗趋势</h2>
          {trend.length === 0 ? (
            <div className="empty-state py-8">
              <p className="empty-state-desc">暂无数据</p>
            </div>
          ) : (
            <div className="space-y-2">
              {trend.slice(-10).map((day) => {
                const maxTokens = Math.max(...trend.map((d) => d.total_tokens));
                const width = maxTokens > 0 ? (day.total_tokens / maxTokens) * 100 : 0;
                return (
                  <div key={day.date} className="flex items-center gap-2">
                    <span className="w-16" style={{ fontSize: "11px", color: "var(--color-text-tertiary)" }}>
                      {day.date.slice(5)}
                    </span>
                    <div
                      className="flex-1 rounded h-5 overflow-hidden"
                      style={{ background: "var(--color-bg-subtle)" }}
                    >
                      <div
                        className="h-5 rounded transition-all duration-500"
                        style={{ width: `${width}%`, background: "var(--color-primary)" }}
                      />
                    </div>
                    <span
                      className="w-16 text-right font-medium"
                      style={{ fontSize: "11px", color: "var(--color-text-tertiary)" }}
                    >
                      {day.total_tokens.toLocaleString()}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* 模型分布 */}
        <div className="panel">
          <h2 className="panel-title mb-4">按模型分布</h2>
          {modelUsage.length === 0 ? (
            <div className="empty-state py-8">
              <p className="empty-state-desc">暂无数据</p>
            </div>
          ) : (
            <div className="space-y-4">
              {modelUsage.map((model) => {
                const maxTokens = Math.max(...modelUsage.map((m) => m.total_tokens));
                const width = maxTokens > 0 ? (model.total_tokens / maxTokens) * 100 : 0;
                return (
                  <div key={model.model}>
                    <div className="flex justify-between text-sm mb-1.5">
                      <span className="font-medium" style={{ color: "var(--color-text-secondary)" }}>
                        {model.model || "unknown"}
                      </span>
                      <span style={{ color: "var(--color-text-tertiary)" }}>{model.total_tokens.toLocaleString()}</span>
                    </div>
                    <div className="w-full rounded-full h-2" style={{ background: "var(--color-bg-subtle)" }}>
                      <div
                        className="h-2 rounded-full transition-all duration-500"
                        style={{ width: `${width}%`, background: "var(--color-primary)" }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* 预算设置弹窗 */}
      {showBudgetModal && (
        <div className="modal-overlay" onClick={() => setShowBudgetModal(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <div className="flex items-center justify-between">
                <h3 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
                  预算设置
                </h3>
                <button
                  onClick={() => setShowBudgetModal(false)}
                  className="p-1 rounded-lg transition-colors"
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
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>
            <div className="modal-body space-y-4">
              <div>
                <label className="label">月度 Token 上限</label>
                <input
                  type="number"
                  value={budgetLimit}
                  onChange={(e) => setBudgetLimit(e.target.value)}
                  className="input"
                />
              </div>
              <div>
                <label className="label">告警阈值 (0-1)</label>
                <input
                  type="number"
                  step="0.1"
                  min="0"
                  max="1"
                  value={budgetThreshold}
                  onChange={(e) => setBudgetThreshold(e.target.value)}
                  className="input"
                />
              </div>
            </div>
            <div className="modal-footer">
              <button onClick={() => setShowBudgetModal(false)} className="btn-secondary">
                取消
              </button>
              <button onClick={handleSaveBudget} className="btn-primary">
                保存
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
