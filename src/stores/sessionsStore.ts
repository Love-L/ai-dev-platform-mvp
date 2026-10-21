import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { getAgentSessions, stopAgent } from "../lib/tauri";
import type { AgentSession } from "../lib/types";

interface SessionsState {
  sessions: AgentSession[];
  loading: boolean;
  error: string | null;
  /** 列表是否包含已归档会话（默认隐藏） */
  showArchived: boolean;
  fetchSessions: (agentId?: string) => Promise<void>;
  /** 切换"显示已归档"并立即刷新 */
  setShowArchived: (v: boolean) => Promise<void>;
  stopSession: (sessionId: string) => Promise<void>;
  /**
   * 订阅 `session:status` 事件实现自动刷新（模块级单例，多页面复用同一监听）。
   * 返回释放函数；最后一个订阅者释放时才真正取消监听。
   */
  ensureStatusSubscription: () => Promise<() => void>;
}

/** 全局单例监听（避免多组件重复注册） */
let statusUnlisten: (() => void) | null = null;
let statusRefCount = 0;

export const useSessionsStore = create<SessionsState>((set, get) => ({
  sessions: [],
  loading: false,
  error: null,
  showArchived: false,

  fetchSessions: async (agentId) => {
    set({ loading: true, error: null });
    try {
      const data = await getAgentSessions(agentId, get().showArchived);
      set({ sessions: data, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setShowArchived: async (v) => {
    set({ showArchived: v });
    await get().fetchSessions();
  },

  stopSession: async (sessionId) => {
    await stopAgent(sessionId);
    await get().fetchSessions();
  },

  ensureStatusSubscription: async () => {
    if (!statusUnlisten) {
      statusUnlisten = await listen<{ session_id: string; status: string }>("session:status", () => {
        // 任何生命周期变化（running/completed/failed/deleted）都刷新列表
        void get().fetchSessions();
      });
    }
    statusRefCount += 1;
    let released = false;
    return () => {
      if (released) return;
      released = true;
      statusRefCount -= 1;
      if (statusRefCount <= 0 && statusUnlisten) {
        statusUnlisten();
        statusUnlisten = null;
      }
    };
  },
}));
