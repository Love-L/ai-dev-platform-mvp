import { useEffect, useRef, useState } from "react";
import { Brain, Loader2 } from "lucide-react";
import type { ReactNode } from "react";
import { useSessionMessagesStore } from "../stores/sessionMessagesStore";
import MessageBubble from "./MessageBubble";

interface SessionChatProps {
  sessionId: string;
  /** 会话进程是否在运行（决定是否展示流式指示与允许输入） */
  running: boolean;
  onAllowAndRetry?: (toolName?: string) => void;
  onUseAcceptEdits?: () => void;
  /** 被拒卡片操作进行中（禁用按钮） */
  deniedBusy?: boolean;
  /** 空会话时的引导区（冷启动任务卡等），缺省展示默认文案 */
  emptyState?: ReactNode;
}

/**
 * 会话消息视图（模块 16 / F076–F080）。
 *
 * - 落库消息按 `seq` 有序渲染；
 * - 增量文本/思考由 store 缓冲实时展示，落库后自动清空；
 * - 用户上滚时暂停自动到底部，并显示「回到最新」。
 */
export default function SessionChat({
  sessionId,
  running,
  onAllowAndRetry,
  onUseAcceptEdits,
  deniedBusy,
  emptyState,
}: SessionChatProps) {
  const {
    messages,
    streamingText,
    streamingThinking,
    thinkingTokens,
    loading,
    hasMore,
    error,
    subscribe,
    loadOlder,
    reset,
  } = useSessionMessagesStore();

  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    subscribe(sessionId).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });

    return () => {
      disposed = true;
      unlisten?.();
      reset();
    };
  }, [sessionId, subscribe, reset]);

  // 自动滚动到底部（用户上滚时暂停）
  useEffect(() => {
    if (!autoScroll) return;
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages, streamingText, streamingThinking, autoScroll]);

  function handleScroll() {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    setAutoScroll(atBottom);
  }

  const showThinkingStream = running && (streamingThinking.length > 0 || thinkingTokens > 0);

  return (
    <div className="relative h-full">
      <div ref={scrollRef} onScroll={handleScroll} className="h-full overflow-y-auto px-4 py-3 space-y-2">
        {hasMore && (
          <div className="flex justify-center">
            <button
              type="button"
              onClick={loadOlder}
              className="text-xs px-3 py-1 rounded-full"
              style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-secondary)" }}
            >
              加载更早的消息
            </button>
          </div>
        )}

        {loading && (
          <div
            className="flex items-center justify-center gap-2 py-6 text-xs"
            style={{ color: "var(--color-text-tertiary)" }}
          >
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
            加载会话消息…
          </div>
        )}

        {!loading && messages.length === 0 && !streamingText && !showThinkingStream && (
          <div className="text-center py-10">
            {emptyState ?? (
              <>
                <p className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
                  暂无消息
                </p>
                <p className="text-xs mt-1" style={{ color: "var(--color-text-tertiary)" }}>
                  在下方输入指令开始与 Agent 对话
                </p>
              </>
            )}
          </div>
        )}

        {messages.map((message) => (
          <MessageBubble
            key={message.id}
            message={message}
            onAllowAndRetry={onAllowAndRetry}
            onUseAcceptEdits={onUseAcceptEdits}
            deniedBusy={deniedBusy}
          />
        ))}

        {/* 思考增量（折叠态仅显示计数，避免抖动） */}
        {showThinkingStream && (
          <details className="rounded-lg px-3 py-2" style={{ border: "1px solid var(--color-border-subtle)" }}>
            <summary
              className="text-xs flex items-center gap-2 cursor-pointer"
              style={{ color: "var(--color-text-secondary)" }}
            >
              <Brain className="w-3.5 h-3.5" style={{ color: "var(--color-primary)" }} />
              思考中…{thinkingTokens > 0 ? `（约 ${thinkingTokens} tokens）` : ""}
            </summary>
            {streamingThinking && (
              <div
                className="whitespace-pre-wrap break-words text-[12px] leading-relaxed mt-2"
                style={{ color: "var(--color-text-tertiary)" }}
              >
                {streamingThinking}
              </div>
            )}
          </details>
        )}

        {/* 正文增量 */}
        {streamingText && (
          <div className="flex justify-start">
            <div
              className="max-w-[85%] rounded-xl px-3.5 py-2.5"
              style={{ background: "var(--color-bg-subtle)", color: "var(--color-text)" }}
            >
              <div className="whitespace-pre-wrap break-words text-sm leading-relaxed">
                {streamingText}
                <span className="animate-pulse" style={{ color: "var(--color-primary)" }}>
                  ▍
                </span>
              </div>
            </div>
          </div>
        )}

        {error && (
          <p className="text-center text-[11px]" style={{ color: "var(--color-danger)" }}>
            {error}
          </p>
        )}
      </div>

      {!autoScroll && (
        <button
          type="button"
          onClick={() => {
            setAutoScroll(true);
            const el = scrollRef.current;
            if (el) el.scrollTop = el.scrollHeight;
          }}
          className="absolute bottom-3 right-4 text-xs px-3 py-1.5 rounded-full shadow"
          style={{ background: "var(--color-primary)", color: "#fff" }}
        >
          ↓ 回到最新
        </button>
      )}
    </div>
  );
}
