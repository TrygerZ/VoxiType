import { create } from "zustand";
import type { Settings } from "../types/app";
import {
  formatTauriError,
  getSettings,
  setWhisperCppPaths,
  updateSetting,
} from "../lib/tauri";

interface SettingsStore {
  settings: Settings;
  loaded: boolean;
  error: string | null;
  load: () => Promise<void>;
  update: (key: string, value: unknown) => Promise<void>;
  /** Persist whisper.cpp paths via the picker-gated command. Pass null to
   *  leave a path untouched; the backend rejects non-dialog paths. */
  updateWhisperPaths: (
    binaryPath: string | null,
    modelPath: string | null,
  ) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>((set) => {
  let requestSeq = 0;

  return {
    settings: {},
    loaded: false,
    error: null,

    load: async () => {
      const seq = ++requestSeq;
      try {
        const settings = await getSettings();
        if (seq === requestSeq) {
          set({ settings, loaded: true, error: null });
        }
      } catch (err: unknown) {
        const message = formatTauriError(err);
        console.error("Failed to load settings:", err);
        if (seq === requestSeq) {
          set({ loaded: true, error: message });
        }
      }
    },

    update: async (key, value) => {
      const prev = useSettingsStore.getState().settings[key];
      set((s) => ({ settings: { ...s.settings, [key]: value } }));
      try {
        await updateSetting(key, value);
      } catch (err) {
        // rollback only if the current value is still the optimistic one we set
        if (useSettingsStore.getState().settings[key] === value) {
          set((s) => ({ settings: { ...s.settings, [key]: prev } }));
        }
        throw err;
      }
    },

    updateWhisperPaths: async (binaryPath, modelPath) => {
      const current = useSettingsStore.getState().settings;
      const prevBinary = current.whisper_cpp_binary_path;
      const prevModel = current.whisper_cpp_model_path;
      set((s) => ({
        settings: {
          ...s.settings,
          ...(binaryPath !== null ? { whisper_cpp_binary_path: binaryPath } : {}),
          ...(modelPath !== null ? { whisper_cpp_model_path: modelPath } : {}),
        },
      }));
      try {
        await setWhisperCppPaths(binaryPath, modelPath);
      } catch (err) {
        const latest = useSettingsStore.getState().settings;
        const rollback: Partial<Settings> = {};
        if (binaryPath !== null && latest.whisper_cpp_binary_path === binaryPath) {
          rollback.whisper_cpp_binary_path = prevBinary;
        }
        if (modelPath !== null && latest.whisper_cpp_model_path === modelPath) {
          rollback.whisper_cpp_model_path = prevModel;
        }
        if (Object.keys(rollback).length > 0) {
          set((s) => ({ settings: { ...s.settings, ...rollback } }));
        }
        throw err;
      }
    },
  };
});
