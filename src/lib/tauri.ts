// Thin typed wrappers around Tauri invoke/listen.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  DeviceInfo,
  DictFilter,
  DictionaryEntry,
  HistoryFilter,
  Settings,
  TranscriptionEntry,
} from "../types/app";

// --- Recording ---
export const startRecording = () => invoke<void>("start_recording");
export const stopRecording = () => invoke<void>("stop_recording");
export const cancelRecording = () => invoke<void>("cancel_recording");
export const getState = () => invoke<string>("get_state");
export const getAudioLevel = () => invoke<number>("get_audio_level");

// --- Settings ---
export const getSettings = () => invoke<Settings>("get_settings");
export const updateSetting = (key: string, value: unknown) =>
  invoke<void>("update_setting", { key, value });

// --- History ---
export const getHistory = (filter?: HistoryFilter) =>
  invoke<TranscriptionEntry[]>("get_history", { filter });
export const searchHistory = (query: string) =>
  invoke<TranscriptionEntry[]>("search_history", { query });
export const deleteHistory = (id: string) =>
  invoke<void>("delete_history", { id });
export const pinHistory = (id: string, pinned: boolean) =>
  invoke<void>("pin_history", { id, pinned });
export const reInject = (id: string) => invoke<void>("re_inject", { id });

// --- Dictionary ---
export const getDictionary = (filter?: DictFilter) =>
  invoke<DictionaryEntry[]>("get_dictionary", { filter });
export const addDictionaryWord = (entry: DictionaryEntry) =>
  invoke<void>("add_dictionary_word", { entry });
export const deleteDictionaryWord = (id: string) =>
  invoke<void>("delete_dictionary_word", { id });

// --- System ---
export const getMicrophones = () => invoke<DeviceInfo[]>("get_microphones");
export const getAppInfo = () => invoke<AppInfo>("get_app_info");

// --- Events ---
export function onEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload));
}
