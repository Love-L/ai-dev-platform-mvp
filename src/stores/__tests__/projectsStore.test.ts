import { describe, it, expect, vi, beforeEach } from "vitest";
import { useProjectsStore } from "../projectsStore";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("useProjectsStore (L1-013)", () => {
  beforeEach(() => {
    useProjectsStore.setState({ projects: [], loading: false, error: null });
  });

  it("initializes with empty state", () => {
    const state = useProjectsStore.getState();
    expect(state.projects).toEqual([]);
  });

  it("fetchProjects loads projects", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue([{ id: "1", name: "项目A" }]);

    await useProjectsStore.getState().fetchProjects();

    const state = useProjectsStore.getState();
    expect(state.projects).toHaveLength(1);
  });

  it("fetchProjects handles errors", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockRejectedValue(new Error("network"));

    await useProjectsStore.getState().fetchProjects();

    const state = useProjectsStore.getState();
    expect(state.error).toContain("network");
  });

  it("addProject creates and refreshes", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke)
      .mockResolvedValueOnce({ id: "2", name: "新项目" })
      .mockResolvedValueOnce([{ id: "2", name: "新项目" }]);

    const result = await useProjectsStore.getState().addProject("新项目");
    expect(result).toEqual({ id: "2", name: "新项目" });
  });

  it("removeProject deletes and refreshes", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(undefined).mockResolvedValueOnce([]);

    await useProjectsStore.getState().removeProject("1");
    expect(invoke).toHaveBeenCalledWith("delete_project", { id: "1" });
  });
});
