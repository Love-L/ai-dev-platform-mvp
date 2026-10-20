import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, it, expect, vi, beforeEach } from "vitest";
import Agents from "../Agents";
import type { Agent } from "../../lib/types";

const mocks = vi.hoisted(() => ({
  verifyAgents: vi.fn(),
  getAgentSessions: vi.fn(),
}));

vi.mock("../../lib/tauri", () => ({
  verifyAgents: mocks.verifyAgents,
  getAgentSessions: mocks.getAgentSessions,
  discoverAgents: vi.fn().mockResolvedValue([]),
  startAgent: vi.fn(),
  stopAgent: vi.fn(),
  startStructuredSession: vi.fn(),
}));

function makeAgent(status: Agent["status"]): Agent {
  return {
    id: "a1",
    name: "claude-code",
    display_name: "Claude Code",
    binary_path: "C:\\bin\\claude.exe",
    version: "2.1.266",
    status,
    capabilities: undefined,
    created_at: "2026-09-22T00:00:00Z",
  } as Agent;
}

function renderPage() {
  return render(
    <MemoryRouter>
      <Agents />
    </MemoryRouter>,
  );
}

/**
 * 回归：Agent 卡片曾出现「已失效」+「运行中」两个徽章同时存在。
 *
 * 根因：前端用 `agent.status !== "available"` 判断"已失效"，
 * 把合法的 running 状态（由 verify_agents 按会话推导维护）也误判为失效。
 * 状态语义：agents.status ∈ {available, unavailable, running}，
 * running 表示"二进制有效且有运行中会话"（后端按会话表推导）。
 */
describe("Agents 页状态徽章（状态单一来源）", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getAgentSessions.mockResolvedValue([]);
  });

  it('status=running（有运行中会话）只显示"运行中"，不显示"已失效"', async () => {
    mocks.verifyAgents.mockResolvedValue([makeAgent("running")]);

    renderPage();

    await waitFor(() => expect(screen.getByText("运行中")).toBeInTheDocument());
    expect(screen.queryByText("已失效")).not.toBeInTheDocument();
  });

  it('status=unavailable（二进制失效）显示"已失效"与"不可用"', async () => {
    mocks.verifyAgents.mockResolvedValue([makeAgent("unavailable")]);

    renderPage();

    await waitFor(() => expect(screen.getByText("已失效")).toBeInTheDocument());
    expect(screen.getByText("不可用")).toBeInTheDocument();
  });

  it('status=available 显示"可用"', async () => {
    mocks.verifyAgents.mockResolvedValue([makeAgent("available")]);

    renderPage();

    await waitFor(() => expect(screen.getByText("可用")).toBeInTheDocument());
    expect(screen.queryByText("已失效")).not.toBeInTheDocument();
  });

  it("进入页面即自愈：加载走 verify_agents 而非裸 get_agents", async () => {
    mocks.verifyAgents.mockResolvedValue([]);

    renderPage();

    await waitFor(() => expect(mocks.verifyAgents).toHaveBeenCalled());
  });
});
