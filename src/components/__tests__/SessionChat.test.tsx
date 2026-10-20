import { render, screen, waitFor, cleanup, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import SessionChat from "../SessionChat";
import type { SessionMessage } from "../../lib/types";

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  listSessionMessages: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, handler: (event: { payload: unknown }) => void) => {
    mocks.listeners.set(event, handler);
    return Promise.resolve(() => mocks.listeners.delete(event));
  }),
}));

vi.mock("../../lib/tauri", () => ({
  listSessionMessages: mocks.listSessionMessages,
  sendSessionMessage: vi.fn(),
  interruptSession: vi.fn(),
}));

function msg(partial: Partial<SessionMessage>): SessionMessage {
  return {
    id: Math.random().toString(36).slice(2),
    session_id: "s1",
    seq: 1,
    role: "assistant",
    block_type: "text",
    status: "completed",
    created_at: "2026-09-22T10:00:00Z",
    ...partial,
  };
}

function emit(event: string, payload: unknown) {
  mocks.listeners.get(event)?.({ payload });
}

async function renderChat(sessionId = "s1") {
  const utils = render(
    <SessionChat sessionId={sessionId} running onAllowAndRetry={vi.fn()} onUseAcceptEdits={vi.fn()} />,
  );
  // subscribe 是异步注册，等待首页消息加载完成
  await waitFor(() => expect(mocks.listSessionMessages).toHaveBeenCalled());
  return utils;
}

describe("SessionChat 会话契约（模块 16 / F076–F079）", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listeners.clear();
  });

  it("挂载即按契约拉取首页消息（sessionId / undefined / 100）并渲染", async () => {
    mocks.listSessionMessages.mockResolvedValue({
      items: [msg({ seq: 1, role: "user", block_type: "text", content: "你好" })],
      total: 1,
      page: 1,
      page_size: 100,
      total_pages: 1,
    });

    await renderChat();

    expect(mocks.listSessionMessages).toHaveBeenCalledWith("s1", undefined, 100);
    expect(await screen.findByText("你好")).toBeInTheDocument();
    // store 消费的 3 类事件全部注册（tool/result 数据经落库 message 块承载，无需单独订阅）
    for (const event of ["session:message", "session:delta", "session:notice"]) {
      expect(mocks.listeners.has(event)).toBe(true);
    }
  });

  it("正文增量实时渲染，落库块到达后清空缓冲（增量→落库幂等）", async () => {
    mocks.listSessionMessages.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 100,
      total_pages: 0,
    });
    await renderChat();

    emit("session:delta", { session_id: "s1", block_type: "text", delta: "正在" });
    emit("session:delta", { session_id: "s1", block_type: "text", delta: "生成" });
    expect(await screen.findByText(/正在生成/)).toBeInTheDocument();

    // 完整块落库后推送：缓冲清空，仅显示落库内容
    emit("session:message", {
      session_id: "s1",
      message: msg({ seq: 1, block_type: "text", content: "正在生成完整回复" }),
    });
    await waitFor(() => expect(screen.getByText("正在生成完整回复")).toBeInTheDocument());
    expect(screen.queryByText(/正在生成$/)).not.toBeInTheDocument();
  });

  it("同一消息重复下发不重复渲染（事件幂等）", async () => {
    mocks.listSessionMessages.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 100,
      total_pages: 0,
    });
    await renderChat();

    const payload = { session_id: "s1", message: msg({ id: "dup-1", seq: 1, content: "唯一" }) };
    emit("session:message", payload);
    emit("session:message", payload);

    await waitFor(() => expect(screen.getAllByText("唯一")).toHaveLength(1));
  });

  it("思考 Token 增量驱动折叠态计数（F079 实测事件）", async () => {
    mocks.listSessionMessages.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 100,
      total_pages: 0,
    });
    await renderChat();

    emit("session:delta", { session_id: "s1", block_type: "thinking_tokens", delta: "128", total: 128 });

    expect(await screen.findByText(/思考中…/)).toBeInTheDocument();
    expect(screen.getByText(/约 128 tokens/)).toBeInTheDocument();
  });

  it("权限被拒消息渲染被拒卡片并回传工具名（F083）", async () => {
    mocks.listSessionMessages.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 100,
      total_pages: 0,
    });
    const onAllow = vi.fn();
    render(<SessionChat sessionId="s1" running onAllowAndRetry={onAllow} onUseAcceptEdits={vi.fn()} />);
    await waitFor(() => expect(mocks.listSessionMessages).toHaveBeenCalled());

    emit("session:message", {
      session_id: "s1",
      message: msg({
        seq: 1,
        block_type: "permission_denied",
        tool_name: "Write",
        status: "denied",
        content: "权限被拒",
      }),
    });

    fireEvent.click(await screen.findByText("始终允许并重试"));
    expect(onAllow).toHaveBeenCalledWith("Write");
  });

  it("非本会话的事件被过滤（多会话并行不串扰）", async () => {
    mocks.listSessionMessages.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 100,
      total_pages: 0,
    });
    await renderChat("s1");

    emit("session:message", {
      session_id: "other",
      message: msg({ seq: 1, block_type: "text", content: "别的会话" }),
    });

    expect(screen.queryByText("别的会话")).not.toBeInTheDocument();
  });

  it("卸载后取消全部事件订阅", async () => {
    mocks.listSessionMessages.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 100,
      total_pages: 0,
    });
    const { unmount } = await renderChat();
    expect(mocks.listeners.size).toBeGreaterThan(0);

    unmount();
    cleanup();
    expect(mocks.listeners.size).toBe(0);
  });
});
