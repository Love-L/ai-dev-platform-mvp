import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { listSessionMessages, sendSessionMessage, interruptSession } from "../lib/tauri";
import type { SessionMessage, SessionDelta, SessionNotice } from "../lib/types";

const PAGE_SIZE = 100;

interface SessionMessagesState {
  /** 当前订阅的会话 id */
  sessionId: string | null;
  messages: SessionMessage[];
  /** 正文增量缓冲（收到落库消息后清空） */
  streamingText: string;
  /** 思考增量缓冲 */
  streamingThinking: string;
  /** 思考 Token 估算（折叠态展示） */
  thinkingTokens: number;
  loading: boolean;
  sending: boolean;
  hasMore: boolean;
  error: string | null;

  /** 订阅会话事件并加载首页消息，返回取消订阅函数 */
  subscribe: (sessionId: string) => Promise<() => void>;
  loadOlder: () => Promise<void>;
  send: (content: string) => Promise<void>;
  interrupt: () => Promise<void>;
  /** 供页面在会话重建后刷新 */
  reload: () => Promise<void>;
  reset: () => void;
}

/**
 * 会话消息 store（模块 16 / F076–F079）。
 *
 * 设计：
 * - 增量（`session:delta`）只进内存缓冲，用于实时渲染；
 * - 完整块（`session:message`）落库后按 `seq` 有序插入，并**清空对应缓冲**，保证幂等；
 * - 每次进入会话先全量拉取最近一页，用于校准可能丢失的事件。
 */
export const useSessionMessagesStore = create<SessionMessagesState>((set, get) => ({
  sessionId: null,
  messages: [],
  streamingText: "",
  streamingThinking: "",
  thinkingTokens: 0,
  loading: false,
  sending: false,
  hasMore: false,
  error: null,

  subscribe: async (sessionId: string) => {
    set({
      sessionId,
      messages: [],
      streamingText: "",
      streamingThinking: "",
      thinkingTokens: 0,
      error: null,
      loading: true,
    });

    const unlistenMessage = await listen<{ session_id: string; message: SessionMessage }>(
      "session:message",
      (event) => {
        if (event.payload.session_id !== sessionId) return;
        const msg = event.payload.message;
        set((state) => {
          if (state.messages.some((m) => m.id === msg.id)) return state;
          const next = [...state.messages, msg].sort((a, b) => a.seq - b.seq);
          return {
            messages: next,
            // 完整块到达后清空对应增量缓冲
            streamingText: msg.block_type === "text" ? "" : state.streamingText,
            streamingThinking: msg.block_type === "thinking" ? "" : state.streamingThinking,
          };
        });
      },
    );

    const unlistenDelta = await listen<SessionDelta>("session:delta", (event) => {
      if (event.payload.session_id !== sessionId) return;
      const { block_type, delta, total } = event.payload;
      if (block_type === "thinking_tokens") {
        set({ thinkingTokens: total ?? (Number(delta) || 0) });
        return;
      }
      if (block_type === "thinking") {
        set((state) => ({ streamingThinking: state.streamingThinking + delta }));
        return;
      }
      set((state) => ({ streamingText: state.streamingText + delta }));
    });

    const unlistenNotice = await listen<SessionNotice>("session:notice", (event) => {
      if (event.payload.session_id !== sessionId) return;
      if (event.payload.level === "info") return;
      set({ error: event.payload.message });
    });

    await get().reload();

    return () => {
      unlistenMessage();
      unlistenDelta();
      unlistenNotice();
    };
  },

  reload: async () => {
    const sessionId = get().sessionId;
    if (!sessionId) return;
    try {
      const page = await listSessionMessages(sessionId, undefined, PAGE_SIZE);
      set({
        messages: page.items,
        hasMore: page.items.length < page.total,
        loading: false,
        error: null,
      });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  loadOlder: async () => {
    const { sessionId, messages } = get();
    if (!sessionId || messages.length === 0) return;
    const beforeSeq = messages[0].seq;
    try {
      const page = await listSessionMessages(sessionId, beforeSeq, PAGE_SIZE);
      set((state) => ({
        messages: [...page.items, ...state.messages].sort((a, b) => a.seq - b.seq),
        hasMore: page.items.length > 0,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  send: async (content: string) => {
    const sessionId = get().sessionId;
    if (!sessionId || !content.trim()) return;
    set({ sending: true, error: null });
    try {
      await sendSessionMessage(sessionId, content);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    } finally {
      set({ sending: false });
    }
  },

  interrupt: async () => {
    const sessionId = get().sessionId;
    if (!sessionId) return;
    try {
      await interruptSession(sessionId);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  reset: () =>
    set({
      sessionId: null,
      messages: [],
      streamingText: "",
      streamingThinking: "",
      thinkingTokens: 0,
      loading: false,
      sending: false,
      hasMore: false,
      error: null,
    }),
}));
