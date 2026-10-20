import { render } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import RichTextEditor from "../RichTextEditor";

vi.mock("@tiptap/react", async () => {
  const { createElement: createEl } = await import("react");
  const mockEditor = {
    getHTML: () => "<p>test</p>",
    isActive: () => false,
    can: () => ({
      undo: () => true,
      redo: () => true,
    }),
    chain: () => ({
      focus: () => ({
        toggleBold: () => ({ run: vi.fn() }),
        toggleItalic: () => ({ run: vi.fn() }),
        toggleCode: () => ({ run: vi.fn() }),
        toggleHeading: () => ({ run: vi.fn() }),
        toggleBulletList: () => ({ run: vi.fn() }),
        toggleOrderedList: () => ({ run: vi.fn() }),
        toggleCodeBlock: () => ({ run: vi.fn() }),
        setHorizontalRule: () => ({ run: vi.fn() }),
        undo: () => ({ run: vi.fn() }),
        redo: () => ({ run: vi.fn() }),
      }),
    }),
    commands: { setContent: vi.fn() },
    on: vi.fn(),
    off: vi.fn(),
  };
  return {
    useEditor: () => mockEditor,
    EditorContent: () => createEl("div", null, "Editor"),
  };
});

describe("RichTextEditor component", () => {
  it("renders without crashing", () => {
    const { container } = render(<RichTextEditor content="" onChange={vi.fn()} />);
    expect(container).toBeTruthy();
  });
});
