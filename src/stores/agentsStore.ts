import { create } from 'zustand';
import {
  getAgents,
  getAgentSessions,
  discoverAgents,
  verifyAgents,
  startAgent,
  stopAgent,
} from '../lib/tauri';
import type { Agent, AgentSession } from '../lib/types';

interface AgentsState {
  agents: Agent[];
  sessions: AgentSession[];
  loading: boolean;
  /** 检测 / 复核进行中 */
  refreshing: boolean;
  error: string | null;
  /** 从数据库加载 Agent 与会话 */
  fetchAgents: () => Promise<void>;
  /** 重新检测本机已安装的 Agent */
  discover: () => Promise<void>;
  /** 状态治理：进程存活对账 + 二进制有效性复核 */
  verify: () => Promise<void>;
  /** 启动 Agent 并刷新列表 */
  start: (agentPath: string, taskId?: string) => Promise<AgentSession>;
  /** 停止会话并刷新列表 */
  stop: (sessionId: string) => Promise<void>;
}

export const useAgentsStore = create<AgentsState>((set, get) => ({
  agents: [],
  sessions: [],
  loading: false,
  refreshing: false,
  error: null,

  fetchAgents: async () => {
    set({ loading: true, error: null });
    try {
      // 进入页面即自愈：verify_agents 会纠正失效会话（进程对账）与
      // 二进制可用性，避免展示已漂移的 agents.status（如历史遗留的 running）
      const [agents, sessions] = await Promise.all([verifyAgents(), getAgentSessions()]);
      set({ agents, sessions, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  discover: async () => {
    set({ refreshing: true, error: null });
    try {
      await discoverAgents();
      // 检测结果已持久化，重新从数据库加载以保证与页面展示一致
      const [agents, sessions] = await Promise.all([getAgents(), getAgentSessions()]);
      set({ agents, sessions, refreshing: false });
    } catch (e) {
      set({ error: String(e), refreshing: false });
      throw e;
    }
  },

  verify: async () => {
    set({ refreshing: true, error: null });
    try {
      const agents = await verifyAgents();
      const sessions = await getAgentSessions();
      set({ agents, sessions, refreshing: false });
    } catch (e) {
      set({ error: String(e), refreshing: false });
      throw e;
    }
  },

  start: async (agentPath, taskId) => {
    const session = await startAgent(agentPath, taskId);
    await get().fetchAgents();
    return session;
  },

  stop: async (sessionId) => {
    await stopAgent(sessionId);
    await get().fetchAgents();
  },
}));
