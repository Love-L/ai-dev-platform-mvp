import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import AgentTerminal from "../AgentTerminal";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("../../lib/tauri", () => ({
  ptyWrite: vi.fn(() => Promise.resolve()),
  ptyResize: vi.fn(() => Promise.resolve()),
  ptyKill: vi.fn(() => Promise.resolve()),
  ptyAttach: vi.fn(() => Promise.reject(new Error("Agent 进程未运行"))),
  ptyReconnect: vi.fn(() => Promise.reject(new Error("Agent 进程未运行"))),
  saveSessionAttachment: vi.fn(),
}));

vi.mock("@xterm/xterm", () => ({
  Terminal: class MockTerminal {
    rows = 24;
    cols = 80;
    open() {}
    write() {}
    loadAddon() {
      return { dispose: () => {} };
    }
    onData() {
      return { dispose: () => {} };
    }
    onResize() {
      return { dispose: () => {} };
    }
    resize() {}
    clear() {}
    dispose() {}
  },
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class MockFitAddon {
    fit() {}
    proposeDimensions() {
      return { cols: 80, rows: 24 };
    }
  },
}));

describe("AgentTerminal (L1-019)", () => {
  it("renders without crashing", () => {
    const { container } = render(<AgentTerminal sessionId="1" agentName="Claude Code" />);
    expect(container).toBeTruthy();
  });

  it("进程未运行时展示重启入口", async () => {
    render(<AgentTerminal sessionId="1" agentName="Claude Code" />);
    expect(await screen.findByText("Agent 进程未运行")).toBeInTheDocument();
    expect(screen.getByText("重启 Agent 进程")).toBeInTheDocument();
  });
});
