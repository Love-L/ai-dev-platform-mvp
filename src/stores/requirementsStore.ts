import { create } from 'zustand';
import { listRequirements, createRequirement, deleteRequirement } from '../lib/tauri';
import type { Requirement } from '../lib/types';

interface RequirementsState {
  requirements: Requirement[];
  loading: boolean;
  error: string | null;
  fetchRequirements: (status?: string) => Promise<void>;
  addRequirement: (title: string, desc?: string, priority?: string) => Promise<Requirement>;
  removeRequirement: (id: string) => Promise<void>;
}

export const useRequirementsStore = create<RequirementsState>((set, get) => ({
  requirements: [],
  loading: false,
  error: null,

  fetchRequirements: async (status) => {
    set({ loading: true, error: null });
    try {
      const data = await listRequirements(status);
      set({ requirements: data, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  addRequirement: async (title, desc, priority) => {
    const req = await createRequirement(title, desc, priority);
    await get().fetchRequirements();
    return req;
  },

  removeRequirement: async (id) => {
    await deleteRequirement(id);
    await get().fetchRequirements();
  },
}));
