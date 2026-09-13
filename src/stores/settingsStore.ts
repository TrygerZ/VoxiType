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

export const useSettingsStore = create<SettingsStore>((set, get) => {
  let requestSeq = 0;

  const load = async () => {
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
  };

  return {
    settings: {},
    loaded: false,
    error: null,
    load,

    update: async (key, value) => {
      const prev = get().settings[key];
      const prevApiKey =
        typeof get().settings.groq_api_key === "string"
          ? get().settings.groq_api_key
          : "";
      const prevSet = get().settings.groq_api_key_set;
      const isApiKey = key === "groq_api_key";
      set((s) => ({
        settings: {
          ...s.settings,
          ...(isApiKey
            ? {
                groq_api_key: "",
                groq_api_key_set:
                  typeof value === "string" && value.trim().length > 0,
              }
            : { [key]: value }),
        },
      }));
      try {
        await updateSetting(key, value);
      } catch (err) {
        if (isApiKey) {
          set((s) => ({
            settings: {
              ...s.settings,
              groq_api_key: prevApiKey,
              groq_api_key_set: prevSet,
            },
          }));
        } else if (get().settings[key] === value) {
          set((s) => ({ settings: { ...s.settings, [key]: prev } }));
        }
        try {
          await load();
        } catch {
          // ignore
        }
        throw err;
      }
    },

    updateWhisperPaths: async (binaryPath, modelPath) => {
      const { whisper_cpp_binary_path: prevB, whisper_cpp_model_path: prevM } =
        get().settings;
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
        const cur = get().settings;
        const rb: Partial<Settings> = {};
        if (binaryPath !== null && cur.whisper_cpp_binary_path === binaryPath) {
          rb.whisper_cpp_binary_path = prevB;
        }
        if (modelPath !== null && cur.whisper_cpp_model_path === modelPath) {
          rb.whisper_cpp_model_path = prevM;
        }
        if (Object.keys(rb).length > 0) {
          set((s) => ({ settings: { ...s.settings, ...rb } }));
        }
        try {
          await load();
        } catch {
          // ignore
        }
        throw err;
      }
    },
  };
});
