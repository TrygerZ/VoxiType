// Thin typed wrappers around Tauri invoke/listen.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  DeviceInfo,
  DictionaryEntry,
  Snippet,
  TranscriptionEntry,
  UpdateInfo,
  UsageStats,
} from "../types/app";

// --- Recording ---
export const startRecording = () => invoke<void>("start_recording");
export const stopRecording = () => invoke<void>("stop_recording");

// --- Settings ---
export const getSettings = () => invoke<Record<string, unknown>>("get_settings");
export const updateSetting = (key: string, value: unknown) =>
  invoke<void>("update_setting", { key, value });
export const setFloatingWidgetEnabled = (enabled: boolean) =>
  invoke<void>("set_floating_widget_enabled", { enabled });

// --- History ---
export const getHistory = () =>
  invoke<TranscriptionEntry[]>("get_history");
export const searchHistory = (query: string) =>
  invoke<TranscriptionEntry[]>("search_history", { query });
export const deleteHistory = (id: string) =>
  invoke<void>("delete_history", { id });
export const clearHistory = (keepPinned = true) =>
  invoke<number>("clear_history", { keepPinned });
export const pinHistory = (id: string, pinned: boolean) =>
  invoke<void>("pin_history", { id, pinned });
export const reInject = (id: string) => invoke<void>("re_inject", { id });
export const exportHistory = (format: "json" | "csv") =>
  invoke<string>("export_history", { format });

// --- Dictionary ---
export const getDictionary = () =>
  invoke<DictionaryEntry[]>("get_dictionary");
export const addDictionaryWord = (entry: DictionaryEntry) =>
  invoke<void>("add_dictionary_word", { entry });
export const setDictionaryActive = (id: string, active: boolean) =>
  invoke<void>("set_dictionary_active", { id, active });
export const deleteDictionaryWord = (id: string) =>
  invoke<void>("delete_dictionary_word", { id });
export const exportDictionary = () => invoke<string>("export_dictionary");
export const importDictionary = (data: string) =>
  invoke<number>("import_dictionary", { data });

// --- Snippets ---
export const getSnippets = () => invoke<Snippet[]>("get_snippets");
export const addSnippet = (snippet: Snippet) =>
  invoke<void>("add_snippet", { snippet });
export const deleteSnippet = (id: string) =>
  invoke<void>("delete_snippet", { id });

// --- System ---
export const getMicrophones = () => invoke<DeviceInfo[]>("get_microphones");
export const getAppInfo = () => invoke<AppInfo>("get_app_info");
export const checkUpdates = () => invoke<UpdateInfo>("check_updates");

// --- Hotkey ---
export const setHotkey = (key: string, mode: string) =>
  invoke<void>("set_hotkey", { key, mode });

// --- System Utilities ---
export const openUrl = (url: string) =>
  invoke<void>("open_url", { url });

// --- Groq API ---
export const testGroqApi = (key: string) =>
  invoke<void>("test_groq_api", { apiKey: key });

// --- Stats ---
export const getUsageStats = () => invoke<UsageStats>("get_usage_stats");

// --- Events ---
export function onEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload));
}
