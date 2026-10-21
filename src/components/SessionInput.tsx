import { useRef, useState } from "react";
import { Send, Square, Paperclip, X, Loader2 } from "lucide-react";
import { saveSessionAttachment } from "../lib/tauri";
import { showToast } from "./Toast";

interface PendingAttachment {
  name: string;
  path: string;
  size: number;
  /** 图片预览 URL（环境不支持 createObjectURL 时为空） */
  previewUrl?: string;
}

interface SessionInputProps {
  /** 所属会话（附件按会话归档；缺省落 general 目录） */
  sessionId?: string;
  /** 会话进程是否在运行 */
  running: boolean;
  sending: boolean;
  onSend: (content: string) => void;
  onInterrupt: () => void;
  /** 只读提示（如结构化会话的终端视图） */
  readOnlyHint?: string;
}

/** 附件大小上限（与后端 save_session_attachment 一致） */
const MAX_ATTACHMENT_SIZE = 20 * 1024 * 1024;

/** 读取文件字节（FileReader 兼容 jsdom 等缺少 Blob.arrayBuffer 的环境） */
function readFileBytes(file: File): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
    reader.onerror = () => reject(reader.error ?? new Error("读取文件失败"));
    reader.readAsArrayBuffer(file);
  });
}

/**
 * 指令输入区（模块 16 / F077）。
 *
 * - Enter 发送、Shift+Enter 换行；运行中可中断当前执行；
 * - 支持 Ctrl+V 粘贴图片、回形针导入附件：文件落盘到应用数据目录，
 *   发送时把保存路径注入消息文本，由 Agent 读取完成识别。
 */
export default function SessionInput({
  sessionId,
  running,
  sending,
  onSend,
  onInterrupt,
  readOnlyHint,
}: SessionInputProps) {
  const [value, setValue] = useState("");
  const [attachments, setAttachments] = useState<PendingAttachment[]>([]);
  const [attaching, setAttaching] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  if (readOnlyHint) {
    return (
      <div
        className="px-4 py-2.5 text-xs"
        style={{ borderTop: "1px solid var(--color-border-subtle)", color: "var(--color-text-tertiary)" }}
      >
        {readOnlyHint}
      </div>
    );
  }

  /** 读取文件并保存到后端附件目录 */
  async function addFiles(files: File[]) {
    if (files.length === 0 || !running) return;
    setAttaching(true);
    try {
      for (const file of files) {
        if (file.size > MAX_ATTACHMENT_SIZE) {
          showToast("error", `${file.name} 超过 20MB，未添加`);
          continue;
        }
        const bytes = await readFileBytes(file);
        const saved = await saveSessionAttachment(sessionId ?? null, file.name, Array.from(bytes));
        const previewUrl =
          file.type.startsWith("image/") && typeof URL.createObjectURL === "function"
            ? URL.createObjectURL(file)
            : undefined;
        setAttachments((prev) => [...prev, { ...saved, previewUrl }]);
      }
    } catch (error) {
      console.error("附件导入失败:", error);
      showToast("error", "附件导入失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setAttaching(false);
    }
  }

  function removeAttachment(index: number) {
    setAttachments((prev) => {
      const target = prev[index];
      if (target?.previewUrl) URL.revokeObjectURL(target.previewUrl);
      return prev.filter((_, i) => i !== index);
    });
  }

  /** Ctrl+V 粘贴：命中文件（图片）时接管并保存 */
  function handlePaste(e: React.ClipboardEvent<HTMLTextAreaElement>) {
    const items = e.clipboardData?.items;
    if (!items) return;
    const files: File[] = [];
    for (let i = 0; i < items.length; i++) {
      if (items[i].kind === "file") {
        const file = items[i].getAsFile();
        if (file) files.push(file);
      }
    }
    if (files.length > 0) {
      e.preventDefault();
      addFiles(files);
    }
  }

  function submit() {
    const text = value.trim();
    if (sending || !running) return;
    if (!text && attachments.length === 0) return;

    // 附件以固定 marker 区块注入：Agent 读取路径行完成识别，
    // 前端气泡解析该区块渲染缩略图 / 文件 chip（见 MessageBubble）
    let content = text;
    if (attachments.length > 0) {
      const list = attachments.map((a) => `- ${a.path}`).join("\n");
      content = `${text ? text + "\n\n" : ""}📎 附件（请读取并识别以下文件）：\n${list}`;
    }
    setValue("");
    attachments.forEach((a) => {
      if (a.previewUrl) URL.revokeObjectURL(a.previewUrl);
    });
    setAttachments([]);
    onSend(content);
  }

  const canSend = running && !sending && (value.trim().length > 0 || attachments.length > 0);

  return (
    <div className="px-3 py-2.5 space-y-2" style={{ borderTop: "1px solid var(--color-border-subtle)" }}>
      {/* 附件预览 */}
      {attachments.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {attachments.map((att, index) => (
            <span
              key={att.path}
              className="inline-flex items-center gap-1.5 pl-1.5 pr-1 py-1 rounded-lg text-[11px] max-w-[220px]"
              style={{
                background: "var(--color-bg-subtle)",
                color: "var(--color-text-secondary)",
                border: "1px solid var(--color-border-subtle)",
              }}
            >
              {att.previewUrl ? (
                <img src={att.previewUrl} alt="" className="w-4 h-4 rounded object-cover" />
              ) : (
                <Paperclip className="w-3 h-3" />
              )}
              <span className="truncate">{att.name}</span>
              <button
                type="button"
                onClick={() => removeAttachment(index)}
                className="p-0.5 rounded hover:opacity-70"
                title="移除附件"
              >
                <X className="w-3 h-3" />
              </button>
            </span>
          ))}
        </div>
      )}

      <div className="flex items-end gap-2">
        <textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onPaste={handlePaste}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              submit();
            }
          }}
          rows={2}
          disabled={!running}
          placeholder={running ? "输入指令，Enter 发送 / Shift+Enter 换行…" : "会话已结束，请重建或续接会话"}
          className="input flex-1 resize-none text-sm"
          style={{ minHeight: 44 }}
        />

        <div className="flex flex-row items-end gap-1.5 shrink-0">
          <button
            type="button"
            title="添加附件（可直接 Ctrl+V 粘贴图片）"
            onClick={() => fileInputRef.current?.click()}
            disabled={!running || attaching}
            className="btn-secondary text-xs px-2 py-1.5 flex items-center justify-center"
          >
            {attaching ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Paperclip className="w-3.5 h-3.5" />}
          </button>
          <button
            type="button"
            onClick={onInterrupt}
            disabled={!running}
            title="中断当前执行"
            className="btn-secondary text-xs px-2 py-1.5 flex items-center gap-1"
          >
            <Square className="w-3.5 h-3.5" />
            中断
          </button>
          <button
            type="button"
            onClick={submit}
            disabled={!canSend}
            className="btn-primary text-xs px-3 py-1.5 flex items-center gap-1.5"
          >
            <Send className="w-3.5 h-3.5" />
            {sending ? "发送中" : "发送"}
          </button>
        </div>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        multiple
        hidden
        accept="image/*,.pdf,.md,.txt,.csv,.json,.doc,.docx,.xls,.xlsx,.zip"
        onChange={(e) => {
          const files = Array.from(e.target.files ?? []);
          e.target.value = "";
          addFiles(files);
        }}
      />

      <p className="text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
        提示：Ctrl+V 可直接粘贴图片，回形针可导入附件（随消息交给 Agent 读取识别）；
        工具被权限策略拒绝时，可在消息卡片中「始终允许并重试」。
      </p>
    </div>
  );
}
