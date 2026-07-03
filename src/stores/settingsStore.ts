import { create } from "zustand";
import type { Settings } from "../types/app";
import { getSettings, updateSetting } from "../lib/tauri";

interface SettingsStore {
  settings: Settings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (key: string, value: unknown) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: {},
  loaded: false,

  load: async () => {
    try {
      const settings = await getSettings();
      set({ settings, loaded: true });
    } catch {
      set({ loaded: true });
    }
  },

  update: async (key, value) => {
    set((s) => ({ settings: { ...s.settings, [key]: value } }));
    await updateSetting(key, value);
  },
}));
