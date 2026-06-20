// Core app state shared across frontend and backend events.

export type AppStateEnum = "idle" | "recording" | "processing" | "error";

export interface DeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
}

export interface TranscriptionEntry {
  id: string;
  created_at: string;
  text_raw: string;
  text_formatted: string;
  source_lang: string;
  target_lang?: string | null;
  mode: string;
  stt_engine: string;
  stt_confidence?: number | null;
  llm_engine?: string | null;
  duration_ms?: number | null;
  word_count: number;
  character_count: number;
  is_pinned: boolean;
  app_context?: string | null;
}

export interface DictionaryEntry {
  id: string;
  word: string;
  pronunciation?: string | null;
  category: string;
  replacement?: string | null;
  language: string;
  usage_count: number;
  is_active: boolean;
}

export interface HistoryFilter {
  mode?: string;
  source_lang?: string;
  pinned_only?: boolean;
  limit?: number;
  offset?: number;
}

export interface DictFilter {
  category?: string;
  language?: string;
}

export interface AppInfo {
  name: string;
  version: string;
  tauri: string;
}

// Settings is a flat key->value map (values are JSON).
export type Settings = Record<string, unknown>;
