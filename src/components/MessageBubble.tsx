import { useState } from "react";
import {
  Brain,
  Check,
  ChevronDown,
  ChevronRight,
  Coins,
  Copy,
  FileText,
  Link2,
  Paperclip,
  ShieldAlert,
  Wrench,
} from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { SessionMessage } from "../lib/types";
import { IMAGE_EXT, parseAttachmentBlock } from "../lib/sessionAttachments";
import { showToast } from "./Toast";

/** 附件展示行：图片缩略图 + 文件名 chip */
function AttachmentRow({ paths }: { paths: string[] }) {
  if (paths.length === 0) return null;
  return (
    <div className="flex flex-wrap gap-1.5 mt-2">
      {paths.map((p) => {
        const name = p.split(/[\\/]/).pop() || p;
        const isImage = IMAGE_EXT.test(p);
        if (isImage) {
          return (
            <span
              key={p}
              className="inline-flex items-center gap-1 pl-1 pr-1.5 py-1 rounded-lg text-[11px]"
              style={{
                background: "var(--color-bg)",
                border: "1px solid var(--color-border-subtle)",
                color: "var(--color-text-secondary)",
              }}
              title={p}
            >
              <img
                src={convertFileSrc(p)}
                alt={name}
                className="w-8 h-8 rounded object-cover"
                style={{ background: "var(--color-bg-subtle)" }}
                onError={(e) => {
                  // asset 协议 scope 未覆盖时降级为文件图标
                  const el = e.currentTarget;
                  el.style.display = "none";
                }}
              />
              <span className="max-w-[140px] truncate">{name}</span>
            </span>
          );
        }
        return (
          <span
            key={p}
            className="inline-flex items-center gap-1 px-2 py-1 rounded-lg text-[11px]"
            style={{
              background: "var(--color-bg)",
              border: "1px solid var(--color-border-subtle)",
              color: "var(--color-text-secondary)",
            }}
            title={p}
          >
            <FileText className="w-3 h-3" />
            <span className="max-w-[160px] truncate">{name}</span>
          </span>
        );
      })}
    </div>
  );
}

/** 单条消息复制（text 块 hover 出现） */
function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      title="复制消息"
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(text);
          setCopied(true);
          setTimeout(() => setCopied(false), 1200);
        } catch {
          showToast("error", "复制失败");
        }
      }}
      className="opacity-0 group-hover:opacity-100 p-1 rounded transition-opacity"
      style={{ color: "var(--color-text-tertiary)", transition: "opacity var(--transition-fast)" }}
    >
      {copied ? <Check className="w-3 h-3" style={{ color: "var(--color-success)" }} /> : <Copy className="w-3 h-3" />}
    </button>
  );
}

interface MessageBubbleProps {
  message: SessionMessage;
  /** 被拒卡片的「允许并重试」 */
  onAllowAndRetry?: (toolName?: string) => void;
  /** 被拒卡片的「改用 acceptEdits」 */
  onUseAcceptEdits?: () => void;
  /** 被拒卡片操作进行中（禁用按钮，防止重复点击） */
  deniedBusy?: boolean;
}

/** 行内 code 渲染（不引入 Markdown 依赖，保持 P0 体积可控） */
function InlineText({ text }: { text: string }) {
  const parts = text.split(/(`[^`]+`)/g);
  return (
    <>
      {parts.map((part, index) =>
        part.length > 2 && part.startsWith("`") && part.endsWith("`") ? (
          <code
            key={index}
            className="px-1 py-0.5 rounded font-mono text-[12px]"
            style={{ background: "var(--color-bg-subtle)", color: "var(--color-text)" }}
          >
            {part.slice(1, -1)}
          </code>
        ) : (
          <span key={index}>{part}</span>
        ),
      )}
    </>
  );
}

/** 轻量富文本：识别围栏代码块与行内 code */
function RichText({ text }: { text: string }) {
  const segments = text.split("```");
  return (
    <div className="whitespace-pre-wrap break-words text-sm leading-relaxed">
      {segments.map((segment, index) => {
        if (index % 2 === 1) {
          // 去掉语言标识行
          const body = segment.replace(/^[a-zA-Z0-9_+-]*\r?\n/, "");
          return (
            <pre
              key={index}
              className="my-2 p-3 rounded-lg overflow-x-auto text-[12px] font-mono"
              style={{ background: "var(--color-bg-subtle)" }}
            >
              {body.replace(/\n$/, "")}
            </pre>
          );
        }
        return <InlineText key={index} text={segment} />;
      })}
    </div>
  );
}

function Collapsible({
  title,
  icon,
  defaultOpen = false,
  meta,
  children,
}: {
  title: React.ReactNode;
  icon: React.ReactNode;
  defaultOpen?: boolean;
  meta?: React.ReactNode;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="rounded-lg overflow-hidden" style={{ border: "1px solid var(--color-border-subtle)" }}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left"
        style={{ background: "var(--color-bg-subtle)" }}
      >
        {open ? (
          <ChevronDown className="w-3.5 h-3.5" style={{ color: "var(--color-text-tertiary)" }} />
        ) : (
          <ChevronRight className="w-3.5 h-3.5" style={{ color: "var(--color-text-tertiary)" }} />
        )}
        {icon}
        <span className="text-xs font-medium flex-1" style={{ color: "var(--color-text-secondary)" }}>
          {title}
        </span>
        {meta}
      </button>
      {open && <div className="px-3 py-2">{children}</div>}
    </div>
  );
}

function prettyJson(raw?: string | null): string {
  if (!raw) return "";
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

/** 思考块（F079）：默认折叠，头部展示估算 Token */
function ThinkingBlock({ message }: { message: SessionMessage }) {
  const text = message.content ?? "";
  const tokens = message.tokens ?? Math.max(1, Math.round(text.length / 3));
  return (
    <Collapsible
      title="思考过程"
      icon={<Brain className="w-3.5 h-3.5" style={{ color: "var(--color-primary)" }} />}
      meta={
        <span className="text-[10px]" style={{ color: "var(--color-text-tertiary)" }}>
          约 {tokens} tokens
        </span>
      }
    >
      <div
        className="whitespace-pre-wrap break-words text-[12px] leading-relaxed"
        style={{ color: "var(--color-text-tertiary)" }}
      >
        {text}
      </div>
    </Collapsible>
  );
}

/** 工具调用块（F080） */
function ToolUseBlock({ message }: { message: SessionMessage }) {
  return (
    <Collapsible
      title={message.tool_name ? `调用工具 ${message.tool_name}` : "调用工具"}
      icon={<Wrench className="w-3.5 h-3.5" style={{ color: "var(--color-warning, #f9e2af)" }} />}
      meta={
        <span
          className="text-[10px] px-1.5 py-0.5 rounded"
          style={{ background: "var(--color-bg)", color: "var(--color-text-tertiary)" }}
        >
          {message.status === "denied" ? "被拒绝" : message.status === "failed" ? "失败" : "已调用"}
        </span>
      }
    >
      <pre
        className="p-2 rounded-lg overflow-x-auto text-[11px] font-mono"
        style={{ background: "var(--color-bg-subtle)" }}
      >
        {prettyJson(message.content)}
      </pre>
    </Collapsible>
  );
}

/** 工具结果块（F080） */
function ToolResultBlock({ message }: { message: SessionMessage }) {
  const [expanded, setExpanded] = useState(false);
  const text = message.content ?? "";
  const truncated = !expanded && text.length > 600;
  return (
    <div
      className="rounded-lg px-3 py-2"
      style={{ border: "1px solid var(--color-border-subtle)", background: "var(--color-bg-subtle)" }}
    >
      <div className="flex items-center gap-2 mb-1">
        <span className="text-[10px]" style={{ color: "var(--color-text-tertiary)" }}>
          工具结果
        </span>
        {message.status === "failed" && (
          <span className="text-[10px]" style={{ color: "var(--color-danger)" }}>
            失败
          </span>
        )}
      </div>
      <pre className="whitespace-pre-wrap break-words text-[11px] font-mono m-0">
        {truncated ? `${text.slice(0, 600)}…` : text}
      </pre>
      {text.length > 600 && (
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="text-[11px] mt-1"
          style={{ color: "var(--color-primary)" }}
        >
          {expanded ? "收起" : `展开全部（${text.length} 字符）`}
        </button>
      )}
    </div>
  );
}

/** 权限被拒卡片（F083）：给出可操作的三个出口 */
function PermissionDeniedCard({
  message,
  onAllowAndRetry,
  onUseAcceptEdits,
  busy,
}: {
  message: SessionMessage;
  onAllowAndRetry?: (toolName?: string) => void;
  onUseAcceptEdits?: () => void;
  busy?: boolean;
}) {
  return (
    <div
      className="rounded-lg px-3 py-3 space-y-2"
      style={{ border: "1px solid var(--color-warning, #f9e2af)", background: "var(--color-warning-light, #fff8e1)" }}
    >
      <div className="flex items-center gap-2">
        <ShieldAlert className="w-4 h-4" style={{ color: "var(--color-warning, #b98900)" }} />
        <span className="text-xs font-medium" style={{ color: "var(--color-text)" }}>
          操作被权限策略拒绝{message.tool_name ? `（${message.tool_name}）` : ""}
        </span>
      </div>
      <p className="text-[12px] whitespace-pre-wrap" style={{ color: "var(--color-text-secondary)" }}>
        {message.content}
      </p>
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="btn-primary text-xs"
          disabled={busy}
          onClick={() => onAllowAndRetry?.(message.tool_name)}
        >
          {busy ? "处理中…" : "始终允许并重试"}
        </button>
        <button type="button" className="btn-secondary text-xs" disabled={busy} onClick={onUseAcceptEdits}>
          改用 acceptEdits
        </button>
      </div>
      <p className="text-[11px]" style={{ color: "var(--color-text-tertiary)" }}>
        允许后该工具将写入本会话工具白名单，本会话后续调用不再询问；默认白名单可在「设置 → 会话设置」中配置。
      </p>
    </div>
  );
}

/** 单轮结果（F084） */
function ResultBlock({ message }: { message: SessionMessage }) {
  let summary: Record<string, unknown> = {};
  try {
    summary = JSON.parse(message.content ?? "{}");
  } catch {
    summary = {};
  }
  const num = (key: string) => Number(summary[key] ?? 0);
  return (
    <div
      className="flex items-center justify-center gap-3 text-[11px] py-1"
      style={{ color: "var(--color-text-tertiary)" }}
    >
      <Coins className="w-3 h-3" />
      <span>输入 {num("input_tokens").toLocaleString()}</span>
      <span>输出 {num("output_tokens").toLocaleString()}</span>
      {num("cache_read_input_tokens") > 0 && <span>缓存读取 {num("cache_read_input_tokens").toLocaleString()}</span>}
      {num("cost_usd") > 0 && <span>${num("cost_usd").toFixed(4)}</span>}
      {num("duration_ms") > 0 && <span>{(num("duration_ms") / 1000).toFixed(1)}s</span>}
    </div>
  );
}

/** 上下文卡片（Q7：任务/需求绑定需在消息中体现） */
function ContextBlock({ message }: { message: SessionMessage }) {
  let info: Record<string, unknown>;
  try {
    info = JSON.parse(message.content ?? "{}");
  } catch {
    info = {};
  }
  const pairs: Array<[string, unknown]> = (
    [
      ["模式", info.mode],
      ["权限预设", info.permission_mode],
      ["模型", info.model],
      ["工作目录", info.work_dir],
      ["任务", info.task_id],
      ["需求", info.requirement_id],
      ["项目", info.project_id],
    ] as Array<[string, unknown]>
  ).filter(([, value]) => value !== null && value !== undefined && value !== "");

  return (
    <div
      className="rounded-lg px-3 py-2"
      style={{ border: "1px dashed var(--color-primary)", background: "var(--color-primary-subtle)" }}
    >
      <div className="flex items-center gap-2 mb-1.5">
        <Link2 className="w-3.5 h-3.5" style={{ color: "var(--color-primary)" }} />
        <span className="text-xs font-medium" style={{ color: "var(--color-primary)" }}>
          会话上下文
        </span>
      </div>
      <div className="grid grid-cols-2 gap-x-4 gap-y-1">
        {pairs.map(([label, value]) => (
          <div key={label} className="flex items-center justify-between gap-2 text-[11px]">
            <span style={{ color: "var(--color-text-tertiary)" }}>{label}</span>
            <span className="font-mono truncate max-w-[160px]" style={{ color: "var(--color-text-secondary)" }}>
              {String(value)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function MessageBubble({ message, onAllowAndRetry, onUseAcceptEdits, deniedBusy }: MessageBubbleProps) {
  switch (message.block_type) {
    case "thinking":
      return <ThinkingBlock message={message} />;
    case "tool_use":
      return <ToolUseBlock message={message} />;
    case "tool_result":
      return <ToolResultBlock message={message} />;
    case "permission_denied":
      return (
        <PermissionDeniedCard
          message={message}
          onAllowAndRetry={onAllowAndRetry}
          onUseAcceptEdits={onUseAcceptEdits}
          busy={deniedBusy}
        />
      );
    case "result":
      return <ResultBlock message={message} />;
    case "context":
      return <ContextBlock message={message} />;
    case "error":
    case "system":
      return (
        <p className="text-[11px] text-center" style={{ color: "var(--color-text-tertiary)" }}>
          {message.content}
        </p>
      );
    default: {
      // text（user 消息可能携带附件 marker 区块）
      const isUser = message.role === "user";
      const raw = message.content ?? "";
      const { body, paths } = parseAttachmentBlock(raw);
      return (
        <div className={`group relative ${isUser ? "flex justify-end" : "flex justify-start"}`}>
          <div
            className="max-w-[85%] rounded-xl px-3.5 py-2.5"
            style={
              isUser
                ? { background: "var(--color-primary)", color: "#fff" }
                : { background: "var(--color-bg-subtle)", color: "var(--color-text)" }
            }
          >
            {body && <RichText text={body} />}
            {!body && paths.length > 0 && (
              <p className="text-[12px] opacity-80 flex items-center gap-1">
                <Paperclip className="w-3 h-3" /> 附件
              </p>
            )}
            <AttachmentRow paths={paths} />
          </div>
          <div className="absolute -top-2 right-0">
            <CopyButton text={raw} />
          </div>
        </div>
      );
    }
  }
}
