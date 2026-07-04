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
    const prev = useSettingsStore.getState().settings[key];
    set((s) => ({ settings: { ...s.settings, [key]: value } }));
    try {
      await updateSetting(key, value);
    } catch (err) {
      // ponytail: rollback optimistic update on backend failure
      set((s) => ({ settings: { ...s.settings, [key]: prev } }));
      throw err;
    }
  },
}));
