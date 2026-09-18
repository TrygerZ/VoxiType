import "@testing-library/jest-dom/vitest";

import { vi } from "vitest";

vi.mock("../lib/tauri", () => ({
  formatTauriError: (error: unknown) => {
    if (error instanceof Error) return error.message;
    if (error && typeof error === "object" && "message" in error) {
      return String((error as { message: unknown }).message);
    }
    return String(error);
  },
  startRecording: vi.fn().mockResolvedValue(undefined),
  stopRecording: vi.fn().mockResolvedValue(undefined),
  getSettings: vi.fn().mockResolvedValue({}),
  updateSetting: vi.fn().mockResolvedValue(undefined),
  setWhisperCppPaths: vi.fn().mockResolvedValue(undefined),
  testGroqApi: vi.fn().mockResolvedValue(undefined),
  testWhisperCpp: vi.fn().mockResolvedValue(undefined),
  setHotkey: vi.fn().mockResolvedValue(undefined),
  onEvent: vi.fn().mockResolvedValue(() => undefined),
  invoke: vi.fn().mockResolvedValue(undefined),
  getDataDirectory: vi.fn().mockResolvedValue({
    active: "/default/app/data",
    default: "/default/app/data",
    pending: null,
    lastError: null,
  }),
  pickDataDirectory: vi.fn().mockResolvedValue(null),
  setDataDirectory: vi.fn().mockResolvedValue(undefined),
  restartApp: vi.fn().mockResolvedValue(undefined),
  setFloatingWidgetEnabled: vi.fn().mockResolvedValue(undefined),
  resetWidgetIdleTimer: vi.fn().mockResolvedValue(undefined),
  ackWidgetHide: vi.fn().mockResolvedValue(undefined),
  getDictionary: vi.fn().mockResolvedValue([]),
  addDictionaryWord: vi.fn().mockResolvedValue(undefined),
  deleteDictionaryWord: vi.fn().mockResolvedValue(undefined),
  setDictionaryActive: vi.fn().mockResolvedValue(undefined),
  getHistory: vi.fn().mockResolvedValue([]),
  searchHistory: vi.fn().mockResolvedValue([]),
  deleteHistory: vi.fn().mockResolvedValue(undefined),
  clearHistory: vi.fn().mockResolvedValue(undefined),
  pinHistory: vi.fn().mockResolvedValue(undefined),
  reInject: vi.fn().mockResolvedValue(undefined),
  getSnippets: vi.fn().mockResolvedValue([]),
  addSnippet: vi.fn().mockResolvedValue(undefined),
  deleteSnippet: vi.fn().mockResolvedValue(undefined),
  getUsageStats: vi.fn().mockResolvedValue({
    total_words: 0,
    total_duration_ms: 0,
    total_sessions: 0,
  }),
}));

vi.mock("../assets/icons/hourglass.svg?react", () => ({
  default: () => null,
}));
vi.mock("../assets/icons/sparkle.svg?react", () => ({
  default: () => null,
}));
vi.mock("../assets/icons/scroll-text.svg?react", () => ({
  default: () => null,
}));
