import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import SessionInput from "../SessionInput";

const mockInvoke = vi.mocked(invoke);

function setup(overrides: Partial<Parameters<typeof SessionInput>[0]> = {}) {
  const onSend = vi.fn();
  const onInterrupt = vi.fn();
  render(
    <SessionInput
      sessionId="sess-1"
      running
      sending={false}
      onSend={onSend}
      onInterrupt={onInterrupt}
      {...overrides}
    />,
  );
  return { onSend, onInterrupt };
}

function type(text: string) {
  const textarea = screen.getByPlaceholderText(/输入指令/);
  fireEvent.change(textarea, { target: { value: text } });
  return textarea;
}

describe("SessionInput（模块 16 / F077）", () => {
  it("Enter 发送指令并清空输入框", () => {
    const { onSend } = setup();
    const textarea = type("列出当前目录");

    fireEvent.keyDown(textarea, { key: "Enter" });

    expect(onSend).toHaveBeenCalledTimes(1);
    expect(onSend).toHaveBeenCalledWith("列出当前目录");
    expect((textarea as HTMLTextAreaElement).value).toBe("");
  });

  it("Shift+Enter 不发送（换行）", () => {
    const { onSend } = setup();
    const textarea = type("第一行");

    fireEvent.keyDown(textarea, { key: "Enter", shiftKey: true });

    expect(onSend).not.toHaveBeenCalled();
  });

  it("空白内容不发送", () => {
    const { onSend } = setup();
    const textarea = type("   ");

    fireEvent.keyDown(textarea, { key: "Enter" });

    expect(onSend).not.toHaveBeenCalled();
  });

  it("会话未运行时输入与按钮禁用，提示重建会话", () => {
    setup({ running: false });

    const textarea = screen.getByPlaceholderText(/会话已结束/);
    expect(textarea).toBeDisabled();
    expect(screen.getByText("发送")).toBeDisabled();
    expect(screen.getByText("中断")).toBeDisabled();
  });

  it("发送中禁用发送按钮", () => {
    setup({ sending: true });
    type("abc");
    expect(screen.getByText("发送中")).toBeDisabled();
  });

  it("中断按钮触发 onInterrupt", () => {
    const { onInterrupt } = setup();
    fireEvent.click(screen.getByText("中断"));
    expect(onInterrupt).toHaveBeenCalledTimes(1);
  });

  it("readOnlyHint 渲染只读提示（结构化会话终端视图）", () => {
    render(<SessionInput running sending={false} onSend={vi.fn()} onInterrupt={vi.fn()} readOnlyHint="只读视图" />);
    expect(screen.getByText("只读视图")).toBeInTheDocument();
    expect(screen.queryByPlaceholderText(/输入指令/)).not.toBeInTheDocument();
  });
});

describe("SessionInput 附件（图片粘贴 / 导入识别）", () => {
  function pasteImage(textarea: HTMLElement) {
    const file = new File([new Uint8Array([1, 2])], "shot.png", { type: "image/png" });
    // jsdom 的 ClipboardEvent 不透传 clipboardData，手工构造并派发 paste
    const ev = new Event("paste", { bubbles: true, cancelable: true }) as Event & {
      clipboardData: { items: unknown[] };
    };
    ev.clipboardData = {
      items: [{ kind: "file", type: "image/png", getAsFile: () => file }],
    };
    textarea.dispatchEvent(ev);
  }

  it("粘贴图片时保存附件并显示 chip", async () => {
    mockInvoke.mockResolvedValue({ name: "shot.png", path: "C:\\att\\shot.png", size: 2 });
    setup();
    const textarea = screen.getByPlaceholderText(/输入指令/);

    pasteImage(textarea);

    await screen.findByText("shot.png");
    expect(mockInvoke).toHaveBeenCalledWith(
      "save_session_attachment",
      expect.objectContaining({ sessionId: "sess-1", fileName: "shot.png" }),
    );
  });

  it("携带附件发送时消息包含保存路径", async () => {
    mockInvoke.mockResolvedValue({ name: "shot.png", path: "C:\\att\\shot.png", size: 2 });
    const { onSend } = setup();
    const textarea = screen.getByPlaceholderText(/输入指令/);

    pasteImage(textarea);
    await screen.findByText("shot.png");

    fireEvent.change(textarea, { target: { value: "看下这张图" } });
    fireEvent.keyDown(textarea, { key: "Enter" });

    expect(onSend).toHaveBeenCalledTimes(1);
    const sent = onSend.mock.calls[0][0] as string;
    expect(sent).toContain("看下这张图");
    expect(sent).toContain("C:\\att\\shot.png");
  });

  it("仅附件（无文本）也可发送", async () => {
    mockInvoke.mockResolvedValue({ name: "shot.png", path: "C:\\att\\shot.png", size: 2 });
    const { onSend } = setup();
    const textarea = screen.getByPlaceholderText(/输入指令/);

    pasteImage(textarea);
    await screen.findByText("shot.png");

    fireEvent.keyDown(textarea, { key: "Enter" });

    expect(onSend).toHaveBeenCalledTimes(1);
    expect(onSend.mock.calls[0][0]).toContain("C:\\att\\shot.png");
  });

  it("移除附件后不再随消息发送", async () => {
    mockInvoke.mockResolvedValue({ name: "shot.png", path: "C:\\att\\shot.png", size: 2 });
    const { onSend } = setup();
    const textarea = screen.getByPlaceholderText(/输入指令/);

    pasteImage(textarea);
    await screen.findByText("shot.png");
    fireEvent.click(screen.getByTitle("移除附件"));

    await waitFor(() => expect(screen.queryByText("shot.png")).not.toBeInTheDocument());
    fireEvent.change(textarea, { target: { value: "纯文本" } });
    fireEvent.keyDown(textarea, { key: "Enter" });

    expect(onSend).toHaveBeenCalledWith("纯文本");
  });
});
