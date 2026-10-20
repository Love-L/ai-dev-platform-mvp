import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  getStatistics,
  getTokenUsageSummary,
  getTokenUsageTrend,
  getTokenUsageByModel,
  getTokenBudget,
  setTokenBudget,
} from "../../src/lib/tauri";

const mockInvoke = vi.mocked(invoke);

describe("L3 Full-stack: 统计与 Token 追踪全链路", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // L3-028: 获取统计数据
  it("L3-028: getStatistics 返回完整统计", async () => {
    const mockStats = {
      total_requirements: 10,
      total_tasks: 25,
      completed_tasks: 15,
      total_projects: 3,
    };
    mockInvoke.mockResolvedValue(mockStats);

    const result = await getStatistics();

    expect(result.total_requirements).toBe(10);
    expect(result.completed_tasks).toBe(15);
  });

  // L3-029: Token 使用摘要
  it("L3-029: getTokenUsageSummary 返回摘要", async () => {
    const mockSummary = {
      total_tokens: 150000,
      total_cost: 4.5,
      period: "2026-09",
    };
    mockInvoke.mockResolvedValue(mockSummary);

    const result = await getTokenUsageSummary();

    expect(result.total_tokens).toBe(150000);
    expect(result.total_cost).toBe(4.5);
  });

  // L3-030: Token 使用趋势
  it("L3-030: getTokenUsageTrend 返回趋势数据", async () => {
    const mockTrend = [
      { date: "2026-09-01", tokens: 5000, cost: 0.15 },
      { date: "2026-09-02", tokens: 8000, cost: 0.24 },
    ];
    mockInvoke.mockResolvedValue(mockTrend);

    const result = await getTokenUsageTrend(7);

    expect(result).toHaveLength(2);
    expect(result[0].tokens).toBe(5000);
  });

  // L3-031: Token 按模型统计
  it("L3-031: getTokenUsageByModel 返回模型分布", async () => {
    const mockModelUsage = [
      { model: "claude-3-opus", tokens: 100000, cost: 3.0 },
      { model: "claude-3-sonnet", tokens: 50000, cost: 1.5 },
    ];
    mockInvoke.mockResolvedValue(mockModelUsage);

    const result = await getTokenUsageByModel();

    expect(result).toHaveLength(2);
    expect(result[0].model).toBe("claude-3-opus");
  });

  // L3-032: Token 预算设置
  it("L3-032: setTokenBudget 调用正确", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await setTokenBudget(1000, 80);

    expect(mockInvoke).toHaveBeenCalledWith("set_token_budget", {
      monthlyLimit: 1000,
      alertThreshold: 80,
    });
  });

  // L3-033: Token 预算查询
  it("L3-033: getTokenBudget 返回预算信息", async () => {
    const mockBudget = {
      monthly_limit: 1000,
      alert_threshold: 80,
      current_usage: 500,
    };
    mockInvoke.mockResolvedValue(mockBudget);

    const result = await getTokenBudget();

    expect(result?.monthly_limit).toBe(1000);
    expect(result?.current_usage).toBe(500);
  });
});
