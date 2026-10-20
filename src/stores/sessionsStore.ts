import { create } from "zustand";
import { getAgentSessions, stopAgent } from "../lib/tauri";
import type { AgentSession } from "../lib/types";

interface SessionsState {
  sessions: AgentSession[];
  loading: boolean;
  error: string | null;
  fetchSessions: (agentId?: string) => Promise<void>;
  stopSession: (sessionId: string) => Promise<void>;
}

export const useSessionsStore = create<SessionsState>((set, get) => ({
  sessions: [],
  loading: false,
  error: null,

  fetchSessions: async (agentId) => {
    set({ loading: true, error: null });
    try {
      const data = await getAgentSessions(agentId);
      set({ sessions: data, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  stopSession: async (sessionId) => {
    await stopAgent(sessionId);
    await get().fetchSessions();
  },
}));
