import { create } from "zustand";
import { getSetting, setSetting } from "../lib/tauri";

interface SettingsState {
  settings: Record<string, string>;
  loading: boolean;
  error: string | null;
  fetchSetting: (key: string) => Promise<void>;
  fetchSettings: (keys: string[]) => Promise<void>;
  saveSetting: (key: string, value: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: {},
  loading: false,
  error: null,

  fetchSetting: async (key) => {
    set({ loading: true, error: null });
    try {
      const value = await getSetting(key);
      set((state) => ({
        settings: { ...state.settings, [key]: value || "" },
        loading: false,
      }));
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  fetchSettings: async (keys) => {
    set({ loading: true, error: null });
    try {
      const entries = await Promise.all(
        keys.map(async (key) => {
          const value = await getSetting(key);
          return [key, value || ""] as const;
        }),
      );
      set((state) => ({
        settings: { ...state.settings, ...Object.fromEntries(entries) },
        loading: false,
      }));
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  saveSetting: async (key, value) => {
    await setSetting(key, value);
    set((state) => ({
      settings: { ...state.settings, [key]: value },
    }));
  },
}));
