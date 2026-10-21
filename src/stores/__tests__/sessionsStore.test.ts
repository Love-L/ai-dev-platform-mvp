import { describe, it, expect, vi, beforeEach } from "vitest";
import { useSessionsStore } from "../sessionsStore";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("useSessionsStore (L1-017)", () => {
  beforeEach(() => {
    useSessionsStore.setState({ sessions: [], loading: false, error: null, showArchived: false });
  });

  it("initializes with empty state", () => {
    const state = useSessionsStore.getState();
    expect(state.sessions).toEqual([]);
  });

  it("fetchSessions loads sessions", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue([{ id: "1", status: "running" }]);

    await useSessionsStore.getState().fetchSessions();

    const state = useSessionsStore.getState();
    expect(state.sessions).toHaveLength(1);
  });

  it("fetchSessions with agentId filter", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue([{ id: "1", agent_id: "a1" }]);

    await useSessionsStore.getState().fetchSessions("a1");

    expect(invoke).toHaveBeenCalledWith("get_agent_sessions", {
      agentId: "a1",
      includeArchived: false,
    });
  });

  it("setShowArchived refetches with includeArchived=true", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue([{ id: "9", archived: 1 }]);

    await useSessionsStore.getState().setShowArchived(true);

    expect(useSessionsStore.getState().showArchived).toBe(true);
    expect(invoke).toHaveBeenCalledWith("get_agent_sessions", {
      agentId: undefined,
      includeArchived: true,
    });
  });

  it("stopSession calls IPC", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(undefined).mockResolvedValueOnce([]);

    await useSessionsStore.getState().stopSession("1");
    expect(invoke).toHaveBeenCalledWith("stop_agent", { sessionId: "1" });
  });

  it("fetchSessions handles errors", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockRejectedValue(new Error("timeout"));

    await useSessionsStore.getState().fetchSessions();

    const state = useSessionsStore.getState();
    expect(state.error).toContain("timeout");
  });
});
