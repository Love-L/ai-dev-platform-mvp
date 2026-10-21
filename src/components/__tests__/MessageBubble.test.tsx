import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import MessageBubble from "../MessageBubble";
import { parseAttachmentBlock } from "../../lib/sessionAttachments";
import type { SessionMessage } from "../../lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => `asset://${p}`,
  invoke: vi.fn(),
}));

describe("parseAttachmentBlock (P1-10)", () => {
  it("splits body and attachment paths (current marker)", () => {
    const text = "请分析这张截图\n\n📎 附件（请读取并识别以下文件）：\n- C:/data/a.png\n- C:/data/b.pdf";
    const { body, paths } = parseAttachmentBlock(text);
    expect(body).toBe("请分析这张截图");
    expect(paths).toEqual(["C:/data/a.png", "C:/data/b.pdf"]);
  });

  it("keeps legacy marker compatible", () => {
    const text = "看看这个\n\n📎 附件已保存到本地，请读取并识别以下文件：\n  - C:/old/fig.png";
    const { body, paths } = parseAttachmentBlock(text);
    expect(body).toBe("看看这个");
    expect(paths).toEqual(["C:/old/fig.png"]);
  });

  it("returns whole text when no marker", () => {
    const { body, paths } = parseAttachmentBlock("普通消息");
    expect(body).toBe("普通消息");
    expect(paths).toEqual([]);
  });
});

function makeMessage(overrides: Partial<SessionMessage>): SessionMessage {
  return {
    id: "m1",
    session_id: "s1",
    seq: 1,
    role: "user",
    block_type: "text",
    content: "",
    status: "completed",
    created_at: "2026-10-21T00:00:00Z",
    ...overrides,
  } as SessionMessage;
}

describe("MessageBubble 附件渲染 (P1-10)", () => {
  it("renders file chips for attachments", () => {
    render(
      <MessageBubble
        message={makeMessage({
          content: "任务说明\n\n📎 附件（请读取并识别以下文件）：\n- C:/docs/需求.md",
        })}
      />,
    );
    expect(screen.getByText("任务说明")).toBeInTheDocument();
    expect(screen.getByText("需求.md")).toBeInTheDocument();
  });

  it("copy button copies full raw text", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    render(<MessageBubble message={makeMessage({ content: "hello world" })} />);
    fireEvent.click(screen.getByTitle("复制消息"));
    await vi.waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("hello world");
    });
  });
});
