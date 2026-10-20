import { create } from "zustand";
import { listProjects, createProject, deleteProject } from "../lib/tauri";
import type { Project } from "../lib/types";

interface ProjectsState {
  projects: Project[];
  loading: boolean;
  error: string | null;
  fetchProjects: () => Promise<void>;
  addProject: (name: string, description?: string, key?: string) => Promise<Project>;
  removeProject: (id: string) => Promise<void>;
}

export const useProjectsStore = create<ProjectsState>((set, get) => ({
  projects: [],
  loading: false,
  error: null,

  fetchProjects: async () => {
    set({ loading: true, error: null });
    try {
      const data = await listProjects();
      set({ projects: data, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  addProject: async (name, description, key) => {
    const project = await createProject(name, description, key);
    await get().fetchProjects();
    return project;
  },

  removeProject: async (id) => {
    await deleteProject(id);
    await get().fetchProjects();
  },
}));
