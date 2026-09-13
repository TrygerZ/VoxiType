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

export interface AppInfo {
  name: string;
  version: string;
  tauri: string;
  data_dir?: string;
  db_path?: string;
}

export interface Snippet {
  id: string;
  name: string;
  trigger_phrase: string;
  content: string;
  category?: string | null;
  mode?: string | null;
  usage_count: number;
  is_active: boolean;
}

export interface UsageStats {
  total_words: number;
  total_duration_ms: number;
  total_sessions: number;
}

export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version: string;
  notes: string;
  url: string;
}

// Settings is a flat key->value map (values are JSON).
export interface KnownSettings {
  groq_api_key?: string;
  groq_api_key_set?: boolean;
  stt_engine?: string;
  stt_language?: string;
  whisper_cpp_binary_path?: string;
  whisper_cpp_model_path?: string;
  whisper_cpp_threads?: number;
  llm_engine?: string;
  llm_model?: string;
  active_mode?: string;
  language?: string;
  sound_cues?: boolean;
  translation_enabled?: boolean;
  translation_target?: string;
  command_mode?: boolean;
  telemetry?: boolean;
  per_app_mode?: boolean;
  floating_widget?: boolean;
  floating_widget_auto_hide_seconds?: number;
  onboarding_completed?: boolean;
  mic_device?: string;
  hotkey?: unknown;
}

export type Settings = KnownSettings & Record<string, unknown>;
